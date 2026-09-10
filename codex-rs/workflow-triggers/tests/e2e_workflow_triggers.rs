//! WO-011 end-to-end tests: scheduling, triggers, sharing, and
//! installation.
//!
//! Every scenario is **static**: in-memory doubles stand in for the host
//! seams (forge, catalog, authorization, instance control, browser and
//! desktop runtimes), so no network, browser, or desktop runtime is
//! touched. The scenarios map to the Work Order's acceptance bullets:
//!
//! - discovery -> install -> bind -> fire -> run -> verify (the golden
//!   path, over the full teaching/publication pipeline);
//! - trigger idempotency: a double fire is one instance transition plus
//!   a recorded no-op;
//! - scheduling: due occurrences fire, a re-polled (crashed) scheduler
//!   dedupes, eligibility gating blocks unauthorized fires, and daily
//!   schedules fire at their UTC boundary;
//! - install/bind diagnostics: missing resources, unauthorized bindings,
//!   dependency drift, undeclared triggers;
//! - authorization checks at install and fire time;
//! - version integrity: sealed versions install, tampered records never
//!   fire;
//! - rebinding without source change;
//! - resume and retry pin the immutable version;
//! - routing across all eight trigger classes;
//! - explicit updates never fire silently;
//! - ordinary Codex compatibility: with nothing installed, a fire is an
//!   inert, recorded rejection.
//!
//! The workspace clippy.toml allows `expect`/`unwrap` in test code.
#![allow(clippy::expect_used)]
#![allow(clippy::unwrap_used)]

use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;

use codex_browser_use_adapter::AllowDeny;
use codex_browser_use_adapter::BridgeProbe;
use codex_browser_use_adapter::BrowserAction;
use codex_browser_use_adapter::BrowserActionKind;
use codex_browser_use_adapter::BrowserActionOutcome;
use codex_browser_use_adapter::BrowserActionResult;
use codex_browser_use_adapter::BrowserObservation;
use codex_browser_use_adapter::BrowserSessionIdentity;
use codex_browser_use_adapter::BrowserUseConfigSnapshot;
use codex_browser_use_adapter::BrowserUseRequirement;
use codex_browser_use_adapter::OriginPolicyRequirement;
use codex_browser_use_adapter::OriginPolicySnapshot;
use codex_computer_use_adapter::AccessRequirement;
use codex_computer_use_adapter::BridgeCallFailure;
use codex_computer_use_adapter::BridgeIdentity;
use codex_computer_use_adapter::BridgeReadinessFailure;
use codex_computer_use_adapter::ComputerUseBridge;
use codex_computer_use_adapter::ComputerUseBridgeFactory;
use codex_computer_use_adapter::ComputerUsePolicy;
use codex_computer_use_adapter::NormalizedAction;
use codex_computer_use_adapter::NormalizedActionResult;
use codex_computer_use_adapter::ScreenIdentity;
use codex_computer_use_adapter::ScreenObservation;
use codex_computer_use_adapter::UnixMsClock;
use codex_execution_contracts::Action;
use codex_execution_contracts::ActionInputs;
use codex_execution_contracts::ActionTarget;
use codex_execution_contracts::CapabilityRegistry;
use codex_execution_contracts::EnvironmentAdapter;
use codex_execution_contracts::ExecutionEnvironment;
use codex_execution_contracts::OperationId;
use codex_execution_contracts::ResourceBinding;
use codex_execution_contracts::ResourceId;
use codex_teaching_compiler::ApprovalDecision;
use codex_teaching_compiler::ApprovalDecisionKind;
use codex_teaching_compiler::SimulationConfig;
use codex_teaching_compiler::TeachingMode;
use codex_teaching_compiler::TeachingSession;
use codex_teaching_compiler::TrajectoryEvent;
use codex_teaching_compiler::compile;
use codex_workflow_app::BROWSER_CAPABILITY;
use codex_workflow_app::BrowserActionExecutor;
use codex_workflow_app::BrowserTurn;
use codex_workflow_app::BrowserUseEnvironmentAdapter;
use codex_workflow_app::COMPUTER_CAPABILITY;
use codex_workflow_app::ComputerUseEnvironmentAdapter;
use codex_workflow_app::InMemoryApprovalSource;
use codex_workflow_app::InMemoryEvidenceStore;
use codex_workflow_app::InMemoryInstanceStore;
use codex_workflow_app::InMemoryVersionStore;
use codex_workflow_app::LifecycleDeps;
use codex_workflow_app::PublishRequest;
use codex_workflow_app::PublishedArtifact;
use codex_workflow_app::RecordingEventSink;
use codex_workflow_app::ScriptedAction;
use codex_workflow_app::ScriptedActionSource;
use codex_workflow_app::WorkflowLifecycle;
use codex_workflow_app::WorkflowVersionStore;
use codex_workflow_app::publish;
use codex_workflow_contracts::Attribution;
use codex_workflow_contracts::CapabilityId;
use codex_workflow_contracts::CapabilityRequirement;
use codex_workflow_contracts::ContentDigest;
use codex_workflow_contracts::DependencyKey;
use codex_workflow_contracts::DependencyLock;
use codex_workflow_contracts::DevelopmentRef;
use codex_workflow_contracts::ForgeKind;
use codex_workflow_contracts::IR_FORMAT_VERSION;
use codex_workflow_contracts::ImmutableSourceRevision;
use codex_workflow_contracts::IrNodeId;
use codex_workflow_contracts::MANIFEST_FORMAT_VERSION;
use codex_workflow_contracts::RepositoryRelativePath;
use codex_workflow_contracts::ResolvedDependency;
use codex_workflow_contracts::ResolvedDependencyIdentity;
use codex_workflow_contracts::ResourceRequirement;
use codex_workflow_contracts::ResourceTypeId;
use codex_workflow_contracts::RevisionSha;
use codex_workflow_contracts::SemanticVersion;
use codex_workflow_contracts::SkillDependency;
use codex_workflow_contracts::SkillName;
use codex_workflow_contracts::StepNode;
use codex_workflow_contracts::TriggerClass;
use codex_workflow_contracts::WaitFor;
use codex_workflow_contracts::WaitNode;
use codex_workflow_contracts::WorkflowDefinition;
use codex_workflow_contracts::WorkflowDefinitionId;
use codex_workflow_contracts::WorkflowDependencies;
use codex_workflow_contracts::WorkflowInstanceStatus;
use codex_workflow_contracts::WorkflowIr;
use codex_workflow_contracts::WorkflowIrNode;
use codex_workflow_contracts::WorkflowManifest;
use codex_workflow_contracts::WorkflowProvenance;
use codex_workflow_contracts::WorkflowRepository;
use codex_workflow_contracts::WorkflowRepositoryId;
use codex_workflow_contracts::WorkflowTrigger;
use codex_workflow_contracts::WorkflowVersion;
use codex_workflow_forge::InMemoryForge;
use codex_workflow_forge::InstallRegistry;
use codex_workflow_forge::PublishedVersionRef;
use codex_workflow_forge::UpdateDecision;
use codex_workflow_forge::WorkflowForgeError;
use codex_workflow_triggers::DependencyBinding;
use codex_workflow_triggers::FireOutcome;
use codex_workflow_triggers::FixedClock;
use codex_workflow_triggers::InMemoryInstallationStore;
use codex_workflow_triggers::InMemoryInstanceControl;
use codex_workflow_triggers::InMemoryPackageCatalog;
use codex_workflow_triggers::InMemoryResourceAuthorizer;
use codex_workflow_triggers::InMemoryTriggerLedger;
use codex_workflow_triggers::IncomingTrigger;
use codex_workflow_triggers::InstallRequest;
use codex_workflow_triggers::InstallationStore;
use codex_workflow_triggers::InstalledConfiguration;
use codex_workflow_triggers::InstanceSettlement;
use codex_workflow_triggers::PackageQuery;
use codex_workflow_triggers::ScheduleSpec;
use codex_workflow_triggers::TriggerBinding;
use codex_workflow_triggers::TriggerDeps;
use codex_workflow_triggers::TriggerDiagnosticCode;
use codex_workflow_triggers::TriggerEventKey;
use codex_workflow_triggers::TriggerLedger;
use codex_workflow_triggers::TriggerReport;
use codex_workflow_triggers::WorkflowTriggerError;
use codex_workflow_triggers::WorkflowTriggerPlane;
use pretty_assertions::assert_eq;

const BROWSER_PROFILE: &str = "browser_profile";

fn node_id(value: &str) -> IrNodeId {
    IrNodeId::parse(value).expect("node id")
}

fn definition_id() -> WorkflowDefinitionId {
    WorkflowDefinitionId::parse("triage-report").expect("definition id")
}

fn repository_id() -> WorkflowRepositoryId {
    WorkflowRepositoryId::parse("https://github.com/acme/ops-workflows").expect("repository id")
}

fn capability(value: &str) -> CapabilityId {
    CapabilityId::parse(value).expect("capability id")
}

fn resource_type(value: &str) -> ResourceTypeId {
    ResourceTypeId::parse(value).expect("resource type")
}

fn resource_binding(resource_type: &str, resource: &str) -> ResourceBinding {
    ResourceBinding {
        resource_type: ResourceTypeId::parse(resource_type).expect("resource type"),
        resource: ResourceId::parse(resource).expect("resource"),
        holder: ExecutionEnvironment::Browser,
    }
}

/// A pinned immutable source revision for directly-sealed versions.
fn pinned_revision() -> ImmutableSourceRevision {
    ImmutableSourceRevision::pin(
        RevisionSha::parse(format!("{:040}", 0xabc)).expect("revision sha"),
        Some(DevelopmentRef::branch("main").expect("branch")),
    )
}

/// A single-step IR with no declared capabilities: the walk completes
/// without any adapter.
fn step_ir() -> WorkflowIr {
    let mut nodes = BTreeMap::new();
    nodes.insert(
        node_id("step-001"),
        WorkflowIrNode::Step(StepNode {
            capabilities: Vec::new(),
            roles: Vec::new(),
            description: Some("prepare the triage report".to_string()),
            next: None,
        }),
    );
    WorkflowIr {
        ir_format: IR_FORMAT_VERSION,
        entry: node_id("step-001"),
        nodes,
        conditions: BTreeMap::new(),
    }
}

/// A step followed by a wait for a human trigger: the run pauses at the
/// wait node.
fn wait_ir() -> WorkflowIr {
    let mut nodes = BTreeMap::new();
    nodes.insert(
        node_id("step-001"),
        WorkflowIrNode::Step(StepNode {
            capabilities: Vec::new(),
            roles: Vec::new(),
            description: Some("prepare the triage report".to_string()),
            next: Some(node_id("wait-signoff")),
        }),
    );
    nodes.insert(
        node_id("wait-signoff"),
        WorkflowIrNode::Wait(WaitNode {
            wait_for: WaitFor::Trigger(TriggerClass::HumanEvent),
            next: None,
        }),
    );
    WorkflowIr {
        ir_format: IR_FORMAT_VERSION,
        entry: node_id("step-001"),
        nodes,
        conditions: BTreeMap::new(),
    }
}

/// Seals a control-plane-authored definition directly through the frozen
/// contract (the same `WorkflowVersion::seal` primitive the publication
/// pipeline ends with), with declared triggers and dependencies.
fn sealed_version(
    id: &str,
    version: (u64, u64, u64),
    triggers: Vec<TriggerClass>,
    dependencies: WorkflowDependencies,
    lock: DependencyLock,
) -> WorkflowVersion {
    let definition = WorkflowDefinition {
        id: WorkflowDefinitionId::parse(id).expect("definition id"),
        description: None,
        ir: step_ir(),
        roles: BTreeMap::new(),
        triggers: triggers
            .into_iter()
            .map(|trigger| WorkflowTrigger {
                trigger,
                description: None,
            })
            .collect(),
        dependencies,
    };
    WorkflowVersion::seal(
        definition,
        repository_id(),
        pinned_revision(),
        SemanticVersion::new(version.0, version.1, version.2),
        lock,
        None,
    )
    .expect("sealed version")
}

fn user_event(workflow: &WorkflowDefinitionId, key: &str) -> IncomingTrigger {
    IncomingTrigger {
        trigger: TriggerClass::User,
        key: TriggerEventKey::parse(key).expect("key"),
        occurred_at_unix_ms: 0,
        source: "user:test".to_string(),
        routing: None,
        target: Some(workflow.clone()),
        payload_digest: None,
    }
}

fn external_event(trigger: TriggerClass, key: &str, endpoint: Option<&str>) -> IncomingTrigger {
    IncomingTrigger {
        trigger,
        key: TriggerEventKey::parse(key).expect("key"),
        occurred_at_unix_ms: 0,
        source: format!("external:{key}"),
        routing: endpoint.map(str::to_string),
        target: None,
        payload_digest: Some("ab".repeat(32)),
    }
}

fn install_request(version: WorkflowVersion) -> InstallRequest {
    InstallRequest {
        version,
        resources: Vec::new(),
        dependencies: Vec::new(),
        trigger_bindings: vec![TriggerBinding::direct(TriggerClass::User)],
        schedules: Vec::new(),
        policy: codex_execution_contracts::BindingPolicy::default(),
        max_walk_steps: 1_000,
    }
}

/// The test harness: a trigger plane plus a WO-010 lifecycle over
/// in-memory seams, with the handles needed to observe every plane.
struct Harness {
    plane: WorkflowTriggerPlane,
    lifecycle: WorkflowLifecycle,
    versions: InMemoryVersionStore,
    instances: InMemoryInstanceStore,
    evidence: InMemoryEvidenceStore,
    approvals: InMemoryApprovalSource,
    events: RecordingEventSink,
    ledger: InMemoryTriggerLedger,
    authorizer: InMemoryResourceAuthorizer,
    installations: InMemoryInstallationStore,
    control: InMemoryInstanceControl,
    clock: FixedClock,
}

fn harness(
    action_script: Vec<ScriptedAction>,
    adapters: Vec<Arc<dyn EnvironmentAdapter>>,
) -> Harness {
    let versions = InMemoryVersionStore::new();
    let instances = InMemoryInstanceStore::new();
    let evidence = InMemoryEvidenceStore::new();
    let approvals = InMemoryApprovalSource::approving("tech-lead", evidence.clone());
    let actions = ScriptedActionSource::new(action_script);
    let events = RecordingEventSink::new();
    let mut registry = CapabilityRegistry::new();
    for adapter in adapters {
        registry
            .register_adapter(adapter)
            .expect("register adapter");
    }
    let lifecycle = WorkflowLifecycle::new(LifecycleDeps {
        versions: Box::new(versions.clone()),
        instances: Box::new(instances.clone()),
        evidence: Box::new(evidence.clone()),
        approvals: Box::new(approvals.clone()),
        actions: Box::new(actions),
        events: Box::new(events.clone()),
        registry,
    });
    let ledger = InMemoryTriggerLedger::new();
    let authorizer = InMemoryResourceAuthorizer::new();
    let installations = InMemoryInstallationStore::new();
    let control = InMemoryInstanceControl::new(instances.clone());
    let clock = FixedClock::new(1_000);
    let plane = WorkflowTriggerPlane::new(TriggerDeps {
        ledger: Box::new(ledger.clone()),
        catalog: Box::new(InMemoryPackageCatalog::new()),
        authorizer: Box::new(authorizer.clone()),
        control: Box::new(control.clone()),
        installations: Box::new(installations.clone()),
        clock: Box::new(clock.clone()),
        versions: Box::new(versions.clone()),
        installs: InstallRegistry::new(),
    });
    Harness {
        plane,
        lifecycle,
        versions,
        instances,
        evidence,
        approvals,
        events,
        ledger,
        authorizer,
        installations,
        control,
        clock,
    }
}

/// Installs a sealed version with a User trigger binding.
fn install_user(harness: &mut Harness, version: WorkflowVersion) -> InstalledConfiguration {
    harness
        .plane
        .install(install_request(version))
        .expect("install")
}

fn assert_single_started(reports: &[TriggerReport]) -> &TriggerReport {
    assert_eq!(reports.len(), 1);
    assert!(matches!(
        &reports[0].outcome,
        FireOutcome::Started {
            status: InstanceSettlement::Completed,
            ..
        }
    ));
    &reports[0]
}

fn diagnostics_codes(outcome: &FireOutcome) -> Vec<TriggerDiagnosticCode> {
    match outcome {
        FireOutcome::NotEligible { diagnostics }
        | FireOutcome::InstantiationFailed { diagnostics } => diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code)
            .collect(),
        other => panic!("expected diagnostics, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Golden path: discovery -> install -> bind -> fire -> run -> verify,
// over the full teaching/publication pipeline.
// ---------------------------------------------------------------------------

/// Teaches the two-step triage workflow (browser step, desktop step) and
/// drives the compiler to an approved, digest-pinned workflow.
fn taught_approved() -> codex_teaching_compiler::ApprovedWorkflow {
    let mut session = TeachingSession::new(TeachingMode::Demonstrate);
    session
        .record(
            codex_teaching_compiler::RecordOrigin::Demonstration,
            TrajectoryEvent::Observation {
                text: "the issue tracker is open".to_string(),
            },
            Vec::new(),
        )
        .expect("record observation");
    session
        .record(
            codex_teaching_compiler::RecordOrigin::Demonstration,
            TrajectoryEvent::Action {
                text: "Open the issues list.".to_string(),
            },
            vec![
                codex_teaching_compiler::TeachingEvidence::new(
                    "open-issues",
                    "rollout://demo/0001",
                    "ab".repeat(32),
                )
                .expect("evidence"),
            ],
        )
        .expect("record action");
    session
        .record(
            codex_teaching_compiler::RecordOrigin::Demonstration,
            TrajectoryEvent::Result {
                text: "the list shows triaged issues".to_string(),
            },
            Vec::new(),
        )
        .expect("record result");
    session
        .record(
            codex_teaching_compiler::RecordOrigin::Demonstration,
            TrajectoryEvent::Action {
                text: "Paste the summary into the tracker app.".to_string(),
            },
            vec![
                codex_teaching_compiler::TeachingEvidence::new(
                    "paste-summary",
                    "rollout://demo/0002",
                    "cd".repeat(32),
                )
                .expect("evidence"),
            ],
        )
        .expect("record action");
    session.close();

    let mut candidate = compile(&session).expect("compile");
    let summary = candidate.validate().expect("validate");
    assert!(summary.is_clean());
    let simulation = candidate
        .simulate(SimulationConfig::default())
        .expect("simulate");
    assert!(simulation.allows_publication());
    candidate
        .approve(
            ApprovalDecision::new("tech-lead", "review-1", ApprovalDecisionKind::Approved)
                .expect("decision"),
        )
        .expect("approve");
    candidate.finalize(definition_id()).expect("finalize")
}

/// Publishes the approved workflow through the git lifecycle (forge
/// repository, commit, ref resolution, seal).
fn published_taught() -> PublishedArtifact {
    let repository = repository_id();
    let mut forge = InMemoryForge::new(ForgeKind::parse("github").expect("forge kind"));
    let manifest = WorkflowManifest {
        manifest_version: MANIFEST_FORMAT_VERSION,
        workflow: definition_id(),
        display_name: Some("triage-report".to_string()),
        description: None,
        repository: repository.clone(),
        definition_path: RepositoryRelativePath::parse("workflows/triage-report.json")
            .expect("definition path"),
        provenance: None,
    };
    let branch = DevelopmentRef::branch("main").expect("branch");
    forge
        .seed_repository(
            repository.clone(),
            branch.clone(),
            vec![Attribution {
                name: "acme-bot".to_string(),
                contact: None,
            }],
            vec![manifest],
        )
        .expect("seed repository");
    forge
        .commit(&repository, &branch, "publish v1.0.0", "acme-bot")
        .expect("commit");
    let revision = forge
        .resolve_ref_blocking(&repository, &branch)
        .expect("resolve ref");
    let step_bindings = BTreeMap::from([
        (
            node_id("step-001"),
            vec![CapabilityRequirement {
                capability: capability(BROWSER_CAPABILITY),
                purpose: Some("open the issues list in the browser".to_string()),
            }],
        ),
        (
            node_id("step-002"),
            vec![CapabilityRequirement {
                capability: capability(COMPUTER_CAPABILITY),
                purpose: Some("paste the summary into the tracker app".to_string()),
            }],
        ),
    ]);
    publish(PublishRequest {
        approved: taught_approved(),
        repository: repository.clone(),
        source_revision: revision,
        semantic_version: SemanticVersion::new(1, 0, 0),
        bindings: step_bindings,
        dependency_lock: DependencyLock::default(),
        provenance: Some(WorkflowProvenance {
            authors: vec![Attribution {
                name: "acme-bot".to_string(),
                contact: None,
            }],
            forked_from: None,
            license: None,
            upgrade_policy: None,
        }),
    })
    .expect("publish")
}

/// A browser turn that succeeds, capturing an observation.
fn success_turn() -> BrowserTurn {
    BrowserTurn {
        result: BrowserActionResult {
            outcome: BrowserActionOutcome::Succeeded,
            detail: Some("stub ok".to_string()),
        },
        observation: Some(BrowserObservation {
            origin: "https://issues.example.com".to_string(),
            url: Some("https://issues.example.com/list".to_string()),
            title: Some("Issues".to_string()),
            tab_ids: vec!["tab-1".to_string()],
            redacted: true,
        }),
        failure_kind: None,
    }
}

/// The scripted browser executor: serves scripted turns in order.
struct ScriptedBrowserExecutor {
    script: Mutex<Vec<BrowserTurn>>,
    actions: Mutex<Vec<BrowserActionKind>>,
}

impl ScriptedBrowserExecutor {
    fn new(script: Vec<BrowserTurn>) -> Self {
        Self {
            script: Mutex::new(script),
            actions: Mutex::new(Vec::new()),
        }
    }

    fn executed_kinds(&self) -> Vec<BrowserActionKind> {
        self.actions.lock().unwrap().clone()
    }
}

impl BrowserActionExecutor for ScriptedBrowserExecutor {
    fn execute(&self, _session: &BrowserSessionIdentity, action: &BrowserAction) -> BrowserTurn {
        self.actions.lock().unwrap().push(action.kind.clone());
        let mut script = self.script.lock().unwrap();
        if script.is_empty() {
            success_turn()
        } else {
            script.remove(0)
        }
    }
}

/// A healthy stub computer-use bridge counting executions.
struct StubComputerBridge {
    calls: Arc<AtomicUsize>,
}

impl ComputerUseBridge for StubComputerBridge {
    fn identity(&self) -> BridgeIdentity {
        BridgeIdentity {
            bridge_kind: "stub-computer-use".to_string(),
            runtime: "node_repl".to_string(),
            version: "1.0.0-test".to_string(),
        }
    }

    fn probe(&self) -> Result<(), BridgeReadinessFailure> {
        Ok(())
    }

    fn execute(
        &self,
        action: NormalizedAction,
    ) -> Result<NormalizedActionResult, BridgeCallFailure> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(NormalizedActionResult::succeeded(&action, 0))
    }

    fn observe(&self) -> Result<ScreenObservation, BridgeCallFailure> {
        Ok(ScreenObservation {
            screen: ScreenIdentity {
                index: 0,
                name: Some("stub-screen".to_string()),
            },
            applications: Vec::new(),
            captured_at_unix_ms: 0,
        })
    }
}

/// Factory provisioning healthy stub bridges.
struct StubComputerFactory {
    calls: Arc<AtomicUsize>,
}

impl ComputerUseBridgeFactory for StubComputerFactory {
    fn provision(&self) -> Result<Box<dyn ComputerUseBridge>, BridgeReadinessFailure> {
        Ok(Box::new(StubComputerBridge {
            calls: self.calls.clone(),
        }))
    }
}

fn browser_requirement() -> BrowserUseRequirement {
    BrowserUseRequirement {
        origins: BTreeMap::from([(
            "https://issues.example.com".to_string(),
            OriginPolicyRequirement {
                access: Some(AllowDeny::Allow),
                ..Default::default()
            },
        )]),
        ..Default::default()
    }
}

fn browser_config() -> BrowserUseConfigSnapshot {
    BrowserUseConfigSnapshot {
        origins: BTreeMap::from([(
            "https://issues.example.com".to_string(),
            OriginPolicySnapshot {
                access: Some(AllowDeny::Allow),
                ..Default::default()
            },
        )]),
        ..Default::default()
    }
}

fn healthy_probe() -> BridgeProbe {
    BridgeProbe {
        native_bridge_present: true,
        native_runtime_provisioned: true,
        detail: None,
    }
}

fn computer_policy(access: AccessRequirement) -> ComputerUsePolicy {
    ComputerUsePolicy {
        default_app_access: Some(access),
        ..Default::default()
    }
}

fn navigate_action(url: &str) -> Action {
    Action {
        operation: OperationId::parse("navigate").expect("operation"),
        target: Some(ActionTarget::parse(url).expect("target")),
        inputs: ActionInputs::default(),
    }
}

fn click_action() -> Action {
    let mut inputs = ActionInputs::default();
    inputs.insert("x", serde_json::json!(120));
    inputs.insert("y", serde_json::json!(40));
    Action {
        operation: OperationId::parse("click").expect("operation"),
        target: Some(ActionTarget::parse("step-002").expect("target")),
        inputs,
    }
}

fn default_action_script() -> Vec<ScriptedAction> {
    vec![
        ScriptedAction::new(
            "step-001",
            BROWSER_CAPABILITY,
            navigate_action("https://issues.example.com"),
        )
        .expect("scripted navigate"),
        ScriptedAction::new("step-002", COMPUTER_CAPABILITY, click_action())
            .expect("scripted click"),
    ]
}

/// The catalog's repository snapshot for the golden path.
fn catalog_repository() -> WorkflowRepository {
    WorkflowRepository {
        identity: repository_id(),
        forge: ForgeKind::parse("github").expect("forge kind"),
        origin: None,
        forked_from: None,
        default_branch: DevelopmentRef::branch("main").expect("branch"),
        branches: Vec::new(),
        workflows: vec![WorkflowManifest {
            manifest_version: MANIFEST_FORMAT_VERSION,
            workflow: definition_id(),
            display_name: Some("triage-report".to_string()),
            description: None,
            repository: repository_id(),
            definition_path: RepositoryRelativePath::parse("workflows/triage-report.json")
                .expect("definition path"),
            provenance: None,
        }],
        maintainers: Vec::new(),
    }
}

/// The golden-path harness with browser/computer adapters and a seeded
/// catalog.
struct GoldenHarness {
    plane: WorkflowTriggerPlane,
    lifecycle: WorkflowLifecycle,
    instances: InMemoryInstanceStore,
    evidence: InMemoryEvidenceStore,
    authorizer: InMemoryResourceAuthorizer,
    catalog: InMemoryPackageCatalog,
    browser_executor: Arc<ScriptedBrowserExecutor>,
    computer_calls: Arc<AtomicUsize>,
}

fn golden_harness() -> GoldenHarness {
    let browser_executor = Arc::new(ScriptedBrowserExecutor::new(vec![success_turn()]));
    let browser = Arc::new(
        BrowserUseEnvironmentAdapter::new(
            browser_requirement(),
            healthy_probe(),
            browser_config(),
            browser_executor.clone(),
        )
        .expect("browser adapter"),
    );
    let computer_calls = Arc::new(AtomicUsize::new(0));
    let computer = Arc::new(
        ComputerUseEnvironmentAdapter::with_clock(
            computer_policy(AccessRequirement::Allow),
            Arc::new(StubComputerFactory {
                calls: computer_calls.clone(),
            }),
            Arc::new(|| 1_000u64) as UnixMsClock,
        )
        .expect("computer adapter"),
    );
    let versions = InMemoryVersionStore::new();
    let instances = InMemoryInstanceStore::new();
    let evidence = InMemoryEvidenceStore::new();
    let approvals = InMemoryApprovalSource::approving("tech-lead", evidence.clone());
    let actions = ScriptedActionSource::new(default_action_script());
    let events = RecordingEventSink::new();
    let mut registry = CapabilityRegistry::new();
    registry
        .register_adapter(browser)
        .expect("register browser");
    registry
        .register_adapter(computer)
        .expect("register computer");
    let lifecycle = WorkflowLifecycle::new(LifecycleDeps {
        versions: Box::new(versions.clone()),
        instances: Box::new(instances.clone()),
        evidence: Box::new(evidence.clone()),
        approvals: Box::new(approvals),
        actions: Box::new(actions),
        events: Box::new(events),
        registry,
    });
    let ledger = InMemoryTriggerLedger::new();
    let authorizer = InMemoryResourceAuthorizer::new();
    let catalog = InMemoryPackageCatalog::with_repositories(vec![catalog_repository()]);
    let control = InMemoryInstanceControl::new(instances.clone());
    let plane = WorkflowTriggerPlane::new(TriggerDeps {
        ledger: Box::new(ledger),
        catalog: Box::new(catalog.clone()),
        authorizer: Box::new(authorizer.clone()),
        control: Box::new(control),
        installations: Box::new(InMemoryInstallationStore::new()),
        clock: Box::new(FixedClock::new(1_000)),
        versions: Box::new(versions),
        installs: InstallRegistry::new(),
    });
    GoldenHarness {
        plane,
        lifecycle,
        instances,
        evidence,
        authorizer,
        catalog,
        browser_executor,
        computer_calls,
    }
}

#[tokio::test]
async fn e2e_discover_install_fire_user_run_and_verify() {
    // Publish through the real teaching/publication pipeline.
    let artifact = published_taught();
    artifact.version.verify_integrity().expect("integrity");

    // Discover the package through the catalog seam (the WO-009
    // repository discovery semantics underneath).
    let mut golden = golden_harness();
    golden.catalog.publish(artifact.version.clone());
    let listings = golden
        .plane
        .discover(&PackageQuery {
            name_contains: Some("triage".to_string()),
            ..PackageQuery::default()
        })
        .expect("discover");
    assert_eq!(listings.len(), 1);
    assert_eq!(listings[0].workflow, definition_id());
    assert_eq!(
        listings[0].versions,
        vec![PublishedVersionRef::of(&artifact.version)]
    );

    // Fetch the sealed record and install it with explicit, authorized
    // resource bindings (adapter-side types for the browser/computer
    // environments) and a User trigger binding.
    let fetched = golden
        .plane
        .fetch_version(&artifact.version.version_id)
        .expect("fetch")
        .expect("version present");
    golden
        .authorizer
        .allow(BROWSER_PROFILE, "profile-default", "codex-connectors:test");
    golden
        .authorizer
        .allow("desktop_session", "tracker-app", "codex-connectors:test");
    let configuration = golden
        .plane
        .install(InstallRequest {
            version: fetched,
            resources: vec![
                resource_binding(BROWSER_PROFILE, "profile-default"),
                ResourceBinding {
                    resource_type: resource_type("desktop_session"),
                    resource: ResourceId::parse("tracker-app").expect("resource"),
                    holder: ExecutionEnvironment::Computer,
                },
            ],
            dependencies: Vec::new(),
            trigger_bindings: vec![TriggerBinding::direct(TriggerClass::User)],
            schedules: Vec::new(),
            policy: codex_execution_contracts::BindingPolicy::default(),
            max_walk_steps: 1_000,
        })
        .expect("install");
    assert_eq!(configuration.workflow, definition_id());
    assert_eq!(configuration.version_id, artifact.version.version_id);
    assert_eq!(configuration.authorizations.len(), 2);
    assert!(
        golden
            .plane
            .install_registry()
            .is_installed(&definition_id())
    );

    // Fire a user trigger: one instance starts, runs across the mixed
    // environments, and settles successfully.
    let reports = golden
        .plane
        .ingest(
            &mut golden.lifecycle,
            user_event(&definition_id(), "user-1"),
        )
        .await
        .expect("ingest");
    let report = assert_single_started(&reports);
    let instance_id = match &report.outcome {
        FireOutcome::Started { instance, .. } => *instance,
        other => panic!("expected a started instance, got {other:?}"),
    };
    assert_eq!(golden.instances.len(), 1);
    let instance = golden.instances.snapshot(&instance_id).expect("instance");
    assert_eq!(instance.status, WorkflowInstanceStatus::Succeeded);
    // The control-plane event id is durably correlated on the instance.
    assert_eq!(
        instance
            .trigger
            .as_ref()
            .and_then(|trigger| trigger.event_id.clone()),
        Some(report.event_id.clone())
    );
    assert_eq!(
        instance.trigger.as_ref().map(|trigger| trigger.trigger),
        Some(TriggerClass::User)
    );
    // Mixed-environment execution really happened.
    assert_eq!(golden.browser_executor.executed_kinds().len(), 1);
    assert_eq!(golden.computer_calls.load(Ordering::SeqCst), 1);
    // The run verifies end to end: version integrity + evidence digests.
    let verified = golden
        .lifecycle
        .verify_run(&instance_id)
        .expect("verify run");
    assert_eq!(verified.version, artifact.version.version_id);
    for reference in &verified.evidence {
        assert!(
            golden.evidence.verify(reference).expect("evidence verify"),
            "evidence {} must digest to its recorded value",
            reference.locator
        );
    }
}

// ---------------------------------------------------------------------------
// Trigger idempotency.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn e2e_double_fire_is_one_transition_and_a_recorded_noop() {
    let version = sealed_version(
        "triage-report",
        (1, 0, 0),
        Vec::new(),
        WorkflowDependencies::default(),
        DependencyLock::default(),
    );
    let mut harness = harness(Vec::new(), Vec::new());
    install_user(&mut harness, version);

    // First fire: one instance transition (Pending -> Running ->
    // Succeeded through the control plane).
    let reports = harness
        .plane
        .ingest(
            &mut harness.lifecycle,
            user_event(&definition_id(), "user-1"),
        )
        .await
        .expect("ingest");
    assert_single_started(&reports);
    let first_event_id = reports[0].event_id.clone();
    assert_eq!(harness.instances.len(), 1);
    let events_after_first = harness.events.len();

    // Second fire of the SAME event key: a recorded no-op.
    let reports = harness
        .plane
        .ingest(
            &mut harness.lifecycle,
            user_event(&definition_id(), "user-1"),
        )
        .await
        .expect("ingest");
    assert_eq!(reports.len(), 1);
    assert_eq!(reports[0].event_id, first_event_id);
    assert_eq!(reports[0].outcome, FireOutcome::Duplicate);

    // Exactly one instance transition; the lifecycle was untouched by
    // the duplicate.
    assert_eq!(harness.instances.len(), 1);
    assert_eq!(harness.events.len(), events_after_first);
    let key = TriggerEventKey::parse("user-1").expect("key");
    let record = harness
        .ledger
        .record(&definition_id(), &key)
        .expect("record");
    assert_eq!(record.event_id, first_event_id);
    // The ledger audit shows both settlements: the transition and the
    // no-op.
    assert_eq!(record.settlements.len(), 2);
    assert!(matches!(
        &record.settlements[0],
        FireOutcome::Started {
            status: InstanceSettlement::Completed,
            ..
        }
    ));
    assert_eq!(record.settlements[1], FireOutcome::Duplicate);
}

// ---------------------------------------------------------------------------
// Scheduling.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn e2e_schedule_polls_fire_and_repolling_dedupes_durably() {
    let version = sealed_version(
        "triage-report",
        (1, 0, 0),
        Vec::new(),
        WorkflowDependencies::default(),
        DependencyLock::default(),
    );
    let mut harness = harness(Vec::new(), Vec::new());
    harness.clock.advance_to(10_000);
    let mut request = install_request(version);
    request.trigger_bindings = vec![TriggerBinding::direct(TriggerClass::Schedule)];
    request.schedules = vec![ScheduleSpec::Every { period_ms: 1_000 }];
    harness.plane.install(request).expect("install");
    // Anchored at t=10_000: occurrences are 11_000, 12_000, ...

    // At the anchor: nothing due yet.
    let reports = harness
        .plane
        .poll_schedules(&mut harness.lifecycle)
        .await
        .expect("poll");
    assert!(reports.is_empty());

    // Two occurrences fall due.
    harness.clock.advance_to(12_500);
    let reports = harness
        .plane
        .poll_schedules(&mut harness.lifecycle)
        .await
        .expect("poll");
    assert_eq!(reports.len(), 2);
    for report in &reports {
        assert!(matches!(
            &report.outcome,
            FireOutcome::Started {
                status: InstanceSettlement::Completed,
                ..
            }
        ));
    }
    assert_eq!(harness.instances.len(), 2);

    // Polling again at the same instant: nothing new (the cursor
    // consumed the window).
    let reports = harness
        .plane
        .poll_schedules(&mut harness.lifecycle)
        .await
        .expect("poll");
    assert!(reports.is_empty());
    assert_eq!(harness.instances.len(), 2);

    // Simulate a crashed scheduler: the cursor is rewound on disk, the
    // poll recomputes the same occurrences, and the ledger turns them
    // into recorded no-ops — never a second instance.
    let mut configuration = harness
        .installations
        .load(&definition_id())
        .expect("load")
        .expect("configured");
    for schedule in &mut configuration.schedules {
        schedule.cursor_unix_ms = 10_000;
    }
    harness
        .installations
        .save(configuration)
        .expect("save rewound cursor");
    let reports = harness
        .plane
        .poll_schedules(&mut harness.lifecycle)
        .await
        .expect("re-poll");
    assert_eq!(reports.len(), 2);
    for report in &reports {
        assert_eq!(report.outcome, FireOutcome::Duplicate);
    }
    assert_eq!(harness.instances.len(), 2);
}

#[tokio::test]
async fn e2e_daily_schedule_fires_at_its_utc_boundary() {
    let version = sealed_version(
        "daily-report",
        (1, 0, 0),
        Vec::new(),
        WorkflowDependencies::default(),
        DependencyLock::default(),
    );
    let mut harness = harness(Vec::new(), Vec::new());
    harness.clock.advance_to(10_000);
    let mut request = install_request(version);
    request.trigger_bindings = vec![TriggerBinding::direct(TriggerClass::Schedule)];
    request.schedules = vec![ScheduleSpec::DailyAtUtc {
        hour_utc: 9,
        minute_utc: 30,
    }];
    harness.plane.install(request).expect("install");

    // Just before the 09:30 UTC boundary: not due.
    harness.clock.advance_to(34_199_999);
    let reports = harness
        .plane
        .poll_schedules(&mut harness.lifecycle)
        .await
        .expect("poll");
    assert!(reports.is_empty());

    // At the boundary: one fire.
    harness.clock.advance_to(34_200_000);
    let reports = harness
        .plane
        .poll_schedules(&mut harness.lifecycle)
        .await
        .expect("poll");
    assert_eq!(reports.len(), 1);
    assert!(matches!(
        &reports[0].outcome,
        FireOutcome::Started {
            status: InstanceSettlement::Completed,
            ..
        }
    ));
    assert_eq!(harness.instances.len(), 1);

    // Re-polling at the same instant stays consumed.
    let reports = harness
        .plane
        .poll_schedules(&mut harness.lifecycle)
        .await
        .expect("poll");
    assert!(reports.is_empty());
    assert_eq!(harness.instances.len(), 1);
}

#[tokio::test]
async fn e2e_schedule_eligibility_gating_blocks_unauthorized_fires() {
    // The version declares a resource requirement; the schedule is
    // registered, then the authorization is revoked.
    let dependencies = WorkflowDependencies {
        resources: vec![ResourceRequirement {
            resource_type: resource_type(BROWSER_PROFILE),
            purpose: None,
        }],
        ..Default::default()
    };
    let version = sealed_version(
        "triage-report",
        (1, 0, 0),
        Vec::new(),
        dependencies,
        DependencyLock::default(),
    );
    let mut harness = harness(Vec::new(), Vec::new());
    harness.clock.advance_to(10_000);
    harness
        .authorizer
        .allow(BROWSER_PROFILE, "profile-default", "codex-connectors:test");
    let mut request = install_request(version);
    request.trigger_bindings = vec![TriggerBinding::direct(TriggerClass::Schedule)];
    request.schedules = vec![ScheduleSpec::Every { period_ms: 1_000 }];
    request
        .resources
        .push(resource_binding(BROWSER_PROFILE, "profile-default"));
    harness.plane.install(request).expect("install");

    harness
        .authorizer
        .revoke(BROWSER_PROFILE, "profile-default");
    harness.clock.advance_to(11_000);
    let reports = harness
        .plane
        .poll_schedules(&mut harness.lifecycle)
        .await
        .expect("poll");
    assert_eq!(reports.len(), 1);
    assert_eq!(
        diagnostics_codes(&reports[0].outcome),
        vec![TriggerDiagnosticCode::ResourceNotAuthorized]
    );
    // No instance was created and nothing executed: the lifecycle is
    // untouched.
    assert_eq!(harness.instances.len(), 0);
    assert!(!harness.lifecycle.is_active());
    assert!(harness.events.is_empty());
    // The rejection is durably recorded.
    let key = TriggerEventKey::parse("schedule:schedule-1:11000").expect("key");
    let record = harness
        .ledger
        .record(&definition_id(), &key)
        .expect("record");
    assert_eq!(
        diagnostics_codes(&record.settlements[0]),
        vec![TriggerDiagnosticCode::ResourceNotAuthorized]
    );
}

// ---------------------------------------------------------------------------
// Install/bind diagnostics and authorization checks.
// ---------------------------------------------------------------------------

fn version_with_resource_and_skill() -> (WorkflowVersion, ContentDigest) {
    let skill = SkillName::parse("browser-use").expect("skill");
    let dependencies = WorkflowDependencies {
        resources: vec![ResourceRequirement {
            resource_type: resource_type(BROWSER_PROFILE),
            purpose: None,
        }],
        skills: vec![SkillDependency {
            skill: skill.clone(),
            version_requirement: None,
        }],
        ..Default::default()
    };
    let digest = ContentDigest::try_from(format!("sha256:{}", "ab".repeat(32))).expect("digest");
    let mut lock = DependencyLock::default();
    lock.insert(ResolvedDependency {
        key: DependencyKey::Skill { skill },
        resolved: ResolvedDependencyIdentity::Skill {
            digest: digest.clone(),
            version: None,
        },
        provenance: None,
    });
    let version = sealed_version(
        "triage-report",
        (1, 0, 0),
        vec![TriggerClass::User],
        dependencies,
        lock,
    );
    (version, digest)
}

fn skill_binding(digest: &ContentDigest) -> DependencyBinding {
    DependencyBinding {
        key: "skill:browser-use".to_string(),
        binding: "skills/browser-use@1".to_string(),
        integrity: Some(digest.clone()),
    }
}

fn install_codes(error: &WorkflowTriggerError) -> Vec<TriggerDiagnosticCode> {
    let WorkflowTriggerError::InstallDiagnostics { diagnostics, .. } = error else {
        panic!("expected install diagnostics, got {error:?}")
    };
    diagnostics
        .iter()
        .map(|diagnostic| diagnostic.code)
        .collect()
}

#[test]
fn e2e_install_bind_diagnostics_are_structured() {
    let (version, digest) = version_with_resource_and_skill();
    let mut harness = harness(Vec::new(), Vec::new());

    // Missing resource binding.
    let mut request = install_request(version.clone());
    request.dependencies.push(skill_binding(&digest));
    let error = harness
        .plane
        .install(request)
        .expect_err("missing resource");
    assert_eq!(
        install_codes(&error),
        vec![TriggerDiagnosticCode::ResourceMissing]
    );

    // Unauthorized resource binding.
    let mut request = install_request(version.clone());
    request
        .resources
        .push(resource_binding(BROWSER_PROFILE, "profile-default"));
    request.dependencies.push(skill_binding(&digest));
    let error = harness
        .plane
        .install(request)
        .expect_err("unauthorized resource");
    assert_eq!(
        install_codes(&error),
        vec![TriggerDiagnosticCode::ResourceNotAuthorized]
    );

    // Dependency left unbound.
    harness
        .authorizer
        .allow(BROWSER_PROFILE, "profile-default", "codex-connectors:test");
    let mut request = install_request(version.clone());
    request
        .resources
        .push(resource_binding(BROWSER_PROFILE, "profile-default"));
    let error = harness
        .plane
        .install(request)
        .expect_err("unbound dependency");
    assert_eq!(
        install_codes(&error),
        vec![TriggerDiagnosticCode::DependencyUnbound]
    );

    // Dependency drift: a binding whose integrity does not match the
    // immutable lock (the silent-upgrade guard).
    let drifted = ContentDigest::try_from(format!("sha256:{}", "cd".repeat(32))).expect("digest");
    let mut request = install_request(version.clone());
    request
        .resources
        .push(resource_binding(BROWSER_PROFILE, "profile-default"));
    request.dependencies.push(skill_binding(&drifted));
    let error = harness
        .plane
        .install(request)
        .expect_err("dependency drift");
    assert_eq!(
        install_codes(&error),
        vec![TriggerDiagnosticCode::DependencyDrift]
    );

    // Undeclared trigger class on a version that declares its triggers.
    let mut request = install_request(version.clone());
    request
        .resources
        .push(resource_binding(BROWSER_PROFILE, "profile-default"));
    request.dependencies.push(skill_binding(&digest));
    request.trigger_bindings = vec![TriggerBinding::direct(TriggerClass::Webhook)];
    let error = harness
        .plane
        .install(request)
        .expect_err("undeclared trigger");
    assert_eq!(
        install_codes(&error),
        vec![TriggerDiagnosticCode::TriggerNotDeclared]
    );

    // Nothing was installed by any failed attempt.
    assert_eq!(harness.installations.list().expect("list").len(), 0);
    assert!(
        !harness
            .plane
            .install_registry()
            .is_installed(&definition_id())
    );

    // The fully explicit, authorized configuration installs.
    let mut request = install_request(version);
    request
        .resources
        .push(resource_binding(BROWSER_PROFILE, "profile-default"));
    request.dependencies.push(skill_binding(&digest));
    harness.plane.install(request).expect("install");
    assert!(
        harness
            .plane
            .install_registry()
            .is_installed(&definition_id())
    );
}

#[tokio::test]
async fn e2e_authorization_is_rechecked_at_fire_time() {
    let (version, digest) = version_with_resource_and_skill();
    let mut harness = harness(Vec::new(), Vec::new());
    harness
        .authorizer
        .allow(BROWSER_PROFILE, "profile-default", "codex-connectors:test");
    let mut request = install_request(version);
    request
        .resources
        .push(resource_binding(BROWSER_PROFILE, "profile-default"));
    request.dependencies.push(skill_binding(&digest));
    harness.plane.install(request).expect("install");

    // Revoke after install: availability was never authorization.
    harness
        .authorizer
        .revoke(BROWSER_PROFILE, "profile-default");
    let reports = harness
        .plane
        .ingest(
            &mut harness.lifecycle,
            user_event(&definition_id(), "user-1"),
        )
        .await
        .expect("ingest");
    assert_eq!(reports.len(), 1);
    assert_eq!(
        diagnostics_codes(&reports[0].outcome),
        vec![TriggerDiagnosticCode::ResourceNotAuthorized]
    );
    assert_eq!(harness.instances.len(), 0);
    assert!(!harness.lifecycle.is_active());

    // Re-allow: a fresh event fires normally, and every check was
    // recorded by the authorization plane.
    harness
        .authorizer
        .allow(BROWSER_PROFILE, "profile-default", "codex-connectors:test");
    let reports = harness
        .plane
        .ingest(
            &mut harness.lifecycle,
            user_event(&definition_id(), "user-2"),
        )
        .await
        .expect("ingest");
    assert_single_started(&reports);
    assert_eq!(harness.instances.len(), 1);
    assert!(harness.authorizer.requests().len() >= 3);
}

// ---------------------------------------------------------------------------
// Version integrity.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn e2e_tampered_version_records_never_fire() {
    let version = sealed_version(
        "triage-report",
        (1, 0, 0),
        Vec::new(),
        WorkflowDependencies::default(),
        DependencyLock::default(),
    );
    let mut harness = harness(Vec::new(), Vec::new());
    install_user(&mut harness, version.clone());

    // Silently mutate the stored record under the same identity.
    let mut tampered = version;
    tampered.definition.description = Some("silently mutated".to_string());
    harness
        .versions
        .publish(tampered)
        .expect("publish tampered");

    let reports = harness
        .plane
        .ingest(
            &mut harness.lifecycle,
            user_event(&definition_id(), "user-1"),
        )
        .await
        .expect("ingest");
    assert_eq!(reports.len(), 1);
    assert_eq!(
        diagnostics_codes(&reports[0].outcome),
        vec![TriggerDiagnosticCode::VersionIntegrity]
    );
    // Nothing selected, nothing instantiated, nothing executed.
    assert_eq!(harness.instances.len(), 0);
    assert!(!harness.lifecycle.is_active());
    assert!(harness.events.is_empty());
}

// ---------------------------------------------------------------------------
// Rebinding without source change.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn e2e_rebinding_changes_bindings_not_the_semantic_source() {
    let dependencies = WorkflowDependencies {
        resources: vec![ResourceRequirement {
            resource_type: resource_type(BROWSER_PROFILE),
            purpose: None,
        }],
        ..Default::default()
    };
    let version = sealed_version(
        "triage-report",
        (1, 0, 0),
        Vec::new(),
        dependencies,
        DependencyLock::default(),
    );
    let version_id = version.version_id.clone();
    let definition_digest = version.identity.definition_digest.clone();
    let mut harness = harness(Vec::new(), Vec::new());
    harness
        .authorizer
        .allow(BROWSER_PROFILE, "profile-a", "codex-connectors:test");
    let mut request = install_request(version);
    request
        .resources
        .push(resource_binding(BROWSER_PROFILE, "profile-a"));
    harness.plane.install(request).expect("install");

    // First fire binds profile-a.
    let reports = harness
        .plane
        .ingest(
            &mut harness.lifecycle,
            user_event(&definition_id(), "user-1"),
        )
        .await
        .expect("ingest");
    assert_single_started(&reports);
    let bound = harness
        .lifecycle
        .registry()
        .resource_binding(&resource_type(BROWSER_PROFILE))
        .expect("resource bound");
    assert_eq!(bound.resource.as_ref(), "profile-a");

    // Rebind to profile-b: authorized, audited, and explicit.
    harness
        .authorizer
        .allow(BROWSER_PROFILE, "profile-b", "codex-connectors:test");
    let configuration = harness
        .plane
        .rebind_resource(
            &definition_id(),
            resource_binding(BROWSER_PROFILE, "profile-b"),
        )
        .expect("rebind");
    assert_eq!(
        configuration
            .resource_binding(&resource_type(BROWSER_PROFILE))
            .map(|binding| binding.resource.as_ref().to_string()),
        Some("profile-b".to_string())
    );
    let audits = harness.installations.audits();
    assert_eq!(audits.len(), 1);
    assert_eq!(audits[0].from.as_deref(), Some("profile-a"));
    assert_eq!(audits[0].to, "profile-b");

    // The immutable semantic source is untouched: one record, same
    // identity, same definition digest, integrity still verifies.
    assert_eq!(harness.versions.len(), 1);
    let stored = harness
        .versions
        .load(&version_id)
        .expect("load")
        .expect("stored");
    assert_eq!(stored.version_id, version_id);
    assert_eq!(stored.identity.definition_digest, definition_digest);
    stored.verify_integrity().expect("integrity after rebind");

    // The next fire binds the new resource instance.
    let reports = harness
        .plane
        .ingest(
            &mut harness.lifecycle,
            user_event(&definition_id(), "user-2"),
        )
        .await
        .expect("ingest");
    assert_single_started(&reports);
    let bound = harness
        .lifecycle
        .registry()
        .resource_binding(&resource_type(BROWSER_PROFILE))
        .expect("resource bound");
    assert_eq!(bound.resource.as_ref(), "profile-b");
    assert_eq!(harness.instances.len(), 2);
    // Both instances pin the same immutable version.
    for instance in harness.instances.records() {
        assert_eq!(instance.version, version_id);
    }

    // Rebinding a type neither declared nor bound is refused (the
    // resource is authorized, so the declaration check is isolated).
    harness
        .authorizer
        .allow("typo_profile", "profile-c", "codex-connectors:test");
    let error = harness
        .plane
        .rebind_resource(
            &definition_id(),
            resource_binding("typo_profile", "profile-c"),
        )
        .expect_err("undeclared rebind");
    assert_eq!(
        install_codes(&error),
        vec![TriggerDiagnosticCode::ResourceNotDeclared]
    );
}

// ---------------------------------------------------------------------------
// Resume and retry pin the immutable version.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn e2e_resume_and_retry_pin_the_immutable_version() {
    // A step that waits for a human trigger, with both the start and the
    // resume classes declared.
    let definition = WorkflowDefinition {
        id: definition_id(),
        description: None,
        ir: wait_ir(),
        roles: BTreeMap::new(),
        triggers: vec![
            WorkflowTrigger {
                trigger: TriggerClass::User,
                description: None,
            },
            WorkflowTrigger {
                trigger: TriggerClass::HumanEvent,
                description: None,
            },
        ],
        dependencies: WorkflowDependencies::default(),
    };
    let version = WorkflowVersion::seal(
        definition,
        repository_id(),
        pinned_revision(),
        SemanticVersion::new(1, 0, 0),
        DependencyLock::default(),
        None,
    )
    .expect("sealed version");
    let version_id = version.version_id.clone();
    let mut harness = harness(Vec::new(), Vec::new());
    let mut request = install_request(version);
    request.trigger_bindings = vec![
        TriggerBinding::direct(TriggerClass::User),
        TriggerBinding::direct(TriggerClass::HumanEvent),
    ];
    harness.plane.install(request).expect("install");

    // The user start runs to the wait node and pauses, awaiting the
    // human event.
    let reports = harness
        .plane
        .ingest(&mut harness.lifecycle, user_event(&definition_id(), "u1"))
        .await
        .expect("ingest");
    assert_eq!(reports.len(), 1);
    let instance_id = match &reports[0].outcome {
        FireOutcome::Started {
            instance,
            status:
                InstanceSettlement::Paused {
                    node,
                    awaiting: Some(TriggerClass::HumanEvent),
                },
        } => {
            assert_eq!(node, &node_id("wait-signoff"));
            *instance
        }
        other => panic!("expected a paused instance, got {other:?}"),
    };
    assert_eq!(
        harness
            .instances
            .snapshot(&instance_id)
            .expect("instance")
            .status,
        WorkflowInstanceStatus::Paused
    );
    let awaits = harness
        .ledger
        .awaiting(&definition_id(), TriggerClass::HumanEvent)
        .expect("awaiting");
    assert_eq!(awaits.len(), 1);

    // The human event resumes the paused instance through the control
    // seam: the durable Paused -> Running transition, same version.
    let reports = harness
        .plane
        .ingest(
            &mut harness.lifecycle,
            external_event(TriggerClass::HumanEvent, "h1", None),
        )
        .await
        .expect("ingest human");
    // External routing: the direct human-event binding matches the
    // unrouted envelope.
    assert_eq!(reports.len(), 1);
    assert_eq!(
        reports[0].outcome,
        FireOutcome::Resumed {
            instances: vec![instance_id]
        }
    );
    let resumed = harness.instances.snapshot(&instance_id).expect("instance");
    assert_eq!(resumed.status, WorkflowInstanceStatus::Running);
    assert_eq!(resumed.version, version_id);
    let directives = harness.control.directives();
    assert_eq!(directives.len(), 1);
    assert_eq!(directives[0].instance, instance_id);
    // The await is resolved.
    assert_eq!(
        harness
            .ledger
            .awaiting(&definition_id(), TriggerClass::HumanEvent)
            .expect("awaiting")
            .len(),
        0
    );

    // A retry (new user fire) starts a new instance pinning the SAME
    // immutable version; the resumed instance is untouched.
    let reports = harness
        .plane
        .ingest(&mut harness.lifecycle, user_event(&definition_id(), "u2"))
        .await
        .expect("ingest retry");
    assert_eq!(reports.len(), 1);
    assert!(matches!(
        &reports[0].outcome,
        FireOutcome::Started {
            status: InstanceSettlement::Paused { .. },
            ..
        }
    ));
    assert_eq!(harness.instances.len(), 2);
    for instance in harness.instances.records() {
        assert_eq!(instance.version, version_id);
    }
    // One version record, still sealed.
    assert_eq!(harness.versions.len(), 1);
    harness
        .versions
        .load(&version_id)
        .expect("load")
        .expect("stored")
        .verify_integrity()
        .expect("integrity");
    // The resumed instance kept its Running status.
    assert_eq!(
        harness
            .instances
            .snapshot(&instance_id)
            .expect("instance")
            .status,
        WorkflowInstanceStatus::Running
    );
}

// ---------------------------------------------------------------------------
// Routing across all eight trigger classes.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn e2e_routes_external_events_across_all_trigger_classes() {
    let version = sealed_version(
        "triage-report",
        (1, 0, 0),
        TriggerClass::ALL.to_vec(),
        WorkflowDependencies::default(),
        DependencyLock::default(),
    );
    let mut harness = harness(Vec::new(), Vec::new());
    let mut request = install_request(version);
    request.trigger_bindings = vec![
        TriggerBinding::direct(TriggerClass::User),
        TriggerBinding::direct(TriggerClass::Schedule),
        TriggerBinding::direct(TriggerClass::WorkflowEvent),
        TriggerBinding::direct(TriggerClass::HumanEvent),
        TriggerBinding::routed(TriggerClass::Webhook, "ops-hook"),
        TriggerBinding::routed(TriggerClass::ConnectorEvent, "conn-1"),
        TriggerBinding::routed(TriggerClass::BrowserEvent, "session-1"),
        TriggerBinding::routed(TriggerClass::ComputerEvent, "session-2"),
    ];
    harness.plane.install(request).expect("install");

    // Direct classes fire on unrouted envelopes.
    for (class, key) in [
        (TriggerClass::User, "user-1"),
        (TriggerClass::Schedule, "schedule-1"),
        (TriggerClass::WorkflowEvent, "workflow-1"),
        (TriggerClass::HumanEvent, "human-1"),
    ] {
        let reports = harness
            .plane
            .ingest(&mut harness.lifecycle, external_event(class, key, None))
            .await
            .expect("ingest");
        assert_eq!(reports.len(), 1, "{class:?} must route to one workflow");
        assert!(
            matches!(
                &reports[0].outcome,
                FireOutcome::Started {
                    status: InstanceSettlement::Completed,
                    ..
                }
            ),
            "{class:?} must start one instance"
        );
    }

    // Endpoint-routed classes fire only on their registered endpoint.
    for (class, key, endpoint) in [
        (TriggerClass::Webhook, "webhook-1", "ops-hook"),
        (TriggerClass::ConnectorEvent, "connector-1", "conn-1"),
        (TriggerClass::BrowserEvent, "browser-1", "session-1"),
        (TriggerClass::ComputerEvent, "computer-1", "session-2"),
    ] {
        let reports = harness
            .plane
            .ingest(
                &mut harness.lifecycle,
                external_event(class, key, Some(endpoint)),
            )
            .await
            .expect("ingest");
        assert_eq!(reports.len(), 1, "{class:?} must route to one workflow");
        assert!(matches!(
            &reports[0].outcome,
            FireOutcome::Started {
                status: InstanceSettlement::Completed,
                ..
            }
        ));
    }

    // All eight classes fired exactly one instance each.
    assert_eq!(harness.instances.len(), 8);

    // An unrouted webhook matches nothing (its binding is endpoint-
    // routed) and stays inert.
    let reports = harness
        .plane
        .ingest(
            &mut harness.lifecycle,
            external_event(TriggerClass::Webhook, "webhook-2", None),
        )
        .await
        .expect("ingest");
    assert!(reports.is_empty());
    // An unknown endpoint matches nothing.
    let reports = harness
        .plane
        .ingest(
            &mut harness.lifecycle,
            external_event(TriggerClass::Webhook, "webhook-3", Some("nope")),
        )
        .await
        .expect("ingest");
    assert!(reports.is_empty());
    assert_eq!(harness.instances.len(), 8);
}

// ---------------------------------------------------------------------------
// Ordinary Codex compatibility.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn e2e_no_installation_is_an_inert_recorded_rejection() {
    let mut harness = harness(Vec::new(), Vec::new());
    let reports = harness
        .plane
        .ingest(
            &mut harness.lifecycle,
            user_event(&definition_id(), "user-1"),
        )
        .await
        .expect("ingest");
    assert_eq!(reports.len(), 1);
    assert_eq!(
        diagnostics_codes(&reports[0].outcome),
        vec![TriggerDiagnosticCode::NotInstalled]
    );
    // Nothing executed: no instances, no events, no approvals, no
    // workflow activity.
    assert_eq!(harness.instances.len(), 0);
    assert!(harness.events.is_empty());
    assert_eq!(harness.approvals.requests().len(), 0);
    assert!(!harness.lifecycle.is_active());
    assert!(harness.evidence.is_empty());
    // The rejection is durably recorded.
    let key = TriggerEventKey::parse("user-1").expect("key");
    let record = harness
        .ledger
        .record(&definition_id(), &key)
        .expect("record");
    assert_eq!(
        diagnostics_codes(&record.settlements[0]),
        vec![TriggerDiagnosticCode::NotInstalled]
    );
}

// ---------------------------------------------------------------------------
// Explicit updates.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn e2e_explicit_updates_never_fire_silently() {
    let v1 = sealed_version(
        "triage-report",
        (1, 0, 0),
        Vec::new(),
        WorkflowDependencies::default(),
        DependencyLock::default(),
    );
    let v2 = sealed_version(
        "triage-report",
        (1, 1, 0),
        Vec::new(),
        WorkflowDependencies::default(),
        DependencyLock::default(),
    );
    let v1_id = v1.version_id.clone();
    let v2_id = v2.version_id.clone();
    let mut harness = harness(Vec::new(), Vec::new());
    install_user(&mut harness, v1.clone());

    // Installing over the existing installation is rejected: changes
    // must be explicit.
    let error = harness
        .plane
        .install(install_request(v2.clone()))
        .expect_err("already installed");
    assert!(matches!(
        error,
        WorkflowTriggerError::Forge(WorkflowForgeError::AlreadyInstalled { .. })
    ));

    // The explicit update path: propose, decide, reconfigure.
    let plan = harness
        .plane
        .install_registry()
        .propose_update(
            &definition_id(),
            &PublishedVersionRef::of(&v2),
            Some("upgrade to v1.1.0".to_string()),
        )
        .expect("propose");
    let decision = harness
        .plane
        .install_registry_mut()
        .decide_update(&plan, UpdateDecision::Apply)
        .expect("decide");
    assert!(decision.applied);

    // Until the configuration is re-bound, fires are rejected with the
    // drift diagnostic — never a silent version change.
    let reports = harness
        .plane
        .ingest(&mut harness.lifecycle, user_event(&definition_id(), "u1"))
        .await
        .expect("ingest");
    assert_eq!(reports.len(), 1);
    assert_eq!(
        diagnostics_codes(&reports[0].outcome),
        vec![TriggerDiagnosticCode::VersionDrift]
    );
    assert_eq!(harness.instances.len(), 0);

    // Reconfigure to the installed version, then fire: the new instance
    // pins v2 while v1 remains a valid, verifiable record.
    harness
        .plane
        .reconfigure(&definition_id(), install_request(v2.clone()))
        .expect("reconfigure");
    let reports = harness
        .plane
        .ingest(&mut harness.lifecycle, user_event(&definition_id(), "u2"))
        .await
        .expect("ingest");
    let report = assert_single_started(&reports);
    let instance = match &report.outcome {
        FireOutcome::Started { instance, .. } => *instance,
        other => panic!("expected a started instance, got {other:?}"),
    };
    assert_eq!(
        harness
            .instances
            .snapshot(&instance)
            .expect("instance")
            .version,
        v2_id
    );
    assert_eq!(harness.versions.len(), 2);
    harness
        .versions
        .load(&v1_id)
        .expect("load")
        .expect("v1 stored")
        .verify_integrity()
        .expect("v1 integrity");
    assert_eq!(harness.plane.install_registry().history().len(), 1);
}

// ---------------------------------------------------------------------------
// Capability readiness gating at instantiation.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn e2e_unready_capabilities_fail_the_instantiation_gate() {
    // The version's step declares a capability no adapter provides: the
    // static gate passes (the configuration is complete), and the
    // lifecycle's instantiation gate refuses the binding plan, settling
    // a `Failed` instance with structured diagnostics. Nothing ever
    // dispatches.
    let definition = WorkflowDefinition {
        id: definition_id(),
        description: None,
        ir: {
            let mut nodes = BTreeMap::new();
            nodes.insert(
                node_id("step-001"),
                WorkflowIrNode::Step(StepNode {
                    capabilities: vec![CapabilityRequirement {
                        capability: capability("navigate_web"),
                        purpose: None,
                    }],
                    roles: Vec::new(),
                    description: None,
                    next: None,
                }),
            );
            WorkflowIr {
                ir_format: IR_FORMAT_VERSION,
                entry: node_id("step-001"),
                nodes,
                conditions: BTreeMap::new(),
            }
        },
        roles: BTreeMap::new(),
        triggers: Vec::new(),
        dependencies: WorkflowDependencies::default(),
    };
    let version = WorkflowVersion::seal(
        definition,
        repository_id(),
        pinned_revision(),
        SemanticVersion::new(1, 0, 0),
        DependencyLock::default(),
        None,
    )
    .expect("sealed version");
    let mut harness = harness(Vec::new(), Vec::new());
    install_user(&mut harness, version);

    let reports = harness
        .plane
        .ingest(
            &mut harness.lifecycle,
            user_event(&definition_id(), "user-1"),
        )
        .await
        .expect("ingest");
    assert_eq!(reports.len(), 1);
    assert_eq!(
        diagnostics_codes(&reports[0].outcome),
        vec![TriggerDiagnosticCode::ReadinessDenied]
    );
    // The control plane settled a Failed instance with the structured
    // reason; no action was dispatched anywhere.
    assert_eq!(harness.instances.len(), 1);
    let instance = harness.instances.records().remove(0);
    assert_eq!(instance.status, WorkflowInstanceStatus::Failed);
    assert_eq!(
        instance
            .trigger
            .as_ref()
            .and_then(|trigger| trigger.event_id.clone()),
        Some(reports[0].event_id.clone())
    );
    assert_eq!(harness.approvals.requests().len(), 0);
}
