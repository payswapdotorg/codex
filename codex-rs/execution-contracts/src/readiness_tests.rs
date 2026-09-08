use super::*;
use crate::CapabilityBindingId;
use crate::DiagnosticCode;
use codex_workflow_contracts::ContentDigest;
use codex_workflow_contracts::EvidenceKind;
use codex_workflow_contracts::EvidenceReference;
use pretty_assertions::assert_eq;

fn authorization_grant() -> AuthorizationGrant {
    let binding =
        CapabilityBindingId::parse("codex-browser-use:navigate_web").expect("valid binding");
    AuthorizationGrant::new(
        binding,
        EvidenceReference {
            kind: EvidenceKind::Approval,
            locator: "approvals/1".to_owned(),
            digest: ContentDigest::of(&serde_json::json!({ "decision": "approved" }))
                .expect("digestable approval"),
        },
    )
    .expect("valid grant")
}

fn failure_diagnostic() -> BindingDiagnostic {
    BindingDiagnostic::new(DiagnosticCode::ProbeFailed, "probe transport failed")
}

fn not_installed_diagnostic() -> BindingDiagnostic {
    BindingDiagnostic::new(DiagnosticCode::NotInstalled, "runtime not installed")
}

fn environment_lost_diagnostic() -> BindingDiagnostic {
    BindingDiagnostic::new(DiagnosticCode::EnvironmentLost, "browser closed")
}

#[test]
fn tracker_starts_declared_with_empty_history() {
    let tracker = ReadinessTracker::new();
    assert_eq!(tracker.state(), ReadinessState::Declared);
    assert!(tracker.history().is_empty());
    assert_eq!(tracker.diagnostic(), None);
}

#[test]
fn the_healthy_path_walks_the_frozen_lifecycle_in_order() {
    let mut tracker = ReadinessTracker::new();
    assert_eq!(
        tracker
            .advance(TransitionCause::Registered)
            .expect("registered"),
        ReadinessState::Available
    );
    assert_eq!(
        tracker
            .advance(TransitionCause::ProbeSucceeded)
            .expect("probed"),
        ReadinessState::Ready
    );
    assert_eq!(
        tracker
            .advance(TransitionCause::Authorized(authorization_grant()))
            .expect("authorized"),
        ReadinessState::Authorized
    );
    assert_eq!(
        tracker
            .advance(TransitionCause::ResourcesAttached)
            .expect("bound"),
        ReadinessState::Bound
    );
    assert_eq!(
        tracker
            .advance(TransitionCause::ActionDispatched)
            .expect("dispatched"),
        ReadinessState::Executing
    );
    assert_eq!(
        tracker
            .advance(TransitionCause::ActionCompleted)
            .expect("completed"),
        ReadinessState::Bound
    );
    assert_eq!(tracker.history().len(), 6);
    for pair in tracker.history().windows(2) {
        assert_eq!(
            pair[0].to, pair[1].from,
            "history transitions must be contiguous"
        );
    }
    assert!(
        tracker
            .history()
            .iter()
            .all(|entry| entry.cause.target() == entry.to)
    );
}

#[test]
fn every_cause_targets_exactly_one_state() {
    let cases: Vec<TransitionCause> = vec![
        TransitionCause::Registered,
        TransitionCause::ProbeSucceeded,
        TransitionCause::CapabilityNotInstalled(not_installed_diagnostic()),
        TransitionCause::ProbeError(failure_diagnostic()),
        TransitionCause::Authorized(authorization_grant()),
        TransitionCause::ResourcesAttached,
        TransitionCause::ActionDispatched,
        TransitionCause::ActionCompleted,
        TransitionCause::ActionFailed(failure_diagnostic()),
        TransitionCause::PreparationFailed(failure_diagnostic()),
        TransitionCause::EnvironmentLost(environment_lost_diagnostic()),
        TransitionCause::Recovered,
        TransitionCause::Rediscovered,
    ];
    let expected_targets = [
        ReadinessState::Available,
        ReadinessState::Ready,
        ReadinessState::Unavailable,
        ReadinessState::Failed,
        ReadinessState::Authorized,
        ReadinessState::Bound,
        ReadinessState::Executing,
        ReadinessState::Bound,
        ReadinessState::Failed,
        ReadinessState::Failed,
        ReadinessState::Unavailable,
        ReadinessState::Ready,
        ReadinessState::Available,
    ];
    for (cause, expected) in cases.iter().zip(expected_targets) {
        assert_eq!(cause.target(), expected, "cause `{}`", cause.name());
        assert!(
            !cause.legal_from().is_empty(),
            "cause `{}` must be legal from at least one state",
            cause.name()
        );
    }
}

#[test]
fn failure_states_always_carry_diagnostics() {
    let mut tracker = ReadinessTracker::new();
    tracker
        .advance(TransitionCause::Registered)
        .expect("registered");
    assert_eq!(
        tracker
            .advance(TransitionCause::ProbeError(failure_diagnostic()))
            .expect("probe error"),
        ReadinessState::Failed
    );
    let diagnostic = tracker
        .diagnostic()
        .expect("failed state carries a diagnostic");
    assert_eq!(diagnostic.code, DiagnosticCode::ProbeFailed);
}

#[test]
fn illegal_transitions_name_the_current_state_and_cause() {
    let mut tracker = ReadinessTracker::new();
    let error = tracker
        .advance(TransitionCause::Authorized(authorization_grant()))
        .expect_err("authorization requires READY");
    assert!(matches!(
        error,
        ExecutionContractError::IllegalTransition {
            current: ReadinessState::Declared,
            cause: "authorized"
        }
    ));
}

#[test]
fn recovery_reestablishes_readiness_from_failed() {
    let mut tracker = ReadinessTracker::new();
    tracker
        .advance(TransitionCause::Registered)
        .expect("registered");
    tracker
        .advance(TransitionCause::ProbeError(failure_diagnostic()))
        .expect("probe error");
    assert_eq!(tracker.state(), ReadinessState::Failed);
    assert_eq!(
        tracker
            .advance(TransitionCause::Recovered)
            .expect("recovered"),
        ReadinessState::Ready
    );
    assert_eq!(tracker.diagnostic(), None);
}

#[test]
fn rediscovery_reenters_the_lifecycle_from_unavailable() {
    let mut tracker = ReadinessTracker::new();
    tracker
        .advance(TransitionCause::CapabilityNotInstalled(
            not_installed_diagnostic(),
        ))
        .expect("not installed");
    assert_eq!(tracker.state(), ReadinessState::Unavailable);
    assert!(tracker.diagnostic().is_some());
    assert_eq!(
        tracker
            .advance(TransitionCause::Rediscovered)
            .expect("rediscovered"),
        ReadinessState::Available
    );
    assert_eq!(tracker.diagnostic(), None);
}

#[test]
fn environment_loss_is_legal_from_live_states_only() {
    for state in ReadinessState::ALL {
        let mut tracker = ReadinessTracker::new();
        tracker.state = state;
        let result = tracker.advance(TransitionCause::EnvironmentLost(
            environment_lost_diagnostic(),
        ));
        let live = matches!(
            state,
            ReadinessState::Available
                | ReadinessState::Ready
                | ReadinessState::Authorized
                | ReadinessState::Bound
                | ReadinessState::Executing
        );
        assert_eq!(result.is_ok(), live, "environment loss from {state}");
    }
}

#[test]
fn every_state_rejects_at_least_one_cause() {
    // A total state machine where some cause is legal everywhere would
    // mean states are indistinguishable. Spot-check the hard gates.
    let mut tracker = ReadinessTracker::new();
    tracker.state = ReadinessState::Declared;
    assert!(tracker.advance(TransitionCause::ResourcesAttached).is_err());
    tracker.state = ReadinessState::Available;
    assert!(tracker.advance(TransitionCause::ActionDispatched).is_err());
    tracker.state = ReadinessState::Ready;
    assert!(tracker.advance(TransitionCause::ActionDispatched).is_err());
    tracker.state = ReadinessState::Authorized;
    assert!(tracker.advance(TransitionCause::ActionDispatched).is_err());
    tracker.state = ReadinessState::Bound;
    assert!(
        tracker
            .advance(TransitionCause::Authorized(authorization_grant()))
            .is_err()
    );
    tracker.state = ReadinessState::Failed;
    assert!(tracker.advance(TransitionCause::Registered).is_err());
    tracker.state = ReadinessState::Unavailable;
    assert!(tracker.advance(TransitionCause::Recovered).is_err());
}

#[test]
fn binding_execution_supports_parallel_dispatch_reentry() {
    let mut tracker = ReadinessTracker::new();
    tracker
        .advance(TransitionCause::Registered)
        .expect("registered");
    tracker
        .advance(TransitionCause::ProbeSucceeded)
        .expect("probed");
    tracker
        .advance(TransitionCause::Authorized(authorization_grant()))
        .expect("authorized");
    tracker
        .advance(TransitionCause::ResourcesAttached)
        .expect("bound");
    tracker
        .advance(TransitionCause::ActionDispatched)
        .expect("first dispatch");
    // A second action may dispatch while the first is executing, so
    // parallel workflow steps can share one binding.
    assert_eq!(
        tracker
            .advance(TransitionCause::ActionDispatched)
            .expect("re-dispatch while executing"),
        ReadinessState::Executing
    );
}

#[test]
fn selectable_states_passed_every_gate() {
    for state in ReadinessState::ALL {
        let selectable = matches!(
            state,
            ReadinessState::Ready
                | ReadinessState::Authorized
                | ReadinessState::Bound
                | ReadinessState::Executing
        );
        assert_eq!(
            state.is_selectable(),
            selectable,
            "{state} selectability must match the gate definition"
        );
    }
}

#[test]
fn payload_bearing_causes_round_trip_through_serde() {
    let grant = authorization_grant();
    let causes = vec![
        TransitionCause::CapabilityNotInstalled(not_installed_diagnostic()),
        TransitionCause::ProbeError(failure_diagnostic()),
        TransitionCause::Authorized(grant),
        TransitionCause::ActionFailed(failure_diagnostic()),
        TransitionCause::EnvironmentLost(environment_lost_diagnostic()),
    ];
    for cause in causes {
        let serialized = serde_json::to_string(&cause).expect("serializable");
        let round_tripped: TransitionCause =
            serde_json::from_str(&serialized).expect("deserializable");
        assert_eq!(round_tripped, cause, "{serialized} must round-trip");
    }
}

#[test]
fn readiness_states_and_transitions_round_trip_through_serde() {
    for state in ReadinessState::ALL {
        let serialized = serde_json::to_string(&state).expect("serializable");
        let round_tripped: ReadinessState =
            serde_json::from_str(&serialized).expect("deserializable");
        assert_eq!(round_tripped, state);
    }
    let transition = ReadinessTransition {
        from: ReadinessState::Available,
        to: ReadinessState::Ready,
        cause: TransitionCause::ProbeSucceeded,
    };
    let serialized = serde_json::to_string(&transition).expect("serializable");
    let round_tripped: ReadinessTransition =
        serde_json::from_str(&serialized).expect("deserializable");
    assert_eq!(round_tripped, transition);
}
