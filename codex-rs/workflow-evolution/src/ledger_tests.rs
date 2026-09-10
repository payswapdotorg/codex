//! Ledger and retention tests.

use codex_workflow_contracts::ContentDigest;
use codex_workflow_contracts::EvidenceKind;
use codex_workflow_contracts::EvidenceReference;
use codex_workflow_forge::PublishedVersionRef;
use pretty_assertions::assert_eq;

use super::EvolutionLedger;
use super::RetentionPolicy;
use crate::CandidateId;
use crate::CandidateProvenance;
use crate::ImprovementCandidate;
use crate::ProposedChange;
use crate::WorkflowEvolutionError;
use crate::test_support::probe_version;

/// One synthetic evidence reference.
fn reference(kind: EvidenceKind, sequence: usize) -> EvidenceReference {
    EvidenceReference {
        kind,
        locator: format!("unit/{sequence}"),
        digest: ContentDigest::try_from(format!("sha256:{}", "ab".repeat(32))).expect("digest"),
    }
}

/// One minimal candidate with the given provenance size.
fn candidate(evidence: usize) -> ImprovementCandidate {
    ImprovementCandidate {
        id: CandidateId::parse(format!("unit-candidate-{evidence}")).expect("candidate id"),
        incumbent: PublishedVersionRef::of(&probe_version()),
        change: ProposedChange::Schedule {
            spec: codex_workflow_triggers::ScheduleSpec::Every { period_ms: 60_000 },
        },
        rationale: "unit test candidate".to_string(),
        provenance: CandidateProvenance {
            evidence: (0..evidence)
                .map(|sequence| reference(EvidenceKind::Observation, sequence))
                .collect(),
            runs: Vec::new(),
        },
    }
}

#[test]
fn recording_enforces_the_evidence_provenance_cap_loudly() {
    let mut ledger = EvolutionLedger::new(RetentionPolicy {
        max_candidates: 8,
        max_evidence_per_candidate: 2,
    })
    .expect("ledger");
    assert!(ledger.record(candidate(2)).is_ok());
    let refusal = ledger.record(candidate(3));
    assert!(matches!(
        refusal,
        Err(WorkflowEvolutionError::ProvenanceTooLarge {
            count: 3,
            cap: 2,
            ..
        })
    ));
}

#[test]
fn recording_refuses_duplicates() {
    let mut ledger = EvolutionLedger::new(RetentionPolicy {
        max_candidates: 8,
        max_evidence_per_candidate: 8,
    })
    .expect("ledger");
    let candidate = candidate(1);
    assert!(ledger.record(candidate.clone()).is_ok());
    let refusal = ledger.record(candidate);
    assert!(matches!(
        refusal,
        Err(WorkflowEvolutionError::CandidateAlreadyKnown { .. })
    ));
}

#[test]
fn recording_refuses_credential_contamination() {
    let mut ledger = EvolutionLedger::new(RetentionPolicy {
        max_candidates: 8,
        max_evidence_per_candidate: 8,
    })
    .expect("ledger");
    let mut contaminated = candidate(0);
    contaminated.rationale =
        "token pasted from https://alice:ghp_ABCDEFGHIJKLMNOPQRSTUVWXYZ1234@github.com".to_string();
    assert!(matches!(
        ledger.record(contaminated),
        Err(WorkflowEvolutionError::CredentialContamination { .. })
    ));
}

#[test]
fn retention_bounds_candidates_first_in_first_out() {
    let mut ledger = EvolutionLedger::new(RetentionPolicy {
        max_candidates: 2,
        max_evidence_per_candidate: 8,
    })
    .expect("ledger");
    for evidence in 0..4 {
        assert!(ledger.record(candidate(evidence)).is_ok());
        // The bound is enforced on every insert, never deferred.
        assert!(ledger.entries().len() <= 2);
    }
    let ids: Vec<String> = ledger
        .entries()
        .iter()
        .map(|entry| entry.candidate.id.to_string())
        .collect();
    assert_eq!(
        ids,
        vec![
            "unit-candidate-2".to_string(),
            "unit-candidate-3".to_string(),
        ],
        "the oldest entries leave first"
    );
}

#[test]
fn zero_caps_are_refused() {
    let result = EvolutionLedger::new(RetentionPolicy {
        max_candidates: 0,
        max_evidence_per_candidate: 8,
    });
    assert!(matches!(
        result,
        Err(WorkflowEvolutionError::InvalidCandidate { .. })
    ));
}
