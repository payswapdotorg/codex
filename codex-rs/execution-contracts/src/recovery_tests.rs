use super::*;
use crate::AdapterId;
use crate::BindingClass;
use crate::BindingDiagnostic;
use crate::CapabilityBindingId;
use crate::DiagnosticCode;
use crate::ExecutionContractError;
use crate::ExecutionEnvironment;
use crate::FailureKind;
use crate::FallbackReason;
use crate::FallbackRecord;
use crate::ReadinessState;
use crate::SelectedBinding;
use codex_workflow_contracts::CapabilityId;
use codex_workflow_contracts::CapabilityRequirement;
use codex_workflow_contracts::ContentDigest;
use codex_workflow_contracts::EvidenceKind;
use codex_workflow_contracts::EvidenceReference;
use pretty_assertions::assert_eq;

fn binding() -> CapabilityBindingId {
    CapabilityBindingId::parse("codex-browser-use:navigate_web").expect("valid binding")
}

fn approval_evidence() -> EvidenceReference {
    EvidenceReference {
        kind: EvidenceKind::Approval,
        locator: "approvals/7".to_owned(),
        digest: ContentDigest::of(&serde_json::json!({ "decision": "approved" }))
            .expect("digestable approval"),
    }
}

fn decision() -> BindingDecision {
    BindingDecision {
        requirement: CapabilityRequirement {
            capability: CapabilityId::parse("navigate_web").expect("valid capability"),
            purpose: Some("open the dashboard".to_owned()),
        },
        selected: SelectedBinding {
            binding: CapabilityBindingId::parse("external-browser:navigate_web")
                .expect("valid binding"),
            adapter: AdapterId::parse("external-browser").expect("valid adapter"),
            environment: ExecutionEnvironment::Browser,
            class: BindingClass::Compatible,
        },
        fallback: Some(FallbackRecord {
            skipped: SelectedBinding {
                binding: binding(),
                adapter: AdapterId::parse("codex-browser-use").expect("valid adapter"),
                environment: ExecutionEnvironment::Browser,
                class: BindingClass::CodexNative,
            },
            reason: FallbackReason::PreferredNotReady {
                state: ReadinessState::Failed,
                diagnostic: Some(BindingDiagnostic::new(
                    DiagnosticCode::ProbeFailed,
                    "browser runtime missing",
                )),
            },
        }),
    }
}

#[test]
fn takeover_requests_have_bounded_instructions() {
    assert!(
        TakeoverRequest::new(
            binding(),
            TakeoverReason::ExecutionFailure,
            "finish the checkout form"
        )
        .is_ok()
    );
    assert!(TakeoverRequest::new(binding(), TakeoverReason::HumanGate, "").is_err());
    assert!(
        TakeoverRequest::new(
            binding(),
            TakeoverReason::OperatorIntervention,
            "x".repeat(TakeoverRequest::MAX_INSTRUCTION_BYTES + 1)
        )
        .is_err()
    );
}

#[test]
fn takeover_grants_require_approval_evidence() {
    let request = TakeoverRequest::new(
        binding(),
        TakeoverReason::ExecutionFailure,
        "finish the checkout form",
    )
    .expect("valid request");
    assert!(
        TakeoverGrant::new(request.clone(), approval_evidence()).is_ok(),
        "takeover with an approval must be grantable"
    );
    let non_approval = EvidenceReference {
        kind: EvidenceKind::Observation,
        locator: "observations/1".to_owned(),
        digest: ContentDigest::of(&serde_json::json!({ "page": "loaded" }))
            .expect("digestable observation"),
    };
    let error = TakeoverGrant::new(request, non_approval)
        .expect_err("takeover cannot bypass authorization");
    assert!(matches!(
        error,
        ExecutionContractError::ExpectedApprovalEvidence { .. }
    ));
}

#[test]
fn recovery_records_validate_takeover_and_rebind_consistency() {
    let grant = TakeoverGrant::new(
        TakeoverRequest::new(
            binding(),
            TakeoverReason::ExecutionFailure,
            "finish the checkout form",
        )
        .expect("valid request"),
        approval_evidence(),
    )
    .expect("valid grant");
    let recovery = Recovery {
        binding: binding(),
        failure: ExecutionFailure::new(FailureKind::Permanent, "element not found")
            .expect("valid failure"),
        strategy: RecoveryStrategy::Takeover(grant),
        outcome: RecoveryOutcome::Escalated,
    };
    recovery.validate().expect("consistent recovery");

    let mismatched = Recovery {
        binding: CapabilityBindingId::parse("terminal.local:run_terminal_command")
            .expect("other binding"),
        failure: ExecutionFailure::new(FailureKind::Permanent, "element not found")
            .expect("valid failure"),
        strategy: RecoveryStrategy::Takeover(
            TakeoverGrant::new(
                TakeoverRequest::new(
                    binding(),
                    TakeoverReason::ExecutionFailure,
                    "finish the checkout form",
                )
                .expect("valid request"),
                approval_evidence(),
            )
            .expect("valid grant"),
        ),
        outcome: RecoveryOutcome::Escalated,
    };
    assert!(mismatched.validate().is_err());
}

#[test]
fn recovery_rebinds_must_select_a_different_binding() {
    let same_binding = Recovery {
        binding: binding(),
        failure: ExecutionFailure::new(FailureKind::Unavailable, "browser closed")
            .expect("valid failure"),
        strategy: RecoveryStrategy::Rebind(BindingDecision {
            requirement: CapabilityRequirement {
                capability: CapabilityId::parse("navigate_web").expect("valid capability"),
                purpose: None,
            },
            selected: SelectedBinding {
                binding: binding(),
                adapter: AdapterId::parse("codex-browser-use").expect("valid adapter"),
                environment: ExecutionEnvironment::Browser,
                class: BindingClass::CodexNative,
            },
            fallback: None,
        }),
        outcome: RecoveryOutcome::Failed,
    };
    assert!(same_binding.validate().is_err());
}

#[test]
fn recovery_records_convert_to_recovery_evidence() {
    let recovery = Recovery {
        binding: binding(),
        failure: ExecutionFailure::new(FailureKind::Timeout, "page never loaded")
            .expect("valid failure"),
        strategy: RecoveryStrategy::Retry,
        outcome: RecoveryOutcome::Recovered,
    };
    let reference = recovery
        .to_evidence_reference("recovery/1")
        .expect("evidence conversion");
    assert_eq!(reference.kind, EvidenceKind::Recovery);
    assert_eq!(
        reference.digest,
        ContentDigest::of(&recovery).expect("digestable recovery")
    );
}

#[test]
fn fallback_decisions_convert_to_recovery_evidence() {
    let decision = decision();
    let reference = decision
        .to_evidence_reference("bindings/1")
        .expect("evidence conversion");
    assert_eq!(
        reference.kind,
        EvidenceKind::Recovery,
        "fallback selections are rebind-class recovery evidence"
    );
    assert_eq!(
        reference.digest,
        ContentDigest::of(&decision).expect("digestable decision")
    );
}
