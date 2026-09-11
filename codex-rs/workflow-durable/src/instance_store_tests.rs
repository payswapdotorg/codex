//! Durable workflow instance store tests: parity with the in-memory
//! double, duplicate rejection, and drop-and-reload.

use pretty_assertions::assert_eq;

use super::DurableInstanceStore;
use crate::testutil::cleanup;
use crate::testutil::instance;
use crate::testutil::sealed_version;
use crate::testutil::temp_root;
use codex_workflow_app::InMemoryInstanceStore;
use codex_workflow_app::WorkflowAppError;
use codex_workflow_app::WorkflowInstanceStore;
use codex_workflow_contracts::EvidenceKind;
use codex_workflow_contracts::EvidenceReference;
use codex_workflow_contracts::WorkflowInstanceStatus;

#[test]
fn create_save_load_match_the_in_memory_double() {
    let root = temp_root("instance-parity");
    let mut durable = DurableInstanceStore::open(root.join("instances.json")).expect("open");
    let mut memory = InMemoryInstanceStore::new();
    let version = sealed_version("release-notes", (1, 4, 2));
    let record = instance(
        &version.definition.id,
        &version.version_id,
        WorkflowInstanceStatus::Pending,
    );

    durable.create(record.clone()).expect("create durable");
    memory.create(record.clone()).expect("create memory");
    assert_eq!(
        durable.load(&record.instance_id).expect("load durable"),
        memory.load(&record.instance_id).expect("load memory")
    );

    // A duplicate create is rejected with the same error as the double.
    let durable_error = durable
        .create(record.clone())
        .expect_err("duplicate rejected");
    let memory_error = memory
        .create(record.clone())
        .expect_err("duplicate rejected");
    assert!(
        matches!(
            (durable_error, memory_error),
            (
                WorkflowAppError::InstanceAlreadyExists { .. },
                WorkflowAppError::InstanceAlreadyExists { .. }
            )
        ),
        "duplicate identities are rejected identically"
    );

    // Saving status changes and appended evidence matches too.
    let mut advanced = record.clone();
    advanced.status = WorkflowInstanceStatus::Running;
    advanced.record_evidence(EvidenceReference {
        kind: EvidenceKind::Observation,
        locator: "workflow-app/observation/1".to_string(),
        digest: codex_workflow_contracts::ContentDigest::of(&serde_json::json!({
            "note": "observation payload"
        }))
        .expect("digest"),
    });
    durable.save(advanced.clone()).expect("save durable");
    memory.save(advanced.clone()).expect("save memory");
    assert_eq!(
        durable.load(&record.instance_id).expect("load durable"),
        memory.load(&record.instance_id).expect("load memory")
    );
    assert_eq!(durable.records(), memory.records());
    assert_eq!(durable.len(), memory.len());
    cleanup(&root);
}

#[test]
fn statuses_and_evidence_survive_drop_and_reload() {
    let root = temp_root("instance-reload");
    let path = root.join("instances.json");
    let version = sealed_version("release-notes", (1, 4, 2));
    let mut record = instance(
        &version.definition.id,
        &version.version_id,
        WorkflowInstanceStatus::Pending,
    );
    {
        let mut store = DurableInstanceStore::open(&path).expect("open");
        store.create(record.clone()).expect("create");
        record.status = WorkflowInstanceStatus::Running;
        store.save(record.clone()).expect("save running");
        record.status = WorkflowInstanceStatus::Paused;
        record.record_evidence(EvidenceReference {
            kind: EvidenceKind::Approval,
            locator: "workflow-app/approval/1".to_string(),
            digest: codex_workflow_contracts::ContentDigest::of(&serde_json::json!({
                "note": "approval payload"
            }))
            .expect("digest"),
        });
        store.save(record.clone()).expect("save paused");
    }

    let store = DurableInstanceStore::open(&path).expect("reopen");
    assert_eq!(
        store.load(&record.instance_id).expect("load"),
        Some(record),
        "the instance record, its paused status, and its evidence re-observe identically"
    );
    assert_eq!(store.records().len(), 1);
    cleanup(&root);
}

#[test]
fn missing_instances_load_as_none() {
    let root = temp_root("instance-missing");
    let store = DurableInstanceStore::open(root.join("instances.json")).expect("open");
    let version = sealed_version("release-notes", (1, 4, 2));
    let absent = instance(
        &version.definition.id,
        &version.version_id,
        WorkflowInstanceStatus::Pending,
    );
    assert_eq!(store.load(&absent.instance_id).expect("load"), None);
    assert!(store.is_empty());
    cleanup(&root);
}
