//! Tests for the mobile-device environment adapter.

use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;

use pretty_assertions::assert_eq;

use codex_execution_contracts::Action;
use codex_execution_contracts::ActionInputs;
use codex_execution_contracts::AdapterId;
use codex_execution_contracts::BindingClass;
use codex_execution_contracts::CapabilityBindingId;
use codex_execution_contracts::EnvironmentAdapter;
use codex_execution_contracts::ExecutionEnvironment;
use codex_execution_contracts::OperationId;
use codex_execution_contracts::PrepareRequest;
use codex_execution_contracts::ResourceBinding;
use codex_execution_contracts::SessionHandle;
use codex_workflow_contracts::CapabilityId;
use codex_workflow_contracts::EvidenceKind;
use codex_workflow_contracts::ResourceTypeId;

use crate::bridge::BridgeCallFailure;
use crate::bridge::BridgeEndpoint;
use crate::bridge::BridgeFailureKind;
use crate::bridge::BridgeHealth;
use crate::mobile_device::MOBILE_DEVICE_ADAPTER_ID;
use crate::mobile_device::MOBILE_DEVICE_CAPABILITY;
use crate::mobile_device::MOBILE_DEVICE_RESOURCE;
use crate::mobile_device::MobileDeviceAction;
use crate::mobile_device::MobileDeviceBridge;
use crate::mobile_device::MobileDeviceClass;
use crate::mobile_device::MobileDeviceObservation;
use crate::mobile_device::MobileDeviceOutcome;
use crate::mobile_device::MobileDevicePolicy;
use crate::mobile_device_adapter::MobileDeviceEnvironmentAdapter;
use crate::policy::Access;
use crate::policy::AccessTable;

const MOBILE_BINDING: &str = "codex-mobile-device-env:control_mobile_device";

/// A stub mobile-device bridge counting calls.
struct StubMobileBridge {
    reachable: bool,
    attaches: Arc<AtomicUsize>,
    executes: Arc<AtomicUsize>,
}

impl StubMobileBridge {
    fn reachable() -> (Self, Arc<AtomicUsize>, Arc<AtomicUsize>) {
        let attaches = Arc::new(AtomicUsize::new(0));
        let executes = Arc::new(AtomicUsize::new(0));
        (
            Self {
                reachable: true,
                attaches: attaches.clone(),
                executes: executes.clone(),
            },
            attaches,
            executes,
        )
    }

    fn unreachable() -> Self {
        Self {
            reachable: false,
            attaches: Arc::new(AtomicUsize::new(0)),
            executes: Arc::new(AtomicUsize::new(0)),
        }
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

impl MobileDeviceBridge for StubMobileBridge {
    fn health(&self) -> Result<BridgeHealth, BridgeCallFailure> {
        if self.reachable {
            Ok(BridgeHealth {
                endpoint: BridgeEndpoint {
                    bridge_kind: "adb".to_string(),
                    runtime: "stub-device-bridge".to_string(),
                    version: "1.0.0-test".to_string(),
                },
                latency_ms: Some(3),
            })
        } else {
            Err(BridgeCallFailure::new(
                BridgeFailureKind::Lost,
                "stub device bridge unreachable",
            ))
        }
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
        Ok(MobileDeviceOutcome {
            observation: Some(Self::observation(device)),
            detail: Some("stub ok".to_string()),
        })
    }
}

/// An allow-everything policy.
fn allowing_policy() -> MobileDevicePolicy {
    MobileDevicePolicy {
        device_access: AccessTable {
            default: Some(Access::Allow),
            entries: std::collections::BTreeMap::new(),
        },
        allow_session_takeover: None,
    }
}

/// A prepare request for the mobile binding with an optional device
/// resource.
fn prepare_request(resource: Option<&str>) -> PrepareRequest {
    let resources = resource
        .map(|instance| {
            vec![ResourceBinding {
                resource_type: ResourceTypeId::parse(MOBILE_DEVICE_RESOURCE)
                    .expect("resource type"),
                resource: codex_execution_contracts::ResourceId::parse(instance)
                    .expect("resource id"),
                holder: ExecutionEnvironment::Computer,
            }]
        })
        .unwrap_or_default();
    PrepareRequest {
        binding: CapabilityBindingId::parse(MOBILE_BINDING).expect("binding id"),
        capability: CapabilityId::parse(MOBILE_DEVICE_CAPABILITY).expect("capability"),
        resources,
    }
}

/// A tap action for the mobile vocabulary.
fn tap_action() -> Action {
    let mut inputs = ActionInputs::default();
    inputs.insert("x", serde_json::json!(480));
    inputs.insert("y", serde_json::json!(1200));
    Action {
        operation: OperationId::parse("tap").expect("operation"),
        target: None,
        inputs,
    }
}

fn session_handle() -> SessionHandle {
    SessionHandle::parse(MOBILE_BINDING).expect("session handle")
}

#[test]
fn descriptor_registers_as_a_compatible_computer_alternate() {
    let (bridge, _, _) = StubMobileBridge::reachable();
    let adapter =
        MobileDeviceEnvironmentAdapter::new(allowing_policy(), Arc::new(bridge)).expect("adapter");
    let descriptor = adapter.descriptor();
    descriptor.validate().expect("valid descriptor");
    assert_eq!(
        descriptor.adapter,
        AdapterId::parse(MOBILE_DEVICE_ADAPTER_ID).expect("adapter id")
    );
    assert_eq!(descriptor.environment, ExecutionEnvironment::Computer);
    assert_eq!(descriptor.class, BindingClass::Compatible);
    let capabilities: Vec<CapabilityId> = descriptor
        .provides
        .iter()
        .map(|provided| provided.capability.clone())
        .collect();
    assert_eq!(
        capabilities,
        vec![CapabilityId::parse(MOBILE_DEVICE_CAPABILITY).expect("capability")]
    );
    assert_eq!(
        descriptor.provides[0].resources,
        vec![ResourceTypeId::parse(MOBILE_DEVICE_RESOURCE).expect("resource type")]
    );
}

#[tokio::test]
async fn unreachable_bridges_probe_not_installed() {
    let adapter = MobileDeviceEnvironmentAdapter::new(
        allowing_policy(),
        Arc::new(StubMobileBridge::unreachable()),
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
async fn prepare_denies_devices_the_policy_denies() {
    let (bridge, attaches, _) = StubMobileBridge::reachable();
    let policy = MobileDevicePolicy {
        device_access: AccessTable {
            default: Some(Access::Deny),
            entries: std::collections::BTreeMap::new(),
        },
        allow_session_takeover: None,
    };
    let adapter = MobileDeviceEnvironmentAdapter::new(policy, Arc::new(bridge)).expect("adapter");
    let failure = EnvironmentAdapter::prepare(&adapter, prepare_request(Some("pixel-8-emulator")))
        .await
        .expect_err("policy denial");
    assert_eq!(
        failure.kind,
        codex_execution_contracts::FailureKind::PolicyDenied
    );
    assert!(failure.message.contains("denies device `pixel-8-emulator`"));
    assert_eq!(attaches.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn execute_runs_through_the_bridge_and_journals_evidence() {
    let (bridge, attaches, executes) = StubMobileBridge::reachable();
    let adapter =
        MobileDeviceEnvironmentAdapter::new(allowing_policy(), Arc::new(bridge)).expect("adapter");
    EnvironmentAdapter::prepare(&adapter, prepare_request(Some("pixel-8-emulator")))
        .await
        .expect("prepare");
    let result = EnvironmentAdapter::execute(&adapter, &session_handle(), tap_action()).await;
    assert_eq!(
        result.outcome,
        codex_execution_contracts::ActionOutcome::Succeeded
    );
    assert_eq!(result.observations.len(), 1);
    assert_eq!(
        result.observations[0].environment,
        ExecutionEnvironment::Computer
    );
    let outputs = result.outputs.expect("outputs");
    assert_eq!(outputs["environment"], "mobile-device");
    assert_eq!(outputs["device"], "pixel-8-emulator");
    assert_eq!(outputs["operation"], "tap");
    assert_eq!(outputs["outcome"], "succeeded");
    assert_eq!(attaches.load(Ordering::SeqCst), 1);
    assert_eq!(executes.load(Ordering::SeqCst), 1);

    // Journal: attach observation, execute observation, action trace.
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
    assert_eq!(
        adapter.session_binding(),
        Some(CapabilityBindingId::parse(MOBILE_BINDING).expect("binding"))
    );
}

#[tokio::test]
async fn execute_fails_unsupported_operations() {
    let (bridge, _, executes) = StubMobileBridge::reachable();
    let adapter =
        MobileDeviceEnvironmentAdapter::new(allowing_policy(), Arc::new(bridge)).expect("adapter");
    EnvironmentAdapter::prepare(&adapter, prepare_request(Some("pixel-8-emulator")))
        .await
        .expect("prepare");
    let action = Action {
        operation: OperationId::parse("shake").expect("operation"),
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
            .contains("unsupported mobile-device operation `shake`")
    );
    assert_eq!(executes.load(Ordering::SeqCst), 0);
}
