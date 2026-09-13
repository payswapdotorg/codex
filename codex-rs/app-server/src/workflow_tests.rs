//! Service-level tests for the workflow control plane mount (RWO-001).
//!
//! These drive the mount exactly as the CLI does: teach (all three
//! modes) -> compile -> review -> approve -> publish, then run/list/get/
//! cancel durable instances, including the kill/restart durability story
//! (a fresh control plane over the same root re-observes every instance,
//! and a post-crash `Running` orphan recovers through the startup sweep
//! into an explicit resume).

use super::*;

use codex_workflow_app::RunPosition;
use codex_workflow_app::RunPositionStore;
use codex_workflow_app::WalkPosition;
use codex_workflow_app::WorkflowInstanceStore;
use rpc::WorkflowApprovalDecision;
use rpc::WorkflowDemonstrationKind;
use rpc::WorkflowTeachMode;

const COMMIT_SHA: &str = "0123456789abcdef0123456789abcdef01234567";

fn commit_sha() -> String {
    COMMIT_SHA.to_string()
}

fn expected_steps(mode: WorkflowTeachMode) -> u64 {
    match mode {
        WorkflowTeachMode::Demonstrate | WorkflowTeachMode::Instruct => 1,
        WorkflowTeachMode::Hybrid => 2,
    }
}

/// Teaches one workflow through a session and compiles it.
fn teach_and_compile(
    plane: &WorkflowControlPlane,
    mode: WorkflowTeachMode,
    name: &str,
) -> rpc::WorkflowCompileResponse {
    let start = plane
        .teach_start(rpc::WorkflowTeachStartParams {
            mode,
            name: Some(name.to_string()),
        })
        .expect("teach_start");
    if mode != WorkflowTeachMode::Demonstrate {
        plane
            .teach_instruct(rpc::WorkflowTeachInstructParams {
                session_id: start.session_id.clone(),
                text: "Summarize the daily progress report.".to_string(),
                evidence: Vec::new(),
            })
            .expect("teach_instruct");
    }
    if mode != WorkflowTeachMode::Instruct {
        plane
            .teach_demonstrate(rpc::WorkflowTeachDemonstrateParams {
                session_id: start.session_id.clone(),
                kind: WorkflowDemonstrationKind::Action,
                text: "Open the reports list.".to_string(),
                evidence: Vec::new(),
            })
            .expect("teach_demonstrate");
    }
    let reconcile = plane
        .teach_reconcile(rpc::WorkflowTeachReconcileParams {
            session_id: start.session_id.clone(),
        })
        .expect("teach_reconcile");
    assert_eq!(reconcile.status, rpc::WorkflowTeachSessionStatus::Closed);
    plane
        .compile(rpc::WorkflowCompileParams {
            session_id: start.session_id,
        })
        .expect("compile")
}

/// Publishes one taught workflow and returns its identity.
async fn publish_taught(plane: &WorkflowControlPlane, name: &str) -> rpc::WorkflowPublishResponse {
    let compiled = teach_and_compile(plane, WorkflowTeachMode::Instruct, name);
    plane
        .approve(rpc::WorkflowApproveParams {
            candidate_id: compiled.candidate_id.clone(),
            approver: "tech-lead".to_string(),
            reference: "review-1".to_string(),
            decision: WorkflowApprovalDecision::Approved,
        })
        .expect("approve");
    plane
        .publish(rpc::WorkflowPublishParams {
            candidate_id: compiled.candidate_id,
            repository: None,
            commit_sha: commit_sha(),
            semantic_version: None,
        })
        .expect("publish")
}

#[test]
fn teach_compile_review_approve_publish_in_all_three_modes() {
    let root = tempfile::tempdir().expect("root");
    let plane = WorkflowControlPlane::new(root.path());
    for mode in [
        WorkflowTeachMode::Demonstrate,
        WorkflowTeachMode::Instruct,
        WorkflowTeachMode::Hybrid,
    ] {
        let compiled = teach_and_compile(&plane, mode, "daily-standup-report");
        assert_eq!(compiled.status, rpc::WorkflowCandidateStatus::Validated);
        assert!(compiled.validation.clean, "{mode:?}: {compiled:?}");
        assert_eq!(
            compiled.simulation.outcome,
            rpc::WorkflowSimulationOutcomeKind::Completed
        );
        assert_eq!(compiled.step_count, expected_steps(mode));

        let review = plane
            .review(rpc::WorkflowReviewParams {
                candidate_id: compiled.candidate_id.clone(),
            })
            .expect("review");
        assert_eq!(review.steps.len(), expected_steps(mode) as usize);
        assert_eq!(review.steps[0].node_id, "step-001");
        assert_eq!(
            review.steps[0].origin,
            if mode == WorkflowTeachMode::Instruct {
                rpc::WorkflowStepOrigin::Instructed
            } else {
                rpc::WorkflowStepOrigin::Observed
            }
        );
        assert!(!review.binding_proposals.is_empty());
        assert_eq!(review.status, rpc::WorkflowCandidateStatus::Validated);

        let approved = plane
            .approve(rpc::WorkflowApproveParams {
                candidate_id: compiled.candidate_id.clone(),
                approver: "tech-lead".to_string(),
                reference: "review-1".to_string(),
                decision: WorkflowApprovalDecision::Approved,
            })
            .expect("approve");
        assert_eq!(approved.status, rpc::WorkflowCandidateStatus::Approved);

        let published = plane
            .publish(rpc::WorkflowPublishParams {
                candidate_id: compiled.candidate_id.clone(),
                repository: None,
                commit_sha: commit_sha(),
                semantic_version: None,
            })
            .expect("publish");
        // The Workflow Identity fields are visible on the response.
        assert_eq!(published.workflow, "daily-standup-report");
        assert_eq!(published.semantic_version, "1.0.0");
        assert!(published.version_id.starts_with("sha256:"));
        assert!(published.definition_digest.starts_with("sha256:"));
        assert!(published.dependency_lock_digest.starts_with("sha256:"));
        assert_eq!(published.repository, "local/workflows/taught");
        assert_eq!(published.commit_sha, COMMIT_SHA);
        assert!(
            published
                .binding_resolution
                .approved_digest
                .starts_with("sha256:")
        );
        assert!(
            published
                .binding_resolution
                .executable_digest
                .starts_with("sha256:")
        );
    }
}

#[test]
fn publish_requires_an_approved_candidate() {
    let root = tempfile::tempdir().expect("root");
    let plane = WorkflowControlPlane::new(root.path());
    let compiled = teach_and_compile(&plane, WorkflowTeachMode::Instruct, "gate-check");
    let error = plane
        .publish(rpc::WorkflowPublishParams {
            candidate_id: compiled.candidate_id,
            repository: None,
            commit_sha: commit_sha(),
            semantic_version: None,
        })
        .expect_err("publish before approve must fail");
    assert!(
        error.to_string().contains("approved"),
        "unexpected error: {error}"
    );
}

#[test]
fn rejection_leaves_the_candidate_unapproved() {
    let root = tempfile::tempdir().expect("root");
    let plane = WorkflowControlPlane::new(root.path());
    let compiled = teach_and_compile(&plane, WorkflowTeachMode::Instruct, "reject-check");
    let rejected = plane
        .approve(rpc::WorkflowApproveParams {
            candidate_id: compiled.candidate_id.clone(),
            approver: "tech-lead".to_string(),
            reference: "review-1".to_string(),
            decision: WorkflowApprovalDecision::Rejected,
        })
        .expect("approve");
    assert_eq!(rejected.decision, WorkflowApprovalDecision::Rejected);
    assert_eq!(rejected.status, rpc::WorkflowCandidateStatus::Validated);
    let error = plane
        .publish(rpc::WorkflowPublishParams {
            candidate_id: compiled.candidate_id,
            repository: None,
            commit_sha: commit_sha(),
            semantic_version: None,
        })
        .expect_err("publish after rejection must fail");
    assert!(
        error.to_string().contains("approved"),
        "unexpected error: {error}"
    );
}

#[test]
fn publish_validates_the_identity_inputs() {
    let root = tempfile::tempdir().expect("root");
    let plane = WorkflowControlPlane::new(root.path());
    let compiled = teach_and_compile(&plane, WorkflowTeachMode::Instruct, "identity-inputs");
    plane
        .approve(rpc::WorkflowApproveParams {
            candidate_id: compiled.candidate_id.clone(),
            approver: "tech-lead".to_string(),
            reference: "review-1".to_string(),
            decision: WorkflowApprovalDecision::Approved,
        })
        .expect("approve");
    let abbreviated = plane
        .publish(rpc::WorkflowPublishParams {
            candidate_id: compiled.candidate_id.clone(),
            repository: None,
            commit_sha: "abbrev".to_string(),
            semantic_version: None,
        })
        .expect_err("abbreviated sha must fail");
    assert!(
        abbreviated.to_string().contains("commit sha"),
        "unexpected error: {abbreviated}"
    );
    let repository = plane
        .publish(rpc::WorkflowPublishParams {
            candidate_id: compiled.candidate_id.clone(),
            repository: Some("not a remote".to_string()),
            commit_sha: commit_sha(),
            semantic_version: None,
        })
        .expect_err("invalid repository must fail");
    assert!(
        repository.to_string().contains("repository"),
        "unexpected error: {repository}"
    );
    let version = plane
        .publish(rpc::WorkflowPublishParams {
            candidate_id: compiled.candidate_id,
            repository: None,
            commit_sha: commit_sha(),
            semantic_version: Some("not-a-version".to_string()),
        })
        .expect_err("invalid semantic version must fail");
    assert!(
        version.to_string().contains("semantic version"),
        "unexpected error: {version}"
    );
}

#[tokio::test]
async fn instances_run_list_get_and_survive_restart() {
    let root = tempfile::tempdir().expect("root");
    let plane = WorkflowControlPlane::new(root.path());
    let published = publish_taught(&plane, "restart-durable-workflow").await;

    let run = plane
        .instance_run(rpc::WorkflowInstanceRunParams {
            version_id: published.version_id.clone(),
        })
        .await
        .expect("instance_run");
    assert_eq!(run.status, rpc::WorkflowInstanceStatus::Succeeded);
    assert_eq!(run.terminal.kind, rpc::WorkflowRunTerminalKind::Completed);
    assert_eq!(run.workflow, "restart-durable-workflow");
    assert_eq!(run.version_id, published.version_id);
    assert_eq!(run.path, vec!["step-001".to_string()]);

    // The kill/restart story: a fresh control plane over the same root
    // re-observes the durable instance, version pin included.
    let reopened = WorkflowControlPlane::new(root.path());
    let listed = reopened
        .instance_list(rpc::WorkflowInstanceListParams {})
        .expect("instance_list");
    assert_eq!(listed.instances.len(), 1);
    assert_eq!(listed.instances[0].instance_id, run.instance_id);
    assert_eq!(
        listed.instances[0].status,
        rpc::WorkflowInstanceStatus::Succeeded
    );
    assert_eq!(listed.instances[0].version_id, published.version_id);
    // Terminal instances never look resumable.
    assert!(listed.instances[0].position.is_none());

    let got = reopened
        .instance_get(rpc::WorkflowInstanceGetParams {
            instance_id: run.instance_id.clone(),
        })
        .expect("instance_get");
    assert_eq!(got.instance.status, rpc::WorkflowInstanceStatus::Succeeded);
    assert_eq!(got.instance.workflow, "restart-durable-workflow");
    assert!(got.instance.position.is_none());
    assert!(
        got.evidence.is_empty(),
        "a capability-less taught workflow records no evidence"
    );
}

/// Seeds one post-crash `Running` instance with a mid-walk position, the
/// state a kill -9 between checkpoints leaves behind.
fn seed_orphaned_running_instance(
    root: &Path,
    workflow: &str,
    version_id: &str,
) -> WorkflowInstanceId {
    let stores = DurableStores::open(root).expect("open durable stores");
    let instance_id = WorkflowInstanceId::generate();
    let instance = WorkflowInstance::new(
        instance_id,
        WorkflowDefinitionId::parse(workflow).expect("workflow id"),
        WorkflowVersionId::try_from(version_id).expect("version id"),
        WorkflowInstanceStatus::Running,
    );
    let mut instances = stores.instances.clone();
    instances.create(instance).expect("seed instance");
    let position = RunPosition {
        instance: instance_id,
        version: WorkflowVersionId::try_from(version_id).expect("version id"),
        walk: WalkPosition {
            current: Some(codex_workflow_contracts::IrNodeId::parse("step-001").expect("node")),
            continuations: Vec::new(),
            steps_taken: 0,
            path: Vec::new(),
        },
        decisions: BTreeMap::new(),
        policy: Default::default(),
        max_steps: 10_000,
        resources: Vec::new(),
    };
    let mut positions = stores.run_positions.clone();
    positions.save(position).expect("seed position");
    instance_id
}

#[tokio::test]
async fn orphaned_running_instance_recovers_through_explicit_resume() {
    let root = tempfile::tempdir().expect("root");
    let plane = WorkflowControlPlane::new(root.path());
    let published = publish_taught(&plane, "orphan-recovery-workflow").await;
    let orphan = seed_orphaned_running_instance(
        root.path(),
        "orphan-recovery-workflow",
        &published.version_id,
    );

    // A fresh control plane (the restart) lists instances: the startup
    // sweep finds the orphaned `Running` record and pauses it — never
    // silently `Running`, never auto-executing.
    let reopened = WorkflowControlPlane::new(root.path());
    let listed = reopened
        .instance_list(rpc::WorkflowInstanceListParams {})
        .expect("instance_list");
    assert_eq!(listed.instances.len(), 1);
    assert_eq!(listed.instances[0].instance_id, orphan.to_string());
    assert_eq!(
        listed.instances[0].status,
        rpc::WorkflowInstanceStatus::Paused
    );
    // The persisted position is visible with the pause.
    let position = listed.instances[0]
        .position
        .as_ref()
        .expect("paused instances keep their position");
    assert_eq!(position.current_node.as_deref(), Some("step-001"));

    // Resume is explicit: the paused instance continues and settles.
    let resumed = reopened
        .instance_resume(rpc::WorkflowInstanceResumeParams {
            instance_id: orphan.to_string(),
        })
        .await
        .expect("instance_resume");
    assert_eq!(
        resumed.instance.status,
        rpc::WorkflowInstanceStatus::Succeeded
    );
    assert!(resumed.instance.position.is_none());
}

#[tokio::test]
async fn cancel_is_explicit_and_terminal() {
    let root = tempfile::tempdir().expect("root");
    let plane = WorkflowControlPlane::new(root.path());
    let published = publish_taught(&plane, "cancel-workflow").await;
    let orphan =
        seed_orphaned_running_instance(root.path(), "cancel-workflow", &published.version_id);
    // The startup sweep pauses the orphaned run first.
    let listed = plane
        .instance_list(rpc::WorkflowInstanceListParams {})
        .expect("instance_list");
    assert_eq!(
        listed.instances[0].status,
        rpc::WorkflowInstanceStatus::Paused
    );

    let cancelled = plane
        .instance_cancel(rpc::WorkflowInstanceCancelParams {
            instance_id: orphan.to_string(),
            reason: "operator takeover".to_string(),
        })
        .expect("instance_cancel");
    assert_eq!(
        cancelled.instance.status,
        rpc::WorkflowInstanceStatus::Cancelled
    );
    // A cancelled instance never looks resumable.
    assert!(cancelled.instance.position.is_none());
    // Double-cancel is an explicit error, never a silent no-op.
    let error = plane
        .instance_cancel(rpc::WorkflowInstanceCancelParams {
            instance_id: orphan.to_string(),
            reason: "double cancel".to_string(),
        })
        .expect_err("double cancel must fail");
    assert!(
        error.to_string().contains("cannot transition"),
        "unexpected error: {error}"
    );
    // Cancelling a settled instance is refused too.
    let settled = plane
        .instance_run(rpc::WorkflowInstanceRunParams {
            version_id: published.version_id,
        })
        .await
        .expect("instance_run");
    let refused = plane
        .instance_cancel(rpc::WorkflowInstanceCancelParams {
            instance_id: settled.instance_id,
            reason: "too late".to_string(),
        })
        .expect_err("cancel of a settled instance must fail");
    assert!(
        refused.to_string().contains("cannot transition"),
        "unexpected error: {refused}"
    );
}

#[tokio::test]
async fn resume_requires_a_paused_instance_with_a_position() {
    let root = tempfile::tempdir().expect("root");
    let plane = WorkflowControlPlane::new(root.path());
    let published = publish_taught(&plane, "resume-guard-workflow").await;
    let run = plane
        .instance_run(rpc::WorkflowInstanceRunParams {
            version_id: published.version_id,
        })
        .await
        .expect("instance_run");
    // A settled instance cannot resume: its position was discarded.
    let error = plane
        .instance_resume(rpc::WorkflowInstanceResumeParams {
            instance_id: run.instance_id.clone(),
        })
        .await
        .expect_err("resume of a settled instance must fail");
    assert!(
        error.to_string().contains("paused"),
        "unexpected error: {error}"
    );
    let missing = plane
        .instance_resume(rpc::WorkflowInstanceResumeParams {
            instance_id: "8b6d2f4e-70f3-4c92-9d0f-2ad51d4e9c11".to_string(),
        })
        .await
        .expect_err("resume of an unknown instance must fail");
    assert!(
        missing.to_string().contains("run position"),
        "unexpected error: {missing}"
    );
    let bad_version = plane
        .instance_run(rpc::WorkflowInstanceRunParams {
            version_id: "sha256:not-hex".to_string(),
        })
        .await
        .expect_err("run of a malformed version id must fail");
    assert!(
        bad_version.to_string().contains("version id"),
        "unexpected error: {bad_version}"
    );
}

#[test]
fn teach_mode_mismatches_are_rejected() {
    let root = tempfile::tempdir().expect("root");
    let plane = WorkflowControlPlane::new(root.path());
    let start = plane
        .teach_start(rpc::WorkflowTeachStartParams {
            mode: WorkflowTeachMode::Demonstrate,
            name: None,
        })
        .expect("teach_start");
    assert_eq!(start.name, DEFAULT_WORKFLOW_NAME);
    let error = plane
        .teach_instruct(rpc::WorkflowTeachInstructParams {
            session_id: start.session_id.clone(),
            text: "Not allowed in demonstrate mode.".to_string(),
            evidence: Vec::new(),
        })
        .expect_err("instruct in demonstrate mode must fail");
    assert!(
        error.to_string().contains("mode"),
        "unexpected error: {error}"
    );
    let unknown = plane
        .teach_reconcile(rpc::WorkflowTeachReconcileParams {
            session_id: "ws-none".to_string(),
        })
        .expect_err("unknown session must fail");
    assert!(
        unknown.to_string().contains("unknown teaching session"),
        "unexpected error: {unknown}"
    );
    let evidence = plane
        .teach_demonstrate(rpc::WorkflowTeachDemonstrateParams {
            session_id: start.session_id.clone(),
            kind: WorkflowDemonstrationKind::Action,
            text: "Open the list.".to_string(),
            evidence: vec![rpc::WorkflowTeachEvidenceInput {
                label: "open".to_string(),
                locator: "rollout://abc".to_string(),
                sha256: "not-a-digest".to_string(),
            }],
        })
        .expect_err("invalid evidence must fail");
    assert!(
        evidence.to_string().contains("evidence"),
        "unexpected error: {evidence}"
    );
}

#[test]
fn compile_requires_a_reconciled_session() {
    let root = tempfile::tempdir().expect("root");
    let plane = WorkflowControlPlane::new(root.path());
    let start = plane
        .teach_start(rpc::WorkflowTeachStartParams {
            mode: WorkflowTeachMode::Instruct,
            name: Some("open-session".to_string()),
        })
        .expect("teach_start");
    let error = plane
        .compile(rpc::WorkflowCompileParams {
            session_id: start.session_id,
        })
        .expect_err("compile of an open session must fail");
    assert!(
        error.to_string().contains("reconciled"),
        "unexpected error: {error}"
    );
}

#[test]
fn review_of_an_unknown_candidate_is_rejected() {
    let root = tempfile::tempdir().expect("root");
    let plane = WorkflowControlPlane::new(root.path());
    let error = plane
        .review(rpc::WorkflowReviewParams {
            candidate_id: "cand-none".to_string(),
        })
        .expect_err("review of an unknown candidate must fail");
    assert!(
        error.to_string().contains("unknown candidate"),
        "unexpected error: {error}"
    );
}
