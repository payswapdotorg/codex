use super::*;
use crate::ContentDigest;
use crate::EvidenceKind;
use pretty_assertions::assert_eq;

fn version_id() -> WorkflowVersionId {
    WorkflowVersionId::from_digest(
        ContentDigest::of(&serde_json::json!({ "workflow": "release-notes" })).expect("digest"),
    )
}

fn instance() -> WorkflowInstance {
    WorkflowInstance::new(
        WorkflowInstanceId::generate(),
        WorkflowDefinitionId::parse("release-notes").expect("valid workflow id"),
        version_id(),
        WorkflowInstanceStatus::Pending,
    )
}

#[test]
fn instances_round_trip_through_serde() {
    let mut instance = instance();
    instance.status = WorkflowInstanceStatus::Running;
    instance.trigger = Some(crate::TriggerSource {
        trigger: crate::TriggerClass::Schedule,
        event_id: Some("accepted-42".to_owned()),
    });
    instance.record_evidence(EvidenceReference {
        kind: EvidenceKind::Observation,
        locator: "observation:1".to_owned(),
        digest: ContentDigest::of(&serde_json::json!({ "step": "check" })).expect("digest"),
    });

    let serialized = serde_json::to_string(&instance).expect("instance serializes");
    let parsed: WorkflowInstance = serde_json::from_str(&serialized).expect("instance parses");

    assert_eq!(instance, parsed);
}

#[test]
fn instances_pin_an_immutable_version() {
    let instance = instance();

    // The pinned version is a content-addressed identity: there is no field
    // through which a branch, tag, or moving ref could redirect a running
    // instance.
    let serialized = serde_json::to_value(&instance).expect("instance serializes");
    assert!(
        serialized["version"]
            .as_str()
            .is_some_and(|id| id.starts_with("sha256:"))
    );
    assert!(
        !serialized.to_string().contains("branch"),
        "instances must not carry movable refs, got: {serialized}"
    );
}

#[test]
fn instance_statuses_serialize_as_camel_case() {
    let statuses = [
        (WorkflowInstanceStatus::Pending, "\"pending\""),
        (WorkflowInstanceStatus::Running, "\"running\""),
        (WorkflowInstanceStatus::Paused, "\"paused\""),
        (WorkflowInstanceStatus::Succeeded, "\"succeeded\""),
        (WorkflowInstanceStatus::Failed, "\"failed\""),
        (WorkflowInstanceStatus::Cancelled, "\"cancelled\""),
    ];
    for (status, expected) in statuses {
        let serialized = serde_json::to_string(&status).expect("status serializes");
        assert_eq!(serialized, expected);
    }
}

#[test]
fn evidence_is_append_only_by_construction() {
    let mut instance = instance();
    let first = EvidenceReference {
        kind: EvidenceKind::Approval,
        locator: "approval:1".to_owned(),
        digest: ContentDigest::of(&serde_json::json!({ "approval": 1 })).expect("digest"),
    };
    instance.record_evidence(first.clone());
    instance.record_evidence(EvidenceReference {
        kind: EvidenceKind::Trace,
        locator: "trace:1".to_owned(),
        digest: ContentDigest::of(&serde_json::json!({ "trace": 1 })).expect("digest"),
    });

    assert_eq!(instance.evidence.first(), Some(&first));
    assert_eq!(instance.evidence.len(), 2);
}
