//! WO-008 teaching-compiler end-to-end pipeline tests.
//!
//! The workspace clippy.toml allows `expect` in `#[cfg(test)]` code; the
//! same intent applies to these integration-test setup helpers, so the deny
//! is explicitly lifted for this file (assertion-style failure is exactly
//! what setup `expect` provides).
#![allow(clippy::expect_used)]

use codex_teaching_compiler::{
    ApprovalDecision, ApprovalDecisionKind, CandidateStatus, RecordOrigin, SessionPolicy,
    SimulationConfig, TeachingCompilerError, TeachingMode, TeachingSession, TrajectoryEvent,
    WorkflowCandidate, compile,
};
use codex_workflow_contracts::WorkflowDefinitionId;

fn definition_id() -> WorkflowDefinitionId {
    WorkflowDefinitionId::parse("wo-008/teaching-pipeline").expect("definition id")
}

fn taught_candidate() -> WorkflowCandidate {
    let mut session = TeachingSession::new(TeachingMode::Demonstrate);
    session
        .record(
            RecordOrigin::Demonstration,
            TrajectoryEvent::Action {
                text: "Open the list.".to_string(),
            },
            Vec::new(),
        )
        .expect("record");
    session.close();
    compile(&session).expect("compile")
}

fn decision(reference: &str) -> ApprovalDecision {
    ApprovalDecision::new("tech-lead", reference, ApprovalDecisionKind::Approved).expect("decision")
}

#[test]
fn happy_path_reaches_an_approved_workflow() {
    let mut candidate = taught_candidate();
    candidate.validate().expect("validate");
    candidate
        .simulate(SimulationConfig::default())
        .expect("simulate");
    candidate
        .approve(decision("review-approval"))
        .expect("approve");
    let approved = candidate.finalize(definition_id()).expect("finalize");
    assert_eq!(approved.definition.id, definition_id());
    assert!(candidate.approval().is_some());
}

#[test]
fn finalization_requires_a_current_simulation() {
    let mut candidate = taught_candidate();
    candidate.validate().expect("validate");
    candidate.approve(decision("review-8")).expect("approve");
    assert!(matches!(
        candidate.finalize(definition_id()),
        Err(TeachingCompilerError::MissingSimulation)
    ));
}

#[test]
fn approval_is_bound_to_exact_candidate_content() {
    let mut candidate = taught_candidate();
    candidate.validate().expect("validate");
    candidate.approve(decision("review-9")).expect("approve");
    candidate.set_policy(SessionPolicy {
        allow_optimization: false,
        max_records: 4,
    });
    assert_eq!(candidate.status(), CandidateStatus::Compiled);
    assert!(candidate.approval().is_none());
    assert!(matches!(
        candidate.approve(decision("review-10")),
        Err(TeachingCompilerError::InvalidState { .. })
    ));
}

#[test]
fn compilation_is_stable_across_definition_assembly() {
    let first = taught_candidate();
    let second = taught_candidate();
    let definition_a = first.to_definition(definition_id()).expect("definition");
    let definition_b = second.to_definition(definition_id()).expect("definition");
    assert_eq!(definition_a, definition_b);
}

#[test]
fn rejected_decisions_are_not_approvals() {
    let mut candidate = taught_candidate();
    candidate.validate().expect("validate");
    let rejected = ApprovalDecision::new("tech-lead", "review-11", ApprovalDecisionKind::Rejected)
        .expect("decision");
    assert!(matches!(
        candidate.approve(rejected),
        Err(TeachingCompilerError::InvalidApproval { .. })
    ));
    assert_eq!(candidate.status(), CandidateStatus::Validated);
}

#[test]
fn approved_output_is_digest_pinned_and_stable() {
    let definition = definition_id();
    let mut first = taught_candidate();
    first.validate().expect("validate");
    first
        .simulate(SimulationConfig::default())
        .expect("simulate");
    first.approve(decision("review-digest-1")).expect("approve");
    let approved_first = first.finalize(definition.clone()).expect("finalize");

    let mut second = taught_candidate();
    second.validate().expect("validate");
    second
        .simulate(SimulationConfig::default())
        .expect("simulate");
    second
        .approve(decision("review-digest-2"))
        .expect("approve");
    let approved_second = second.finalize(definition.clone()).expect("finalize");

    // The digest covers the definition's semantic content only: identical
    // teaching produces identical pinned content even when the approval
    // references differ.
    assert_eq!(approved_first.digest, approved_second.digest);
    assert!(approved_first.digest.as_str().starts_with("sha256:"));
    assert_eq!(approved_first.digest.as_str().len(), "sha256:".len() + 64);
    assert!(approved_first.simulation.allows_publication());

    // Publication immutability: the finalized candidate is locked. No
    // further lifecycle transition can silently change the approved
    // content or produce a second version from it.
    assert_eq!(first.status(), CandidateStatus::PublicationReady);
    assert!(matches!(
        first.validate(),
        Err(TeachingCompilerError::InvalidState { .. })
    ));
    assert!(matches!(
        first.approve(decision("review-after-finalize")),
        Err(TeachingCompilerError::InvalidState { .. })
    ));
    assert!(matches!(
        first.finalize(definition),
        Err(TeachingCompilerError::InvalidState { .. })
    ));
}
