//! WO-010 end-to-end workflow lifecycle tests.
//!
//! Every scenario is **static**: in-memory doubles stand in for the host
//! seams (browser executor, computer-use bridge, forge, stores), so no
//! network, browser, or desktop runtime is touched. The scenarios map to
//! the Work Order's acceptance bullets:
//!
//! - create -> validate -> publish -> install -> run (mixed environments)
//!   -> recover -> verify (the golden path);
//! - failure/recovery: transient retry, unavailable rebind with fallback
//!   record, permanent escalation;
//! - approval/security: approval denial, binding-policy scope denial,
//!   computer-use policy denial;
//! - version immutability: tampered records never execute; running
//!   instances pin their version across install upgrades;
//! - ordinary Codex compatibility: with no workflow selected, nothing
//!   executes.
//!
//! The workspace clippy.toml allows `expect`/`unwrap` in test code; the
//! same intent is spelled out here for integration-test setup.
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
use codex_browser_use_adapter::CapabilityLifecycleState as BrowserLifecycleState;
use codex_browser_use_adapter::OriginPolicyRequirement;
use codex_browser_use_adapter::OriginPolicySnapshot;
use codex_computer_use_adapter::AccessRequirement;
use codex_computer_use_adapter::BridgeCallFailure;
use codex_computer_use_adapter::BridgeIdentity;
use codex_computer_use_adapter::BridgeReadinessFailure;
use codex_computer_use_adapter::CapabilityLifecycleState as ComputerLifecycleState;
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
use codex_execution_contracts::ActionResult;
use codex_execution_contracts::ActionTarget;
use codex_execution_contracts::AdapterDescriptor;
use codex_execution_contracts::AdapterExecuteFuture;
use codex_execution_contracts::AdapterId;
use codex_execution_contracts::AdapterPrepareFuture;
use codex_execution_contracts::AdapterProbeFuture;
use codex_execution_contracts::BindingClass;
use codex_execution_contracts::BindingDecision;
use codex_execution_contracts::BindingPolicy;
use codex_execution_contracts::CapabilityBindingId;
use codex_execution_contracts::CapabilityRegistry;
use codex_execution_contracts::EnvironmentAdapter;
use codex_execution_contracts::EnvironmentScope;
use codex_execution_contracts::ExecutionEnvironment;
use codex_execution_contracts::FailureKind;
use codex_execution_contracts::FallbackReason;
use codex_execution_contracts::FallbackRecord;
use codex_execution_contracts::OperationId;
use codex_execution_contracts::PrepareRequest;
use codex_execution_contracts::PreparedSession;
use codex_execution_contracts::ProbeReport;
use codex_execution_contracts::ProvidedCapability;
use codex_execution_contracts::ReadinessState;
use codex_execution_contracts::ResourceBinding;
use codex_execution_contracts::ResourceId;
use codex_execution_contracts::SessionHandle;
use codex_teaching_compiler::ApprovalDecision;
use codex_teaching_compiler::ApprovalDecisionKind;
use codex_teaching_compiler::SimulationConfig;
use codex_teaching_compiler::TeachingMode;
use codex_teaching_compiler::TeachingSession;
use codex_teaching_compiler::TrajectoryEvent;
use codex_teaching_compiler::WorkflowCandidate;
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
use codex_workflow_app::InstantiateRequest;
use codex_workflow_app::LifecycleDeps;
use codex_workflow_app::PublishRequest;
use codex_workflow_app::PublishedArtifact;
use codex_workflow_app::RecordingEventSink;
use codex_workflow_app::RunTerminal;
use codex_workflow_app::ScriptedAction;
use codex_workflow_app::ScriptedActionSource;
use codex_workflow_app::WalkConfig;
use codex_workflow_app::WorkflowAppError;
use codex_workflow_app::WorkflowEvent;
use codex_workflow_app::WorkflowLifecycle;
use codex_workflow_app::WorkflowVersionStore;
use codex_workflow_app::install_version;
use codex_workflow_app::publish;
use codex_workflow_contracts::Attribution;
use codex_workflow_contracts::CapabilityId;
use codex_workflow_contracts::CapabilityRequirement;
use codex_workflow_contracts::DependencyLock;
use codex_workflow_contracts::DevelopmentRef;
use codex_workflow_contracts::EvidenceKind;
use codex_workflow_contracts::ForgeKind;
use codex_workflow_contracts::IR_FORMAT_VERSION;
use codex_workflow_contracts::IrNodeId;
use codex_workflow_contracts::MANIFEST_FORMAT_VERSION;
use codex_workflow_contracts::RepositoryRelativePath;
use codex_workflow_contracts::ResourceTypeId;
use codex_workflow_contracts::StepNode;
use codex_workflow_contracts::TriggerClass;
use codex_workflow_contracts::TriggerSource;
use codex_workflow_contracts::WaitFor;
use codex_workflow_contracts::WaitNode;
use codex_workflow_contracts::WorkflowDefinitionId;
use codex_workflow_contracts::WorkflowInstanceStatus;
use codex_workflow_contracts::WorkflowIr;
use codex_workflow_contracts::WorkflowIrNode;
use codex_workflow_contracts::WorkflowManifest;
use codex_workflow_contracts::WorkflowProvenance;
use codex_workflow_contracts::WorkflowRepositoryId;
use codex_workflow_forge::InMemoryForge;
use codex_workflow_forge::InstallRegistry;
use codex_workflow_forge::UpdateDecision;
use codex_workflow_forge::publish_release;
use pretty_assertions::assert_eq;

const BROWSER_BINDING: &str = "codex-browser-use-env:navigate_web";
const COMPUTER_BINDING: &str = "codex-computer-use-env:control_desktop_app";
const API_BINDING: &str = "acme-web-api:navigate_web";

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

fn binding_id(value: &str) -> CapabilityBindingId {
    CapabilityBindingId::parse(value).expect("binding id")
}

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

/// The reviewed step bindings: the browser step and the desktop step.
fn step_bindings() -> BTreeMap<IrNodeId, Vec<CapabilityRequirement>> {
    BTreeMap::from([
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
    ])
}

/// Publishes an approved workflow through the git lifecycle: forge
/// repository, development commit, ref resolution, seal, release, install.
fn publish_installed(
    approved: codex_teaching_compiler::ApprovedWorkflow,
    tag: &str,
    version: (u64, u64, u64),
) -> (PublishedArtifact, InstallRegistry) {
    publish_with(approved, definition_id(), tag, version, step_bindings())
}

/// Publishes with an explicit workflow identity and binding map.
fn publish_with(
    approved: codex_teaching_compiler::ApprovedWorkflow,
    workflow: WorkflowDefinitionId,
    tag: &str,
    version: (u64, u64, u64),
    bindings: BTreeMap<IrNodeId, Vec<CapabilityRequirement>>,
) -> (PublishedArtifact, InstallRegistry) {
    let repository = repository_id();
    let mut forge = InMemoryForge::new(ForgeKind::parse("github").expect("forge kind"));
    let manifest = WorkflowManifest {
        manifest_version: MANIFEST_FORMAT_VERSION,
        workflow,
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
        .commit(&repository, &branch, format!("publish {tag}"), "acme-bot")
        .expect("commit");
    let revision = forge
        .resolve_ref_blocking(&repository, &branch)
        .expect("resolve ref");
    let artifact = publish(PublishRequest {
        approved,
        repository: repository.clone(),
        source_revision: revision.clone(),
        semantic_version: codex_workflow_contracts::SemanticVersion::new(
            version.0, version.1, version.2,
        ),
        bindings,
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
    .expect("publish");
    artifact.version.verify_integrity().expect("integrity");
    let release = publish_release(
        &repository,
        &[],
        tag,
        &revision,
        std::slice::from_ref(&artifact.reference),
    )
    .expect("release");
    assert_eq!(release.versions, vec![artifact.version.version_id.clone()]);
    let mut installs = InstallRegistry::new();
    install_version(&mut installs, &artifact).expect("install");
    (artifact, installs)
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

/// A browser turn that fails with the given classification.
fn failed_turn(kind: FailureKind, reason: &str) -> BrowserTurn {
    BrowserTurn {
        result: BrowserActionResult {
            outcome: BrowserActionOutcome::Failed {
                reason: reason.to_string(),
            },
            detail: None,
        },
        observation: None,
        failure_kind: Some(kind),
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

/// A compatible alternate browser binding (API environment) used for the
/// rebind recovery scenario.
struct ApiFallbackAdapter {
    descriptor: AdapterDescriptor,
    capability: CapabilityId,
    calls: Arc<AtomicUsize>,
}

impl ApiFallbackAdapter {
    fn new(calls: Arc<AtomicUsize>) -> Result<Self, WorkflowAppError> {
        let capability = capability(BROWSER_CAPABILITY);
        Ok(Self {
            descriptor: AdapterDescriptor {
                adapter: AdapterId::parse("acme-web-api")?,
                environment: ExecutionEnvironment::Api,
                class: BindingClass::Compatible,
                provides: vec![ProvidedCapability {
                    capability: capability.clone(),
                    resources: Vec::new(),
                }],
            },
            capability,
            calls,
        })
    }
}

impl EnvironmentAdapter for ApiFallbackAdapter {
    fn descriptor(&self) -> &AdapterDescriptor {
        &self.descriptor
    }

    fn probe(&self) -> AdapterProbeFuture<'_> {
        Box::pin(async move { Ok(ProbeReport::ready()) })
    }

    fn prepare(&self, request: PrepareRequest) -> AdapterPrepareFuture<'_> {
        Box::pin(async move {
            let handle = SessionHandle::parse(request.binding.as_ref())
                .expect("binding ids are valid handles");
            Ok(PreparedSession {
                binding: request.binding,
                session: handle,
            })
        })
    }

    fn execute<'a>(
        &'a self,
        _session: &'a SessionHandle,
        _action: Action,
    ) -> AdapterExecuteFuture<'a> {
        let binding = CapabilityBindingId::derive(&self.descriptor.adapter, &self.capability);
        let calls = self.calls.clone();
        Box::pin(async move {
            calls.fetch_add(1, Ordering::SeqCst);
            ActionResult::succeeded(binding)
                .with_outputs(serde_json::json!({"via": "compatible-api"}))
        })
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
        ScriptedAction::new(COMPUTER_BINDING_STEP, COMPUTER_CAPABILITY, click_action())
            .expect("scripted click"),
    ]
}

const COMPUTER_BINDING_STEP: &str = "step-002";

fn resources() -> Vec<ResourceBinding> {
    vec![
        ResourceBinding {
            resource_type: ResourceTypeId::parse("browser_profile").expect("resource type"),
            resource: ResourceId::parse("profile-default").expect("resource"),
            holder: ExecutionEnvironment::Browser,
        },
        ResourceBinding {
            resource_type: ResourceTypeId::parse("desktop_session").expect("resource type"),
            resource: ResourceId::parse("tracker-app").expect("resource"),
            holder: ExecutionEnvironment::Computer,
        },
    ]
}

/// The test harness: a lifecycle over in-memory seams plus the handles
/// needed to observe every plane.
struct Harness {
    lifecycle: WorkflowLifecycle,
    versions: InMemoryVersionStore,
    instances: InMemoryInstanceStore,
    evidence: InMemoryEvidenceStore,
    approvals: InMemoryApprovalSource,
    actions: ScriptedActionSource,
    events: RecordingEventSink,
    browser: Arc<BrowserUseEnvironmentAdapter>,
    computer: Arc<ComputerUseEnvironmentAdapter>,
    browser_executor: Arc<ScriptedBrowserExecutor>,
    computer_calls: Arc<AtomicUsize>,
}

fn harness(
    browser_script: Vec<BrowserTurn>,
    action_script: Vec<ScriptedAction>,
    computer_access: AccessRequirement,
    extra_adapters: Vec<Arc<dyn EnvironmentAdapter>>,
) -> Harness {
    let browser_executor = Arc::new(ScriptedBrowserExecutor::new(browser_script));
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
            computer_policy(computer_access),
            Arc::new(StubComputerFactory {
                calls: computer_calls.clone(),
            }),
            Arc::new(|| 1_000u64) as UnixMsClock,
        )
        .expect("computer adapter"),
    );
    let mut registry = CapabilityRegistry::new();
    registry
        .register_adapter(browser.clone())
        .expect("register browser adapter");
    registry
        .register_adapter(computer.clone())
        .expect("register computer adapter");
    for adapter in extra_adapters {
        registry
            .register_adapter(adapter)
            .expect("register adapter");
    }
    let versions = InMemoryVersionStore::new();
    let instances = InMemoryInstanceStore::new();
    let evidence = InMemoryEvidenceStore::new();
    let approvals = InMemoryApprovalSource::approving("tech-lead", evidence.clone());
    let actions = ScriptedActionSource::new(action_script);
    let events = RecordingEventSink::new();
    let lifecycle = WorkflowLifecycle::new(LifecycleDeps {
        versions: Box::new(versions.clone()),
        instances: Box::new(instances.clone()),
        evidence: Box::new(evidence.clone()),
        approvals: Box::new(approvals.clone()),
        actions: Box::new(actions.clone()),
        events: Box::new(events.clone()),
        registry,
    });
    Harness {
        lifecycle,
        versions,
        instances,
        evidence,
        approvals,
        actions,
        events,
        browser,
        computer,
        browser_executor,
        computer_calls,
    }
}

/// Selects, instantiates, and runs `artifact` in `harness`.
async fn run_installed(
    harness: &mut Harness,
    artifact: &PublishedArtifact,
) -> codex_workflow_app::RunOutcome {
    harness
        .versions
        .publish(artifact.version.clone())
        .expect("store version");
    harness
        .lifecycle
        .select_version(&artifact.version.version_id)
        .expect("select version");
    let instance = harness
        .lifecycle
        .instantiate(InstantiateRequest {
            trigger: Some(TriggerSource {
                trigger: TriggerClass::User,
                event_id: None,
            }),
            policy: BindingPolicy::default(),
            resources: resources(),
            walk: WalkConfig::default(),
        })
        .await
        .expect("instantiate");
    assert_eq!(instance.status, WorkflowInstanceStatus::Running);
    harness.lifecycle.run().await.expect("run")
}

/// Verifies every evidence reference on a run's instance record.
fn verify_evidence(harness: &Harness, verified: &codex_workflow_app::VerifiedRun) {
    for reference in &verified.evidence {
        assert!(
            harness.evidence.verify(reference).expect("evidence verify"),
            "evidence {} must digest to its recorded value",
            reference.locator
        );
    }
}

#[tokio::test]
async fn e2e_create_validate_publish_install_run_mixed_environments_verify() {
    // create -> validate (inside taught_approved) -> publish -> install.
    let approved = taught_approved();
    let (artifact, installs) = publish_installed(approved, "v1.0.0", (1, 0, 0));
    assert!(installs.is_installed(&definition_id()));

    // The reviewed bindings became declared, digested requirements: the
    // executable definition differs from the taught one, and the steps
    // now declare capabilities.
    assert_ne!(
        artifact.resolution.executable_digest,
        artifact.resolution.approved_digest
    );
    let WorkflowIrNode::Step(step) = artifact
        .version
        .definition
        .ir
        .nodes
        .get(&node_id("step-001"))
        .expect("step-001")
    else {
        panic!("step-001 must be a step node");
    };
    assert_eq!(step.capabilities.len(), 1);
    assert_eq!(
        step.capabilities[0].capability,
        capability(BROWSER_CAPABILITY)
    );

    // Select resolves the published workflow identity.
    // run across mixed environments.
    let mut harness = harness(
        vec![success_turn()],
        default_action_script(),
        AccessRequirement::Allow,
        Vec::new(),
    );
    let outcome = run_installed(&mut harness, &artifact).await;

    assert_eq!(outcome.terminal, RunTerminal::Completed);
    assert_eq!(outcome.instance.status, WorkflowInstanceStatus::Succeeded);
    assert_eq!(outcome.path, vec![node_id("step-001"), node_id("step-002")]);

    // Mixed-environment execution: the browser turn ran on the browser
    // adapter and the desktop action on the computer adapter.
    assert_eq!(
        harness.browser_executor.executed_kinds(),
        vec![BrowserActionKind::Navigate]
    );
    assert_eq!(harness.computer_calls.load(Ordering::SeqCst), 1);
    assert_eq!(
        harness.browser.capability_state(),
        BrowserLifecycleState::Executing
    );
    assert_eq!(
        harness.computer.capability_state(),
        ComputerLifecycleState::Bound
    );

    // The observable event sequence, end to end.
    let instance_id = outcome.instance.instance_id;
    let expected = vec![
        WorkflowEvent::VersionSelected {
            version: artifact.version.version_id.clone(),
        },
        WorkflowEvent::InstanceCreated {
            instance: instance_id,
            workflow: definition_id(),
            version: artifact.version.version_id.clone(),
            status: WorkflowInstanceStatus::Pending,
        },
        WorkflowEvent::BindingAuthorized {
            instance: instance_id,
            binding: binding_id(BROWSER_BINDING),
            environment: ExecutionEnvironment::Browser,
        },
        WorkflowEvent::BindingAuthorized {
            instance: instance_id,
            binding: binding_id(COMPUTER_BINDING),
            environment: ExecutionEnvironment::Computer,
        },
        WorkflowEvent::ResourcesBound {
            instance: instance_id,
            binding: binding_id(BROWSER_BINDING),
            resources: vec![ResourceTypeId::parse("browser_profile").expect("resource type")],
        },
        WorkflowEvent::ResourcesBound {
            instance: instance_id,
            binding: binding_id(COMPUTER_BINDING),
            resources: vec![ResourceTypeId::parse("desktop_session").expect("resource type")],
        },
        WorkflowEvent::InstanceStarted {
            instance: instance_id,
        },
        WorkflowEvent::StepStarted {
            instance: instance_id,
            node: node_id("step-001"),
        },
        WorkflowEvent::StepCompleted {
            instance: instance_id,
            node: node_id("step-001"),
        },
        WorkflowEvent::StepStarted {
            instance: instance_id,
            node: node_id("step-002"),
        },
        WorkflowEvent::StepCompleted {
            instance: instance_id,
            node: node_id("step-002"),
        },
        WorkflowEvent::RunCompleted {
            instance: instance_id,
        },
    ];
    assert_eq!(harness.events.events(), expected);

    // Evidence: two approvals, three observations (1 browser + 2 desktop),
    // two traces, no recovery records.
    let verified = harness
        .lifecycle
        .verify_run(&instance_id)
        .expect("verify run");
    assert_eq!(verified.instance.status, WorkflowInstanceStatus::Succeeded);
    assert_eq!(verified.version, artifact.version.version_id);
    let count = |kind: EvidenceKind| {
        verified
            .evidence
            .iter()
            .filter(|reference| reference.kind == kind)
            .count()
    };
    assert_eq!(count(EvidenceKind::Approval), 2);
    assert_eq!(count(EvidenceKind::Observation), 3);
    assert_eq!(count(EvidenceKind::Trace), 2);
    assert_eq!(count(EvidenceKind::Recovery), 0);
    verify_evidence(&harness, &verified);

    // The capability adapters' own evidence planes recorded the run.
    let browser_records = harness.browser.session_records();
    assert_eq!(browser_records.len(), 2);
    assert_eq!(browser_records[0].kind, EvidenceKind::Observation);
    assert_eq!(browser_records[1].kind, EvidenceKind::Trace);
    assert!(!harness.computer.evidence_records().is_empty());

    // Approvals crossed the seam exactly once per binding.
    assert_eq!(harness.approvals.requests().len(), 2);
}

#[tokio::test]
async fn e2e_transient_failure_retries_with_a_fresh_approval_and_recovers() {
    let (artifact, _installs) = publish_installed(taught_approved(), "v1.0.0", (1, 0, 0));
    let mut harness = harness(
        vec![
            failed_turn(FailureKind::Transient, "stub transient timeout"),
            success_turn(),
        ],
        default_action_script(),
        AccessRequirement::Allow,
        Vec::new(),
    );
    let outcome = run_installed(&mut harness, &artifact).await;

    assert_eq!(outcome.terminal, RunTerminal::Completed);
    assert_eq!(outcome.instance.status, WorkflowInstanceStatus::Succeeded);

    // The browser executed the failed turn and the retry.
    assert_eq!(harness.browser_executor.executed_kinds().len(), 2);

    // The retry re-crossed the approval gate: a fresh grant for the same
    // binding.
    let browser_requests = harness
        .approvals
        .requests()
        .iter()
        .filter(|request| request.binding == binding_id(BROWSER_BINDING))
        .count();
    assert_eq!(browser_requests, 2);

    // One verified recovery record and one Recovered event.
    let instance_id = outcome.instance.instance_id;
    let verified = harness
        .lifecycle
        .verify_run(&instance_id)
        .expect("verify run");
    let recoveries = verified
        .evidence
        .iter()
        .filter(|reference| reference.kind == EvidenceKind::Recovery)
        .count();
    assert_eq!(recoveries, 1);
    let recovery_payloads = verified
        .evidence
        .iter()
        .filter(|reference| reference.kind == EvidenceKind::Recovery)
        .filter_map(|reference| harness.evidence.payload(&reference.locator))
        .map(serde_json::from_value::<codex_execution_contracts::Recovery>)
        .collect::<Result<Vec<_>, _>>()
        .expect("recovery records deserialize");
    assert_eq!(recovery_payloads.len(), 1);
    assert!(matches!(
        recovery_payloads[0].strategy,
        codex_execution_contracts::RecoveryStrategy::Retry
    ));
    assert_eq!(
        recovery_payloads[0].outcome,
        codex_execution_contracts::RecoveryOutcome::Recovered
    );
    assert!(harness.events.events().iter().any(|event| {
        matches!(
            event,
            WorkflowEvent::Recovered { strategy, .. } if strategy == "retry"
        )
    }));
    verify_evidence(&harness, &verified);
}

#[tokio::test]
async fn e2e_unavailable_binding_rebinds_to_a_compatible_adapter() {
    let (artifact, _installs) = publish_installed(taught_approved(), "v1.0.0", (1, 0, 0));
    let api_calls = Arc::new(AtomicUsize::new(0));
    let api = Arc::new(ApiFallbackAdapter::new(api_calls.clone()).expect("api adapter"));
    let mut harness = harness(
        vec![failed_turn(FailureKind::Unavailable, "browser bridge lost")],
        default_action_script(),
        AccessRequirement::Allow,
        vec![api],
    );
    let outcome = run_installed(&mut harness, &artifact).await;

    assert_eq!(outcome.terminal, RunTerminal::Completed);
    assert_eq!(outcome.instance.status, WorkflowInstanceStatus::Succeeded);

    // The failed attempt ran on the native browser binding; the recovered
    // action ran on the compatible API binding.
    assert_eq!(harness.browser_executor.executed_kinds().len(), 1);
    assert_eq!(api_calls.load(Ordering::SeqCst), 1);

    // The alternate needed its own authorization: browser + computer + api.
    assert_eq!(harness.approvals.requests().len(), 3);

    // The fallback decision and the verified recovery are recorded as
    // recovery evidence, and the decision names the skipped binding.
    let instance_id = outcome.instance.instance_id;
    let verified = harness
        .lifecycle
        .verify_run(&instance_id)
        .expect("verify run");
    let recovery_payloads = verified
        .evidence
        .iter()
        .filter(|reference| reference.kind == EvidenceKind::Recovery)
        .filter_map(|reference| harness.evidence.payload(&reference.locator))
        .collect::<Vec<_>>();
    assert_eq!(recovery_payloads.len(), 2);
    let decision = recovery_payloads
        .into_iter()
        .find_map(|payload| serde_json::from_value::<BindingDecision>(payload).ok())
        .expect("binding decision evidence");
    assert_eq!(decision.selected.binding, binding_id(API_BINDING));
    let recovered = harness
        .events
        .events()
        .into_iter()
        .find(|event| matches!(event, WorkflowEvent::Recovered { .. }))
        .expect("recovered event");
    let WorkflowEvent::Recovered {
        strategy, binding, ..
    } = recovered
    else {
        panic!("recovered event shape");
    };
    assert_eq!(strategy.as_str(), "rebind");
    assert_eq!(binding, binding_id(API_BINDING));
    assert!(matches!(
        decision.fallback,
        Some(FallbackRecord {
            reason: FallbackReason::PreferredNotReady { .. },
            ..
        })
    ));
    verify_evidence(&harness, &verified);
}

#[tokio::test]
async fn e2e_permanent_failure_escalates_and_fails_the_instance() {
    let (artifact, _installs) = publish_installed(taught_approved(), "v1.0.0", (1, 0, 0));
    let mut harness = harness(
        vec![failed_turn(FailureKind::Permanent, "element not found")],
        default_action_script(),
        AccessRequirement::Allow,
        Vec::new(),
    );
    let outcome = run_installed(&mut harness, &artifact).await;

    assert!(matches!(outcome.terminal, RunTerminal::Failed { .. }));
    assert_eq!(outcome.instance.status, WorkflowInstanceStatus::Failed);
    // The run stopped at the first step: the desktop step never executed.
    assert_eq!(outcome.path, vec![node_id("step-001")]);
    assert_eq!(harness.computer_calls.load(Ordering::SeqCst), 0);
    assert_eq!(harness.actions.remaining(), 1);

    let events = harness.events.events();
    assert!(events
        .iter()
        .any(|event| matches!(event, WorkflowEvent::ActionFailed { kind, .. } if *kind == FailureKind::Permanent)));
    assert!(
        events
            .iter()
            .any(|event| matches!(event, WorkflowEvent::Escalated { .. }))
    );
    assert!(
        events
            .iter()
            .any(|event| matches!(event, WorkflowEvent::RunFailed { .. }))
    );
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, WorkflowEvent::StepCompleted { .. }))
    );

    // The escalation is recorded as recovery evidence.
    let instance_id = outcome.instance.instance_id;
    let verified = harness
        .lifecycle
        .verify_run(&instance_id)
        .expect("verify run");
    let escalation = verified
        .evidence
        .iter()
        .filter(|reference| reference.kind == EvidenceKind::Recovery)
        .filter_map(|reference| harness.evidence.payload(&reference.locator))
        .map(|payload| {
            serde_json::from_value::<codex_execution_contracts::Recovery>(payload)
                .expect("recovery record")
        })
        .collect::<Vec<_>>();
    assert_eq!(escalation.len(), 1);
    assert!(matches!(
        escalation[0].strategy,
        codex_execution_contracts::RecoveryStrategy::Escalate
    ));
    assert_eq!(
        escalation[0].outcome,
        codex_execution_contracts::RecoveryOutcome::Escalated
    );
    verify_evidence(&harness, &verified);
}

#[tokio::test]
async fn e2e_approval_denial_blocks_execution_before_any_action() {
    let (artifact, _installs) = publish_installed(taught_approved(), "v1.0.0", (1, 0, 0));
    let mut harness = harness(
        vec![success_turn()],
        default_action_script(),
        AccessRequirement::Allow,
        Vec::new(),
    );
    harness
        .versions
        .publish(artifact.version.clone())
        .expect("store version");
    harness
        .lifecycle
        .select_version(&artifact.version.version_id)
        .expect("select version");
    harness.approvals.deny(binding_id(BROWSER_BINDING));

    let error = harness
        .lifecycle
        .instantiate(InstantiateRequest {
            trigger: None,
            policy: BindingPolicy::default(),
            resources: resources(),
            walk: WalkConfig::default(),
        })
        .await
        .expect_err("approval denied");
    assert!(matches!(error, WorkflowAppError::ApprovalDenied { .. }));

    // The durable instance record settled as Failed before any execution.
    let records = harness.instances.records();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].status, WorkflowInstanceStatus::Failed);

    // Nothing executed: no events beyond creation/failure, no actions
    // dispatched, no adapter preparation.
    let events = harness.events.events();
    assert!(
        events
            .iter()
            .any(|event| matches!(event, WorkflowEvent::InstanceCreated { .. }))
    );
    assert!(
        events
            .iter()
            .any(|event| matches!(event, WorkflowEvent::RunFailed { .. }))
    );
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, WorkflowEvent::BindingAuthorized { .. }))
    );
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, WorkflowEvent::InstanceStarted { .. }))
    );
    assert_eq!(harness.actions.remaining(), 2);
    assert_eq!(harness.browser_executor.executed_kinds().len(), 0);
    assert_eq!(harness.computer_calls.load(Ordering::SeqCst), 0);
    assert_eq!(
        harness.browser.capability_state(),
        BrowserLifecycleState::Declared
    );
}

#[tokio::test]
async fn e2e_policy_scope_denies_the_browser_environment() {
    let (artifact, _installs) = publish_installed(taught_approved(), "v1.0.0", (1, 0, 0));
    let mut harness = harness(
        vec![success_turn()],
        default_action_script(),
        AccessRequirement::Allow,
        Vec::new(),
    );
    harness
        .versions
        .publish(artifact.version.clone())
        .expect("store version");
    harness
        .lifecycle
        .select_version(&artifact.version.version_id)
        .expect("select version");

    // Policy scopes execution to the computer environment only: the
    // browser requirement cannot be resolved.
    let error = harness
        .lifecycle
        .instantiate(InstantiateRequest {
            trigger: None,
            policy: BindingPolicy {
                environments: EnvironmentScope::Only(vec![ExecutionEnvironment::Computer]),
                ..Default::default()
            },
            resources: resources(),
            walk: WalkConfig::default(),
        })
        .await
        .expect_err("policy denial");
    assert!(matches!(
        error,
        WorkflowAppError::BindingPlanFailed { code, .. }
            if code == codex_execution_contracts::DiagnosticCode::PolicyDenied
    ));

    let records = harness.instances.records();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].status, WorkflowInstanceStatus::Failed);
    assert_eq!(harness.approvals.requests().len(), 0);
    assert_eq!(harness.actions.remaining(), 2);
    assert_eq!(harness.browser_executor.executed_kinds().len(), 0);
    assert_eq!(harness.computer_calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn e2e_computer_policy_denial_fails_actions_and_escalates() {
    let (artifact, _installs) = publish_installed(taught_approved(), "v1.0.0", (1, 0, 0));
    let mut harness = harness(
        vec![success_turn()],
        default_action_script(),
        AccessRequirement::Deny,
        Vec::new(),
    );
    let outcome = run_installed(&mut harness, &artifact).await;

    // The computer-use policy gate refuses the target application; the
    // failure escalates instead of silently substituting anything.
    assert!(matches!(outcome.terminal, RunTerminal::Failed { .. }));
    assert_eq!(outcome.instance.status, WorkflowInstanceStatus::Failed);
    assert_eq!(harness.computer_calls.load(Ordering::SeqCst), 0);
    let events = harness.events.events();
    assert!(events.iter().any(|event| {
        matches!(event, WorkflowEvent::ActionFailed { kind, .. } if *kind == FailureKind::PolicyDenied)
    }));
    assert!(
        events
            .iter()
            .any(|event| matches!(event, WorkflowEvent::Escalated { .. }))
    );
}

#[tokio::test]
async fn ordinary_codex_behavior_is_untouched_without_a_workflow() {
    let mut harness = harness(
        vec![success_turn()],
        default_action_script(),
        AccessRequirement::Allow,
        Vec::new(),
    );
    assert!(!harness.lifecycle.is_active());

    // Every run-path operation is an explicit no-op error.
    let error = harness
        .lifecycle
        .run()
        .await
        .expect_err("no active workflow");
    assert!(matches!(error, WorkflowAppError::NoActiveWorkflow));

    // No events, no evidence, no instances, no adapter work: nothing in
    // this crate executed anything.
    assert!(harness.events.is_empty());
    assert!(harness.instances.is_empty());
    assert!(harness.evidence.is_empty());
    assert_eq!(harness.approvals.requests().len(), 0);
    assert_eq!(harness.actions.remaining(), 2);
    assert_eq!(harness.browser_executor.executed_kinds().len(), 0);
    assert_eq!(harness.computer_calls.load(Ordering::SeqCst), 0);
    assert_eq!(
        harness.browser.capability_state(),
        BrowserLifecycleState::Declared
    );
    assert_eq!(
        harness.computer.capability_state(),
        ComputerLifecycleState::Declared
    );
    for registered in harness.lifecycle.registry().bindings() {
        assert_eq!(registered.readiness.state(), ReadinessState::Declared);
    }
}

#[tokio::test]
async fn running_instance_pins_its_version_across_an_install_upgrade() {
    // Two immutable versions of the same taught content.
    let (v1, mut installs) = publish_installed(taught_approved(), "v1.0.0", (1, 0, 0));
    let (v2, _) = publish_installed(taught_approved(), "v1.1.0", (1, 1, 0));
    assert_ne!(v1.version.version_id, v2.version.version_id);

    let mut harness = harness(
        vec![success_turn()],
        default_action_script(),
        AccessRequirement::Allow,
        Vec::new(),
    );
    harness
        .versions
        .publish(v1.version.clone())
        .expect("store v1");
    harness
        .versions
        .publish(v2.version.clone())
        .expect("store v2");
    harness
        .lifecycle
        .select_version(&v1.version.version_id)
        .expect("select v1");
    let instance = harness
        .lifecycle
        .instantiate(InstantiateRequest {
            trigger: None,
            policy: BindingPolicy::default(),
            resources: resources(),
            walk: WalkConfig::default(),
        })
        .await
        .expect("instantiate");
    let instance_id = instance.instance_id;
    let outcome = harness.lifecycle.run().await.expect("run");
    assert_eq!(outcome.instance.status, WorkflowInstanceStatus::Succeeded);

    // Upgrade the installation through the explicit, reviewable path.
    let plan = installs
        .propose_update(&definition_id(), &v2.reference, Some("v1.1.0".to_string()))
        .expect("propose update");
    let decision = installs
        .decide_update(&plan, UpdateDecision::Apply)
        .expect("decide update");
    assert!(decision.applied);
    assert_eq!(
        installs
            .installed(&definition_id())
            .expect("installed")
            .installed
            .version_id,
        v2.version.version_id
    );

    // The already-settled instance still pins and verifies against v1.
    let verified = harness
        .lifecycle
        .verify_run(&instance_id)
        .expect("verify run");
    assert_eq!(verified.version, v1.version.version_id);
    assert_eq!(verified.instance.status, WorkflowInstanceStatus::Succeeded);
    verify_evidence(&harness, &verified);
    assert_eq!(installs.history_for(&definition_id()).len(), 1);
}

#[tokio::test]
async fn tampered_version_records_never_become_executable() {
    let (artifact, _installs) = publish_installed(taught_approved(), "v1.0.0", (1, 0, 0));
    let mut harness = harness(
        vec![success_turn()],
        default_action_script(),
        AccessRequirement::Allow,
        Vec::new(),
    );
    harness
        .versions
        .publish(artifact.version.clone())
        .expect("store version");
    harness
        .lifecycle
        .select_version(&artifact.version.version_id)
        .expect("select version");
    // Silently mutate the stored record under the same identity.
    let mut tampered = artifact.version.clone();
    tampered.definition.description = Some("silently mutated".to_string());
    harness
        .versions
        .publish(tampered)
        .expect("publish tampered");

    let error = harness
        .lifecycle
        .select_version(&artifact.version.version_id)
        .expect_err("integrity failure");
    assert!(matches!(error, WorkflowAppError::VersionIntegrity { .. }));
}

#[tokio::test]
async fn wait_node_pauses_the_running_instance() {
    // Control-plane-authored IR: one step, then a wait for a human event.
    // The teaching compiler still gates publication (validation,
    // simulation, approval, digest-pinned finalization).
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
    let ir = WorkflowIr {
        ir_format: IR_FORMAT_VERSION,
        entry: node_id("step-001"),
        nodes,
        conditions: BTreeMap::new(),
    };
    let mut candidate =
        WorkflowCandidate::from_ir(ir, Some("authored await-signoff workflow".to_string()))
            .expect("candidate");
    let summary = candidate.validate().expect("validate");
    assert!(summary.is_clean());
    candidate
        .simulate(SimulationConfig::default())
        .expect("simulate");
    candidate
        .approve(
            ApprovalDecision::new("tech-lead", "review-wait", ApprovalDecisionKind::Approved)
                .expect("decision"),
        )
        .expect("approve");
    let approved = candidate
        .finalize(WorkflowDefinitionId::parse("await-signoff").expect("definition id"))
        .expect("finalize");

    let (artifact, _installs) = publish_with(
        approved,
        WorkflowDefinitionId::parse("await-signoff").expect("definition id"),
        "v1.0.0",
        (1, 0, 0),
        BTreeMap::new(),
    );

    let mut harness = harness(Vec::new(), Vec::new(), AccessRequirement::Allow, Vec::new());
    let outcome = run_installed(&mut harness, &artifact).await;

    // The run paused at the wait node; nothing executed and nothing failed.
    assert_eq!(
        outcome.terminal,
        RunTerminal::Paused {
            node: node_id("wait-signoff"),
            reason: "wait".to_string(),
        }
    );
    assert_eq!(outcome.instance.status, WorkflowInstanceStatus::Paused);
    assert_eq!(
        outcome.path,
        vec![node_id("step-001"), node_id("wait-signoff")]
    );
    assert_eq!(harness.browser_executor.executed_kinds().len(), 0);
    assert_eq!(harness.computer_calls.load(Ordering::SeqCst), 0);
    assert!(
        harness
            .events
            .events()
            .iter()
            .any(|event| matches!(event, WorkflowEvent::RunPaused { .. }))
    );
    let verified = harness
        .lifecycle
        .verify_run(&outcome.instance.instance_id)
        .expect("verify run");
    assert_eq!(verified.instance.status, WorkflowInstanceStatus::Paused);
    verify_evidence(&harness, &verified);
}
