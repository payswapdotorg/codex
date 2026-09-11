//! Durable evidence store tests: locator/digest minting parity with the
//! in-memory double, promote parity, and drop-and-reload stability.

use pretty_assertions::assert_eq;

use super::DurableEvidenceStore;
use crate::testutil::cleanup;
use crate::testutil::temp_root;
use codex_workflow_app::EvidenceStore;
use codex_workflow_app::InMemoryEvidenceStore;
use codex_workflow_contracts::EvidenceKind;

/// The payload sequence both stores are driven with.
fn payloads() -> Vec<serde_json::Value> {
    vec![
        serde_json::json!({ "kind": "observation", "text": "the tracker is open" }),
        serde_json::json!({ "kind": "approval", "approver": "tech-lead", "verdict": "approved" }),
        serde_json::json!({ "kind": "recovery", "strategy": "retry", "attempt": 1 }),
    ]
}

#[test]
fn locator_and_digest_minting_match_the_in_memory_double() {
    let root = temp_root("evidence-parity");
    let mut durable = DurableEvidenceStore::open(root.join("evidence.jsonl")).expect("open");
    let mut memory = InMemoryEvidenceStore::new();

    for payload in payloads() {
        let durable_reference = durable
            .store(EvidenceKind::Observation, &payload)
            .expect("store durable");
        let memory_reference = memory
            .store(EvidenceKind::Observation, &payload)
            .expect("store memory");
        assert_eq!(
            durable_reference, memory_reference,
            "identical payloads mint identical locators and digests"
        );
    }
    assert_eq!(durable.len(), memory.len());

    // The verify step agrees on both planes.
    let reference = memory
        .store(
            EvidenceKind::TestResult,
            &serde_json::json!({ "suite": "smoke", "passed": true }),
        )
        .expect("store memory");
    let durable_reference = durable
        .store(
            EvidenceKind::TestResult,
            &serde_json::json!({ "suite": "smoke", "passed": true }),
        )
        .expect("store durable");
    assert_eq!(durable_reference, reference);
    assert!(durable.verify(&reference).expect("verify durable"));
    assert!(memory.verify(&reference).expect("verify memory"));
    cleanup(&root);
}

#[test]
fn promote_matches_the_in_memory_double_without_persisting() {
    let root = temp_root("evidence-promote");
    let mut durable = DurableEvidenceStore::open(root.join("evidence.jsonl")).expect("open");
    let mut memory = InMemoryEvidenceStore::new();
    let digest_hex = "ab".repeat(32);

    let durable_reference = durable
        .promote(
            EvidenceKind::Trace,
            "codex-env-adapters/browser/7",
            &digest_hex,
        )
        .expect("promote durable");
    let memory_reference = memory
        .promote(
            EvidenceKind::Trace,
            "codex-env-adapters/browser/7",
            &digest_hex,
        )
        .expect("promote memory");
    assert_eq!(durable_reference, memory_reference);

    // Promote validates the digest exactly like the double and records
    // nothing: the store stays empty and the locator stays unknown.
    assert!(durable.is_empty());
    assert!(
        !durable
            .verify(&durable_reference)
            .expect("promoted locators carry no stored payload")
    );
    assert_eq!(
        durable
            .promote(EvidenceKind::Trace, "any", "not-a-digest")
            .is_err(),
        memory
            .promote(EvidenceKind::Trace, "any", "not-a-digest")
            .is_err(),
        "invalid digests are rejected identically"
    );
    cleanup(&root);
}

#[test]
fn locators_and_digests_survive_drop_and_reload() {
    let root = temp_root("evidence-reload");
    let path = root.join("evidence.jsonl");
    let pairs = [
        (EvidenceKind::Observation, serde_json::json!({ "n": 1 })),
        (EvidenceKind::Artifact, serde_json::json!({ "n": 2 })),
        (EvidenceKind::Recovery, serde_json::json!({ "n": 3 })),
    ];
    let mut references = Vec::new();
    {
        let mut store = DurableEvidenceStore::open(&path).expect("open");
        for (kind, payload) in pairs.iter().cloned() {
            references.push(store.store(kind, &payload).expect("store"));
        }
    }

    let mut store = DurableEvidenceStore::open(&path).expect("reopen");
    assert_eq!(store.len(), references.len());
    for (reference, (_, payload)) in references.iter().zip(pairs.iter()) {
        assert!(store.verify(reference).expect("verify"), "{reference:?}");
        assert_eq!(
            store.payload(&reference.locator).expect("payload"),
            *payload
        );
    }

    // The sequence resumes: the next store never reuses a locator, in
    // memory or across restarts.
    let next = store
        .store(EvidenceKind::Trace, &serde_json::json!({ "n": 4 }))
        .expect("store after reload");
    assert_eq!(next.locator, "workflow-app/trace/4");
    cleanup(&root);
}

#[test]
fn journal_bytes_are_content_stable_across_reload() {
    let root = temp_root("evidence-stable");
    let path = root.join("evidence.jsonl");
    {
        let mut store = DurableEvidenceStore::open(&path).expect("open");
        for payload in payloads() {
            store
                .store(EvidenceKind::Observation, &payload)
                .expect("store");
        }
    }
    let bytes = std::fs::read(&path).expect("read journal");
    // A pure reload rewrites nothing.
    let _ = DurableEvidenceStore::open(&path).expect("reopen");
    assert_eq!(std::fs::read(&path).expect("reread journal"), bytes);
    // Re-storing the same payloads through a fresh store reproduces the
    // same committed bytes (deterministic, content-stable format).
    let mut store = DurableEvidenceStore::open(&path).expect("reopen");
    for payload in payloads() {
        store
            .store(EvidenceKind::Observation, &payload)
            .expect("store again");
    }
    let round_tripped = std::fs::read(&path).expect("reread journal");
    assert_eq!(
        round_tripped.len(),
        bytes.len() * 2,
        "the same payload sequence doubles the journal byte-for-byte"
    );
    assert_eq!(&round_tripped[..bytes.len()], &bytes[..]);
    cleanup(&root);
}
