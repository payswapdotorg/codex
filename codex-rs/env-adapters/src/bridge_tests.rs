//! Tests for the shared bridge vocabulary.

use pretty_assertions::assert_eq;

use crate::bridge::BridgeCallFailure;
use crate::bridge::BridgeFailureKind;
use crate::bridge::normalized_failure;
use codex_execution_contracts::FAILURE_MESSAGE_MAX_BYTES;
use codex_execution_contracts::FailureKind;

#[test]
fn failure_kinds_map_onto_recovery_classifications() {
    let cases = [
        (BridgeFailureKind::Lost, FailureKind::Unavailable),
        (BridgeFailureKind::Timeout, FailureKind::Timeout),
        (BridgeFailureKind::Rejected, FailureKind::PolicyDenied),
        (BridgeFailureKind::Protocol, FailureKind::Permanent),
    ];
    for (kind, expected) in cases {
        let failure = BridgeCallFailure::new(kind, "stub detail");
        assert_eq!(failure.failure_kind(), expected);
    }
}

#[test]
fn messages_name_the_failure_class() {
    let lost = BridgeCallFailure::new(BridgeFailureKind::Lost, "process exited");
    assert_eq!(lost.message(), "bridge lost: process exited");
    let rejected = BridgeCallFailure::new(BridgeFailureKind::Rejected, "host refused");
    assert_eq!(rejected.message(), "bridge rejected the call: host refused");
}

#[test]
fn normalized_failures_clamp_and_fill() {
    let empty = normalized_failure(FailureKind::Permanent, "");
    assert_eq!(empty.kind, FailureKind::Permanent);
    assert_eq!(empty.message, "unspecified environment bridge failure");

    // Multi-byte untrusted output must truncate at a character boundary
    // instead of panicking the clamp.
    let oversized = "é".repeat(FAILURE_MESSAGE_MAX_BYTES);
    let clamped = normalized_failure(FailureKind::Unavailable, oversized);
    assert_eq!(clamped.kind, FailureKind::Unavailable);
    assert!(clamped.message.len() <= FAILURE_MESSAGE_MAX_BYTES);

    let bounded = normalized_failure(FailureKind::Transient, "glitch");
    assert_eq!(bounded.message, "glitch");
}
