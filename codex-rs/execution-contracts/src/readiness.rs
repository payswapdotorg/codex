//! Capability binding readiness.
//!
//! The frozen architecture defines an explicit readiness lifecycle for
//! every environment/capability binding:
//!
//! ```text
//! DECLARED -> AVAILABLE -> READY -> AUTHORIZED -> BOUND -> EXECUTING
//! ```
//!
//! with `FAILED` and `UNAVAILABLE` as diagnosable failure states. The
//! dispatch-policy invariant *capability availability does not imply
//! authorization* is structural: `AVAILABLE`, `READY`, and `AUTHORIZED`
//! are distinct states reached through distinct, separately evidenced
//! transitions. A binding can only execute after every gate has been
//! passed in order.
//!
//! Transitions are cause-driven: each [`TransitionCause`] targets exactly
//! one state, so an illegal transition is always a diagnosable contract
//! violation instead of an arbitrary jump. Every transition is recorded
//! in an append-only history, making readiness observable and auditable.
//!
//! The healthy path is strictly forward; regressions travel through
//! `FAILED` or `UNAVAILABLE` so that every loss of readiness carries a
//! diagnostic.

use serde::Deserialize;
use serde::Serialize;
use std::fmt;

use crate::AuthorizationGrant;
use crate::BindingDiagnostic;
use crate::ExecutionContractError;

/// Lifecycle state of a capability binding.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ReadinessState {
    /// The binding is registered but not yet evaluated.
    Declared,
    /// The capability's implementation is installed and discoverable.
    Available,
    /// The binding is provisioned and usable (probe succeeded).
    Ready,
    /// Policy and approval allow the binding to be used.
    Authorized,
    /// Required resources are attached.
    Bound,
    /// An action is in flight on the binding.
    Executing,
    /// A failure occurred; the binding carries a diagnostic.
    Failed,
    /// The binding cannot currently serve at all; carries a diagnostic.
    Unavailable,
}

impl fmt::Display for ReadinessState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl ReadinessState {
    /// The canonical wire name, matching the serde form.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Declared => "declared",
            Self::Available => "available",
            Self::Ready => "ready",
            Self::Authorized => "authorized",
            Self::Bound => "bound",
            Self::Executing => "executing",
            Self::Failed => "failed",
            Self::Unavailable => "unavailable",
        }
    }

    /// Every readiness state defined by the frozen architecture.
    pub const ALL: [Self; 8] = [
        Self::Declared,
        Self::Available,
        Self::Ready,
        Self::Authorized,
        Self::Bound,
        Self::Executing,
        Self::Failed,
        Self::Unavailable,
    ];

    /// Whether the state is one of the two diagnosable failure states.
    pub const fn is_diagnostic(self) -> bool {
        matches!(self, Self::Failed | Self::Unavailable)
    }

    /// Whether a binding in this state may be selected by resolution.
    ///
    /// Selectable states are the ones that have passed every readiness
    /// gate: `READY` or later, excluding the failure states. `EXECUTING`
    /// remains selectable so parallel steps can dispatch additional
    /// actions to an already-busy binding.
    pub const fn is_selectable(self) -> bool {
        matches!(
            self,
            Self::Ready | Self::Authorized | Self::Bound | Self::Executing
        )
    }
}

/// The cause of one readiness transition.
///
/// Every cause targets exactly one state (see [`TransitionCause::target`]),
/// and each cause is only legal from a fixed set of source states. This
/// makes the state machine total and diagnosable: applying a cause in the
/// wrong state is a contract violation that names both the state and the
/// cause.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub enum TransitionCause {
    /// Adapter registration accepted the binding.
    Registered,
    /// An adapter probe confirmed the binding is provisioned.
    ProbeSucceeded,
    /// An adapter probe reported the capability is not installed.
    CapabilityNotInstalled(BindingDiagnostic),
    /// An adapter probe errored while evaluating the binding.
    ProbeError(BindingDiagnostic),
    /// An authorization grant was accepted for the binding.
    Authorized(AuthorizationGrant),
    /// All required resources were attached to the binding.
    ResourcesAttached,
    /// An action was dispatched to the binding.
    ActionDispatched,
    /// The dispatched action completed.
    ActionCompleted,
    /// The dispatched action failed.
    ActionFailed(BindingDiagnostic),
    /// Session preparation failed before dispatch.
    PreparationFailed(BindingDiagnostic),
    /// The environment or session was lost.
    EnvironmentLost(BindingDiagnostic),
    /// Recovery re-established readiness after a failure.
    Recovered,
    /// The capability became available again after being unavailable.
    Rediscovered,
}

impl TransitionCause {
    /// The state this cause always transitions to.
    pub const fn target(&self) -> ReadinessState {
        match self {
            Self::Registered => ReadinessState::Available,
            Self::ProbeSucceeded | Self::Recovered => ReadinessState::Ready,
            Self::CapabilityNotInstalled(_) | Self::EnvironmentLost(_) => {
                ReadinessState::Unavailable
            }
            Self::ProbeError(_) | Self::ActionFailed(_) | Self::PreparationFailed(_) => {
                ReadinessState::Failed
            }
            Self::Authorized(_) => ReadinessState::Authorized,
            Self::ResourcesAttached => ReadinessState::Bound,
            Self::ActionDispatched => ReadinessState::Executing,
            Self::ActionCompleted => ReadinessState::Bound,
            Self::Rediscovered => ReadinessState::Available,
        }
    }

    /// The source states this cause is legal from.
    pub fn legal_from(&self) -> &'static [ReadinessState] {
        match self {
            Self::Registered => &[ReadinessState::Declared],
            Self::ProbeSucceeded => &[ReadinessState::Available, ReadinessState::Failed],
            Self::CapabilityNotInstalled(_) => &[
                ReadinessState::Declared,
                ReadinessState::Available,
                ReadinessState::Failed,
                ReadinessState::Unavailable,
            ],
            Self::ProbeError(_) => &[
                ReadinessState::Declared,
                ReadinessState::Available,
                ReadinessState::Failed,
                ReadinessState::Unavailable,
            ],
            Self::Authorized(_) => &[ReadinessState::Ready],
            Self::ResourcesAttached => &[ReadinessState::Authorized],
            Self::ActionDispatched => &[ReadinessState::Bound, ReadinessState::Executing],
            Self::ActionCompleted => &[ReadinessState::Executing],
            Self::ActionFailed(_) => &[ReadinessState::Executing],
            Self::PreparationFailed(_) => &[ReadinessState::Bound, ReadinessState::Executing],
            Self::EnvironmentLost(_) => &[
                ReadinessState::Available,
                ReadinessState::Ready,
                ReadinessState::Authorized,
                ReadinessState::Bound,
                ReadinessState::Executing,
            ],
            Self::Recovered => &[ReadinessState::Failed],
            Self::Rediscovered => &[ReadinessState::Failed, ReadinessState::Unavailable],
        }
    }

    /// The diagnostic this cause carries, when it has one.
    ///
    /// Causes that transition into a failure state are required to carry a
    /// diagnostic.
    pub fn diagnostic(&self) -> Option<&BindingDiagnostic> {
        match self {
            Self::CapabilityNotInstalled(diagnostic)
            | Self::ProbeError(diagnostic)
            | Self::ActionFailed(diagnostic)
            | Self::PreparationFailed(diagnostic)
            | Self::EnvironmentLost(diagnostic) => Some(diagnostic),
            _ => None,
        }
    }

    /// The cause's name, stable for error reporting.
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Registered => "registered",
            Self::ProbeSucceeded => "probeSucceeded",
            Self::CapabilityNotInstalled(_) => "capabilityNotInstalled",
            Self::ProbeError(_) => "probeError",
            Self::Authorized(_) => "authorized",
            Self::ResourcesAttached => "resourcesAttached",
            Self::ActionDispatched => "actionDispatched",
            Self::ActionCompleted => "actionCompleted",
            Self::ActionFailed(_) => "actionFailed",
            Self::PreparationFailed(_) => "preparationFailed",
            Self::EnvironmentLost(_) => "environmentLost",
            Self::Recovered => "recovered",
            Self::Rediscovered => "rediscovered",
        }
    }
}

/// One recorded readiness transition.
///
/// Transitions are append-only history entries: they capture where the
/// binding was, where it moved, and why, so readiness is observable and
/// auditable after the fact.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReadinessTransition {
    /// The state the binding transitioned from.
    pub from: ReadinessState,
    /// The state the binding transitioned to.
    pub to: ReadinessState,
    /// Why the transition happened.
    pub cause: TransitionCause,
}

/// Tracks the readiness of one capability binding.
///
/// The tracker owns the binding's position in the readiness state machine,
/// its current diagnostic (when failed or unavailable), and the
/// append-only transition history. It is the single authority for state
/// changes: callers propose causes, the tracker validates legality.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReadinessTracker {
    state: ReadinessState,
    diagnostic: Option<BindingDiagnostic>,
    history: Vec<ReadinessTransition>,
}

impl Default for ReadinessTracker {
    fn default() -> Self {
        Self {
            state: ReadinessState::Declared,
            diagnostic: None,
            history: Vec::new(),
        }
    }
}

impl ReadinessTracker {
    /// Creates a tracker in the `DECLARED` state with empty history.
    pub fn new() -> Self {
        Self::default()
    }

    /// The binding's current readiness state.
    pub const fn state(&self) -> ReadinessState {
        self.state
    }

    /// The binding's current diagnostic, when it is failed or unavailable.
    pub fn diagnostic(&self) -> Option<&BindingDiagnostic> {
        self.diagnostic.as_ref()
    }

    /// The append-only transition history, oldest first.
    pub fn history(&self) -> &[ReadinessTransition] {
        self.history.as_slice()
    }

    /// Applies a transition cause, validating legality and diagnostics.
    ///
    /// Returns the new state, or an error naming the current state and the
    /// rejected cause. Entering `FAILED` or `UNAVAILABLE` always carries a
    /// diagnostic: the five failure-bound causes embed one structurally, so
    /// the invariant is guaranteed by the type system. Healthy-state
    /// transitions clear the diagnostic.
    pub fn advance(
        &mut self,
        cause: TransitionCause,
    ) -> Result<ReadinessState, ExecutionContractError> {
        let target = cause.target();
        if !cause.legal_from().contains(&self.state) {
            return Err(ExecutionContractError::IllegalTransition {
                current: self.state,
                cause: cause.name(),
            });
        }
        let diagnostic = if target.is_diagnostic() {
            cause.diagnostic().cloned()
        } else {
            None
        };
        self.history.push(ReadinessTransition {
            from: self.state,
            to: target,
            cause,
        });
        self.state = target;
        self.diagnostic = diagnostic;
        Ok(target)
    }
}

#[cfg(test)]
#[path = "readiness_tests.rs"]
mod tests;
