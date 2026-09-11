//! Unit tests for the provider evidence journal.

use codex_workflow_contracts::EvidenceKind;
use pretty_assertions::assert_eq;

use crate::evidence::EVIDENCE_LOCATOR_PREFIX;
use crate::evidence::ResourceEvidenceJournal;

#[test]
fn journal_locators_are_deterministic_and_provider_scoped() {
    let mut journal = ResourceEvidenceJournal::new("fixture-sandbox-cloud");
    let first = journal
        .record(
            EvidenceKind::Artifact,
            &serde_json::json!({"fact": "created"}),
        )
        .expect("record");
    let second = journal
        .record(EvidenceKind::Trace, &serde_json::json!({"fact": "probed"}))
        .expect("record");
    assert_eq!(
        first.locator,
        "codex-resource-providers/fixture-sandbox-cloud/0"
    );
    assert_eq!(
        second.locator,
        "codex-resource-providers/fixture-sandbox-cloud/1"
    );
    assert!(first.locator.starts_with(EVIDENCE_LOCATOR_PREFIX));

    let mut replay = ResourceEvidenceJournal::new("fixture-sandbox-cloud");
    let replayed = replay
        .record(
            EvidenceKind::Artifact,
            &serde_json::json!({"fact": "created"}),
        )
        .expect("record");
    assert_eq!(replayed.locator, first.locator);
    assert_eq!(replayed.digest_hex, first.digest_hex);
}

#[test]
fn journal_digests_are_stable_and_discriminating() {
    let mut journal = ResourceEvidenceJournal::new("fixture-local-containers");
    let payload = serde_json::json!({
        "provider": "fixture-local-containers",
        "fact": "resource_transition",
        "resource": "ws-0001",
        "op": "resized",
    });
    let first = journal
        .record(EvidenceKind::Artifact, &payload)
        .expect("record");
    let again = journal
        .record(EvidenceKind::Artifact, &payload)
        .expect("record");
    assert_eq!(first.digest_hex, again.digest_hex);
    assert_eq!(first.kind, EvidenceKind::Artifact);

    let other = journal
        .record(
            EvidenceKind::Recovery,
            &serde_json::json!({
                "provider": "fixture-local-containers",
                "fact": "container_removed",
            }),
        )
        .expect("record");
    assert_ne!(first.digest_hex, other.digest_hex);
    assert_eq!(other.kind, EvidenceKind::Recovery);
    assert_eq!(first.digest_hex.len(), 64);
    assert!(
        first
            .digest_hex
            .chars()
            .all(|character| character.is_ascii_hexdigit() && !character.is_ascii_uppercase())
    );
}

#[test]
fn journal_returns_records_oldest_first() {
    let mut journal = ResourceEvidenceJournal::new("fixture-journal");
    assert!(journal.is_empty());
    assert_eq!(journal.len(), 0);
    for index in 0..3 {
        journal
            .record(EvidenceKind::Artifact, &serde_json::json!({"seq": index}))
            .expect("record");
    }
    let records = journal.records();
    assert_eq!(records.len(), 3);
    assert_eq!(journal.len(), 3);
    assert!(!journal.is_empty());
    assert_eq!(
        records
            .iter()
            .map(|record| record.locator.clone())
            .collect::<Vec<_>>(),
        vec![
            "codex-resource-providers/fixture-journal/0".to_owned(),
            "codex-resource-providers/fixture-journal/1".to_owned(),
            "codex-resource-providers/fixture-journal/2".to_owned(),
        ]
    );
    assert_eq!(
        records
            .iter()
            .map(|record| record.payload.get("seq").cloned())
            .collect::<Vec<_>>(),
        vec![
            Some(serde_json::json!(0)),
            Some(serde_json::json!(1)),
            Some(serde_json::json!(2)),
        ]
    );
}
