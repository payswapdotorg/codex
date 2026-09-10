//! Tests for the remote-desktop environment adapter.

use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;

use pretty_assertions::assert_eq;

use codex_execution_contracts::Action;
use codex_execution_contracts::ActionInputs;
use codex_execution_contracts::AdapterDescriptor;
use codex_execution_contracts::AdapterId;
use codex_execution_contracts::BindingClass;
use codex_execution_contracts::CapabilityBindingId;
use codex_execution_contracts::EnvironmentAdapter;
use codex_execution_contracts::ExecutionEnvironment;
use codex_execution_contracts::OperationId;
use codex_execution_contracts::PrepareRequest;
use codex_execution_contracts::ProvidedCapability;
use codex_execution_contracts::ResourceBinding;
use codex_execution_contracts::SessionHandle;
use codex_workflow_contracts::CapabilityId;
use codex_workflow_contracts::EvidenceKind;
use codex_workflow_contracts::ResourceTypeId;

use crate::bridge::BridgeCallFailure;
use crate::bridge::BridgeEndpoint;
use crate::bridge::BridgeFailureKind;
use crate::bridge::BridgeHealth;
use crate::policy::Access;
use crate::policy::AccessTable;
use crate::remote_desktop::REMOTE_DESKTOP_ADAPTER_ID;
use crate::remote_desktop::REMOTE_DESKTOP_CAPABILITY;
use crate::remote_desktop::REMOTE_DESKTOP_SESSION_RESOURCE;
use crate::remote_desktop::RemoteDesktopAction;
use crate::remote_desktop::RemoteDesktopBridge;
use crate::remote_desktop::RemoteDesktopOutcome;
use crate::remote_desktop::RemoteDesktopPolicy;
use crate::remote_desktop::RemoteScreenObservation;
use crate::remote_desktop::RemoteSessionState;
use crate::remote_desktop_adapter::RemoteDesktopEnvironmentAdapter;
use crate::session::OwnerToken;
use crate::session::TakeoverMode;

const REMOTE_BINDING: &str = "codex-remote-desktop-env:control_remote_desktop";

/// A stub remote-desktop bridge counting calls.
struct StubRemoteBridge {
    reachable: bool,
    connects: Arc<AtomicUsize>,
    executes: Arc<AtomicUsize>,
}

impl StubRemoteBridge {
    fn reachable() -> (Self, Arc<AtomicUsize>, Arc<AtomicUsize>) {
        let connects = Arc::new(AtomicUsize::new(0));
        let executes = Arc::new(AtomicUsize::new(0));
        (
            Self {
                reachable: true,
                connects: connects.clone(),
                executes: executes.clone(),
            },
            connects,
            executes,
        )
    }

    fn unreachable() -> Self {
        Self {
            reachable: false,
            connects: Arc::new(AtomicUsize::new(0)),
            executes: Arc::new(AtomicUsize::new(0)),
        }
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

impl RemoteDesktopBridge for StubRemoteBridge {
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

/// An allow-everything policy.
fn allowing_policy() -> RemoteDesktopPolicy {
    RemoteDesktopPolicy {
        host_access: AccessTable {
            default: Some(Access::Allow),
            entries: std::collections::BTreeMap::new(),
        },
        allow_session_takeover: None,
    }
}

/// A prepare request for the remote binding with an optional host resource.
fn prepare_request(resource: Option<&str>) -> PrepareRequest {
    let resources = resource
        .map(|instance| {
            vec![ResourceBinding {
                resource_type: ResourceTypeId::parse(REMOTE_DESKTOP_SESSION_RESOURCE)
                    .expect("resource type"),
                resource: codex_execution_contracts::ResourceId::parse(instance)
                    .expect("resource id"),
                holder: ExecutionEnvironment::Computer,
            }]
        })
        .unwrap_or_default();
    PrepareRequest {
        binding: CapabilityBindingId::parse(REMOTE_BINDING).expect("binding id"),
        capability: CapabilityId::parse(REMOTE_DESKTOP_CAPABILITY).expect("capability"),
        resources,
    }
}

/// A type-text action for the remote vocabulary.
fn type_text_action() -> Action {
    let mut inputs = ActionInputs::default();
    inputs.insert("text", serde_json::json!("deploy summary"));
    Action {
        operation: OperationId::parse("type_text").expect("operation"),
        target: None,
        inputs,
    }
}

fn session_handle() -> SessionHandle {
    SessionHandle::parse(REMOTE_BINDING).expect("session handle")
}

#[test]
fn descriptor_registers_as_a_compatible_computer_alternate() {
    let (bridge, _, _) = StubRemoteBridge::reachable();
    let adapter =
        RemoteDesktopEnvironmentAdapter::new(allowing_policy(), Arc::new(bridge)).expect("adapter");
    let descriptor = adapter.descriptor();
    descriptor.validate().expect("valid descriptor");
    assert_eq!(
        descriptor.adapter,
        AdapterId::parse(REMOTE_DESKTOP_ADAPTER_ID).expect("id")
    );
    assert_eq!(descriptor.environment, ExecutionEnvironment::Computer);
    assert_eq!(descriptor.class, BindingClass::Compatible);
    let mut capabilities: Vec<CapabilityId> = descriptor
        .provides
        .iter()
        .map(|provided| provided.capability.clone())
        .collect();
    capabilities.sort();
    let mut expected = vec![
        CapabilityId::parse("control_desktop_app").expect("capability"),
        CapabilityId::parse("control_remote_desktop").expect("capability"),
    ];
    expected.sort();
    assert_eq!(capabilities, expected);
    for provided in &descriptor.provides {
        assert_eq!(provided.resources.len(), 1);
        assert_eq!(
            provided.resources[0],
            ResourceTypeId::parse(REMOTE_DESKTOP_SESSION_RESOURCE).expect("resource")
        );
    }
}

#[test]
fn reserved_environment_classes_still_fail_loudly() {
    // WO-005 froze the reserved gate: MOBILE/REMOTE_DESKTOP reject
    // registration until the Work Order that owns their activation flips
    // it. This adapter therefore registers behind the peer Computer class
    // (see the descriptor test); asserting the frozen gate keeps that
    // decision visible here.
    let descriptor = AdapterDescriptor {
        adapter: AdapterId::parse(REMOTE_DESKTOP_ADAPTER_ID).expect("adapter id"),
        environment: ExecutionEnvironment::Mobile,
        class: BindingClass::Compatible,
        provides: vec![ProvidedCapability {
            capability: CapabilityId::parse(REMOTE_DESKTOP_CAPABILITY).expect("capability"),
            resources: Vec::new(),
        }],
    };
    assert!(matches!(
        descriptor.validate(),
        Err(codex_execution_contracts::ExecutionContractError::ReservedEnvironment { .. })
    ));
}

#[tokio::test]
async fn unreachable_bridges_probe_not_installed() {
    let adapter = RemoteDesktopEnvironmentAdapter::new(
        allowing_policy(),
        Arc::new(StubRemoteBridge::unreachable()),
    )
    .expect("adapter");
    let report = EnvironmentAdapter::probe(&adapter)
        .await
        .expect("probe report");
    assert_eq!(
        report.status,
        codex_execution_contracts::ProbeStatus::NotInstalled
    );
    let diagnostic = report.diagnostic.expect("diagnostic");
    assert_eq!(
        diagnostic.code,
        codex_execution_contracts::DiagnosticCode::NotInstalled
    );
    assert!(diagnostic.message.contains("not reachable"));
}

#[tokio::test]
async fn prepare_denies_hosts_the_policy_denies() {
    let (bridge, connects, _) = StubRemoteBridge::reachable();
    let policy = RemoteDesktopPolicy {
        host_access: AccessTable {
            default: Some(Access::Deny),
            entries: std::collections::BTreeMap::new(),
        },
        allow_session_takeover: None,
    };
    let adapter = RemoteDesktopEnvironmentAdapter::new(policy, Arc::new(bridge)).expect("adapter");
    let failure = EnvironmentAdapter::prepare(&adapter, prepare_request(Some("ops-remote-host-1")))
        .await
        .expect_err("policy denial");
    assert_eq!(
        failure.kind,
        codex_execution_contracts::FailureKind::PolicyDenied
    );
    assert!(failure.message.contains("denies host `ops-remote-host-1`"));
    assert_eq!(connects.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn prepare_requires_the_session_resource() {
    let (bridge, connects, _) = StubRemoteBridge::reachable();
    let adapter =
        RemoteDesktopEnvironmentAdapter::new(allowing_policy(), Arc::new(bridge)).expect("adapter");
    let failure = EnvironmentAdapter::prepare(&adapter, prepare_request(None))
        .await
        .expect_err("missing resource");
    assert_eq!(
        failure.kind,
        codex_execution_contracts::FailureKind::Permanent
    );
    assert!(failure.message.contains("requires an attached"));
    assert_eq!(connects.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn execute_runs_through_the_bridge_and_journals_evidence() {
    let (bridge, connects, executes) = StubRemoteBridge::reachable();
    let adapter =
        RemoteDesktopEnvironmentAdapter::new(allowing_policy(), Arc::new(bridge)).expect("adapter");
    EnvironmentAdapter::prepare(&adapter, prepare_request(Some("ops-remote-host-1")))
        .await
        .expect("prepare");
    let result = EnvironmentAdapter::execute(&adapter, &session_handle(), type_text_action()).await;
    assert_eq!(
        result.outcome,
        codex_execution_contracts::ActionOutcome::Succeeded
    );
    assert_eq!(result.observations.len(), 1);
    assert_eq!(
        result.observations[0].environment,
        ExecutionEnvironment::Computer
    );
    assert_eq!(
        result.observations[0].adapter,
        AdapterId::parse(REMOTE_DESKTOP_ADAPTER_ID).expect("adapter id")
    );
    let outputs = result.outputs.expect("outputs");
    assert_eq!(outputs["environment"], "remote-desktop");
    assert_eq!(outputs["host"], "ops-remote-host-1");
    assert_eq!(outputs["operation"], "type_text");
    assert_eq!(outputs["outcome"], "succeeded");
    assert_eq!(connects.load(Ordering::SeqCst), 1);
    assert_eq!(executes.load(Ordering::SeqCst), 1);

    // Journal: connect observation, execute observation, action trace.
    let records = adapter.evidence_records();
    let kinds: Vec<EvidenceKind> = records.iter().map(|record| record.kind).collect();
    assert_eq!(
        kinds,
        vec![
            EvidenceKind::Observation,
            EvidenceKind::Observation,
            EvidenceKind::Trace
        ]
    );
    assert!(records.iter().all(|record| {
        record
            .locator
            .starts_with("codex-env-adapters/codex-remote-desktop-env/")
    }));
    assert_eq!(
        adapter.session_binding(),
        Some(CapabilityBindingId::parse(REMOTE_BINDING).expect("binding"))
    );
}

#[tokio::test]
async fn execute_fails_unsupported_operations() {
    let (bridge, _, executes) = StubRemoteBridge::reachable();
    let adapter =
        RemoteDesktopEnvironmentAdapter::new(allowing_policy(), Arc::new(bridge)).expect("adapter");
    EnvironmentAdapter::prepare(&adapter, prepare_request(Some("ops-remote-host-1")))
        .await
        .expect("prepare");
    let action = Action {
        operation: OperationId::parse("teleport").expect("operation"),
        target: None,
        inputs: ActionInputs::default(),
    };
    let result = EnvironmentAdapter::execute(&adapter, &session_handle(), action).await;
    assert_eq!(
        result.outcome,
        codex_execution_contracts::ActionOutcome::Failed
    );
    let failure = result.failure.expect("failure");
    assert_eq!(
        failure.kind,
        codex_execution_contracts::FailureKind::Permanent
    );
    assert!(
        failure
            .message
            .contains("unsupported remote-desktop operation `teleport`")
    );
    assert_eq!(executes.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn takeover_displaces_the_owner_until_re_prepare() {
    let (bridge, _, _) = StubRemoteBridge::reachable();
    let policy = RemoteDesktopPolicy {
        host_access: AccessTable {
            default: Some(Access::Allow),
            entries: std::collections::BTreeMap::new(),
        },
        allow_session_takeover: Some(true),
    };
    let adapter = RemoteDesktopEnvironmentAdapter::new(policy, Arc::new(bridge)).expect("adapter");
    EnvironmentAdapter::prepare(&adapter, prepare_request(Some("ops-remote-host-1")))
        .await
        .expect("prepare");

    // Forced takeover requires a reason.
    let denied = adapter
        .takeover_session(OwnerToken::new("ops-human"), TakeoverMode::Forced, None)
        .expect_err("reason required");
    assert!(
        denied
            .to_string()
            .contains("forced takeover requires a recorded reason")
    );

    let receipt = adapter
        .takeover_session(
            OwnerToken::new("ops-human"),
            TakeoverMode::Forced,
            Some("operator intervention".to_string()),
        )
        .expect("takeover");
    assert_eq!(
        receipt.binding,
        CapabilityBindingId::parse(REMOTE_BINDING).expect("binding")
    );
    assert_eq!(receipt.target, "ops-remote-host-1");

    // Execution after the takeover requires a fresh prepare.
    let displaced =
        EnvironmentAdapter::execute(&adapter, &session_handle(), type_text_action()).await;
    assert_eq!(
        displaced.outcome,
        codex_execution_contracts::ActionOutcome::Failed
    );
    assert_eq!(
        displaced.failure.expect("failure").kind,
        codex_execution_contracts::FailureKind::Unavailable
    );

    // The next dispatch re-prepares, re-acquires, and executes.
    EnvironmentAdapter::prepare(&adapter, prepare_request(Some("ops-remote-host-1")))
        .await
        .expect("re-prepare");
    let recovered =
        EnvironmentAdapter::execute(&adapter, &session_handle(), type_text_action()).await;
    assert_eq!(
        recovered.outcome,
        codex_execution_contracts::ActionOutcome::Succeeded
    );

    // The takeover receipt was journaled as recovery-class evidence.
    let records = adapter.evidence_records();
    let recoveries: Vec<&crate::evidence::AdapterEvidenceRecord> = records
        .iter()
        .filter(|record| record.kind == EvidenceKind::Recovery)
        .collect();
    assert_eq!(recoveries.len(), 1);
    assert_eq!(recoveries[0].payload["mode"], "forced");
}
