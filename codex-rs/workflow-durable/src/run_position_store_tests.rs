//! Durable run-position store tests: parity with the in-memory double,
//! checkpoint replacement, discard, and drop-and-reload.

use pretty_assertions::assert_eq;

use super::DurableRunPositionStore;
use crate::testutil::cleanup;
use crate::testutil::instance;
use crate::testutil::run_position;
use crate::testutil::sealed_version;
use crate::testutil::temp_root;
use codex_workflow_app::InMemoryRunPositionStore;
use codex_workflow_app::RunPositionStore;
use codex_workflow_contracts::IrNodeId;
use codex_workflow_contracts::WorkflowInstanceStatus;

fn node(value: &str) -> IrNodeId {
    IrNodeId::parse(value).expect("node id")
}

#[test]
fn save_load_discard_match_the_in_memory_double() {
    let root = temp_root("position-parity");
    let mut durable = DurableRunPositionStore::open(root.join("run-positions.json")).expect("open");
    let mut memory = InMemoryRunPositionStore::new();
    let version = sealed_version("release-notes", (1, 4, 2));
    let record = instance(
        &version.definition.id,
        &version.version_id,
        WorkflowInstanceStatus::Running,
    );
    let position = run_position(
        &record.instance_id,
        &version.version_id,
        Some(node("step-002")),
    );

    durable.save(position.clone()).expect("save durable");
    memory.save(position).expect("save memory");
    assert_eq!(
        durable.load(&record.instance_id).expect("load durable"),
        memory.load(&record.instance_id).expect("load memory")
    );
    assert_eq!(durable.records().len(), 1);
    assert_eq!(durable.len(), memory.len());

    // Discarding an instance with no position is an ordinary no-op.
    let absent = instance(
        &version.definition.id,
        &version.version_id,
        WorkflowInstanceStatus::Running,
    );
    durable
        .discard(&absent.instance_id)
        .expect("discard absent");
    memory.discard(&absent.instance_id).expect("discard absent");
    assert_eq!(
        durable.load(&record.instance_id).expect("load durable"),
        memory.load(&record.instance_id).expect("load memory")
    );

    durable
        .discard(&record.instance_id)
        .expect("discard durable");
    memory.discard(&record.instance_id).expect("discard memory");
    assert_eq!(
        durable.load(&record.instance_id).expect("load durable"),
        memory.load(&record.instance_id).expect("load memory")
    );
    assert!(durable.is_empty());
    assert_eq!(durable.len(), memory.len());
    cleanup(&root);
}

#[test]
fn save_replaces_the_earlier_checkpoint() {
    let root = temp_root("position-replace");
    let mut store = DurableRunPositionStore::open(root.join("run-positions.json")).expect("open");
    let version = sealed_version("release-notes", (1, 4, 2));
    let record = instance(
        &version.definition.id,
        &version.version_id,
        WorkflowInstanceStatus::Running,
    );

    // The start checkpoint, then the pause checkpoint: the latest one
    // is the position a reload observes.
    store
        .save(run_position(
            &record.instance_id,
            &version.version_id,
            Some(node("step-001")),
        ))
        .expect("save start");
    store
        .save(run_position(
            &record.instance_id,
            &version.version_id,
            Some(node("step-002")),
        ))
        .expect("save pause");
    assert_eq!(store.len(), 1);
    assert_eq!(
        store
            .load(&record.instance_id)
            .expect("load")
            .expect("position")
            .walk
            .current,
        Some(node("step-002")),
        "the latest checkpoint replaces the earlier one"
    );
    cleanup(&root);
}

#[test]
fn positions_survive_drop_and_reload() {
    let root = temp_root("position-reload");
    let path = root.join("run-positions.json");
    let version = sealed_version("release-notes", (1, 4, 2));
    let record = instance(
        &version.definition.id,
        &version.version_id,
        WorkflowInstanceStatus::Paused,
    );
    let position = run_position(
        &record.instance_id,
        &version.version_id,
        Some(node("step-002")),
    );
    {
        let mut store = DurableRunPositionStore::open(&path).expect("open");
        store.save(position.clone()).expect("save checkpoint");
    }

    let store = DurableRunPositionStore::open(&path).expect("reopen");
    assert_eq!(
        store.load(&record.instance_id).expect("load"),
        Some(position),
        "the persisted position re-observes identically after a restart"
    );
    assert_eq!(store.records().len(), 1);
    cleanup(&root);
}

#[test]
fn discards_survive_drop_and_reload() {
    let root = temp_root("position-discard");
    let path = root.join("run-positions.json");
    let version = sealed_version("release-notes", (1, 4, 2));
    let kept = instance(
        &version.definition.id,
        &version.version_id,
        WorkflowInstanceStatus::Paused,
    );
    let discarded = instance(
        &version.definition.id,
        &version.version_id,
        WorkflowInstanceStatus::Succeeded,
    );
    {
        let mut store = DurableRunPositionStore::open(&path).expect("open");
        store
            .save(run_position(
                &kept.instance_id,
                &version.version_id,
                Some(node("step-002")),
            ))
            .expect("save kept");
        store
            .save(run_position(
                &discarded.instance_id,
                &version.version_id,
                None,
            ))
            .expect("save discarded");
        store.discard(&discarded.instance_id).expect("discard");
    }

    let store = DurableRunPositionStore::open(&path).expect("reopen");
    assert!(
        store.load(&discarded.instance_id).expect("load").is_none(),
        "the terminal settlement's discard survives the restart"
    );
    assert!(
        store.load(&kept.instance_id).expect("load").is_some(),
        "the paused instance's position survives the restart"
    );
    cleanup(&root);
}

#[test]
fn unparseable_snapshots_fail_deterministically_at_open() {
    let root = temp_root("position-unparseable");
    let path = root.join("run-positions.json");
    std::fs::write(&path, b"{ not valid json").expect("write garbage");

    let error = DurableRunPositionStore::open(&path).expect_err("garbage must fail");
    assert!(
        matches!(error, crate::DurableStoreError::Serialization { .. }),
        "an unparseable snapshot is a deterministic construction failure, got {error:?}"
    );
    cleanup(&root);
}
