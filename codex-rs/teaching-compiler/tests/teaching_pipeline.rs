//! End-to-end publication-gate coverage for `codex-teaching-compiler` (WO-008).
//!
//! These tests exercise the full pipeline across crate boundaries: a taught
//! session is compiled, validated, simulated, approved, and finalized into an
//! [`ApprovedWorkflow`], including the negative paths that the publication
//! gate must enforce (stale simulation, content-bound approval, rejected
//! decisions).

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
