//! Unit tests for persisted run position, resume continuation, and
//! startup reconciliation.
//!
//! The fixture is the in-memory seams over a directly-sealed
//! capability-free IR (`step -> wait -> step`), so the resume path runs
//! without adapters while exercising the real seams: the version store
//! (integrity re-verified), the instance store, the evidence plane, and
//! the run-position double. The control-plane `Paused -> Running`
//! transition is performed by the test directly on the store — the
//! test plays the host's `InstanceControl`, exactly the seam the
//! trigger plane's resume drives.

use std::collections::BTreeMap;

use codex_execution_contracts::BindingPolicy;
use codex_execution_contracts::CapabilityRegistry;
use codex_workflow_contracts::ContentDigest;
use codex_workflow_contracts::DependencyLock;
use codex_workflow_contracts::DevelopmentRef;
use codex_workflow_contracts::IR_FORMAT_VERSION;
use codex_workflow_contracts::ImmutableSourceRevision;
use codex_workflow_contracts::IrNodeId;
use codex_workflow_contracts::RevisionSha;
use codex_workflow_contracts::SemanticVersion;
use codex_workflow_contracts::StepNode;
use codex_workflow_contracts::TriggerClass;
use codex_workflow_contracts::WaitFor;
use codex_workflow_contracts::WaitNode;
use codex_workflow_contracts::WorkflowDefinition;
use codex_workflow_contracts::WorkflowDefinitionId;
use codex_workflow_contracts::WorkflowDependencies;
use codex_workflow_contracts::WorkflowInstance;
use codex_workflow_contracts::WorkflowInstanceId;
use codex_workflow_contracts::WorkflowInstanceStatus;
use codex_workflow_contracts::WorkflowIr;
use codex_workflow_contracts::WorkflowIrNode;
use codex_workflow_contracts::WorkflowRepositoryId;
use codex_workflow_contracts::WorkflowVersion;
use codex_workflow_contracts::WorkflowVersionId;
use pretty_assertions::assert_eq;

use super::*;
use crate::RunTerminal;
use crate::WorkflowEvent;
use crate::lifecycle::InstantiateRequest;
use crate::memory::InMemoryApprovalSource;
use crate::memory::InMemoryEvidenceStore;
use crate::memory::InMemoryInstanceStore;
use crate::memory::InMemoryRunPositionStore;
use crate::memory::InMemoryVersionStore;
use crate::memory::RecordingEventSink;
use crate::memory::ScriptedActionSource;
use crate::port::WorkflowInstanceStore;
use crate::port::WorkflowVersionStore;
use crate::walk::WalkPosition;

/// A pinned immutable source revision for directly-sealed versions.
const COMMIT: &str = "0123456789abcdef0123456789abcdef01234567";

fn node(value: &str) -> IrNodeId {
    IrNodeId::parse(value).expect("node id")
}

/// `step-001 -> wait-signoff -> step-002`: pauses at the wait node and
/// resumes at `step-002` (a pending re-entry point past the pause).
fn resumable_ir() -> WorkflowIr {
    let mut nodes = BTreeMap::new();
    nodes.insert(
        node("step-001"),
        WorkflowIrNode::Step(StepNode {
            capabilities: Vec::new(),
            roles: Vec::new(),
            description: None,
            next: Some(node("wait-signoff")),
        }),
    );
    nodes.insert(
        node("wait-signoff"),
        WorkflowIrNode::Wait(WaitNode {
            wait_for: WaitFor::Trigger(TriggerClass::HumanEvent),
            next: Some(node("step-002")),
        }),
    );
    nodes.insert(
        node("step-002"),
        WorkflowIrNode::Step(StepNode {
            capabilities: Vec::new(),
            roles: Vec::new(),
            description: None,
            next: None,
        }),
    );
    WorkflowIr {
        ir_format: IR_FORMAT_VERSION,
        entry: node("step-001"),
        nodes,
        conditions: BTreeMap::new(),
    }
}

/// Seals the resumable workflow directly through the frozen contract
/// primitive (the same `WorkflowVersion::seal` the publication pipeline
/// ends with).
fn sealed_version() -> WorkflowVersion {
    let definition = WorkflowDefinition {
        id: WorkflowDefinitionId::parse("resume-unit").expect("definition id"),
        description: None,
        ir: resumable_ir(),
        roles: BTreeMap::new(),
        triggers: Vec::new(),
        dependencies: WorkflowDependencies::default(),
    };
    WorkflowVersion::seal(
        definition,
        WorkflowRepositoryId::parse("https://github.com/acme/release-bot").expect("repository id"),
        ImmutableSourceRevision::pin(
            RevisionSha::parse(COMMIT).expect("revision sha"),
            DevelopmentRef::branch("main").ok(),
        ),
        SemanticVersion::new(1, 0, 0),
        DependencyLock::default(),
        None,
    )
    .expect("sealed version")
}

/// A content-addressed version identity for seeded records.
fn any_version_id() -> WorkflowVersionId {
    WorkflowVersionId::from_digest(
        ContentDigest::of(&serde_json::json!({ "workflow": "resume-unit" })).expect("digest"),
    )
}

/// The in-memory seams plus a lifecycle with the run-position seam
/// attached, and every audit handle the tests observe.
struct Fixture {
    lifecycle: WorkflowLifecycle,
    versions: InMemoryVersionStore,
    instances: InMemoryInstanceStore,
    evidence: InMemoryEvidenceStore,
    positions: InMemoryRunPositionStore,
    events: RecordingEventSink,
}

fn fixture() -> Fixture {
    let versions = InMemoryVersionStore::new();
    let instances = InMemoryInstanceStore::new();
    let evidence = InMemoryEvidenceStore::new();
    let approvals = InMemoryApprovalSource::approving("tech-lead", evidence.clone());
    let actions = ScriptedActionSource::new(Vec::new());
    let events = RecordingEventSink::new();
    let positions = InMemoryRunPositionStore::new();
    let lifecycle = WorkflowLifecycle::with_positions(
        LifecycleDeps {
            versions: Box::new(versions.clone()),
            instances: Box::new(instances.clone()),
            evidence: Box::new(evidence.clone()),
            approvals: Box::new(approvals),
            actions: Box::new(actions),
            events: Box::new(events.clone()),
            registry: CapabilityRegistry::new(),
        },
        Box::new(positions.clone()),
    );
    Fixture {
        lifecycle,
        versions,
        instances,
        evidence,
        positions,
        events,
    }
}

/// A lifecycle over the same seams without the run-position seam: the
/// pre-MWO-003 construction path.
fn fixture_without_positions() -> Fixture {
    let mut fixture = fixture();
    fixture.lifecycle = WorkflowLifecycle::new(LifecycleDeps {
        versions: Box::new(fixture.versions.clone()),
        instances: Box::new(fixture.instances.clone()),
        evidence: Box::new(fixture.evidence.clone()),
        approvals: Box::new(InMemoryApprovalSource::approving(
            "tech-lead",
            fixture.evidence.clone(),
        )),
        actions: Box::new(ScriptedActionSource::new(Vec::new())),
        events: Box::new(RecordingEventSink::new()),
        registry: CapabilityRegistry::new(),
    });
    fixture
}

/// One seeded instance record in `status`.
fn seeded_instance(status: WorkflowInstanceStatus) -> WorkflowInstance {
    WorkflowInstance::new(
        WorkflowInstanceId::generate(),
        WorkflowDefinitionId::parse("resume-unit").expect("definition id"),
        any_version_id(),
        status,
    )
}

/// Publishes, selects, and instantiates the resumable version.
async fn started_run(fixture: &mut Fixture) -> WorkflowInstance {
    let version = sealed_version();
    fixture
        .versions
        .publish(version.clone())
        .expect("publish version");
    fixture
        .lifecycle
        .select_version(&version.version_id)
        .expect("select version");
    fixture
        .lifecycle
        .instantiate(InstantiateRequest {
            trigger: None,
            policy: BindingPolicy::default(),
            resources: Vec::new(),
            walk: WalkConfig::default(),
        })
        .await
        .expect("instantiate")
}

/// Performs the control-plane `Paused -> Running` transition on the
/// store directly: the test plays the host's `InstanceControl` seam.
fn control_plane_resumes(fixture: &mut Fixture, instance: &WorkflowInstanceId) {
    let mut record = fixture
        .instances
        .load(instance)
        .expect("load record")
        .expect("seeded record");
    record.status = WorkflowInstanceStatus::Running;
    fixture.instances.save(record).expect("resume record");
}

#[tokio::test]
async fn instantiate_persists_the_starting_position() {
    let mut fixture = fixture();
    let instance = started_run(&mut fixture).await;
    let position = fixture
        .positions
        .position(&instance.instance_id)
        .expect("position persisted at the start checkpoint");
    assert_eq!(position.instance, instance.instance_id);
    assert_eq!(position.version, instance.version);
    assert_eq!(position.walk.current, Some(node("step-001")));
    assert_eq!(position.walk.steps_taken, 0);
    assert!(position.walk.path.is_empty());
    assert!(position.walk.continuations.is_empty());
}

#[tokio::test]
async fn resume_continuation_requires_the_control_plane_transition() {
    let mut fixture = fixture();
    let instance = started_run(&mut fixture).await;
    let outcome = fixture.lifecycle.run().await.expect("run");
    assert_eq!(outcome.instance.status, WorkflowInstanceStatus::Paused);
    // No explicit control-plane transition happened: continuing would be
    // auto-resume, which is forbidden.
    let error = fixture
        .lifecycle
        .resume_run(&instance.instance_id)
        .await
        .expect_err("refused without the control-plane transition");
    assert!(matches!(
        error,
        WorkflowAppError::InstanceNotRunning { current, .. }
            if current == WorkflowInstanceStatus::Paused
    ));
    let record = fixture
        .instances
        .load(&instance.instance_id)
        .expect("load record")
        .expect("paused record");
    assert_eq!(record.status, WorkflowInstanceStatus::Paused);
    assert!(
        fixture
            .events
            .events()
            .iter()
            .all(|event| !matches!(event, WorkflowEvent::RunResumed { .. }))
    );
}

#[tokio::test]
async fn a_resumed_run_continues_from_the_persisted_position_and_completes() {
    let mut fixture = fixture();
    let instance = started_run(&mut fixture).await;
    let outcome = fixture.lifecycle.run().await.expect("run");
    assert_eq!(
        outcome.terminal,
        RunTerminal::Paused {
            node: node("wait-signoff"),
            reason: "wait".to_string(),
        }
    );
    // The pause checkpoint holds the pending re-entry point.
    let position = fixture
        .positions
        .position(&instance.instance_id)
        .expect("position persisted at the pause checkpoint");
    assert_eq!(position.walk.current, Some(node("step-002")));
    assert_eq!(
        position.walk.path,
        vec![node("step-001"), node("wait-signoff")]
    );
    // The control plane resumes; the lifecycle continues the walk.
    control_plane_resumes(&mut fixture, &instance.instance_id);
    let outcome = fixture
        .lifecycle
        .resume_run(&instance.instance_id)
        .await
        .expect("resume continuation");
    assert_eq!(outcome.terminal, RunTerminal::Completed);
    assert_eq!(outcome.instance.status, WorkflowInstanceStatus::Succeeded);
    assert_eq!(
        outcome.path,
        vec![node("step-001"), node("wait-signoff"), node("step-002")]
    );
    // The continuation was observable and the terminal settlement
    // discarded the position.
    assert!(fixture.events.events().iter().any(
        |event| matches!(event, WorkflowEvent::RunResumed { node: Some(pending), .. }
            if *pending == node("step-002"))
    ));
    assert!(fixture.positions.position(&instance.instance_id).is_none());
}

#[tokio::test]
async fn resume_run_refuses_while_an_active_run_is_held() {
    let mut fixture = fixture();
    let instance = started_run(&mut fixture).await;
    // The lifecycle still holds the active run (nothing ran or settled).
    let error = fixture
        .lifecycle
        .resume_run(&instance.instance_id)
        .await
        .expect_err("refused while an active run is held");
    assert!(
        matches!(error, WorkflowAppError::ActiveRunHeld { .. }),
        "expected ActiveRunHeld, got {error:?}"
    );
}

#[tokio::test]
async fn resume_run_without_a_position_store_is_refused() {
    let mut fixture = fixture_without_positions();
    let record = seeded_instance(WorkflowInstanceStatus::Running);
    fixture
        .instances
        .create(record.clone())
        .expect("create record");
    let error = fixture
        .lifecycle
        .resume_run(&record.instance_id)
        .await
        .expect_err("refused without the position seam");
    assert!(
        matches!(error, WorkflowAppError::RunPositionUnavailable { .. }),
        "expected RunPositionUnavailable, got {error:?}"
    );
}

#[tokio::test]
async fn resume_run_requires_a_persisted_position() {
    let mut fixture = fixture();
    let record = seeded_instance(WorkflowInstanceStatus::Running);
    fixture
        .instances
        .create(record.clone())
        .expect("create record");
    let error = fixture
        .lifecycle
        .resume_run(&record.instance_id)
        .await
        .expect_err("refused without a persisted position");
    assert!(
        matches!(error, WorkflowAppError::RunPositionUnavailable { .. }),
        "expected RunPositionUnavailable, got {error:?}"
    );
}

#[tokio::test]
async fn a_position_pinning_a_different_version_is_refused() {
    let mut fixture = fixture();
    let record = seeded_instance(WorkflowInstanceStatus::Running);
    fixture
        .instances
        .create(record.clone())
        .expect("create record");
    let mut position = RunPosition {
        instance: record.instance_id,
        version: WorkflowVersionId::from_digest(
            ContentDigest::of(&serde_json::json!({ "workflow": "other" })).expect("digest"),
        ),
        walk: WalkPosition::default(),
        decisions: BTreeMap::new(),
        policy: BindingPolicy::default(),
        max_steps: 100,
        resources: Vec::new(),
    };
    // Keyed by the record's instance identity but pinning another version.
    fixture
        .positions
        .save(position.clone())
        .expect("persist position");
    let error = fixture
        .lifecycle
        .resume_run(&record.instance_id)
        .await
        .expect_err("refused on the pin mismatch");
    assert!(
        matches!(error, WorkflowAppError::RunPositionUnavailable { .. }),
        "expected RunPositionUnavailable, got {error:?}"
    );
    position.version = record.version.clone();
    fixture.positions.save(position).expect("persist position");
    // With the pin matching, the refusal point moves past the position
    // (the version record is not in the store yet).
    let error = fixture
        .lifecycle
        .resume_run(&record.instance_id)
        .await
        .expect_err("refused on the missing version record");
    assert!(
        matches!(error, WorkflowAppError::VersionUnavailable { .. }),
        "expected VersionUnavailable, got {error:?}"
    );
}

#[tokio::test]
async fn reconcile_startup_transitions_orphaned_running_records_to_paused() {
    let mut fixture = fixture();
    let orphan = seeded_instance(WorkflowInstanceStatus::Running);
    fixture
        .instances
        .create(orphan.clone())
        .expect("create orphan");
    let reconciled = fixture
        .lifecycle
        .reconcile_startup(&fixture.instances.records())
        .expect("sweep");
    assert_eq!(reconciled, vec![orphan.instance_id]);
    let recovered = fixture
        .instances
        .load(&orphan.instance_id)
        .expect("load record")
        .expect("reconciled record");
    assert_eq!(recovered.status, WorkflowInstanceStatus::Paused);
    assert_eq!(recovered.evidence.len(), 1, "the recovery trace");
    assert!(
        fixture
            .events
            .events()
            .iter()
            .any(|event| matches!(event, WorkflowEvent::InstanceReconciled { .. }))
    );
}

#[tokio::test]
async fn the_reconciliation_sweep_is_idempotent() {
    let mut fixture = fixture();
    let orphan = seeded_instance(WorkflowInstanceStatus::Running);
    fixture
        .instances
        .create(orphan.clone())
        .expect("create orphan");
    let first = fixture
        .lifecycle
        .reconcile_startup(&fixture.instances.records())
        .expect("first sweep");
    assert_eq!(first, vec![orphan.instance_id]);
    let second = fixture
        .lifecycle
        .reconcile_startup(&fixture.instances.records())
        .expect("second sweep");
    assert!(second.is_empty(), "the second sweep is a no-op");
    let recovered = fixture
        .instances
        .load(&orphan.instance_id)
        .expect("load record")
        .expect("reconciled record");
    assert_eq!(recovered.status, WorkflowInstanceStatus::Paused);
    assert_eq!(
        recovered.evidence.len(),
        1,
        "no duplicate recovery evidence from the second sweep"
    );
    assert_eq!(
        fixture
            .events
            .events()
            .iter()
            .filter(|event| matches!(event, WorkflowEvent::InstanceReconciled { .. }))
            .count(),
        1
    );
}

#[tokio::test]
async fn reconciliation_leaves_non_running_records_untouched() {
    let mut fixture = fixture();
    let mut records = Vec::new();
    for status in WorkflowInstanceStatus::ALL {
        records.push(seeded_instance(status));
    }
    for record in &records {
        fixture.instances.create(record.clone()).expect("create");
    }
    let reconciled = fixture
        .lifecycle
        .reconcile_startup(&fixture.instances.records())
        .expect("sweep");
    // Exactly the `Running` record reconciled; every other status keeps
    // its record and its (empty) evidence untouched.
    assert_eq!(reconciled.len(), 1);
    for record in &records {
        let stored = fixture
            .instances
            .load(&record.instance_id)
            .expect("load record")
            .expect("stored record");
        if record.status == WorkflowInstanceStatus::Running {
            assert_eq!(stored.status, WorkflowInstanceStatus::Paused);
            assert_eq!(stored.evidence.len(), 1);
        } else {
            assert_eq!(stored.status, record.status);
            assert!(stored.evidence.is_empty());
        }
    }
}

#[tokio::test]
async fn cancel_discards_the_persisted_position() {
    let mut fixture = fixture();
    let instance = started_run(&mut fixture).await;
    assert!(
        fixture.positions.position(&instance.instance_id).is_some(),
        "the start checkpoint persisted the position"
    );
    let cancelled = fixture
        .lifecycle
        .cancel(&instance.instance_id, "operator stop")
        .expect("cancel");
    assert_eq!(cancelled.status, WorkflowInstanceStatus::Cancelled);
    assert!(
        fixture.positions.position(&instance.instance_id).is_none(),
        "a terminal record must never look resumable"
    );
}
