//! Durable workflow version store tests: parity with the in-memory
//! double, drop-and-reload, and load-time integrity re-verification.

use pretty_assertions::assert_eq;

use super::DurableVersionStore;
use crate::testutil::cleanup;
use crate::testutil::sealed_version;
use crate::testutil::temp_root;
use codex_workflow_app::InMemoryVersionStore;
use codex_workflow_app::WorkflowAppError;
use codex_workflow_app::WorkflowVersionStore;

#[test]
fn publish_and_load_match_the_in_memory_double() {
    let root = temp_root("version-parity");
    let mut durable = DurableVersionStore::open(root.join("versions.json")).expect("open");
    let mut memory = InMemoryVersionStore::new();
    let version = sealed_version("release-notes", (1, 4, 2));

    let durable_id = durable.publish(version.clone()).expect("publish durable");
    let memory_id = memory.publish(version).expect("publish memory");
    assert_eq!(durable_id, memory_id);

    let durable_record = durable.load(&durable_id).expect("load durable");
    let memory_record = memory.load(&memory_id).expect("load memory");
    assert_eq!(durable_record, memory_record);
    assert_eq!(durable.len(), memory.len());

    // A missing identity answers `None` exactly like the double.
    let absent = durable
        .load(&sealed_version("other-notes", (9, 9, 9)).version_id)
        .expect("load absent");
    assert_eq!(absent, None);
    cleanup(&root);
}

#[test]
fn state_survives_drop_and_reload() {
    let root = temp_root("version-reload");
    let path = root.join("versions.json");
    let first = sealed_version("release-notes", (1, 4, 2));
    let second = sealed_version("triage-report", (2, 0, 0));
    {
        let mut store = DurableVersionStore::open(&path).expect("open");
        store.publish(first.clone()).expect("publish first");
        store.publish(second.clone()).expect("publish second");
    }
    // The struct (and its file handles) are gone; re-open from disk.
    let store = DurableVersionStore::open(&path).expect("reopen");
    assert_eq!(store.len(), 2);
    assert_eq!(
        store.load(&first.version_id).expect("load first"),
        Some(first),
        "the sealed record re-observes identically after reload"
    );
    assert_eq!(
        store.load(&second.version_id).expect("load second"),
        Some(second.clone())
    );

    // Content stability: re-publishing the identical record produces
    // byte-identical snapshot bytes (the format is deterministic).
    let bytes_before = std::fs::read(&path).expect("read snapshot");
    let mut store = store;
    store.publish(second).expect("republish");
    let bytes_after = std::fs::read(&path).expect("reread snapshot");
    assert_eq!(bytes_before, bytes_after);
    cleanup(&root);
}

#[test]
fn tampered_snapshots_fail_integrity_on_load() {
    let root = temp_root("version-tamper");
    let path = root.join("versions.json");
    let version = sealed_version("release-notes", (1, 4, 2));
    {
        let mut store = DurableVersionStore::open(&path).expect("open");
        store.publish(version.clone()).expect("publish");
    }

    // Tamper with the stored definition description: the snapshot still
    // parses, but the record no longer recomputes its identity.
    let text = std::fs::read_to_string(&path).expect("read snapshot");
    let tampered = text.replace(
        "Draft and publish release notes",
        "Tampered after publication  ",
    );
    std::fs::write(&path, tampered).expect("write tampered snapshot");

    let store = DurableVersionStore::open(&path).expect("tampered snapshot still parses");
    let error = store
        .load(&version.version_id)
        .expect_err("tampered records never load");
    assert!(
        matches!(error, WorkflowAppError::VersionIntegrity { .. }),
        "expected a version integrity failure, got {error:?}"
    );
    cleanup(&root);
}

#[test]
fn unparseable_snapshots_fail_deterministically_at_open() {
    let root = temp_root("version-unparseable");
    let path = root.join("versions.json");
    std::fs::write(&path, b"{ not valid json").expect("write garbage");

    let error = DurableVersionStore::open(&path).expect_err("garbage must fail");
    assert!(
        matches!(error, crate::DurableStoreError::Serialization { .. }),
        "an unparseable snapshot is a deterministic construction failure, got {error:?}"
    );
    cleanup(&root);
}
