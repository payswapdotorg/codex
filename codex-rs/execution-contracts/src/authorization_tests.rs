use super::*;
use crate::CapabilityBindingId;
use crate::ExecutionContractError;
use codex_workflow_contracts::ContentDigest;
use codex_workflow_contracts::EvidenceKind;
use codex_workflow_contracts::EvidenceReference;
use pretty_assertions::assert_eq;

fn approval_evidence() -> EvidenceReference {
    EvidenceReference {
        kind: EvidenceKind::Approval,
        locator: "approvals/1".to_owned(),
        digest: ContentDigest::of(&serde_json::json!({ "decision": "approved" }))
            .expect("digestable approval"),
    }
}

fn other_evidence(kind: EvidenceKind) -> EvidenceReference {
    EvidenceReference {
        kind,
        locator: "evidence/1".to_owned(),
        digest: ContentDigest::of(&serde_json::json!({ "kind": format!("{kind:?}") }))
            .expect("digestable evidence"),
    }
}

#[test]
fn grants_require_approval_evidence() {
    let binding =
        CapabilityBindingId::parse("codex-browser-use:navigate_web").expect("valid binding");
    let grant = AuthorizationGrant::new(binding.clone(), approval_evidence())
        .expect("approval evidence is accepted");
    assert_eq!(grant.binding, binding);
}

#[test]
fn grants_reject_every_non_approval_evidence_kind() {
    let binding =
        CapabilityBindingId::parse("codex-browser-use:navigate_web").expect("valid binding");
    for kind in [
        EvidenceKind::Observation,
        EvidenceKind::Artifact,
        EvidenceKind::Trace,
        EvidenceKind::TestResult,
        EvidenceKind::Recovery,
    ] {
        let error = AuthorizationGrant::new(binding.clone(), other_evidence(kind))
            .expect_err("non-approval evidence must be rejected");
        assert!(
            matches!(
                error,
                ExecutionContractError::ExpectedApprovalEvidence { .. }
            ),
            "{kind:?} must not authorize a binding"
        );
    }
}

#[test]
fn grants_round_trip_through_serde() {
    let binding =
        CapabilityBindingId::parse("terminal.local:run_terminal_command").expect("valid binding");
    let grant = AuthorizationGrant::new(binding, approval_evidence()).expect("valid grant");
    let serialized = serde_json::to_string(&grant).expect("serializable");
    let round_tripped: AuthorizationGrant =
        serde_json::from_str(&serialized).expect("deserializable");
    assert_eq!(round_tripped, grant);
}
