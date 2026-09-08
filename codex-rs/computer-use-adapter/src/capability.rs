//! Capability identity and the capability lifecycle state machine.
//!
//! The lifecycle is the WO-007 contract for binding workflow execution to the
//! native Codex Computer Use capability:
//! `declared -> available -> ready -> authorized -> bound -> executing`, with
//! settling (`executing -> bound`), release (`bound -> authorized`),
//! readiness regression after bridge loss (`bound -> ready`), failure from
//! any live state, and reset (`failed -> declared`). Every applied transition
//! is appended to an audit trail with a monotonic sequence number.

use serde::Deserialize;
use serde::Serialize;

use crate::bridge::BridgeIdentity;
use crate::error::ComputerUseAdapterError;
use crate::session::ComputerUseSessionId;
use crate::session::OwnerToken;

/// Canonical capability identifier bound by this adapter.
pub const COMPUTER_USE_CAPABILITY_ID: &str = "codex.computer-use";

/// Adapter identity recorded in bindings (crate name and version).
pub const ADAPTER_IDENTITY: &str =
    concat!("codex-computer-use-adapter@", env!("CARGO_PKG_VERSION"));

/// Lifecycle state of the computer use capability.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CapabilityLifecycleState {
    /// The capability requirement is declared (adapter construction state).
    Declared,
    /// The native Codex Computer Use surface is present in this build.
    Available,
    /// The bridge/runtime is provisioned and passed its readiness probe.
    Ready,
    /// Policy and host approvals cover the declared targets.
    Authorized,
    /// A session is bound to an owner with recorded binding identity.
    Bound,
    /// A step is executing on the bound session.
    Executing,
    /// Terminal failure; `CapabilityLifecycle::reset` returns to `Declared`.
    Failed,
}

/// One recorded lifecycle transition.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LifecycleTransition {
    /// Monotonic sequence number, starting at 1.
    pub seq: u64,
    /// State before the transition.
    pub from: CapabilityLifecycleState,
    /// State after the transition.
    pub to: CapabilityLifecycleState,
    /// Unix milliseconds of the transition (adapter clock).
    pub at_unix_ms: u64,
    /// Optional note recorded with the transition.
    pub note: Option<String>,
}

/// Lifecycle state machine with an append-only audit trail.
#[derive(Debug)]
pub struct CapabilityLifecycle {
    state: CapabilityLifecycleState,
    next_seq: u64,
    history: Vec<LifecycleTransition>,
}

impl CapabilityLifecycle {
    /// Creates the lifecycle in the `Declared` state.
    pub fn new() -> Self {
        Self {
            state: CapabilityLifecycleState::Declared,
            next_seq: 1,
            history: Vec::new(),
        }
    }

    /// Current state.
    pub fn state(&self) -> CapabilityLifecycleState {
        self.state
    }

    /// Append-only transition audit trail.
    pub fn history(&self) -> &[LifecycleTransition] {
        &self.history
    }

    /// Applies `to` when the edge is legal; otherwise returns
    /// [`ComputerUseAdapterError::InvalidLifecycleTransition`] and leaves the
    /// state unchanged.
    pub fn transition(
        &mut self,
        to: CapabilityLifecycleState,
        at_unix_ms: u64,
        note: Option<String>,
    ) -> Result<LifecycleTransition, ComputerUseAdapterError> {
        if !Self::is_legal(self.state, to) {
            return Err(ComputerUseAdapterError::InvalidLifecycleTransition {
                from: self.state,
                to,
            });
        }
        let transition = LifecycleTransition {
            seq: self.next_seq,
            from: self.state,
            to,
            at_unix_ms,
            note,
        };
        self.next_seq += 1;
        self.state = to;
        self.history.push(transition.clone());
        Ok(transition)
    }

    /// Marks the capability failed from any non-terminal state.
    pub fn fail(
        &mut self,
        at_unix_ms: u64,
        note: Option<String>,
    ) -> Result<LifecycleTransition, ComputerUseAdapterError> {
        self.transition(CapabilityLifecycleState::Failed, at_unix_ms, note)
    }

    /// Resets a failed lifecycle back to `Declared`.
    pub fn reset(
        &mut self,
        at_unix_ms: u64,
        note: Option<String>,
    ) -> Result<LifecycleTransition, ComputerUseAdapterError> {
        self.transition(CapabilityLifecycleState::Declared, at_unix_ms, note)
    }

    fn is_legal(from: CapabilityLifecycleState, to: CapabilityLifecycleState) -> bool {
        if to == CapabilityLifecycleState::Failed {
            return from != CapabilityLifecycleState::Failed;
        }
        matches!(
            (from, to),
            (
                CapabilityLifecycleState::Declared,
                CapabilityLifecycleState::Available
            ) | (
                CapabilityLifecycleState::Available,
                CapabilityLifecycleState::Ready
            ) | (
                CapabilityLifecycleState::Ready,
                CapabilityLifecycleState::Authorized
            ) | (
                CapabilityLifecycleState::Authorized,
                CapabilityLifecycleState::Bound
            ) | (
                CapabilityLifecycleState::Bound,
                CapabilityLifecycleState::Executing
            ) | (
                CapabilityLifecycleState::Executing,
                CapabilityLifecycleState::Bound
            ) | (
                CapabilityLifecycleState::Bound,
                CapabilityLifecycleState::Authorized
            ) | (
                CapabilityLifecycleState::Bound,
                CapabilityLifecycleState::Ready
            ) | (
                CapabilityLifecycleState::Ready,
                CapabilityLifecycleState::Available
            ) | (
                CapabilityLifecycleState::Failed,
                CapabilityLifecycleState::Declared
            )
        )
    }
}

impl Default for CapabilityLifecycle {
    fn default() -> Self {
        Self::new()
    }
}

/// Exact binding identity recorded when a session is bound.
///
/// This is the auditable tuple required by WO-007: canonical capability id,
/// adapter identity, provisioned bridge identity, policy digest in force,
/// session id, owner, and bind timestamp.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CapabilityBinding {
    /// Canonical capability identifier.
    pub capability_id: String,
    /// Adapter identity (crate name and version).
    pub adapter_identity: String,
    /// Identity of the provisioned native bridge.
    pub bridge: BridgeIdentity,
    /// Digest of the policy in force at bind time.
    pub policy_digest: String,
    /// The bound session.
    pub session_id: ComputerUseSessionId,
    /// Owning owner token.
    pub owner: OwnerToken,
    /// Unix milliseconds of the binding.
    pub bound_at_unix_ms: u64,
}

#[cfg(test)]
mod tests {
    use super::CapabilityLifecycle;
    use super::CapabilityLifecycleState as S;
    use crate::ComputerUseAdapterError;

    #[test]
    fn declared_through_executing_ladder() {
        let mut lifecycle = CapabilityLifecycle::new();
        assert_eq!(lifecycle.state(), S::Declared);
        for expected in [
            S::Available,
            S::Ready,
            S::Authorized,
            S::Bound,
            S::Executing,
        ] {
            lifecycle
                .transition(expected, 0, None)
                .expect("legal ladder step");
        }
        assert_eq!(lifecycle.state(), S::Executing);
        assert_eq!(lifecycle.history().len(), 5);
        assert_eq!(lifecycle.history()[0].seq, 1);
        assert_eq!(lifecycle.history()[4].seq, 5);
        assert_eq!(lifecycle.history()[4].from, S::Bound);
        assert_eq!(lifecycle.history()[4].to, S::Executing);
    }

    #[test]
    fn skipped_transitions_are_rejected() {
        let mut lifecycle = CapabilityLifecycle::new();
        let error = lifecycle
            .transition(S::Ready, 0, None)
            .expect_err("skipped transitions must fail");
        assert!(matches!(
            error,
            ComputerUseAdapterError::InvalidLifecycleTransition { .. }
        ));
        assert_eq!(
            lifecycle.state(),
            S::Declared,
            "state unchanged on rejection"
        );
    }

    #[test]
    fn settling_release_regression_failure_and_reset() {
        let mut lifecycle = CapabilityLifecycle::new();
        for expected in [
            S::Available,
            S::Ready,
            S::Authorized,
            S::Bound,
            S::Executing,
        ] {
            lifecycle.transition(expected, 0, None).expect("ladder");
        }
        lifecycle
            .transition(S::Bound, 1, None)
            .expect("executing settles to bound");
        lifecycle
            .transition(S::Authorized, 2, None)
            .expect("bound releases to authorized");
        lifecycle.transition(S::Bound, 3, None).expect("re-bind");
        lifecycle
            .transition(S::Ready, 4, None)
            .expect("bound regresses to ready after bridge loss");
        lifecycle
            .fail(5, Some("bridge unavailable".to_string()))
            .expect("fail from ready");
        assert_eq!(lifecycle.state(), S::Failed);
        lifecycle
            .transition(S::Failed, 6, None)
            .expect_err("failed is terminal");
        lifecycle.reset(7, None).expect("failed resets to declared");
        assert_eq!(lifecycle.state(), S::Declared);
    }

    #[test]
    fn fail_is_rejected_from_failed_state() {
        let mut lifecycle = CapabilityLifecycle::new();
        lifecycle.fail(0, None).expect("fail from declared");
        lifecycle.fail(1, None).expect_err("already failed");
    }
}
