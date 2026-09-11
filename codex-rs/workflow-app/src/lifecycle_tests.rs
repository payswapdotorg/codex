//! Unit tests for the instance status-mutation seam.
//!
//! The seam is the single enforcement point every workflow-app status
//! transition routes through; these tests drive the real seam for every
//! (from, to) status pair, so each transition the frozen contract table
//! forbids is proven rejected exactly where it would be performed, with
//! the refused record left untouched.

use super::*;
use codex_execution_contracts::CapabilityRegistry;
use codex_workflow_contracts::ContentDigest;
use pretty_assertions::assert_eq;

/// A lifecycle over empty in-memory seams: no version is selected and no
/// adapter is registered. The seam under test touches none of them.
fn lifecycle() -> WorkflowLifecycle {
    let evidence = crate::memory::InMemoryEvidenceStore::new();
    WorkflowLifecycle::new(LifecycleDeps {
        versions: Box::new(crate::memory::InMemoryVersionStore::new()),
        instances: Box::new(crate::memory::InMemoryInstanceStore::new()),
        evidence: Box::new(evidence.clone()),
        approvals: Box::new(crate::memory::InMemoryApprovalSource::approving(
            "tech-lead",
            evidence,
        )),
        actions: Box::new(crate::memory::ScriptedActionSource::new(Vec::new())),
        events: Box::new(crate::memory::RecordingEventSink::new()),
        registry: CapabilityRegistry::new(),
    })
}

/// One instance record in `status`, with a pinned content-addressed
/// version identity.
fn seeded_instance(status: WorkflowInstanceStatus) -> WorkflowInstance {
    WorkflowInstance::new(
        WorkflowInstanceId::generate(),
        WorkflowDefinitionId::parse("release-notes").expect("valid workflow id"),
        WorkflowVersionId::from_digest(
            ContentDigest::of(&serde_json::json!({ "workflow": "release-notes" })).expect("digest"),
        ),
        status,
    )
}

#[test]
fn the_seam_enforces_the_declared_table_for_every_status_pair() {
    let mut service = lifecycle();
    for from in WorkflowInstanceStatus::ALL {
        for to in WorkflowInstanceStatus::ALL {
            let mut instance = seeded_instance(from);
            let result = service.transition_status(&mut instance, to);
            if from.can_transition_to(to) {
                assert!(
                    result.is_ok(),
                    "legal pair ({from:?} -> {to:?}) must apply through the seam"
                );
                assert_eq!(instance.status, to, "pair ({from:?} -> {to:?})");
            } else {
                let error = result.expect_err("illegal pair is refused");
                assert!(
                    matches!(
                        error,
                        WorkflowAppError::IllegalStatusTransition {
                            current,
                            target,
                            ..
                        } if current == from && target == to
                    ),
                    "pair ({from:?} -> {to:?}) must be refused with both statuses named"
                );
                assert_eq!(
                    instance.status, from,
                    "refused pair ({from:?} -> {to:?}) must leave the record untouched"
                );
            }
        }
    }
}

#[test]
fn cancelling_an_unknown_instance_is_an_availability_error() {
    let mut service = lifecycle();
    let error = service
        .cancel(&WorkflowInstanceId::generate(), "operator stop")
        .expect_err("unknown instance");
    assert!(matches!(
        error,
        WorkflowAppError::InstanceUnavailable { .. }
    ));
}
