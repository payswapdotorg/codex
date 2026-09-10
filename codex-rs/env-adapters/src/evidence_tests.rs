//! Tests for the adapter evidence journal.

use pretty_assertions::assert_eq;

use crate::evidence::EVIDENCE_LOCATOR_PREFIX;
use crate::evidence::EvidenceJournal;
use codex_workflow_contracts::EvidenceKind;

#[test]
fn records_carry_deterministic_locators_and_stable_digests() {
    let mut journal = EvidenceJournal::new("codex-remote-desktop-env");
    let payload = serde_json::json!({"host": "ops-remote-host-1", "width": 1920});
    let first = journal
        .record(EvidenceKind::Observation, &payload)
        .expect("first record");
    let second = journal
        .record(EvidenceKind::Observation, &payload)
        .expect("second record");

    // Identical payloads digest identically; locators are positional.
    assert_eq!(first.digest_hex, second.digest_hex);
    assert_eq!(
        first.locator,
        format!("{EVIDENCE_LOCATOR_PREFIX}/codex-remote-desktop-env/0")
    );
    assert_eq!(
        second.locator,
        format!("{EVIDENCE_LOCATOR_PREFIX}/codex-remote-desktop-env/1")
    );
    assert_ne!(first.locator, second.locator);
}

#[test]
fn distinct_payloads_digest_distinctly() {
    let mut journal = EvidenceJournal::new("codex-mobile-device-env");
    let first = journal
        .record(
            EvidenceKind::Trace,
            &serde_json::json!({"outcome": "succeeded"}),
        )
        .expect("first record");
    let second = journal
        .record(
            EvidenceKind::Trace,
            &serde_json::json!({"outcome": "succeeded", "detail": "retry"}),
        )
        .expect("second record");
    assert_ne!(first.digest_hex, second.digest_hex);
    assert_eq!(first.kind, EvidenceKind::Trace);
    assert_eq!(second.kind, EvidenceKind::Trace);
}

#[test]
fn journal_counts_track_the_records() {
    let mut journal = EvidenceJournal::new("codex-env-adapters");
    assert!(journal.is_empty());
    assert_eq!(journal.len(), 0);
    journal
        .record(
            EvidenceKind::Recovery,
            &serde_json::json!({"kind": "takeover"}),
        )
        .expect("record");
    assert!(!journal.is_empty());
    assert_eq!(journal.len(), 1);
    assert_eq!(journal.records().len(), 1);
    assert_eq!(journal.records()[0].kind, EvidenceKind::Recovery);
}
