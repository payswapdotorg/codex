use super::*;
use crate::AdapterId;
use crate::CapabilityBindingId;
use crate::ExecutionContractError;
use crate::ExecutionEnvironment;
use crate::Observation;
use pretty_assertions::assert_eq;

fn binding() -> CapabilityBindingId {
    CapabilityBindingId::parse("terminal.local:run_terminal_command").expect("valid binding")
}

fn failure() -> ExecutionFailure {
    ExecutionFailure::new(FailureKind::Transient, "shell exited with 137").expect("valid failure")
}

#[test]
fn failures_require_bounded_nonempty_messages() {
    assert!(ExecutionFailure::new(FailureKind::Permanent, "").is_err());
    assert!(
        ExecutionFailure::new(
            FailureKind::Permanent,
            "x".repeat(FAILURE_MESSAGE_MAX_BYTES)
        )
        .is_ok()
    );
    assert!(
        ExecutionFailure::new(
            FailureKind::Permanent,
            "x".repeat(FAILURE_MESSAGE_MAX_BYTES + 1)
        )
        .is_err()
    );
}

#[test]
fn failed_results_carry_exactly_one_failure() {
    let result = ActionResult::failed(binding(), failure());
    result.validate().expect("consistent failed result");
    assert_eq!(result.outcome, ActionOutcome::Failed);
    assert!(result.failure.is_some());

    let inconsistent = ActionResult {
        binding: binding(),
        outcome: ActionOutcome::Failed,
        failure: None,
        observations: Vec::new(),
        outputs: None,
        duration_ms: None,
    };
    assert!(inconsistent.validate().is_err());
}

#[test]
fn successful_results_carry_no_failure() {
    let mut result = ActionResult::succeeded(binding());
    result.failure = Some(failure());
    assert!(result.validate().is_err());

    let clean = ActionResult::succeeded(binding())
        .with_duration_ms(42)
        .with_outputs(serde_json::json!({ "exit_code": 0 }));
    clean.validate().expect("consistent successful result");
    assert_eq!(clean.duration_ms, Some(42));
}

#[test]
fn results_reject_oversized_outputs() {
    let result = ActionResult::succeeded(binding()).with_outputs(serde_json::Value::String(
        "x".repeat(ACTION_OUTPUTS_MAX_BYTES + 1),
    ));
    let error = result
        .validate()
        .expect_err("oversized outputs must be rejected");
    assert!(matches!(
        error,
        ExecutionContractError::PayloadTooLarge {
            field: "action outputs",
            ..
        }
    ));
}

#[test]
fn results_attach_observations_and_pass_validation_through() {
    let observation = Observation::new(
        ExecutionEnvironment::Terminal,
        AdapterId::parse("terminal.local").expect("valid adapter"),
        serde_json::json!({ "stdout": "built successfully" }),
    );
    let result = ActionResult::succeeded(binding()).with_observations(vec![observation]);
    result
        .validate()
        .expect("attached observations validate transitively");
    assert_eq!(result.observations.len(), 1);
}

#[test]
fn failure_kinds_cover_the_recovery_pipeline() {
    let kinds = [
        FailureKind::Transient,
        FailureKind::Permanent,
        FailureKind::Timeout,
        FailureKind::PolicyDenied,
        FailureKind::Unavailable,
    ];
    for kind in kinds {
        let serialized = serde_json::to_string(&kind).expect("serializable");
        let round_tripped: FailureKind = serde_json::from_str(&serialized).expect("deserializable");
        assert_eq!(round_tripped, kind);
    }
}

#[test]
fn results_round_trip_through_serde() {
    let result = ActionResult::failed(binding(), failure())
        .with_duration_ms(100)
        .with_outputs(serde_json::json!({ "exit_code": 137 }));
    let serialized = serde_json::to_string(&result).expect("serializable");
    let round_tripped: ActionResult = serde_json::from_str(&serialized).expect("deserializable");
    assert_eq!(round_tripped, result);
}
