//! Shared E2E scaffolding for the durable control-plane stores.
//!
//! Every scenario is static: the durable stores under test plus the
//! same in-memory host doubles the WO-010/WO-011 E2E suites use (no
//! network, browser, or desktop runtime). The helpers mirror the
//! fixtures of those suites — directly-sealed versions over a
//! capability-free step IR and a wait IR — so the durable runs drive
//! the identical planes the in-memory baselines drive.

#![allow(clippy::expect_used)]
#![allow(clippy::unwrap_used)]
// Shared between two integration-test binaries, each of which uses only
// part of this scaffolding.
#![allow(dead_code)]

use std::collections::BTreeMap;
use std::path::Path;
use std::path::PathBuf;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use codex_execution_contracts::CapabilityRegistry;
use codex_workflow_app::InMemoryApprovalSource;
use codex_workflow_app::InMemoryEvidenceStore;
use codex_workflow_app::InMemoryInstanceStore;
use codex_workflow_app::InMemoryVersionStore;
use codex_workflow_app::LifecycleDeps;
use codex_workflow_app::RecordingEventSink;
use codex_workflow_app::ScriptedActionSource;
use codex_workflow_app::WorkflowLifecycle;
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
use codex_workflow_contracts::WorkflowIr;
use codex_workflow_contracts::WorkflowIrNode;
use codex_workflow_contracts::WorkflowRepositoryId;
use codex_workflow_contracts::WorkflowVersion;
use codex_workflow_durable::DurableStores;
use codex_workflow_forge::InstallRegistry;
use codex_workflow_triggers::FixedClock;
use codex_workflow_triggers::InMemoryInstallationStore;
use codex_workflow_triggers::InMemoryInstanceControl;
use codex_workflow_triggers::InMemoryPackageCatalog;
use codex_workflow_triggers::InMemoryResourceAuthorizer;
use codex_workflow_triggers::InMemoryTriggerLedger;
use codex_workflow_triggers::IncomingTrigger;
use codex_workflow_triggers::InstallRequest;
use codex_workflow_triggers::TriggerBinding;
use codex_workflow_triggers::TriggerDeps;
use codex_workflow_triggers::TriggerEventKey;
use codex_workflow_triggers::WorkflowTriggerPlane;

/// A pinned immutable source revision for directly-sealed versions.
fn pinned_revision() -> ImmutableSourceRevision {
    ImmutableSourceRevision::pin(
        RevisionSha::parse("0123456789abcdef0123456789abcdef01234567").expect("revision sha"),
        DevelopmentRef::branch("main").ok(),
    )
}

/// The shared test repository identity.
fn repository_id() -> WorkflowRepositoryId {
    WorkflowRepositoryId::parse("https://github.com/acme/release-bot").expect("repository id")
}

/// A single-step IR with no declared capabilities: the run completes
/// without any adapter.
pub fn step_ir() -> WorkflowIr {
    let entry = IrNodeId::parse("step-001").expect("node id");
    let mut nodes = BTreeMap::new();
    nodes.insert(
        entry.clone(),
        WorkflowIrNode::Step(StepNode {
            capabilities: Vec::new(),
            roles: Vec::new(),
            description: Some("prepare the report".to_string()),
            next: None,
        }),
    );
    WorkflowIr {
        ir_format: IR_FORMAT_VERSION,
        entry,
        nodes,
        conditions: BTreeMap::new(),
    }
}

/// A step followed by a wait for a human trigger: the run pauses at the
/// wait node.
pub fn wait_ir() -> WorkflowIr {
    let entry = IrNodeId::parse("step-001").expect("node id");
    let mut nodes = BTreeMap::new();
    nodes.insert(
        entry.clone(),
        WorkflowIrNode::Step(StepNode {
            capabilities: Vec::new(),
            roles: Vec::new(),
            description: Some("prepare the report".to_string()),
            next: Some(IrNodeId::parse("wait-signoff").expect("node id")),
        }),
    );
    nodes.insert(
        IrNodeId::parse("wait-signoff").expect("node id"),
        WorkflowIrNode::Wait(WaitNode {
            wait_for: WaitFor::Trigger(TriggerClass::HumanEvent),
            next: None,
        }),
    );
    WorkflowIr {
        ir_format: IR_FORMAT_VERSION,
        entry,
        nodes,
        conditions: BTreeMap::new(),
    }
}

/// Seals the single-step workflow declaring the User trigger.
pub fn sealed_step_version(id: &str, version: (u64, u64, u64)) -> WorkflowVersion {
    seal(id, step_ir(), vec![TriggerClass::User], version)
}

/// Seals the wait workflow declaring both the start and the resume
/// trigger classes.
pub fn sealed_wait_version(id: &str, version: (u64, u64, u64)) -> WorkflowVersion {
    seal(
        id,
        wait_ir(),
        vec![TriggerClass::User, TriggerClass::HumanEvent],
        version,
    )
}

/// The shared seal primitive over one IR and declared triggers.
fn seal(
    id: &str,
    ir: WorkflowIr,
    triggers: Vec<TriggerClass>,
    version: (u64, u64, u64),
) -> WorkflowVersion {
    let definition = WorkflowDefinition {
        id: WorkflowDefinitionId::parse(id).expect("definition id"),
        description: None,
        ir,
        roles: BTreeMap::new(),
        triggers: triggers
            .into_iter()
            .map(|trigger| codex_workflow_contracts::WorkflowTrigger {
                trigger,
                description: None,
            })
            .collect(),
        dependencies: WorkflowDependencies::default(),
    };
    WorkflowVersion::seal(
        definition,
        repository_id(),
        pinned_revision(),
        SemanticVersion::new(version.0, version.1, version.2),
        DependencyLock::default(),
        None,
    )
    .expect("sealed version")
}

/// A direct user trigger envelope for `workflow` with `key`.
pub fn user_event(workflow: &str, key: &str) -> IncomingTrigger {
    IncomingTrigger {
        trigger: TriggerClass::User,
        key: TriggerEventKey::parse(key).expect("event key"),
        occurred_at_unix_ms: 0,
        source: "user:test".to_string(),
        routing: None,
        target: Some(WorkflowDefinitionId::parse(workflow).expect("definition id")),
        payload_digest: None,
    }
}

/// An external trigger envelope for a human event with `key`.
pub fn human_event(key: &str) -> IncomingTrigger {
    IncomingTrigger {
        trigger: TriggerClass::HumanEvent,
        key: TriggerEventKey::parse(key).expect("event key"),
        occurred_at_unix_ms: 0,
        source: "human:signoff".to_string(),
        routing: None,
        target: None,
        payload_digest: None,
    }
}

/// An install request binding the direct user and human-event
/// triggers.
pub fn install_request(version: WorkflowVersion) -> InstallRequest {
    InstallRequest {
        version,
        resources: Vec::new(),
        dependencies: Vec::new(),
        trigger_bindings: vec![
            TriggerBinding::direct(TriggerClass::User),
            TriggerBinding::direct(TriggerClass::HumanEvent),
        ],
        schedules: Vec::new(),
        policy: codex_execution_contracts::BindingPolicy::default(),
        max_walk_steps: 1_000,
    }
}

/// A unique scratch root directory for the scenario labeled `label`.
pub fn temp_root(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0u128, |elapsed| elapsed.as_nanos());
    let root = std::env::temp_dir().join(format!(
        "codex-workflow-durable-e2e-{}-{label}-{nanos}",
        std::process::id()
    ));
    std::fs::create_dir_all(&root).expect("create the scratch root");
    root
}

/// Best-effort removal of a scratch directory.
pub fn cleanup(root: &Path) {
    let _ = std::fs::remove_dir_all(root);
}

/// The in-memory baseline harness: a lifecycle and trigger plane over
/// the same in-memory doubles the WO-010/WO-011 E2E suites use.
pub struct MemoryHarness {
    /// The trigger plane.
    pub plane: WorkflowTriggerPlane,
    /// The workflow lifecycle.
    pub lifecycle: WorkflowLifecycle,
    /// The in-memory version store (audit handle).
    pub versions: InMemoryVersionStore,
    /// The in-memory instance store (audit handle).
    pub instances: InMemoryInstanceStore,
    /// The in-memory evidence store (audit handle).
    pub evidence: InMemoryEvidenceStore,
    /// The in-memory trigger ledger (audit handle).
    pub ledger: InMemoryTriggerLedger,
    /// The in-memory installation store (audit handle).
    pub installations: InMemoryInstallationStore,
}

/// Builds the in-memory baseline harness.
pub fn memory_harness() -> MemoryHarness {
    let versions = InMemoryVersionStore::new();
    let instances = InMemoryInstanceStore::new();
    let evidence = InMemoryEvidenceStore::new();
    let approvals = InMemoryApprovalSource::approving("tech-lead", evidence.clone());
    let actions = ScriptedActionSource::new(Vec::new());
    let events = RecordingEventSink::new();
    let lifecycle = WorkflowLifecycle::new(LifecycleDeps {
        versions: Box::new(versions.clone()),
        instances: Box::new(instances.clone()),
        evidence: Box::new(evidence.clone()),
        approvals: Box::new(approvals),
        actions: Box::new(actions),
        events: Box::new(events),
        registry: CapabilityRegistry::new(),
    });
    let ledger = InMemoryTriggerLedger::new();
    let installations = InMemoryInstallationStore::new();
    let control = InMemoryInstanceControl::new(instances.clone());
    let plane = WorkflowTriggerPlane::new(TriggerDeps {
        ledger: Box::new(ledger.clone()),
        catalog: Box::new(InMemoryPackageCatalog::new()),
        authorizer: Box::new(InMemoryResourceAuthorizer::new()),
        control: Box::new(control),
        installations: Box::new(installations.clone()),
        clock: Box::new(FixedClock::new(1_000)),
        versions: Box::new(versions.clone()),
        installs: InstallRegistry::new(),
    });
    MemoryHarness {
        plane,
        lifecycle,
        versions,
        instances,
        evidence,
        ledger,
        installations,
    }
}

/// The durable harness: a lifecycle and trigger plane over the durable
/// stores, with the durable ports boxed in place of the in-memory
/// doubles.
///
/// The approval source stays the in-memory double (approval durability
/// is outside MWO-001's six ports, and the double is hard-wired to the
/// in-memory evidence store type); the catalog, authorizer, and clock
/// stay host-owned in-memory doubles exactly as in the baseline.
pub struct DurableHarness {
    /// The trigger plane.
    pub plane: WorkflowTriggerPlane,
    /// The workflow lifecycle.
    pub lifecycle: WorkflowLifecycle,
    /// The durable stores (audit handles).
    pub stores: DurableStores,
}

/// Builds the durable harness over freshly opened stores at `root`.
pub fn durable_harness(root: &Path) -> DurableHarness {
    let stores = DurableStores::open(root).expect("open durable stores");
    harness_over(stores)
}

/// Wires a lifecycle and trigger plane over already-opened durable
/// stores.
pub fn harness_over(stores: DurableStores) -> DurableHarness {
    let approvals = InMemoryApprovalSource::approving("tech-lead", InMemoryEvidenceStore::new());
    let actions = ScriptedActionSource::new(Vec::new());
    let events = RecordingEventSink::new();
    let lifecycle = WorkflowLifecycle::new(LifecycleDeps {
        versions: Box::new(stores.versions.clone()),
        instances: Box::new(stores.instances.clone()),
        evidence: Box::new(stores.evidence.clone()),
        approvals: Box::new(approvals),
        actions: Box::new(actions),
        events: Box::new(events),
        registry: CapabilityRegistry::new(),
    });
    let plane = WorkflowTriggerPlane::new(TriggerDeps {
        ledger: Box::new(stores.ledger.clone()),
        catalog: Box::new(InMemoryPackageCatalog::new()),
        authorizer: Box::new(InMemoryResourceAuthorizer::new()),
        control: Box::new(stores.control.clone()),
        installations: Box::new(stores.installations.clone()),
        clock: Box::new(FixedClock::new(1_000)),
        versions: Box::new(stores.versions.clone()),
        installs: InstallRegistry::new(),
    });
    DurableHarness {
        plane,
        lifecycle,
        stores,
    }
}
