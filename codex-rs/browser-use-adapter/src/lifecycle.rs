//! Capability lifecycle state machine.
//!
//! Models the WO-006 lifecycle `declared -> available -> ready ->
//! authorized -> bound -> executing` with explicit interruption, recovery,
//! and take-over paths, plus a terminal released state. The machine is the
//! adapter's only authority over capability readiness; every transition is
//! recorded and exportable as evidence, and no transition ever mutates
//! workflow semantics.

use crate::bridge::EvidenceRecord;
use crate::error::AdapterError;
use codex_workflow_contracts::EvidenceKind;
use serde::{Deserialize, Serialize};

/// Lifecycle states of the bound capability.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CapabilityLifecycleState {
    /// The workflow declares browser requirements (initial state).
    Declared,
    /// A native (or recorded fallback) capability is present.
    Available,
    /// The runtime is provisioned and configuration is resolved.
    Ready,
    /// The requirement-vs-policy pre-flight passed.
    Authorized,
    /// Bound to a concrete browser session identity.
    Bound,
    /// Executing.
    Executing,
    /// Interrupted; legal recovery depends on the interruption kind.
    Interrupted,
    /// Terminal: the binding was released.
    Released,
}

/// Why an executing capability was interrupted.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum InterruptionKind {
    /// The native runtime disappeared mid-flight.
    RuntimeLost,
    /// Authorization expired; re-authorization is required.
    AuthorizationExpired,
    /// The bound browser session was lost.
    BindingLost,
    /// The native bridge disappeared; re-provisioning is required.
    BridgeMissing,
}

/// What kind of transition happened.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TransitionKind {
    /// Availability was established.
    MarkAvailable,
    /// Readiness was established.
    MarkReady,
    /// Authorization pre-flight passed.
    Authorize,
    /// A binding was established.
    Bind,
    /// Execution began.
    BeginExecution,
    /// An interruption occurred.
    Interrupt(InterruptionKind),
    /// Recovery from an interruption.
    Recover(InterruptionKind),
    /// Session take-over.
    TakeOver,
    /// The binding was released.
    Release,
}

/// One recorded lifecycle transition.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LifecycleTransition {
    /// State before the transition.
    pub from: CapabilityLifecycleState,
    /// State after the transition.
    pub to: CapabilityLifecycleState,
    /// Transition kind.
    pub kind: TransitionKind,
    /// 1-based transition sequence.
    pub sequence: u64,
}

/// The capability lifecycle state machine.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CapabilityLifecycle {
    state: CapabilityLifecycleState,
    sequence: u64,
    interruption: Option<InterruptionKind>,
    history: Vec<LifecycleTransition>,
}

impl CapabilityLifecycle {
    /// Starts at `Declared` — the state implied by a workflow declaring
    /// browser requirements.
    pub fn declared() -> Self {
        Self {
            state: CapabilityLifecycleState::Declared,
            sequence: 0,
            interruption: None,
            history: Vec::new(),
        }
    }

    /// Current state.
    pub fn state(&self) -> CapabilityLifecycleState {
        self.state
    }

    /// The interruption currently recorded, if any.
    pub fn interruption(&self) -> Option<InterruptionKind> {
        self.interruption
    }

    /// All transitions in order.
    pub fn history(&self) -> &[LifecycleTransition] {
        &self.history
    }

    fn push(&mut self, to: CapabilityLifecycleState, kind: TransitionKind) {
        self.sequence += 1;
        let from = self.state;
        self.history.push(LifecycleTransition {
            from,
            to,
            kind,
            sequence: self.sequence,
        });
        self.state = to;
        self.interruption = match kind {
            TransitionKind::Interrupt(interruption) => Some(interruption),
            _ => None,
        };
    }

    fn illegal(&self, attempted: &'static str) -> AdapterError {
        AdapterError::IllegalTransition {
            from: self.state,
            attempted,
        }
    }

    /// `Declared -> Available`.
    pub fn mark_available(&mut self) -> Result<(), AdapterError> {
        if self.state != CapabilityLifecycleState::Declared {
            return Err(self.illegal("mark_available"));
        }
        self.push(
            CapabilityLifecycleState::Available,
            TransitionKind::MarkAvailable,
        );
        Ok(())
    }

    /// `Available -> Ready`.
    pub fn mark_ready(&mut self) -> Result<(), AdapterError> {
        if self.state != CapabilityLifecycleState::Available {
            return Err(self.illegal("mark_ready"));
        }
        self.push(CapabilityLifecycleState::Ready, TransitionKind::MarkReady);
        Ok(())
    }

    /// `Ready -> Authorized`.
    pub fn authorize(&mut self) -> Result<(), AdapterError> {
        if self.state != CapabilityLifecycleState::Ready {
            return Err(self.illegal("authorize"));
        }
        self.push(
            CapabilityLifecycleState::Authorized,
            TransitionKind::Authorize,
        );
        Ok(())
    }

    /// `Authorized -> Bound`.
    pub fn bind(&mut self) -> Result<(), AdapterError> {
        if self.state != CapabilityLifecycleState::Authorized {
            return Err(self.illegal("bind"));
        }
        self.push(CapabilityLifecycleState::Bound, TransitionKind::Bind);
        Ok(())
    }

    /// `Bound -> Executing`.
    pub fn begin_execution(&mut self) -> Result<(), AdapterError> {
        if self.state != CapabilityLifecycleState::Bound {
            return Err(self.illegal("begin_execution"));
        }
        self.push(
            CapabilityLifecycleState::Executing,
            TransitionKind::BeginExecution,
        );
        Ok(())
    }

    /// Any of `Available..Executing -> Interrupted`, recording the kind.
    pub fn interrupt(&mut self, kind: InterruptionKind) -> Result<(), AdapterError> {
        let interruptible = matches!(
            self.state,
            CapabilityLifecycleState::Available
                | CapabilityLifecycleState::Ready
                | CapabilityLifecycleState::Authorized
                | CapabilityLifecycleState::Bound
                | CapabilityLifecycleState::Executing
        );
        if !interruptible {
            return Err(self.illegal("interrupt"));
        }
        self.push(
            CapabilityLifecycleState::Interrupted,
            TransitionKind::Interrupt(kind),
        );
        Ok(())
    }

    /// `Interrupted -> Available`; requires `RuntimeLost` or `BridgeMissing`.
    pub fn recover_to_available(&mut self) -> Result<(), AdapterError> {
        let kind = match (self.state, self.interruption) {
            (
                CapabilityLifecycleState::Interrupted,
                Some(kind @ (InterruptionKind::RuntimeLost | InterruptionKind::BridgeMissing)),
            ) => kind,
            _ => return Err(self.illegal("recover_to_available")),
        };
        self.push(
            CapabilityLifecycleState::Available,
            TransitionKind::Recover(kind),
        );
        Ok(())
    }

    /// `Interrupted -> Ready`; requires `AuthorizationExpired`.
    pub fn recover_to_ready(&mut self) -> Result<(), AdapterError> {
        let kind = match (self.state, self.interruption) {
            (
                CapabilityLifecycleState::Interrupted,
                Some(kind @ InterruptionKind::AuthorizationExpired),
            ) => kind,
            _ => return Err(self.illegal("recover_to_ready")),
        };
        self.push(
            CapabilityLifecycleState::Ready,
            TransitionKind::Recover(kind),
        );
        Ok(())
    }

    /// `Interrupted -> Authorized`; requires `BindingLost`.
    pub fn recover_to_authorized(&mut self) -> Result<(), AdapterError> {
        let kind = match (self.state, self.interruption) {
            (CapabilityLifecycleState::Interrupted, Some(kind @ InterruptionKind::BindingLost)) => {
                kind
            }
            _ => return Err(self.illegal("recover_to_authorized")),
        };
        self.push(
            CapabilityLifecycleState::Authorized,
            TransitionKind::Recover(kind),
        );
        Ok(())
    }

    /// `Bound | Executing -> Bound` (take-over swaps the binding; the
    /// adapter re-enters `Executing` immediately after).
    pub fn take_over(&mut self) -> Result<(), AdapterError> {
        let takeoverable = matches!(
            self.state,
            CapabilityLifecycleState::Bound | CapabilityLifecycleState::Executing
        );
        if !takeoverable {
            return Err(self.illegal("take_over"));
        }
        self.push(CapabilityLifecycleState::Bound, TransitionKind::TakeOver);
        Ok(())
    }

    /// `Executing | Bound | Interrupted -> Released`.
    pub fn release(&mut self) -> Result<(), AdapterError> {
        let releasable = matches!(
            self.state,
            CapabilityLifecycleState::Executing
                | CapabilityLifecycleState::Bound
                | CapabilityLifecycleState::Interrupted
        );
        if !releasable {
            return Err(self.illegal("release"));
        }
        self.push(CapabilityLifecycleState::Released, TransitionKind::Release);
        Ok(())
    }

    /// Exports every transition as evidence. Interruptions, recoveries, and
    /// take-overs map to `EvidenceKind::Recovery`; all other transitions
    /// map to `EvidenceKind::Observation`.
    pub fn evidence_records(&self, scope: &str) -> Vec<EvidenceRecord> {
        self.history
            .iter()
            .map(|transition| {
                let kind = match transition.kind {
                    TransitionKind::Interrupt(_)
                    | TransitionKind::Recover(_)
                    | TransitionKind::TakeOver => EvidenceKind::Recovery,
                    _ => EvidenceKind::Observation,
                };
                EvidenceRecord::from_payload(
                    kind,
                    format!(
                        "codex-browser-use/{scope}/lifecycle/{:04}",
                        transition.sequence
                    ),
                    transition,
                )
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ready() -> CapabilityLifecycle {
        let mut lifecycle = CapabilityLifecycle::declared();
        lifecycle.mark_available().expect("available");
        lifecycle.mark_ready().expect("ready");
        lifecycle
    }

    fn executing() -> CapabilityLifecycle {
        let mut lifecycle = ready();
        lifecycle.authorize().expect("authorized");
        lifecycle.bind().expect("bound");
        lifecycle.begin_execution().expect("executing");
        lifecycle
    }

    #[test]
    fn declared_is_the_starting_state() {
        let lifecycle = CapabilityLifecycle::declared();
        assert_eq!(lifecycle.state(), CapabilityLifecycleState::Declared);
        assert!(lifecycle.history().is_empty());
    }

    #[test]
    fn happy_path_records_every_transition() {
        let mut lifecycle = executing();
        lifecycle.release().expect("released");
        assert_eq!(lifecycle.state(), CapabilityLifecycleState::Released);
        assert_eq!(lifecycle.history().len(), 6);
        assert_eq!(
            lifecycle.history()[0].from,
            CapabilityLifecycleState::Declared
        );
        assert_eq!(
            lifecycle.history()[0].to,
            CapabilityLifecycleState::Available
        );
        assert_eq!(
            lifecycle.history()[5].to,
            CapabilityLifecycleState::Released
        );
        assert_eq!(lifecycle.history()[5].sequence, 6);
    }

    #[test]
    fn illegal_transitions_are_rejected_without_state_change() {
        let mut lifecycle = CapabilityLifecycle::declared();
        let error = lifecycle.bind().expect_err("bind before authorization");
        assert!(matches!(error, AdapterError::IllegalTransition { .. }));
        assert_eq!(lifecycle.state(), CapabilityLifecycleState::Declared);
        lifecycle.authorize().expect_err("authorize before ready");
        assert_eq!(lifecycle.state(), CapabilityLifecycleState::Declared);
    }

    #[test]
    fn interruption_recovery_matches_kind() {
        let mut lifecycle = executing();
        lifecycle
            .interrupt(InterruptionKind::RuntimeLost)
            .expect("interrupted");
        assert_eq!(lifecycle.state(), CapabilityLifecycleState::Interrupted);
        assert_eq!(
            lifecycle.interruption(),
            Some(InterruptionKind::RuntimeLost)
        );
        lifecycle
            .recover_to_ready()
            .expect_err("wrong recovery target for RuntimeLost");
        lifecycle
            .recover_to_available()
            .expect("recovered to available");
        assert_eq!(lifecycle.state(), CapabilityLifecycleState::Available);
        lifecycle.mark_ready().expect("ready");
        lifecycle.authorize().expect("authorized");
        lifecycle.bind().expect("bound");
        lifecycle.begin_execution().expect("executing");
        let kinds: Vec<TransitionKind> = lifecycle.history().iter().map(|t| t.kind).collect();
        assert!(kinds.contains(&TransitionKind::Recover(InterruptionKind::RuntimeLost)));
    }

    #[test]
    fn authorization_expiry_recovers_to_ready_only() {
        let mut lifecycle = executing();
        lifecycle
            .interrupt(InterruptionKind::AuthorizationExpired)
            .expect("interrupted");
        lifecycle
            .recover_to_available()
            .expect_err("RuntimeLost/BridgeMissing-only target");
        lifecycle.recover_to_ready().expect("recovered to ready");
        assert_eq!(lifecycle.state(), CapabilityLifecycleState::Ready);
    }

    #[test]
    fn binding_loss_recovers_to_authorized() {
        let mut lifecycle = executing();
        lifecycle
            .interrupt(InterruptionKind::BindingLost)
            .expect("interrupted");
        lifecycle.recover_to_authorized().expect("recovered");
        assert_eq!(lifecycle.state(), CapabilityLifecycleState::Authorized);
        lifecycle.bind().expect("re-bound");
        lifecycle.begin_execution().expect("executing");
    }

    #[test]
    fn take_over_round_trips_through_bound() {
        let mut lifecycle = executing();
        lifecycle.take_over().expect("takeover");
        assert_eq!(lifecycle.state(), CapabilityLifecycleState::Bound);
        lifecycle.begin_execution().expect("executing");
        let last = &lifecycle.history()[lifecycle.history().len() - 2];
        assert_eq!(last.kind, TransitionKind::TakeOver);
        lifecycle.release().expect("released");
        lifecycle.take_over().expect_err("takeover after release");
    }

    #[test]
    fn lifecycle_evidence_kinds_map_to_recovery_for_takeover() {
        let mut lifecycle = executing();
        lifecycle.take_over().expect("takeover");
        let records = lifecycle.evidence_records("scope123");
        assert_eq!(records.len(), lifecycle.history().len());
        let last = records.last().expect("records");
        assert_eq!(last.kind, EvidenceKind::Recovery);
        assert!(last.locator.contains("scope123"));
        let first = records.first().expect("records");
        assert_eq!(first.kind, EvidenceKind::Observation);
    }
}
