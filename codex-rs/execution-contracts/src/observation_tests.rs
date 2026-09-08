use super::*;
use crate::AdapterId;
use crate::ExecutionContractError;
use crate::ExecutionEnvironment;
use codex_workflow_contracts::ContentDigest;
use codex_workflow_contracts::EvidenceKind;
use pretty_assertions::assert_eq;

fn observation() -> Observation {
    Observation::new(
        ExecutionEnvironment::Browser,
        AdapterId::parse("codex-browser-use").expect("valid adapter"),
        serde_json::json!({ "url": "https://example.com", "title": "Example" }),
    )
}

#[test]
fn observations_carry_environment_adapter_and_payload() {
    let observation = observation();
    observation
        .validate()
        .expect("bounded observation is valid");
    assert_eq!(observation.environment, ExecutionEnvironment::Browser);
}

#[test]
fn observations_reject_oversized_payloads() {
    let oversized = Observation::new(
        ExecutionEnvironment::Terminal,
        AdapterId::parse("terminal.local").expect("valid adapter"),
        serde_json::Value::String("x".repeat(OBSERVATION_MAX_BYTES + 1)),
    );
    let error = oversized
        .validate()
        .expect_err("oversized payloads must be rejected");
    assert!(matches!(
        error,
        ExecutionContractError::PayloadTooLarge {
            field: "observation payload",
            ..
        }
    ));
}

#[test]
fn observations_reject_deeply_nested_payloads() {
    let mut nested = serde_json::Value::Null;
    for _ in 0..(OBSERVATION_MAX_DEPTH + 1) {
        nested = serde_json::Value::Array(vec![nested]);
    }
    let deep = Observation::new(
        ExecutionEnvironment::Browser,
        AdapterId::parse("codex-browser-use").expect("valid adapter"),
        nested,
    );
    assert!(deep.validate().is_err());
}

#[test]
fn observations_reject_reserved_environments() {
    let reserved = Observation::new(
        ExecutionEnvironment::RemoteDesktop,
        AdapterId::parse("rdp-bridge").expect("valid adapter"),
        serde_json::json!({ "screen": "locked" }),
    );
    let error = reserved
        .validate()
        .expect_err("reserved environments cannot observe yet");
    assert!(matches!(
        error,
        ExecutionContractError::ReservedEnvironment { .. }
    ));
}

#[test]
fn observations_convert_to_digest_bearing_evidence() {
    let observation = observation();
    let reference = observation
        .to_evidence_reference("observations/1")
        .expect("evidence conversion");
    assert_eq!(reference.kind, EvidenceKind::Observation);
    assert_eq!(reference.locator, "observations/1");
    assert_eq!(
        reference.digest,
        ContentDigest::of(&observation).expect("digestable observation")
    );
}

#[test]
fn evidence_locators_are_validated() {
    let observation = observation();
    for locator in ["", "line\nbreak", &"x".repeat(513)] {
        let result = observation.to_evidence_reference(locator);
        assert!(
            result.is_err(),
            "locator `{locator}` must be rejected (length {})",
            locator.len()
        );
    }
}
