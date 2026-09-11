//! WO-016 end-to-end resource-provider tests.
//!
//! Every scenario is **static**: in-process doubles stand in for the host
//! seams (the teaching compiler's forge doubles, the WO-015 remote
//! desktop bridge, the terminal test adapter), and the two in-crate
//! fixture providers stand in for concrete infrastructure — so no
//! network, no cloud API, no container runtime, and no vendor SDK is
//! touched. The scenarios map to the Work Order's required evidence:
//!
//! - a workflow's declared resource requirement satisfied through a
//!   provider-minted `ResourceBinding` into workflow-app instantiate (the
//!   golden path, compute class under the Terminal peer);
//! - a remote-desktop-class provider resource reaching the Computer Use
//!   semantic contract through the WO-015 remote-desktop adapter;
//! - provider loss flowing through the existing contract recovery model
//!   (environment loss -> `Unavailable` -> rebind) with workflow semantic
//!   identity intact;
//! - registry-level `EnvironmentLost` -> `Unavailable` -> rebind
//!   resolution with a freshly minted opaque resource id;
//! - unsupported lifecycle operations surfacing explicitly;
//! - provider-neutral conformance parity across the two materially
//!   different fixtures;
//! - provider-neutral binding (no provider types anywhere in workflow
//!   contracts);
//! - the no-op default: ordinary runs are untouched when no provider is
//!   configured.
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

use codex_env_adapters::Access;
use codex_env_adapters::AccessTable;
use codex_env_adapters::BridgeCallFailure;
use codex_env_adapters::BridgeEndpoint;
use codex_env_adapters::BridgeHealth;
use codex_env_adapters::REMOTE_DESKTOP_CAPABILITY;
use codex_env_adapters::REMOTE_DESKTOP_SESSION_RESOURCE;
use codex_env_adapters::RemoteDesktopAction;
use codex_env_adapters::RemoteDesktopBridge;
use codex_env_adapters::RemoteDesktopEnvironmentAdapter;
use codex_env_adapters::RemoteDesktopOutcome;
use codex_env_adapters::RemoteDesktopPolicy;
use codex_env_adapters::RemoteScreenObservation;
use codex_env_adapters::RemoteSessionState;
use codex_execution_contracts::Action;
use codex_execution_contracts::ActionInputs;
use codex_execution_contracts::ActionResult;
use codex_execution_contracts::AdapterDescriptor;
use codex_execution_contracts::AdapterExecuteFuture;
use codex_execution_contracts::AdapterId;
use codex_execution_contracts::AdapterPrepareFuture;
use codex_execution_contracts::AdapterProbeFuture;
use codex_execution_contracts::AuthorizationGrant;
use codex_execution_contracts::BindingClass;
use codex_execution_contracts::BindingDiagnostic;
use codex_execution_contracts::BindingPolicy;
use codex_execution_contracts::CapabilityBindingId;
use codex_execution_contracts::CapabilityRegistry;
use codex_execution_contracts::DiagnosticCode;
use codex_execution_contracts::EnvironmentAdapter;
use codex_execution_contracts::ExecutionEnvironment;
use codex_execution_contracts::ExecutionFailure;
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
use codex_execution_contracts::TransitionCause;
use codex_resource_providers::ExecutionResourceProvider;
use codex_resource_providers::LifecycleOp;
use codex_resource_providers::LifecycleOutcome;
use codex_resource_providers::LocalContainersFixture;
use codex_resource_providers::ProviderCredentials;
use codex_resource_providers::ProviderRegistry;
use codex_resource_providers::ResizeRequest;
use codex_resource_providers::ResourceClass;
use codex_resource_providers::ResourceSpec;
use codex_resource_providers::ResourceStatus;
use codex_resource_providers::SandboxCloudFixture;
use codex_resource_providers::assert_provider_conformance;
use codex_resource_providers::bind;
use codex_resource_providers::bind_as;
use codex_resource_providers::evidence::EVIDENCE_LOCATOR_PREFIX;
use codex_teaching_compiler::ApprovalDecision;
use codex_teaching_compiler::ApprovalDecisionKind;
use codex_teaching_compiler::SimulationConfig;
use codex_teaching_compiler::WorkflowCandidate;
use codex_workflow_app::ApprovalRequest;
use codex_workflow_app::ApprovalSource;
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
use codex_workflow_contracts::ResourceRequirement;
use codex_workflow_contracts::ResourceTypeId;
use codex_workflow_contracts::SemanticVersion;
use codex_workflow_contracts::StepNode;
use codex_workflow_contracts::TriggerClass;
use codex_workflow_contracts::TriggerSource;
use codex_workflow_contracts::WorkflowDefinitionId;
use codex_workflow_contracts::WorkflowInstanceId;
use codex_workflow_contracts::WorkflowInstanceStatus;
use codex_workflow_contracts::WorkflowIr;
use codex_workflow_contracts::WorkflowIrNode;
use codex_workflow_contracts::WorkflowManifest;
use codex_workflow_contracts::WorkflowProvenance;
use codex_workflow_contracts::WorkflowRepositoryId;
use codex_workflow_forge::InMemoryForge;
use codex_workflow_forge::InstallRegistry;
use codex_workflow_forge::publish_release;
use pretty_assertions::assert_eq;

const TERMINAL_ADAPTER_ID: &str = "test-terminal-sandbox";
const TERMINAL_BINDING: &str = "test-terminal-sandbox:run_terminal_command";
const TERMINAL_FALLBACK_ADAPTER_ID: &str = "test-terminal-fallback";
const TERMINAL_FALLBACK_BINDING: &str = "test-terminal-fallback:run_terminal_command";
const RUN_COMMAND_CAPABILITY: &str = "run_terminal_command";
const SANDBOX_COMPUTE_RESOURCE: &str = "sandbox_compute";
const REMOTE_BINDING: &str = "codex-remote-desktop-env:control_remote_desktop";

fn node_id(value: &str) -> IrNodeId {
    IrNodeId::parse(value).expect("node id")
}

fn definition_id() -> WorkflowDefinitionId {
    WorkflowDefinitionId::parse("resource-provider-check").expect("definition id")
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

fn sandbox_type() -> ResourceTypeId {
    ResourceTypeId::parse(SANDBOX_COMPUTE_RESOURCE).expect("resource type")
}

/// A sandbox-shaped spec for `class`, within every fixture quota.
fn sandbox_spec(class: ResourceClass) -> ResourceSpec {
    ResourceSpec {
        class,
        cpu_millis: Some(1_000),
        memory_mib: Some(2_048),
        disk_mib: Some(8_192),
        gpu: None,
        os: None,
        region: None,
        labels: BTreeMap::new(),
        network: None,
    }
}

/// A persistent-workspace spec, within the local fixture quota.
fn persistent_spec() -> ResourceSpec {
    ResourceSpec {
        class: ResourceClass::PersistentWorkspace,
        cpu_millis: Some(500),
        memory_mib: Some(1_024),
        disk_mib: Some(4_096),
        gpu: None,
        os: Some("linux".to_owned()),
        region: None,
        labels: BTreeMap::new(),
        network: None,
    }
}

/// Authors a one-step workflow whose step requires `capability_name`,
/// optionally declaring semantic resource requirements, drives the
/// teaching compiler to an approved definition, and publishes it through
/// the forge lifecycle (seal -> release -> install).
fn authored(
    capability_name: &str,
    declared_resources: &[&str],
) -> (PublishedArtifact, InstallRegistry) {
    let mut nodes = BTreeMap::new();
    let mut bindings = BTreeMap::new();
    nodes.insert(
        node_id("step-001"),
        WorkflowIrNode::Step(StepNode {
            capabilities: Vec::new(),
            roles: Vec::new(),
            description: Some("authored resource-provider check".to_owned()),
            next: None,
        }),
    );
    bindings.insert(
        node_id("step-001"),
        vec![CapabilityRequirement {
            capability: capability(capability_name),
            purpose: Some(format!("step-001 requires {capability_name}")),
        }],
    );
    let ir = WorkflowIr {
        ir_format: IR_FORMAT_VERSION,
        entry: node_id("step-001"),
        nodes,
        conditions: BTreeMap::new(),
    };
    let mut candidate =
        WorkflowCandidate::from_ir(ir, Some("authored resource-provider check".to_string()))
            .expect("candidate");
    let summary = candidate.validate().expect("validate");
    assert!(summary.is_clean());
    candidate
        .simulate(SimulationConfig::default())
        .expect("simulate");
    candidate
        .approve(
            ApprovalDecision::new(
                "tech-lead",
                "review-resources",
                ApprovalDecisionKind::Approved,
            )
            .expect("decision"),
        )
        .expect("approve");
    let mut approved = candidate.finalize(definition_id()).expect("finalize");

    // The publisher declares the workflow's semantic resource
    // requirements after approval-plane review (the documented WO-008
    // hand-off): the definition carries logical resource types only, and
    // the hand-off digest is recomputed to stay consistent.
    if !declared_resources.is_empty() {
        approved.definition.dependencies.resources = declared_resources
            .iter()
            .map(|resource_type| ResourceRequirement {
                resource_type: ResourceTypeId::parse(*resource_type)
                    .expect("declared resource type"),
                purpose: Some(format!("workflow requires {resource_type}")),
            })
            .collect();
        approved.digest = approved.definition.digest().expect("definition digest");
    }

    let repository = repository_id();
    let mut forge = InMemoryForge::new(ForgeKind::parse("github").expect("forge kind"));
    let manifest = WorkflowManifest {
        manifest_version: MANIFEST_FORMAT_VERSION,
        workflow: definition_id(),
        display_name: Some("resource-provider-check".to_string()),
        description: None,
        repository: repository.clone(),
        definition_path: RepositoryRelativePath::parse("workflows/resource-provider-check.json")
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
        .commit(
            &repository,
            &branch,
            "publish v1.0.0".to_string(),
            "acme-bot",
        )
        .expect("commit");
    let revision = forge
        .resolve_ref_blocking(&repository, &branch)
        .expect("resolve ref");
    let artifact = publish(PublishRequest {
        approved,
        repository: repository.clone(),
        source_revision: revision.clone(),
        semantic_version: SemanticVersion::new(1, 0, 0),
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
        "v1.0.0",
        &revision,
        std::slice::from_ref(&artifact.reference),
    )
    .expect("release");
    assert_eq!(release.versions, vec![artifact.version.version_id.clone()]);
    let mut installs = InstallRegistry::new();
    install_version(&mut installs, &artifact).expect("install");
    (artifact, installs)
}

fn terminal_workflow() -> (PublishedArtifact, InstallRegistry) {
    authored(RUN_COMMAND_CAPABILITY, &[SANDBOX_COMPUTE_RESOURCE])
}

fn remote_workflow() -> (PublishedArtifact, InstallRegistry) {
    authored(REMOTE_DESKTOP_CAPABILITY, &[])
}

fn terminal_action() -> Action {
    Action::new(OperationId::parse("run_command").expect("operation"))
}

fn terminal_action_script() -> Vec<ScriptedAction> {
    vec![
        ScriptedAction::new("step-001", RUN_COMMAND_CAPABILITY, terminal_action())
            .expect("terminal action"),
    ]
}

fn remote_type_text_action() -> Action {
    let mut inputs = ActionInputs::default();
    inputs.insert("text", serde_json::json!("deploy summary"));
    Action {
        operation: OperationId::parse("type_text").expect("operation"),
        target: None,
        inputs,
    }
}

fn remote_action_script() -> Vec<ScriptedAction> {
    vec![
        ScriptedAction::new(
            "step-001",
            REMOTE_DESKTOP_CAPABILITY,
            remote_type_text_action(),
        )
        .expect("remote action"),
    ]
}

/// A Terminal-environment test adapter requiring the sandbox_compute
/// resource. It counts successful executions and can fail its first
/// action with a chosen failure classification for recovery scenarios.
struct TerminalSandboxAdapter {
    descriptor: AdapterDescriptor,
    executions: Arc<AtomicUsize>,
    fail_first: Mutex<Option<FailureKind>>,
}

impl TerminalSandboxAdapter {
    fn new(adapter_id: &str, class: BindingClass, fail_first: Option<FailureKind>) -> Arc<Self> {
        let descriptor = AdapterDescriptor {
            adapter: AdapterId::parse(adapter_id).expect("adapter id"),
            environment: ExecutionEnvironment::Terminal,
            class,
            provides: vec![ProvidedCapability {
                capability: capability(RUN_COMMAND_CAPABILITY),
                resources: vec![sandbox_type()],
            }],
        };
        descriptor.validate().expect("descriptor");
        Arc::new(Self {
            descriptor,
            executions: Arc::new(AtomicUsize::new(0)),
            fail_first: Mutex::new(fail_first),
        })
    }

    fn executions(&self) -> usize {
        self.executions.load(Ordering::SeqCst)
    }
}

impl EnvironmentAdapter for TerminalSandboxAdapter {
    fn descriptor(&self) -> &AdapterDescriptor {
        &self.descriptor
    }

    fn probe(&self) -> AdapterProbeFuture<'_> {
        Box::pin(async move { Ok(ProbeReport::ready()) })
    }

    fn prepare(&self, request: PrepareRequest) -> AdapterPrepareFuture<'_> {
        Box::pin(async move {
            let session = match SessionHandle::parse(request.binding.as_ref()) {
                Ok(handle) => handle,
                Err(error) => {
                    let failure = ExecutionFailure::new(FailureKind::Permanent, error.to_string())
                        .expect("failure");
                    return Err(failure);
                }
            };
            Ok(PreparedSession {
                binding: request.binding,
                session,
            })
        })
    }

    fn execute<'a>(
        &'a self,
        session: &'a SessionHandle,
        _action: Action,
    ) -> AdapterExecuteFuture<'a> {
        Box::pin(async move {
            let binding = CapabilityBindingId::parse(session.as_ref()).expect("binding id");
            if let Some(kind) = self.fail_first.lock().expect("fail flag").take() {
                let failure = ExecutionFailure::new(
                    kind,
                    "terminal environment lost its provider resource mid-flight",
                )
                .expect("failure");
                return ActionResult::failed(binding, failure);
            }
            self.executions.fetch_add(1, Ordering::SeqCst);
            ActionResult::succeeded(binding)
        })
    }
}

/// A reachable scripted remote-desktop bridge (the WO-015 double shape).
struct ScriptedRemoteBridge {
    connects: Arc<AtomicUsize>,
    executes: Arc<AtomicUsize>,
}

impl ScriptedRemoteBridge {
    fn new() -> (Arc<Self>, Arc<AtomicUsize>, Arc<AtomicUsize>) {
        let connects = Arc::new(AtomicUsize::new(0));
        let executes = Arc::new(AtomicUsize::new(0));
        let bridge = Arc::new(Self {
            connects: connects.clone(),
            executes: executes.clone(),
        });
        (bridge, connects, executes)
    }
}

impl RemoteDesktopBridge for ScriptedRemoteBridge {
    fn health(&self) -> Result<BridgeHealth, BridgeCallFailure> {
        Ok(BridgeHealth {
            endpoint: BridgeEndpoint {
                bridge_kind: "rdp".to_string(),
                runtime: "stub-device-bridge".to_string(),
                version: "1.0.0-test".to_string(),
            },
            latency_ms: Some(5),
        })
    }

    fn connect(&self, host: &str) -> Result<RemoteScreenObservation, BridgeCallFailure> {
        self.connects.fetch_add(1, Ordering::SeqCst);
        Ok(RemoteScreenObservation {
            host: host.to_string(),
            state: RemoteSessionState::Connected,
            width: 1920,
            height: 1080,
            foreground_application: Some("ops-console".to_string()),
            captured_at_unix_ms: 7,
        })
    }

    fn execute(
        &self,
        host: &str,
        _action: RemoteDesktopAction,
    ) -> Result<RemoteDesktopOutcome, BridgeCallFailure> {
        self.executes.fetch_add(1, Ordering::SeqCst);
        Ok(RemoteDesktopOutcome {
            observation: Some(RemoteScreenObservation {
                host: host.to_string(),
                state: RemoteSessionState::Connected,
                width: 1920,
                height: 1080,
                foreground_application: Some("ops-console".to_string()),
                captured_at_unix_ms: 8,
            }),
            detail: Some("typed".to_string()),
        })
    }
}

fn allowing_remote_policy() -> RemoteDesktopPolicy {
    RemoteDesktopPolicy {
        host_access: AccessTable {
            default: Some(Access::Allow),
            entries: BTreeMap::new(),
        },
        allow_session_takeover: None,
    }
}

/// The workflow-app harness: a lifecycle over the in-memory seams.
struct AppHarness {
    lifecycle: WorkflowLifecycle,
    versions: InMemoryVersionStore,
    instances: InMemoryInstanceStore,
    evidence: InMemoryEvidenceStore,
    actions: ScriptedActionSource,
    events: RecordingEventSink,
}

fn app_harness(registry: CapabilityRegistry, script: Vec<ScriptedAction>) -> AppHarness {
    let versions = InMemoryVersionStore::new();
    let instances = InMemoryInstanceStore::new();
    let evidence = InMemoryEvidenceStore::new();
    let approvals = InMemoryApprovalSource::approving("tech-lead", evidence.clone());
    let actions = ScriptedActionSource::new(script);
    let events = RecordingEventSink::new();
    let lifecycle = WorkflowLifecycle::new(LifecycleDeps {
        versions: Box::new(versions.clone()),
        instances: Box::new(instances.clone()),
        evidence: Box::new(evidence.clone()),
        approvals: Box::new(approvals),
        actions: Box::new(actions.clone()),
        events: Box::new(events.clone()),
        registry,
    });
    AppHarness {
        lifecycle,
        versions,
        instances,
        evidence,
        actions,
        events,
    }
}

/// Selects, instantiates (with `resources`), and runs `artifact`.
async fn run_installed(
    harness: &mut AppHarness,
    artifact: &PublishedArtifact,
    resources: Vec<ResourceBinding>,
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
            resources,
            walk: WalkConfig::default(),
        })
        .await
        .expect("instantiate");
    assert_eq!(instance.status, WorkflowInstanceStatus::Running);
    harness.lifecycle.run().await.expect("run")
}

/// Verifies every evidence reference on a settled run's instance record.
fn verify_instance_evidence(harness: &AppHarness, outcome: &codex_workflow_app::RunOutcome) {
    for reference in &outcome.instance.evidence {
        assert!(
            harness.evidence.verify(reference).expect("evidence verify"),
            "evidence {} must digest to its recorded value",
            reference.locator
        );
    }
}

#[tokio::test]
async fn e2e_workflow_declares_resource_need_bound_by_provider() {
    let (artifact, installs) = terminal_workflow();
    assert!(installs.is_installed(&definition_id()));

    // The published definition declares the resource requirement
    // semantically: a logical type, no provider specifics.
    let declared = &artifact.version.definition.dependencies.resources;
    assert_eq!(declared.len(), 1);
    assert_eq!(declared[0].resource_type, sandbox_type());

    let terminal =
        TerminalSandboxAdapter::new(TERMINAL_ADAPTER_ID, BindingClass::CodexNative, None);
    let mut registry = CapabilityRegistry::new();
    registry
        .register_adapter(terminal.clone())
        .expect("register terminal adapter");

    // Host wiring: the provider is registered with a credential
    // *reference* — an environment-variable name, never a value.
    let provider = Arc::new(SandboxCloudFixture::new(4).expect("fixture"));
    let mut providers = ProviderRegistry::new();
    let provider_id = providers
        .register_with_credentials(
            provider.clone(),
            ProviderCredentials::parse_env_key("SANDBOX_CLOUD_API_KEY").expect("env key"),
        )
        .expect("register provider");
    assert_eq!(
        providers
            .credentials(&provider_id)
            .expect("credential reference")
            .env_key_name(),
        "SANDBOX_CLOUD_API_KEY"
    );
    let fleet = providers.probe_all();
    assert_eq!(fleet.len(), 1);
    assert!(fleet[0].1.is_ok());

    // The provider provisions the resource the workflow requires, and
    // the bind surface mints the plain WO-005 record.
    let sandbox = providers.provider(&provider_id).expect("provider");
    let handle = sandbox
        .create(sandbox_spec(ResourceClass::Sandbox))
        .expect("create sandbox resource");
    let resource_binding = bind(&handle).expect("mint binding");
    assert_eq!(resource_binding.resource_type, sandbox_type());
    assert_eq!(resource_binding.holder, ExecutionEnvironment::Terminal);
    assert_eq!(resource_binding.resource, handle.resource);

    let mut harness = app_harness(registry, terminal_action_script());
    let outcome = run_installed(&mut harness, &artifact, vec![resource_binding.clone()]).await;

    assert_eq!(outcome.terminal, RunTerminal::Completed);
    assert_eq!(outcome.instance.status, WorkflowInstanceStatus::Succeeded);
    assert_eq!(outcome.path, vec![node_id("step-001")]);
    assert_eq!(terminal.executions(), 1);
    assert_eq!(harness.actions.remaining(), 0);
    assert_eq!(harness.instances.records().len(), 1);
    verify_instance_evidence(&harness, &outcome);

    // The provider-minted binding is the active registry binding for the
    // workflow's declared resource type, and the binding plan bound it.
    let active = harness
        .lifecycle
        .registry()
        .resource_binding(&sandbox_type())
        .expect("active resource binding");
    assert_eq!(active, &resource_binding);
    let bound_events = harness
        .events
        .events()
        .iter()
        .filter(|event| {
            matches!(event, WorkflowEvent::ResourcesBound { resources, .. }
            if resources.iter().any(|resource_type| *resource_type == sandbox_type()))
        })
        .count();
    assert_eq!(bound_events, 1);

    // Provider evidence captured the provisioning fact under
    // provider-scoped locators.
    let records = provider.evidence_records();
    assert!(!records.is_empty());
    assert_eq!(records[0].kind, EvidenceKind::Artifact);
    assert!(
        records[0]
            .locator
            .starts_with(&format!("{EVIDENCE_LOCATOR_PREFIX}/fixture-sandbox-cloud/"))
    );
}

#[tokio::test]
async fn e2e_remote_desktop_resource_reaches_computer_use_contract() {
    let (artifact, _installs) = remote_workflow();
    let (bridge, connects, executes) = ScriptedRemoteBridge::new();
    let remote = Arc::new(
        RemoteDesktopEnvironmentAdapter::new(allowing_remote_policy(), bridge)
            .expect("remote adapter"),
    );
    let mut registry = CapabilityRegistry::new();
    registry
        .register_adapter(remote)
        .expect("register remote adapter");

    // The provider provisions a remote-desktop-class resource; the host
    // binds it under the WO-015 remote-desktop logical type the adapter
    // requires, with the class-derived Computer holder.
    let provider = Arc::new(SandboxCloudFixture::new(2).expect("fixture"));
    let handle = provider
        .create(sandbox_spec(ResourceClass::RemoteDesktop))
        .expect("create desktop resource");
    assert_eq!(handle.class, ResourceClass::RemoteDesktop);
    let resource_binding = bind_as(
        &handle,
        ResourceTypeId::parse(REMOTE_DESKTOP_SESSION_RESOURCE).expect("resource type"),
    )
    .expect("bind under the adapter's logical type");
    assert_eq!(resource_binding.holder, ExecutionEnvironment::Computer);
    assert_eq!(resource_binding.resource, handle.resource);

    let mut harness = app_harness(registry, remote_action_script());
    let outcome = run_installed(&mut harness, &artifact, vec![resource_binding.clone()]).await;

    assert_eq!(outcome.terminal, RunTerminal::Completed);
    assert_eq!(outcome.instance.status, WorkflowInstanceStatus::Succeeded);
    assert_eq!(connects.load(Ordering::SeqCst), 1);
    assert_eq!(executes.load(Ordering::SeqCst), 1);
    verify_instance_evidence(&harness, &outcome);

    // The remote-desktop binding routes through the Computer peer
    // environment: the Computer Use semantic contract is the reach, and
    // the provider-minted instance is the resource binding beneath it.
    let binding = harness
        .lifecycle
        .registry()
        .binding(&binding_id(REMOTE_BINDING))
        .expect("remote binding");
    assert_eq!(binding.binding.environment, ExecutionEnvironment::Computer);
    let active = harness
        .lifecycle
        .registry()
        .resource_binding(&ResourceTypeId::parse(REMOTE_DESKTOP_SESSION_RESOURCE).expect("type"))
        .expect("active resource binding");
    assert_eq!(active, &resource_binding);
    assert_eq!(active.holder, ExecutionEnvironment::Computer);
}

#[tokio::test]
async fn e2e_provider_loss_rebinds_through_contract_recovery() {
    let (artifact, _installs) = terminal_workflow();
    let primary = TerminalSandboxAdapter::new(
        TERMINAL_ADAPTER_ID,
        BindingClass::CodexNative,
        Some(FailureKind::Unavailable),
    );
    let fallback =
        TerminalSandboxAdapter::new(TERMINAL_FALLBACK_ADAPTER_ID, BindingClass::Compatible, None);
    let mut registry = CapabilityRegistry::new();
    registry
        .register_adapter(primary.clone())
        .expect("register primary");
    registry
        .register_adapter(fallback.clone())
        .expect("register fallback");

    // The provider provisions the first resource, then loses it: the
    // host observes the loss, mints a replacement (a NEW opaque id), and
    // instantiates the workflow against the replacement binding.
    let provider = Arc::new(SandboxCloudFixture::new(4).expect("fixture"));
    let first = provider
        .create(sandbox_spec(ResourceClass::Sandbox))
        .expect("first resource");
    provider.lose_resource(&first.resource);
    assert_eq!(
        provider.status(&first).expect("status"),
        ResourceStatus::Lost
    );
    let replacement = match provider.lifecycle(&first, LifecycleOp::Create) {
        Ok(LifecycleOutcome::Transitioned(handle)) => handle,
        other => panic!("re-create must mint a replacement, got {other:?}"),
    };
    assert_ne!(replacement.resource, first.resource);
    assert_eq!(replacement.status, ResourceStatus::Active);
    let replacement_binding = bind(&replacement).expect("replacement binding");

    let mut harness = app_harness(registry, terminal_action_script());
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
            resources: vec![replacement_binding.clone()],
            walk: WalkConfig::default(),
        })
        .await
        .expect("instantiate");
    assert_eq!(instance.status, WorkflowInstanceStatus::Running);

    // The run: the primary binding's environment fails mid-flight
    // (Unavailable), the contract recovery rebinds to the compatible
    // alternate binding, and the run completes on the replacement
    // resource.
    let outcome = harness.lifecycle.run().await.expect("run");
    assert_eq!(outcome.terminal, RunTerminal::Completed);
    assert_eq!(outcome.instance.status, WorkflowInstanceStatus::Succeeded);
    assert_eq!(primary.executions(), 0);
    assert_eq!(fallback.executions(), 1);
    verify_instance_evidence(&harness, &outcome);

    // The recovery flowed through the frozen pipeline: action failure
    // classified Unavailable, then a rebind-strategy recovery.
    let events = harness.events.events();
    assert!(events.iter().any(|event| matches!(
        event,
        WorkflowEvent::ActionFailed { kind: FailureKind::Unavailable, binding, .. }
            if *binding == binding_id(TERMINAL_BINDING)
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        WorkflowEvent::Recovered { strategy, binding, .. }
            if strategy == "rebind" && *binding == binding_id(TERMINAL_FALLBACK_BINDING)
    )));
    assert!(
        outcome
            .instance
            .evidence
            .iter()
            .any(|reference| reference.kind == EvidenceKind::Recovery)
    );

    // Semantic identity is untouched: the instance pins the same
    // immutable version, which still verifies end to end, and the
    // replacement resource is the one actually bound.
    assert_eq!(outcome.instance.version, artifact.version.version_id);
    artifact
        .version
        .verify_integrity()
        .expect("integrity intact");
    let active = harness
        .lifecycle
        .registry()
        .resource_binding(&sandbox_type())
        .expect("active resource binding");
    assert_eq!(active, &replacement_binding);
    assert_ne!(active.resource, first.resource);
}

#[tokio::test]
async fn e2e_environment_lost_drives_unavailable_and_rebind_resolution() {
    let (artifact, _installs) = terminal_workflow();
    let primary = TerminalSandboxAdapter::new(TERMINAL_ADAPTER_ID, BindingClass::CodexNative, None);
    let fallback =
        TerminalSandboxAdapter::new(TERMINAL_FALLBACK_ADAPTER_ID, BindingClass::Compatible, None);
    let mut registry = CapabilityRegistry::new();
    registry
        .register_adapter(primary)
        .expect("register primary");
    registry
        .register_adapter(fallback)
        .expect("register fallback");
    registry.refresh_readiness().await.expect("refresh");

    let provider = Arc::new(SandboxCloudFixture::new(4).expect("fixture"));
    let first = provider
        .create(sandbox_spec(ResourceClass::Sandbox))
        .expect("first resource");
    let first_binding = bind(&first).expect("first binding");
    registry
        .bind_resource(first_binding)
        .expect("bind resource");

    let requirement = CapabilityRequirement {
        capability: capability(RUN_COMMAND_CAPABILITY),
        purpose: None,
    };
    let policy = BindingPolicy::default();

    // Initial resolution prefers the Codex-native binding, with no
    // fallback.
    let decision = registry
        .resolve(&requirement, &policy)
        .expect("initial resolution");
    assert_eq!(decision.selected.binding, binding_id(TERMINAL_BINDING));
    assert!(decision.fallback.is_none());

    // Authorize and bind the primary: AUTHORIZED -> BOUND, attaching the
    // first resource.
    let evidence = InMemoryEvidenceStore::new();
    let mut approvals = InMemoryApprovalSource::approving("tech-lead", evidence);
    let grant = AuthorizationGrant::new(
        binding_id(TERMINAL_BINDING),
        approvals
            .approve(&ApprovalRequest {
                instance: WorkflowInstanceId::generate(),
                binding: binding_id(TERMINAL_BINDING),
                environment: ExecutionEnvironment::Terminal,
                capability: capability(RUN_COMMAND_CAPABILITY),
            })
            .expect("approve"),
    )
    .expect("grant");
    registry
        .advance(
            &binding_id(TERMINAL_BINDING),
            TransitionCause::Authorized(grant),
        )
        .expect("authorize");
    let attached_first = registry
        .bind(&binding_id(TERMINAL_BINDING))
        .expect("bind primary");
    assert_eq!(attached_first.len(), 1);
    assert_eq!(attached_first[0].resource, first.resource);

    // Provider loss: EnvironmentLost moves the binding to Unavailable
    // with a diagnosable diagnostic.
    let env_lost = BindingDiagnostic::new(
        DiagnosticCode::EnvironmentLost,
        format!("sandbox provider lost resource {}", first.resource),
    )
    .in_environment(ExecutionEnvironment::Terminal);
    registry
        .advance(
            &binding_id(TERMINAL_BINDING),
            TransitionCause::EnvironmentLost(env_lost.clone()),
        )
        .expect("environment lost");
    let registered = registry
        .binding(&binding_id(TERMINAL_BINDING))
        .expect("primary binding");
    assert_eq!(registered.readiness.state(), ReadinessState::Unavailable);
    assert_eq!(registered.readiness.diagnostic(), Some(&env_lost));

    // Resolution now rebinds: the preferred binding is skipped with the
    // structured fallback reason, and the compatible alternate is
    // selected.
    let rebound = registry
        .resolve(&requirement, &policy)
        .expect("rebind resolution");
    assert_eq!(
        rebound.selected.binding,
        binding_id(TERMINAL_FALLBACK_BINDING)
    );
    match &rebound.fallback {
        Some(FallbackRecord {
            reason: FallbackReason::PreferredNotReady { state, diagnostic },
            ..
        }) => {
            assert_eq!(*state, ReadinessState::Unavailable);
            assert_eq!(diagnostic, &Some(env_lost));
        }
        other => panic!("expected a preferred-not-ready fallback record, got {other:?}"),
    }

    // The rebind decision converts to recovery evidence the control
    // plane must store, keyed by a provider-scoped locator.
    let reference = rebound
        .to_evidence_reference(format!("{EVIDENCE_LOCATOR_PREFIX}/e2e/rebind-1"))
        .expect("recovery evidence");
    assert_eq!(reference.kind, EvidenceKind::Recovery);

    // The host mints a replacement resource (new opaque id), rebinds the
    // resource type, and the fallback binding attaches the NEW
    // resource.
    let replacement = match provider.lifecycle(&first, LifecycleOp::Create) {
        Ok(LifecycleOutcome::Transitioned(handle)) => handle,
        other => panic!("re-create must mint a replacement, got {other:?}"),
    };
    assert_ne!(replacement.resource, first.resource);
    registry
        .bind_resource(bind(&replacement).expect("replacement binding"))
        .expect("rebind resource type");
    let grant = AuthorizationGrant::new(
        binding_id(TERMINAL_FALLBACK_BINDING),
        approvals
            .approve(&ApprovalRequest {
                instance: WorkflowInstanceId::generate(),
                binding: binding_id(TERMINAL_FALLBACK_BINDING),
                environment: ExecutionEnvironment::Terminal,
                capability: capability(RUN_COMMAND_CAPABILITY),
            })
            .expect("approve"),
    )
    .expect("grant");
    registry
        .advance(
            &binding_id(TERMINAL_FALLBACK_BINDING),
            TransitionCause::Authorized(grant),
        )
        .expect("authorize fallback");
    let attached = registry
        .bind(&binding_id(TERMINAL_FALLBACK_BINDING))
        .expect("bind fallback");
    assert_eq!(attached.len(), 1);
    assert_eq!(attached[0].resource_type, sandbox_type());
    assert_eq!(attached[0].resource, replacement.resource);
    assert_ne!(attached[0].resource, first.resource);

    // Workflow immutability and pinning are independent of provider
    // resource ids: the pinned version still verifies and mentions no
    // provider or instance identifiers.
    artifact
        .version
        .verify_integrity()
        .expect("integrity intact");
    let version_json = serde_json::to_string(&artifact.version).expect("serialize version");
    assert!(!version_json.contains("fixture-sandbox-cloud"));
    assert!(!version_json.contains("sbx-"));
}

#[test]
fn e2e_unsupported_lifecycle_operations_are_explicit() {
    let provider = Arc::new(LocalContainersFixture::new().expect("fixture"));
    let workspace = provider.create(persistent_spec()).expect("workspace");

    // Fork is undeclared for local containers: the error names the
    // provider and the operation, and maps onto the contract Permanent
    // family so the recovery pipeline escalates instead of retrying.
    let error = provider
        .lifecycle(&workspace, LifecycleOp::Fork)
        .expect_err("fork must be unsupported");
    assert!(matches!(
        error,
        codex_resource_providers::ResourceProviderError::Unsupported { .. }
    ));
    let rendered = error.to_string();
    assert!(rendered.contains("fixture-local-containers"));
    assert!(rendered.contains("fork"));
    let failure = error.normalized_failure();
    assert_eq!(failure.kind, FailureKind::Permanent);
    assert!(!failure.message.is_empty());
    assert!(!error.rebind_advisable());

    // The same explicit rejection flows through the host wiring
    // registry, and snapshots/pause are equally explicit.
    let mut providers = ProviderRegistry::new();
    let id = providers.register(provider.clone()).expect("register");
    let registered = providers.provider(&id).expect("provider");
    let rejected = registered
        .lifecycle(&workspace, LifecycleOp::Snapshot)
        .expect_err("snapshot must be unsupported");
    assert!(matches!(
        rejected,
        codex_resource_providers::ResourceProviderError::Unsupported { .. }
    ));
    assert!(matches!(
        provider
            .lifecycle(&workspace, LifecycleOp::Pause)
            .expect_err("pause must be unsupported"),
        codex_resource_providers::ResourceProviderError::Unsupported { .. }
    ));

    // Resize IS declared and works; the resource state is unchanged by
    // every rejected operation.
    let resized = provider
        .lifecycle(
            &workspace,
            LifecycleOp::Resize(ResizeRequest {
                cpu_millis: Some(1_000),
                memory_mib: None,
                disk_mib: None,
            }),
        )
        .expect("resize");
    assert_eq!(
        resized.handle().expect("handle").status,
        ResourceStatus::Active
    );
    assert_eq!(
        provider.status(&workspace).expect("status"),
        ResourceStatus::Active
    );
    assert_eq!(
        provider
            .workspace_spec(&workspace.resource)
            .expect("spec kept")
            .cpu_millis,
        Some(1_000)
    );
}

#[test]
fn e2e_conformance_parity_across_both_fixtures() {
    let sandbox = Arc::new(SandboxCloudFixture::new(4).expect("fixture"));
    let local = Arc::new(LocalContainersFixture::new().expect("fixture"));
    let mut providers = ProviderRegistry::new();
    let sandbox_id = providers
        .register(sandbox.clone())
        .expect("register sandbox");
    let local_id = providers.register(local.clone()).expect("register local");
    assert_ne!(sandbox_id, local_id);
    assert_eq!(providers.len(), 2);

    // Identical ops, identical assertions, two materially different
    // implementations: the provider-neutrality proof.
    assert_provider_conformance(sandbox.as_ref());
    assert_provider_conformance(local.as_ref());

    // The declared capability matrices differ materially.
    let sandbox_descriptor = sandbox.descriptor();
    let local_descriptor = local.descriptor();
    assert!(sandbox_descriptor.serves(ResourceClass::Sandbox));
    assert!(sandbox_descriptor.serves(ResourceClass::RemoteDesktop));
    assert!(!sandbox_descriptor.serves(ResourceClass::PersistentWorkspace));
    assert!(!local_descriptor.serves(ResourceClass::Sandbox));
    assert!(local_descriptor.serves(ResourceClass::PersistentWorkspace));

    // The failure modes differ materially: cloud capacity exhaustion vs
    // container over-quota resize.
    let cloud = SandboxCloudFixture::new(1).expect("cloud fixture");
    let _first = cloud
        .create(sandbox_spec(ResourceClass::Sandbox))
        .expect("first cloud resource");
    let exhausted = cloud
        .create(sandbox_spec(ResourceClass::Sandbox))
        .expect_err("capacity");
    assert!(matches!(
        exhausted,
        codex_resource_providers::ResourceProviderError::CapacityExhausted { .. }
    ));
    let containers = LocalContainersFixture::new().expect("container fixture");
    let workspace = containers.create(persistent_spec()).expect("workspace");
    let over = containers
        .lifecycle(
            &workspace,
            LifecycleOp::Resize(ResizeRequest {
                cpu_millis: Some(5_000),
                memory_mib: None,
                disk_mib: None,
            }),
        )
        .expect_err("over-quota resize");
    assert!(matches!(
        over,
        codex_resource_providers::ResourceProviderError::InvalidSpec { .. }
    ));

    // The timings differ materially (cloud latency vs local), and both
    // providers report healthy through the registry.
    let fleet = providers.probe_all();
    assert_eq!(fleet.len(), 2);
    let latencies: Vec<u64> = fleet
        .iter()
        .map(|(_, health)| {
            health
                .as_ref()
                .expect("healthy")
                .latency_ms
                .expect("latency")
        })
        .collect();
    assert_ne!(latencies[0], latencies[1]);

    // The bind surfaces agree with the frozen peers: sandbox-class under
    // Terminal, remote-desktop-class under Computer.
    let desktop = sandbox
        .create(sandbox_spec(ResourceClass::RemoteDesktop))
        .expect("desktop resource");
    let desktop_binding = bind(&desktop).expect("desktop binding");
    assert_eq!(desktop_binding.holder, ExecutionEnvironment::Computer);
    let workspace_handle = local.create(persistent_spec()).expect("workspace");
    let workspace_binding = bind(&workspace_handle).expect("workspace binding");
    assert_eq!(workspace_binding.holder, ExecutionEnvironment::Terminal);
}

#[tokio::test]
async fn e2e_provider_neutral_binding_no_provider_types_in_workflow_contracts() {
    let (artifact, _installs) = terminal_workflow();
    let terminal =
        TerminalSandboxAdapter::new(TERMINAL_ADAPTER_ID, BindingClass::CodexNative, None);
    let mut registry = CapabilityRegistry::new();
    registry
        .register_adapter(terminal)
        .expect("register terminal adapter");

    let provider = Arc::new(SandboxCloudFixture::new(4).expect("fixture"));
    let handle = provider
        .create(sandbox_spec(ResourceClass::Sandbox))
        .expect("create");
    let resource_binding = bind(&handle).expect("mint binding");

    // The binding is a plain WO-005 record: logical type, opaque id,
    // peer holder — exactly three keys, no provider field.
    let binding_json = serde_json::to_value(&resource_binding).expect("serialize binding");
    assert_eq!(binding_json.as_object().expect("object").len(), 3);
    assert_eq!(
        binding_json,
        serde_json::json!({
            "resourceType": SANDBOX_COMPUTE_RESOURCE,
            "resource": handle.resource.as_ref(),
            "holder": "terminal",
        })
    );

    let mut harness = app_harness(registry, terminal_action_script());
    let outcome = run_installed(&mut harness, &artifact, vec![resource_binding]).await;
    assert_eq!(outcome.terminal, RunTerminal::Completed);
    verify_instance_evidence(&harness, &outcome);

    // The workflow version and its IR never mention the provider or its
    // instance ids; the semantic resource requirement is the only
    // resource reference.
    let version_json = serde_json::to_string(&artifact.version).expect("serialize version");
    assert!(
        version_json.contains(SANDBOX_COMPUTE_RESOURCE),
        "the logical resource type is declared semantically"
    );
    assert!(!version_json.contains("fixture-sandbox-cloud"));
    assert!(!version_json.contains("sbx-"));
    let ir_json = serde_json::to_string(&artifact.version.definition.ir).expect("serialize ir");
    assert!(!ir_json.contains("fixture-sandbox-cloud"));
    assert!(!ir_json.contains("sbx-"));

    // Provider names appear only in provider-owned evidence locators.
    let records = provider.evidence_records();
    assert!(!records.is_empty());
    assert!(records.iter().all(|record| {
        record
            .locator
            .starts_with("codex-resource-providers/fixture-sandbox-cloud/")
    }));

    // Workflow-semantic identity is digest-pinned and untouched by the
    // provider binding: the definition and dependency-lock digests still
    // recompute to the pinned values.
    artifact.version.verify_integrity().expect("integrity");
    assert_eq!(
        artifact.version.definition.digest().expect("digest"),
        artifact.version.identity.definition_digest
    );
    assert_eq!(
        artifact
            .version
            .dependency_lock
            .digest()
            .expect("lock digest"),
        artifact.version.identity.dependency_lock_digest
    );
}

#[tokio::test]
async fn e2e_noop_default_ordinary_codex_untouched_without_providers() {
    let (artifact, _installs) = terminal_workflow();
    let terminal =
        TerminalSandboxAdapter::new(TERMINAL_ADAPTER_ID, BindingClass::CodexNative, None);
    let mut registry = CapabilityRegistry::new();
    registry
        .register_adapter(terminal.clone())
        .expect("register terminal adapter");

    // No provider is configured anywhere: the resource-plane registry is
    // empty, and the workflow binds its declared resource need the
    // classic WO-005 way — an opaque, credential-free instance id
    // supplied directly by the host.
    let providers = ProviderRegistry::new();
    assert!(providers.is_empty());
    assert!(providers.probe_all().is_empty());
    let resource_binding = ResourceBinding {
        resource_type: sandbox_type(),
        resource: ResourceId::parse("host-supplied-sandbox").expect("resource"),
        holder: ExecutionEnvironment::Terminal,
    };

    let mut harness = app_harness(registry, terminal_action_script());
    assert!(
        !harness.lifecycle.is_active(),
        "nothing is active before a version is selected"
    );

    let outcome = run_installed(&mut harness, &artifact, vec![resource_binding]).await;
    assert_eq!(outcome.terminal, RunTerminal::Completed);
    assert_eq!(outcome.instance.status, WorkflowInstanceStatus::Succeeded);
    assert_eq!(terminal.executions(), 1);
    verify_instance_evidence(&harness, &outcome);

    let active = harness
        .lifecycle
        .registry()
        .resource_binding(&sandbox_type())
        .expect("bound the classic way");
    assert_eq!(active.resource.as_ref(), "host-supplied-sandbox");
    assert_eq!(active.holder, ExecutionEnvironment::Terminal);
}
