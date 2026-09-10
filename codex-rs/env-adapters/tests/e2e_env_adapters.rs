//! WO-015 end-to-end environment-adapter tests.
//!
//! Every scenario is **static**: in-memory doubles stand in for the host
//! seams (browser executor, computer-use bridge, remote-desktop bridge,
//! mobile device bridge, stores), so no network, browser, desktop, remote
//! session, or device runtime is touched. The scenarios map to the Work
//! Order's required evidence:
//!
//! - mixed-environment run through the workflow-app ports with the
//!   remote-desktop and mobile adapters bound in the same registry as the
//!   native browser and computer environments (the golden path);
//! - adapter readiness/registration: an unreachable bridge fails
//!   readiness with explicit diagnostics and blocks the binding plan;
//! - policy denial paths: registry binding-policy scope denial and
//!   adapter-side host-policy denial;
//! - recovery on bridge loss: the native desktop bridge losing its call
//!   rebinds `control_desktop_app` to the remote-desktop environment with
//!   a recorded fallback, and a transient device-bridge timeout retries
//!   with a fresh approval;
//! - ordinary Codex compatibility: with no workflow selected, nothing
//!   executes.
//!
//! The workspace clippy.toml allows `expect`/`unwrap` in test code; the
//! same intent is spelled out here for integration-test setup.
#![allow(clippy::expect_used)]
#![allow(clippy::unwrap_used)]

use std::collections::BTreeMap;
use std::collections::VecDeque;
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
use codex_computer_use_adapter::BridgeCallFailure as ComputerBridgeCallFailure;
use codex_computer_use_adapter::BridgeIdentity;
use codex_computer_use_adapter::BridgeReadinessFailure as ComputerBridgeReadinessFailure;
use codex_computer_use_adapter::ComputerUseBridge;
use codex_computer_use_adapter::ComputerUseBridgeFactory;
use codex_computer_use_adapter::ComputerUsePolicy;
use codex_computer_use_adapter::NormalizedAction;
use codex_computer_use_adapter::NormalizedActionResult;
use codex_computer_use_adapter::ScreenIdentity;
use codex_computer_use_adapter::ScreenObservation;
use codex_computer_use_adapter::UnixMsClock;
use codex_env_adapters::Access;
use codex_env_adapters::AccessTable;
use codex_env_adapters::BridgeCallFailure;
use codex_env_adapters::BridgeEndpoint;
use codex_env_adapters::BridgeFailureKind;
use codex_env_adapters::BridgeHealth;
use codex_env_adapters::MOBILE_DEVICE_CAPABILITY;
use codex_env_adapters::MOBILE_DEVICE_RESOURCE;
use codex_env_adapters::MobileDeviceAction;
use codex_env_adapters::MobileDeviceBridge;
use codex_env_adapters::MobileDeviceClass;
use codex_env_adapters::MobileDeviceEnvironmentAdapter;
use codex_env_adapters::MobileDeviceObservation;
use codex_env_adapters::MobileDeviceOutcome;
use codex_env_adapters::MobileDevicePolicy;
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
use codex_execution_contracts::ActionTarget;
use codex_execution_contracts::BindingDecision;
use codex_execution_contracts::BindingPolicy;
use codex_execution_contracts::CapabilityBindingId;
use codex_execution_contracts::CapabilityRegistry;
use codex_execution_contracts::EnvironmentScope;
use codex_execution_contracts::ExecutionEnvironment;
use codex_execution_contracts::FailureKind;
use codex_execution_contracts::FallbackReason;
use codex_execution_contracts::FallbackRecord;
use codex_execution_contracts::OperationId;
use codex_execution_contracts::ReadinessState;
use codex_execution_contracts::Recovery;
use codex_execution_contracts::RecoveryStrategy;
use codex_execution_contracts::ResourceBinding;
use codex_execution_contracts::ResourceId;
use codex_teaching_compiler::ApprovalDecision;
use codex_teaching_compiler::ApprovalDecisionKind;
use codex_teaching_compiler::SimulationConfig;
use codex_teaching_compiler::WorkflowCandidate;
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
use codex_workflow_contracts::SemanticVersion;
use codex_workflow_contracts::StepNode;
use codex_workflow_contracts::TriggerClass;
use codex_workflow_contracts::TriggerSource;
use codex_workflow_contracts::WorkflowDefinitionId;
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

const BROWSER_BINDING: &str = "codex-browser-use-env:navigate_web";
const COMPUTER_BINDING: &str = "codex-computer-use-env:control_desktop_app";
const REMOTE_BINDING: &str = "codex-remote-desktop-env:control_remote_desktop";
const REMOTE_DESKTOP_BINDING: &str = "codex-remote-desktop-env:control_desktop_app";
const MOBILE_BINDING: &str = "codex-mobile-device-env:control_mobile_device";

/// One scripted mobile bridge call.
type MobileCall = Result<MobileDeviceOutcome, BridgeCallFailure>;

fn node_id(value: &str) -> IrNodeId {
    IrNodeId::parse(value).expect("node id")
}

fn definition_id() -> WorkflowDefinitionId {
    WorkflowDefinitionId::parse("cross-env-check").expect("definition id")
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

/// Authors a linear workflow whose steps require the given capabilities,
/// drives the teaching compiler to an approved definition, and publishes
/// it through the forge lifecycle (seal -> release -> install).
fn authored(steps: &[(&str, &str)]) -> (PublishedArtifact, InstallRegistry) {
    let mut nodes = BTreeMap::new();
    let mut bindings = BTreeMap::new();
    for (position, (node, capability_name)) in steps.iter().enumerate() {
        let next = steps
            .get(position + 1)
            .map(|(next_node, _)| node_id(next_node));
        nodes.insert(
            node_id(node),
            WorkflowIrNode::Step(StepNode {
                capabilities: Vec::new(),
                roles: Vec::new(),
                description: Some(format!("authored step {node}")),
                next,
            }),
        );
        bindings.insert(
            node_id(node),
            vec![CapabilityRequirement {
                capability: capability(capability_name),
                purpose: Some(format!("step {node} requires {capability_name}")),
            }],
        );
    }
    let ir = WorkflowIr {
        ir_format: IR_FORMAT_VERSION,
        entry: node_id(steps[0].0),
        nodes,
        conditions: BTreeMap::new(),
    };
    let mut candidate =
        WorkflowCandidate::from_ir(ir, Some("authored cross-environment check".to_string()))
            .expect("candidate");
    let summary = candidate.validate().expect("validate");
    assert!(summary.is_clean());
    candidate
        .simulate(SimulationConfig::default())
        .expect("simulate");
    candidate
        .approve(
            ApprovalDecision::new("tech-lead", "review-env", ApprovalDecisionKind::Approved)
                .expect("decision"),
        )
        .expect("approve");
    let approved = candidate.finalize(definition_id()).expect("finalize");

    let repository = repository_id();
    let mut forge = InMemoryForge::new(ForgeKind::parse("github").expect("forge kind"));
    let manifest = WorkflowManifest {
        manifest_version: MANIFEST_FORMAT_VERSION,
        workflow: definition_id(),
        display_name: Some("cross-env-check".to_string()),
        description: None,
        repository: repository.clone(),
        definition_path: RepositoryRelativePath::parse("workflows/cross-env-check.json")
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

fn mixed_workflow() -> (PublishedArtifact, InstallRegistry) {
    authored(&[
        ("step-001", BROWSER_CAPABILITY),
        ("step-002", COMPUTER_CAPABILITY),
        ("step-003", REMOTE_DESKTOP_CAPABILITY),
        ("step-004", MOBILE_DEVICE_CAPABILITY),
    ])
}

fn desktop_workflow() -> (PublishedArtifact, InstallRegistry) {
    authored(&[
        ("step-001", BROWSER_CAPABILITY),
        ("step-002", COMPUTER_CAPABILITY),
    ])
}

fn remote_workflow() -> (PublishedArtifact, InstallRegistry) {
    authored(&[("step-001", REMOTE_DESKTOP_CAPABILITY)])
}

fn mobile_workflow() -> (PublishedArtifact, InstallRegistry) {
    authored(&[("step-001", MOBILE_DEVICE_CAPABILITY)])
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

    fn probe(&self) -> Result<(), ComputerBridgeReadinessFailure> {
        Ok(())
    }

    fn execute(
        &self,
        action: NormalizedAction,
    ) -> Result<NormalizedActionResult, ComputerBridgeCallFailure> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(NormalizedActionResult::succeeded(&action, 0))
    }

    fn observe(&self) -> Result<ScreenObservation, ComputerBridgeCallFailure> {
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
    fn provision(&self) -> Result<Box<dyn ComputerUseBridge>, ComputerBridgeReadinessFailure> {
        Ok(Box::new(StubComputerBridge {
            calls: self.calls.clone(),
        }))
    }
}

/// A computer-use bridge that loses its first call (bridge loss).
struct FlakyComputerBridge {
    calls: Arc<AtomicUsize>,
}

impl ComputerUseBridge for FlakyComputerBridge {
    fn identity(&self) -> BridgeIdentity {
        BridgeIdentity {
            bridge_kind: "flaky-computer-use".to_string(),
            runtime: "node_repl".to_string(),
            version: "1.0.0-test".to_string(),
        }
    }

    fn probe(&self) -> Result<(), ComputerBridgeReadinessFailure> {
        Ok(())
    }

    fn execute(
        &self,
        action: NormalizedAction,
    ) -> Result<NormalizedActionResult, ComputerBridgeCallFailure> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        if self.calls.load(Ordering::SeqCst) == 1 {
            return Err(ComputerBridgeCallFailure::BridgeUnavailable {
                details: "stub desktop bridge lost".to_string(),
            });
        }
        Ok(NormalizedActionResult::succeeded(&action, 0))
    }

    fn observe(&self) -> Result<ScreenObservation, ComputerBridgeCallFailure> {
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

/// Factory provisioning first-call-losing bridges.
struct FlakyComputerFactory {
    calls: Arc<AtomicUsize>,
}

impl ComputerUseBridgeFactory for FlakyComputerFactory {
    fn provision(&self) -> Result<Box<dyn ComputerUseBridge>, ComputerBridgeReadinessFailure> {
        Ok(Box::new(FlakyComputerBridge {
            calls: self.calls.clone(),
        }))
    }
}

/// The scripted remote-desktop bridge.
struct ScriptedRemoteBridge {
    reachable: bool,
    connects: Arc<AtomicUsize>,
    executes: Arc<AtomicUsize>,
}

impl ScriptedRemoteBridge {
    fn new(reachable: bool) -> (Arc<Self>, Arc<AtomicUsize>, Arc<AtomicUsize>) {
        let connects = Arc::new(AtomicUsize::new(0));
        let executes = Arc::new(AtomicUsize::new(0));
        let bridge = Arc::new(Self {
            reachable,
            connects: connects.clone(),
            executes: executes.clone(),
        });
        (bridge, connects, executes)
    }

    fn observation(host: &str) -> RemoteScreenObservation {
        RemoteScreenObservation {
            host: host.to_string(),
            state: RemoteSessionState::Connected,
            width: 1920,
            height: 1080,
            foreground_application: Some("ops-console".to_string()),
            captured_at_unix_ms: 7,
        }
    }
}

impl RemoteDesktopBridge for ScriptedRemoteBridge {
    fn health(&self) -> Result<BridgeHealth, BridgeCallFailure> {
        if self.reachable {
            Ok(BridgeHealth {
                endpoint: BridgeEndpoint {
                    bridge_kind: "rdp".to_string(),
                    runtime: "stub-device-bridge".to_string(),
                    version: "1.0.0-test".to_string(),
                },
                latency_ms: Some(5),
            })
        } else {
            Err(BridgeCallFailure::new(
                BridgeFailureKind::Lost,
                "stub remote bridge unreachable",
            ))
        }
    }

    fn connect(&self, host: &str) -> Result<RemoteScreenObservation, BridgeCallFailure> {
        self.connects.fetch_add(1, Ordering::SeqCst);
        Ok(Self::observation(host))
    }

    fn execute(
        &self,
        host: &str,
        _action: RemoteDesktopAction,
    ) -> Result<RemoteDesktopOutcome, BridgeCallFailure> {
        self.executes.fetch_add(1, Ordering::SeqCst);
        Ok(RemoteDesktopOutcome {
            observation: Some(Self::observation(host)),
            detail: Some("stub ok".to_string()),
        })
    }
}

/// The scripted mobile-device bridge: serves scripted calls in order,
/// defaulting to success.
struct ScriptedMobileBridge {
    attaches: Arc<AtomicUsize>,
    executes: Arc<AtomicUsize>,
    script: Mutex<VecDeque<MobileCall>>,
}

impl ScriptedMobileBridge {
    fn new(script: Vec<MobileCall>) -> (Arc<Self>, Arc<AtomicUsize>, Arc<AtomicUsize>) {
        let attaches = Arc::new(AtomicUsize::new(0));
        let executes = Arc::new(AtomicUsize::new(0));
        let bridge = Arc::new(Self {
            attaches: attaches.clone(),
            executes: executes.clone(),
            script: Mutex::new(script.into_iter().collect::<VecDeque<_>>()),
        });
        (bridge, attaches, executes)
    }

    fn observation(device: &str) -> MobileDeviceObservation {
        MobileDeviceObservation {
            device: device.to_string(),
            device_class: MobileDeviceClass::Emulator,
            foreground_app: Some("com.example.issues".to_string()),
            width: 1080,
            height: 2400,
            captured_at_unix_ms: 9,
        }
    }
}

impl MobileDeviceBridge for ScriptedMobileBridge {
    fn health(&self) -> Result<BridgeHealth, BridgeCallFailure> {
        Ok(BridgeHealth {
            endpoint: BridgeEndpoint {
                bridge_kind: "adb".to_string(),
                runtime: "stub-device-bridge".to_string(),
                version: "1.0.0-test".to_string(),
            },
            latency_ms: Some(3),
        })
    }

    fn attach(&self, device: &str) -> Result<MobileDeviceObservation, BridgeCallFailure> {
        self.attaches.fetch_add(1, Ordering::SeqCst);
        Ok(Self::observation(device))
    }

    fn execute(
        &self,
        device: &str,
        _action: MobileDeviceAction,
    ) -> Result<MobileDeviceOutcome, BridgeCallFailure> {
        self.executes.fetch_add(1, Ordering::SeqCst);
        let mut script = self.script.lock().unwrap();
        match script.pop_front() {
            Some(call) => call,
            None => Ok(MobileDeviceOutcome {
                observation: Some(Self::observation(device)),
                detail: Some("stub ok".to_string()),
            }),
        }
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

fn allowing_remote_policy() -> RemoteDesktopPolicy {
    RemoteDesktopPolicy {
        host_access: AccessTable {
            default: Some(Access::Allow),
            entries: BTreeMap::new(),
        },
        allow_session_takeover: None,
    }
}

fn denying_remote_policy() -> RemoteDesktopPolicy {
    RemoteDesktopPolicy {
        host_access: AccessTable {
            default: Some(Access::Deny),
            entries: BTreeMap::new(),
        },
        allow_session_takeover: None,
    }
}

fn allowing_mobile_policy() -> MobileDevicePolicy {
    MobileDevicePolicy {
        device_access: AccessTable {
            default: Some(Access::Allow),
            entries: BTreeMap::new(),
        },
        allow_session_takeover: None,
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

fn remote_type_text_action() -> Action {
    let mut inputs = ActionInputs::default();
    inputs.insert("text", serde_json::json!("deploy summary"));
    Action {
        operation: OperationId::parse("type_text").expect("operation"),
        target: None,
        inputs,
    }
}

fn mobile_tap_action() -> Action {
    let mut inputs = ActionInputs::default();
    inputs.insert("x", serde_json::json!(480));
    inputs.insert("y", serde_json::json!(1200));
    Action {
        operation: OperationId::parse("tap").expect("operation"),
        target: None,
        inputs,
    }
}

fn mixed_action_script() -> Vec<ScriptedAction> {
    vec![
        ScriptedAction::new(
            "step-001",
            BROWSER_CAPABILITY,
            navigate_action("https://issues.example.com"),
        )
        .expect("browser action"),
        ScriptedAction::new("step-002", COMPUTER_CAPABILITY, click_action())
            .expect("desktop action"),
        ScriptedAction::new(
            "step-003",
            REMOTE_DESKTOP_CAPABILITY,
            remote_type_text_action(),
        )
        .expect("remote action"),
        ScriptedAction::new("step-004", MOBILE_DEVICE_CAPABILITY, mobile_tap_action())
            .expect("mobile action"),
    ]
}

fn desktop_action_script() -> Vec<ScriptedAction> {
    vec![
        ScriptedAction::new(
            "step-001",
            BROWSER_CAPABILITY,
            navigate_action("https://issues.example.com"),
        )
        .expect("browser action"),
        ScriptedAction::new("step-002", COMPUTER_CAPABILITY, click_action())
            .expect("desktop action"),
    ]
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

fn mobile_action_script() -> Vec<ScriptedAction> {
    vec![
        ScriptedAction::new("step-001", MOBILE_DEVICE_CAPABILITY, mobile_tap_action())
            .expect("mobile action"),
    ]
}

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
        ResourceBinding {
            resource_type: ResourceTypeId::parse(REMOTE_DESKTOP_SESSION_RESOURCE)
                .expect("resource type"),
            resource: ResourceId::parse("ops-remote-host-1").expect("resource"),
            holder: ExecutionEnvironment::Computer,
        },
        ResourceBinding {
            resource_type: ResourceTypeId::parse(MOBILE_DEVICE_RESOURCE).expect("resource type"),
            resource: ResourceId::parse("pixel-8-emulator").expect("resource"),
            holder: ExecutionEnvironment::Computer,
        },
    ]
}

/// What one scenario scripts for the harness doubles.
struct Scenario {
    browser_turns: Vec<BrowserTurn>,
    computer_loses_first_call: bool,
    remote_reachable: bool,
    mobile_calls: Vec<MobileCall>,
    remote_policy: RemoteDesktopPolicy,
}

impl Scenario {
    fn healthy() -> Self {
        Self {
            browser_turns: vec![success_turn()],
            computer_loses_first_call: false,
            remote_reachable: true,
            mobile_calls: Vec::new(),
            remote_policy: allowing_remote_policy(),
        }
    }

    fn with_computer_bridge_loss(mut self) -> Self {
        self.computer_loses_first_call = true;
        self
    }

    fn with_unreachable_remote(mut self) -> Self {
        self.remote_reachable = false;
        self
    }

    fn with_remote_policy(mut self, policy: RemoteDesktopPolicy) -> Self {
        self.remote_policy = policy;
        self
    }

    fn with_mobile_calls(mut self, calls: Vec<MobileCall>) -> Self {
        self.mobile_calls = calls;
        self
    }
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
    remote: Arc<RemoteDesktopEnvironmentAdapter>,
    mobile: Arc<MobileDeviceEnvironmentAdapter>,
    browser_executor: Arc<ScriptedBrowserExecutor>,
    computer_calls: Arc<AtomicUsize>,
    remote_connects: Arc<AtomicUsize>,
    remote_executes: Arc<AtomicUsize>,
    mobile_attaches: Arc<AtomicUsize>,
    mobile_executes: Arc<AtomicUsize>,
}

fn harness(scenario: Scenario, script: Vec<ScriptedAction>) -> Harness {
    let browser_executor = Arc::new(ScriptedBrowserExecutor::new(scenario.browser_turns));
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
            if scenario.computer_loses_first_call {
                Arc::new(FlakyComputerFactory {
                    calls: computer_calls.clone(),
                })
            } else {
                Arc::new(StubComputerFactory {
                    calls: computer_calls.clone(),
                })
            },
            Arc::new(|| 1_000u64) as UnixMsClock,
        )
        .expect("computer adapter"),
    );
    let (remote_bridge, remote_connects, remote_executes) =
        ScriptedRemoteBridge::new(scenario.remote_reachable);
    let remote = Arc::new(
        RemoteDesktopEnvironmentAdapter::new(scenario.remote_policy, remote_bridge)
            .expect("remote adapter"),
    );
    let (mobile_bridge, mobile_attaches, mobile_executes) =
        ScriptedMobileBridge::new(scenario.mobile_calls);
    let mobile = Arc::new(
        MobileDeviceEnvironmentAdapter::new(allowing_mobile_policy(), mobile_bridge)
            .expect("mobile adapter"),
    );
    let mut registry = CapabilityRegistry::new();
    registry
        .register_adapter(browser)
        .expect("register browser adapter");
    registry
        .register_adapter(computer)
        .expect("register computer adapter");
    registry
        .register_adapter(remote.clone())
        .expect("register remote adapter");
    registry
        .register_adapter(mobile.clone())
        .expect("register mobile adapter");
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
        remote,
        mobile,
        browser_executor,
        computer_calls,
        remote_connects,
        remote_executes,
        mobile_attaches,
        mobile_executes,
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
async fn e2e_mixed_run_binds_browser_computer_remote_and_mobile_in_one_registry() {
    let (artifact, installs) = mixed_workflow();
    assert!(installs.is_installed(&definition_id()));
    let mut harness = harness(Scenario::healthy(), mixed_action_script());

    // Registration: the four adapters contribute five bindings (the
    // remote-desktop adapter provides both its own capability and the
    // desktop capability as a compatible alternate).
    assert_eq!(harness.lifecycle.registry().bindings().count(), 5);
    let remote_binding = harness
        .lifecycle
        .registry()
        .binding(&binding_id(REMOTE_DESKTOP_BINDING))
        .expect("remote desktop binding");
    assert_eq!(
        remote_binding.binding.environment,
        ExecutionEnvironment::Computer
    );
    assert_eq!(
        remote_binding.binding.class,
        codex_execution_contracts::BindingClass::Compatible
    );

    let outcome = run_installed(&mut harness, &artifact).await;

    // One run, four environments' adapters, no orchestration branches.
    assert_eq!(outcome.terminal, RunTerminal::Completed);
    assert_eq!(outcome.instance.status, WorkflowInstanceStatus::Succeeded);
    assert_eq!(
        outcome.path,
        vec![
            node_id("step-001"),
            node_id("step-002"),
            node_id("step-003"),
            node_id("step-004"),
        ]
    );
    assert_eq!(
        harness.browser_executor.executed_kinds(),
        vec![BrowserActionKind::Navigate]
    );
    assert_eq!(harness.computer_calls.load(Ordering::SeqCst), 1);
    assert_eq!(harness.remote_connects.load(Ordering::SeqCst), 1);
    assert_eq!(harness.remote_executes.load(Ordering::SeqCst), 1);
    assert_eq!(harness.mobile_attaches.load(Ordering::SeqCst), 1);
    assert_eq!(harness.mobile_executes.load(Ordering::SeqCst), 1);

    // The observable event sequence: bindings authorized and bound in
    // binding-id order, then the four steps in order.
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
        WorkflowEvent::BindingAuthorized {
            instance: instance_id,
            binding: binding_id(MOBILE_BINDING),
            environment: ExecutionEnvironment::Computer,
        },
        WorkflowEvent::BindingAuthorized {
            instance: instance_id,
            binding: binding_id(REMOTE_BINDING),
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
        WorkflowEvent::ResourcesBound {
            instance: instance_id,
            binding: binding_id(MOBILE_BINDING),
            resources: vec![ResourceTypeId::parse(MOBILE_DEVICE_RESOURCE).expect("resource type")],
        },
        WorkflowEvent::ResourcesBound {
            instance: instance_id,
            binding: binding_id(REMOTE_BINDING),
            resources: vec![
                ResourceTypeId::parse(REMOTE_DESKTOP_SESSION_RESOURCE).expect("resource type"),
            ],
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
        WorkflowEvent::StepStarted {
            instance: instance_id,
            node: node_id("step-003"),
        },
        WorkflowEvent::StepCompleted {
            instance: instance_id,
            node: node_id("step-003"),
        },
        WorkflowEvent::StepStarted {
            instance: instance_id,
            node: node_id("step-004"),
        },
        WorkflowEvent::StepCompleted {
            instance: instance_id,
            node: node_id("step-004"),
        },
        WorkflowEvent::RunCompleted {
            instance: instance_id,
        },
    ];
    assert_eq!(harness.events.events(), expected);

    // Evidence: four approvals, five observations (browser 1, desktop 2,
    // remote 1, mobile 1), four traces, no recoveries.
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
    assert_eq!(count(EvidenceKind::Approval), 4);
    assert_eq!(count(EvidenceKind::Observation), 5);
    assert_eq!(count(EvidenceKind::Trace), 4);
    assert_eq!(count(EvidenceKind::Recovery), 0);
    verify_evidence(&harness, &verified);
    assert_eq!(harness.approvals.requests().len(), 4);

    // The adapters' own evidence journals recorded the run.
    let remote_kinds: Vec<EvidenceKind> = harness
        .remote
        .evidence_records()
        .iter()
        .map(|record| record.kind)
        .collect();
    assert_eq!(
        remote_kinds,
        vec![
            EvidenceKind::Observation,
            EvidenceKind::Observation,
            EvidenceKind::Trace
        ]
    );
    let mobile_kinds: Vec<EvidenceKind> = harness
        .mobile
        .evidence_records()
        .iter()
        .map(|record| record.kind)
        .collect();
    assert_eq!(
        mobile_kinds,
        vec![
            EvidenceKind::Observation,
            EvidenceKind::Observation,
            EvidenceKind::Trace
        ]
    );
    assert_eq!(
        harness.remote.session_binding(),
        Some(binding_id(REMOTE_BINDING))
    );
    assert_eq!(
        harness.mobile.session_binding(),
        Some(binding_id(MOBILE_BINDING))
    );
}

#[tokio::test]
async fn e2e_unreachable_remote_bridge_blocks_the_plan_with_explicit_diagnostics() {
    let (artifact, _installs) = mixed_workflow();
    let mut harness = harness(
        Scenario::healthy().with_unreachable_remote(),
        mixed_action_script(),
    );
    harness
        .versions
        .publish(artifact.version.clone())
        .expect("store version");
    harness
        .lifecycle
        .select_version(&artifact.version.version_id)
        .expect("select version");

    let error = harness
        .lifecycle
        .instantiate(InstantiateRequest {
            trigger: None,
            policy: BindingPolicy::default(),
            resources: resources(),
            walk: WalkConfig::default(),
        })
        .await
        .expect_err("readiness failure");
    assert!(matches!(
        error,
        WorkflowAppError::BindingPlanFailed { code, .. }
            if code == codex_execution_contracts::DiagnosticCode::NotInstalled
    ));

    // The durable instance settled as Failed before any action executed.
    let records = harness.instances.records();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].status, WorkflowInstanceStatus::Failed);
    assert_eq!(harness.approvals.requests().len(), 0);
    assert_eq!(harness.remote_connects.load(Ordering::SeqCst), 0);
    assert_eq!(harness.remote_executes.load(Ordering::SeqCst), 0);
    assert_eq!(harness.computer_calls.load(Ordering::SeqCst), 0);
    assert_eq!(harness.browser_executor.executed_kinds().len(), 0);
    assert_eq!(harness.actions.remaining(), 4);

    // The readiness state and diagnostics name the unavailable binding.
    let remote = harness
        .lifecycle
        .registry()
        .binding(&binding_id(REMOTE_BINDING))
        .expect("remote binding");
    assert_eq!(remote.readiness.state(), ReadinessState::Unavailable);
    let diagnostic = remote.readiness.diagnostic().expect("diagnostic");
    assert!(
        diagnostic
            .message
            .contains("remote-desktop bridge is not reachable")
    );
    let diagnostics = harness
        .lifecycle
        .registry()
        .diagnose(&capability(REMOTE_DESKTOP_CAPABILITY));
    assert!(!diagnostics.is_empty());
}

#[tokio::test]
async fn e2e_policy_scope_denies_the_new_environments() {
    let (artifact, _installs) = mixed_workflow();
    let mut harness = harness(Scenario::healthy(), mixed_action_script());
    harness
        .versions
        .publish(artifact.version.clone())
        .expect("store version");
    harness
        .lifecycle
        .select_version(&artifact.version.version_id)
        .expect("select version");

    // Policy scopes execution to the browser environment only: every
    // computer-class requirement (native desktop, remote desktop, mobile)
    // is unresolvable and the plan fails before any approval.
    let error = harness
        .lifecycle
        .instantiate(InstantiateRequest {
            trigger: None,
            policy: BindingPolicy {
                environments: EnvironmentScope::Only(vec![ExecutionEnvironment::Browser]),
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
    assert_eq!(harness.approvals.requests().len(), 0);
    assert_eq!(harness.remote_connects.load(Ordering::SeqCst), 0);
    assert_eq!(harness.mobile_executes.load(Ordering::SeqCst), 0);
    assert_eq!(harness.browser_executor.executed_kinds().len(), 0);
    let records = harness.instances.records();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].status, WorkflowInstanceStatus::Failed);
}

#[tokio::test]
async fn e2e_adapter_policy_denial_fails_actions_and_escalates() {
    let (artifact, _installs) = remote_workflow();
    let mut harness = harness(
        Scenario::healthy().with_remote_policy(denying_remote_policy()),
        remote_action_script(),
    );
    let outcome = run_installed(&mut harness, &artifact).await;

    // The adapter-side host policy refuses the remote host before any
    // bridge call; the failure escalates instead of silently
    // substituting anything.
    assert!(matches!(outcome.terminal, RunTerminal::Failed { .. }));
    assert_eq!(outcome.instance.status, WorkflowInstanceStatus::Failed);
    assert_eq!(harness.remote_connects.load(Ordering::SeqCst), 0);
    assert_eq!(harness.remote_executes.load(Ordering::SeqCst), 0);
    let events = harness.events.events();
    assert!(events.iter().any(|event| {
        matches!(
            event,
            WorkflowEvent::ActionFailed { kind, .. } if *kind == FailureKind::PolicyDenied
        )
    }));
    assert!(
        events
            .iter()
            .any(|event| matches!(event, WorkflowEvent::Escalated { .. }))
    );

    // The escalation is recorded as recovery evidence.
    let verified = harness
        .lifecycle
        .verify_run(&outcome.instance.instance_id)
        .expect("verify run");
    let escalation = verified
        .evidence
        .iter()
        .filter(|reference| reference.kind == EvidenceKind::Recovery)
        .filter_map(|reference| harness.evidence.payload(&reference.locator))
        .map(|payload| serde_json::from_value::<Recovery>(payload).expect("recovery record"))
        .collect::<Vec<_>>();
    assert_eq!(escalation.len(), 1);
    assert!(matches!(escalation[0].strategy, RecoveryStrategy::Escalate));
    verify_evidence(&harness, &verified);
}

#[tokio::test]
async fn e2e_desktop_bridge_loss_rebinds_to_the_remote_desktop_environment() {
    let (artifact, _installs) = desktop_workflow();
    let mut harness = harness(
        Scenario::healthy().with_computer_bridge_loss(),
        desktop_action_script(),
    );
    let outcome = run_installed(&mut harness, &artifact).await;

    // The native desktop bridge lost its call; the frozen recovery
    // pipeline rebound the desktop capability to the remote-desktop
    // environment and the run completed.
    assert_eq!(outcome.terminal, RunTerminal::Completed);
    assert_eq!(outcome.instance.status, WorkflowInstanceStatus::Succeeded);
    assert_eq!(harness.computer_calls.load(Ordering::SeqCst), 1);
    assert_eq!(harness.remote_connects.load(Ordering::SeqCst), 1);
    assert_eq!(harness.remote_executes.load(Ordering::SeqCst), 1);

    // The alternate needed its own authorization: browser + native
    // desktop + remote desktop.
    assert_eq!(harness.approvals.requests().len(), 3);

    // The rebind decision and the verified recovery are recorded as
    // recovery evidence, and the decision names the skipped native
    // binding with a fallback record.
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
        .clone()
        .into_iter()
        .find_map(|payload| serde_json::from_value::<BindingDecision>(payload).ok())
        .expect("binding decision evidence");
    assert_eq!(
        decision.selected.binding,
        binding_id(REMOTE_DESKTOP_BINDING)
    );
    assert!(matches!(
        decision.fallback,
        Some(FallbackRecord {
            reason: FallbackReason::PreferredNotReady { .. },
            ..
        })
    ));
    let recovery = recovery_payloads
        .into_iter()
        .find_map(|payload| serde_json::from_value::<Recovery>(payload).ok())
        .expect("recovery record");
    assert!(matches!(recovery.strategy, RecoveryStrategy::Rebind(_)));
    assert_eq!(
        recovery.outcome,
        codex_execution_contracts::RecoveryOutcome::Recovered
    );
    let events = harness.events.events();
    assert!(events.iter().any(|event| {
        matches!(
            event,
            WorkflowEvent::Recovered { strategy, binding, .. }
                if strategy == "rebind" && *binding == binding_id(REMOTE_DESKTOP_BINDING)
        )
    }));
    verify_evidence(&harness, &verified);
}

#[tokio::test]
async fn e2e_transient_mobile_bridge_loss_recovers_by_retry() {
    let (artifact, _installs) = mobile_workflow();
    let mut harness = harness(
        Scenario::healthy().with_mobile_calls(vec![Err(BridgeCallFailure::new(
            BridgeFailureKind::Timeout,
            "device bridge connection glitch",
        ))]),
        mobile_action_script(),
    );
    let outcome = run_installed(&mut harness, &artifact).await;

    // The transient bridge loss retried once on the same binding with a
    // fresh authorization and recovered.
    assert_eq!(outcome.terminal, RunTerminal::Completed);
    assert_eq!(outcome.instance.status, WorkflowInstanceStatus::Succeeded);
    assert_eq!(harness.mobile_attaches.load(Ordering::SeqCst), 1);
    assert_eq!(harness.mobile_executes.load(Ordering::SeqCst), 2);
    assert_eq!(harness.approvals.requests().len(), 2);

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
        .filter_map(|reference| harness.evidence.payload(&reference.locator))
        .map(|payload| serde_json::from_value::<Recovery>(payload).expect("recovery record"))
        .collect::<Vec<_>>();
    assert_eq!(recoveries.len(), 1);
    assert!(matches!(recoveries[0].strategy, RecoveryStrategy::Retry));
    assert_eq!(
        recoveries[0].outcome,
        codex_execution_contracts::RecoveryOutcome::Recovered
    );
    assert!(harness.events.events().iter().any(|event| {
        matches!(
            event,
            WorkflowEvent::Recovered { strategy, .. } if strategy == "retry"
        )
    }));

    // The adapter journal recorded both the failed and the retried call.
    let kinds: Vec<EvidenceKind> = harness
        .mobile
        .evidence_records()
        .iter()
        .map(|record| record.kind)
        .collect();
    assert_eq!(
        kinds,
        vec![
            EvidenceKind::Observation,
            EvidenceKind::Trace,
            EvidenceKind::Observation,
            EvidenceKind::Trace,
        ]
    );
    verify_evidence(&harness, &verified);
}

#[tokio::test]
async fn ordinary_codex_behavior_is_untouched_without_a_workflow() {
    let mut harness = harness(Scenario::healthy(), mixed_action_script());
    assert!(!harness.lifecycle.is_active());

    // Every run-path operation is an explicit no-op error.
    let error = harness
        .lifecycle
        .run()
        .await
        .expect_err("no active workflow");
    assert!(matches!(error, WorkflowAppError::NoActiveWorkflow));

    // No events, no evidence, no instances, no adapter work: nothing
    // executed and no bridge was touched.
    assert!(harness.events.is_empty());
    assert!(harness.instances.is_empty());
    assert!(harness.evidence.is_empty());
    assert_eq!(harness.approvals.requests().len(), 0);
    assert_eq!(harness.actions.remaining(), 4);
    assert_eq!(harness.browser_executor.executed_kinds().len(), 0);
    assert_eq!(harness.computer_calls.load(Ordering::SeqCst), 0);
    assert_eq!(harness.remote_connects.load(Ordering::SeqCst), 0);
    assert_eq!(harness.remote_executes.load(Ordering::SeqCst), 0);
    assert_eq!(harness.mobile_attaches.load(Ordering::SeqCst), 0);
    assert_eq!(harness.mobile_executes.load(Ordering::SeqCst), 0);
    assert_eq!(harness.remote.session_binding(), None);
    assert_eq!(harness.mobile.session_binding(), None);
    for registered in harness.lifecycle.registry().bindings() {
        assert_eq!(registered.readiness.state(), ReadinessState::Declared);
    }
}
