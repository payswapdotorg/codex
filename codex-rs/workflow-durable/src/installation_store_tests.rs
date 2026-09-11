//! Durable installation store tests: parity with the in-memory double,
//! upserts, audit appends, and drop-and-reload.

use pretty_assertions::assert_eq;

use super::DurableInstallationStore;
use crate::testutil::cleanup;
use crate::testutil::installed;
use crate::testutil::rebind;
use crate::testutil::sealed_version;
use crate::testutil::temp_root;
use codex_workflow_triggers::InMemoryInstallationStore;
use codex_workflow_triggers::InstallationStore;
use codex_workflow_triggers::RebindRecord;

/// One rebind audit record for `workflow` at a distinct instant.
fn rebind_at(workflow: &str, to: &str, at_unix_ms: u64) -> RebindRecord {
    RebindRecord {
        at_unix_ms,
        ..rebind(workflow, to)
    }
}

#[test]
fn save_load_list_and_audits_match_the_in_memory_store() {
    let root = temp_root("installation-parity");
    let mut durable = DurableInstallationStore::open(
        root.join("installations.json"),
        root.join("install-audits.jsonl"),
    )
    .expect("open");
    let mut memory = InMemoryInstallationStore::new();
    let first = sealed_version("triage-report", (1, 0, 0));
    let second = sealed_version("release-notes", (2, 1, 0));
    let first_configuration = installed(&first);
    let second_configuration = installed(&second);

    durable
        .save(first_configuration.clone())
        .expect("save durable first");
    durable
        .save(second_configuration.clone())
        .expect("save durable second");
    memory
        .save(first_configuration.clone())
        .expect("save memory first");
    memory
        .save(second_configuration.clone())
        .expect("save memory second");

    assert_eq!(
        durable.list().expect("list durable"),
        memory.list().expect("list memory"),
        "list is ordered by workflow identity like the double"
    );
    assert_eq!(
        durable.load(&first.definition.id).expect("load durable"),
        memory.load(&first.definition.id).expect("load memory")
    );
    assert_eq!(
        durable.load(&first.definition.id).expect("load durable"),
        Some(first_configuration)
    );
    assert_eq!(
        durable.load(&second.definition.id).expect("load durable"),
        Some(second_configuration)
    );

    // Upserts replace the configuration of the same workflow.
    let mut updated = installed(&second);
    updated.max_walk_steps = 42;
    durable.save(updated.clone()).expect("save durable updated");
    memory.save(updated).expect("save memory updated");
    assert_eq!(
        durable.load(&second.definition.id).expect("load durable"),
        memory.load(&second.definition.id).expect("load memory")
    );
    assert_eq!(durable.list().expect("list durable").len(), 2);

    // Audit appends accumulate in order.
    durable
        .append_audit(rebind_at("triage-report", "profile-a", 100))
        .expect("audit durable");
    memory
        .append_audit(rebind_at("triage-report", "profile-a", 100))
        .expect("audit memory");
    durable
        .append_audit(rebind_at("triage-report", "profile-b", 200))
        .expect("audit durable");
    memory
        .append_audit(rebind_at("triage-report", "profile-b", 200))
        .expect("audit memory");
    assert_eq!(durable.audits(), memory.audits());
    cleanup(&root);
}

#[test]
fn configurations_and_audits_survive_drop_and_reload() {
    let root = temp_root("installation-reload");
    let snapshot = root.join("installations.json");
    let audits = root.join("install-audits.jsonl");
    let version = sealed_version("triage-report", (1, 0, 0));
    let configuration = installed(&version);
    let audit = rebind_at("triage-report", "profile-a", 100);
    {
        let mut store = DurableInstallationStore::open(&snapshot, &audits).expect("open");
        store.save(configuration.clone()).expect("save");
        store.append_audit(audit.clone()).expect("audit");
    }

    let store = DurableInstallationStore::open(&snapshot, &audits).expect("reopen");
    assert_eq!(
        store.load(&version.definition.id).expect("load"),
        Some(configuration),
        "installations are loadable after restart"
    );
    assert_eq!(store.list().expect("list"), vec![installed(&version)]);
    assert_eq!(store.audits(), vec![audit]);
    cleanup(&root);
}

#[test]
fn uninstalled_workflows_load_as_none() {
    let root = temp_root("installation-missing");
    let store = DurableInstallationStore::open(
        root.join("installations.json"),
        root.join("install-audits.jsonl"),
    )
    .expect("open");
    assert_eq!(
        store
            .load(&crate::testutil::workflow_id("never-installed"))
            .expect("load"),
        None
    );
    assert!(store.list().expect("list").is_empty());
    assert!(store.audits().is_empty());
    cleanup(&root);
}
