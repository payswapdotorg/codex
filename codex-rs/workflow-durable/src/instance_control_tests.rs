//! Durable instance control tests: the legal `Paused -> Running`
//! transition, error parity with the in-memory double, shared-state
//! composition, and drop-and-reload.

use pretty_assertions::assert_eq;

use super::DurableInstanceControl;
use super::DurableInstanceStore;
use crate::testutil::cleanup;
use crate::testutil::instance;
use crate::testutil::sealed_version;
use crate::testutil::temp_root;
use codex_workflow_app::InMemoryInstanceStore;
use codex_workflow_app::WorkflowInstanceStore;
use codex_workflow_contracts::TriggerClass;
use codex_workflow_contracts::TriggerSource;
use codex_workflow_contracts::WorkflowInstanceStatus;
use codex_workflow_triggers::InMemoryInstanceControl;
use codex_workflow_triggers::InstanceControl;
use codex_workflow_triggers::ResumeDirective;
use codex_workflow_triggers::WorkflowTriggerError;

/// A resume directive for `instance`.
fn directive_for(instance: &codex_workflow_contracts::WorkflowInstance) -> ResumeDirective {
    ResumeDirective {
        instance: instance.instance_id,
        node: codex_workflow_contracts::IrNodeId::parse("wait-signoff").expect("node id"),
        trigger: TriggerSource {
            trigger: TriggerClass::HumanEvent,
            event_id: Some("evt-1".to_string()),
        },
    }
}

#[test]
fn resume_matches_the_in_memory_control_plane() {
    let root = temp_root("control-parity");
    let version = sealed_version("triage-report", (1, 0, 0));
    let workflow = version.definition.id.clone();

    let mut durable_instances =
        DurableInstanceStore::open(root.join("instances.json")).expect("open");
    let mut memory_instances = InMemoryInstanceStore::new();
    let mut durable_record = instance(
        &workflow,
        &version.version_id,
        WorkflowInstanceStatus::Paused,
    );
    let mut memory_record = durable_record.clone();
    durable_instances
        .create(durable_record.clone())
        .expect("create durable");
    memory_instances
        .create(memory_record.clone())
        .expect("create memory");

    let mut durable = DurableInstanceControl::open(
        durable_instances.clone(),
        root.join("resume-directives.jsonl"),
    )
    .expect("open durable control");
    let mut memory = InMemoryInstanceControl::new(memory_instances.clone());

    // The legal transition returns the updated record on both planes.
    let durable_resumed = durable
        .resume(&directive_for(&durable_record))
        .expect("resume durable");
    let memory_resumed = memory
        .resume(&directive_for(&memory_record))
        .expect("resume memory");
    durable_record.status = WorkflowInstanceStatus::Running;
    memory_record.status = WorkflowInstanceStatus::Running;
    assert_eq!(durable_resumed, durable_record);
    assert_eq!(memory_resumed, memory_record);
    let durable_directives = durable.directives();
    let memory_directives = memory.directives();
    assert_eq!(durable_directives.len(), memory_directives.len());
    assert_eq!(durable_directives[0].node, memory_directives[0].node);
    assert_eq!(durable_directives[0].trigger, memory_directives[0].trigger);
    assert_eq!(
        durable_instances
            .load(&durable_record.instance_id)
            .expect("load durable"),
        Some(durable_record.clone())
    );
    assert_eq!(
        memory_instances
            .load(&memory_record.instance_id)
            .expect("load memory"),
        Some(memory_record.clone())
    );

    // A second resume of the now-Running instance is illegal on both
    // planes, with the same error shape.
    assert!(matches!(
        (
            durable.resume(&directive_for(&durable_record)),
            memory.resume(&directive_for(&memory_record))
        ),
        (
            Err(WorkflowTriggerError::IllegalResume { .. }),
            Err(WorkflowTriggerError::IllegalResume { .. })
        )
    ));
    // A missing instance is refused with the same error shape.
    let absent = instance(
        &workflow,
        &version.version_id,
        WorkflowInstanceStatus::Paused,
    );
    assert!(matches!(
        (
            durable.resume(&directive_for(&absent)),
            memory.resume(&directive_for(&absent))
        ),
        (
            Err(WorkflowTriggerError::InstanceUnavailable { .. }),
            Err(WorkflowTriggerError::InstanceUnavailable { .. })
        )
    ));
    cleanup(&root);
}

#[test]
fn only_paused_instances_resume() {
    let root = temp_root("control-only-paused");
    let version = sealed_version("triage-report", (1, 0, 0));
    let workflow = version.definition.id.clone();
    let mut instances = DurableInstanceStore::open(root.join("instances.json")).expect("open");
    let mut control =
        DurableInstanceControl::open(instances.clone(), root.join("resume-directives.jsonl"))
            .expect("open control");

    // Pending, Running, Succeeded, Failed, and Cancelled never resume.
    for status in [
        WorkflowInstanceStatus::Pending,
        WorkflowInstanceStatus::Running,
        WorkflowInstanceStatus::Succeeded,
        WorkflowInstanceStatus::Failed,
        WorkflowInstanceStatus::Cancelled,
    ] {
        let record = instance(&workflow, &version.version_id, status);
        instances.create(record.clone()).expect("create");
        let error = control
            .resume(&directive_for(&record))
            .expect_err("only paused instances resume");
        assert!(
            matches!(error, WorkflowTriggerError::IllegalResume { .. }),
            "{status:?} must not resume, got {error:?}"
        );
    }
    assert!(control.directives().is_empty());
    cleanup(&root);
}

#[test]
fn resumed_status_and_directives_survive_drop_and_reload() {
    let root = temp_root("control-reload");
    let version = sealed_version("triage-report", (1, 0, 0));
    let workflow = version.definition.id.clone();
    let mut record = instance(
        &workflow,
        &version.version_id,
        WorkflowInstanceStatus::Pending,
    );
    let directive = {
        let mut instances = DurableInstanceStore::open(root.join("instances.json")).expect("open");
        let mut control =
            DurableInstanceControl::open(instances.clone(), root.join("resume-directives.jsonl"))
                .expect("open control");
        instances.create(record.clone()).expect("create");
        record.status = WorkflowInstanceStatus::Paused;
        instances.save(record.clone()).expect("save paused");
        let directive = directive_for(&record);
        let resumed = control.resume(&directive).expect("resume");
        assert_eq!(resumed.status, WorkflowInstanceStatus::Running);
        directive
    };

    // Re-open from the same root: the instance is still Running and
    // the directive audit trail is preserved.
    let instances = DurableInstanceStore::open(root.join("instances.json")).expect("reopen");
    let control =
        DurableInstanceControl::open(instances.clone(), root.join("resume-directives.jsonl"))
            .expect("reopen control");
    record.status = WorkflowInstanceStatus::Running;
    assert_eq!(
        instances.load(&record.instance_id).expect("load"),
        Some(record),
        "the resumed status survives restart"
    );
    assert_eq!(control.directives(), vec![directive]);
    // The shared instance store observes the same durable record.
    assert_eq!(instances.records().len(), 1);
    cleanup(&root);
}
