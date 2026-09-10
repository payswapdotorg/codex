//! The mobile-device environment adapter (WO-015).
//!
//! [`MobileDeviceEnvironmentAdapter`] binds mobile-device execution to the
//! WO-005 [`EnvironmentAdapter`] seam:
//!
//! - `probe` reports the host bridge's liveness answer; an unreachable
//!   bridge is an explicit `NotInstalled` diagnostic, never a silently
//!   weaker mechanism;
//! - `prepare` checks the adapter-side device policy against the device
//!   named by the attached `mobile_device` resource, then attaches the
//!   bridge session and journals the initial observation;
//! - `execute` maps the normalized action envelope onto the mobile
//!   vocabulary, executes one action through the bridge, and returns a
//!   normalized result with the captured observation.
//!
//! The operation vocabulary is adapter-owned; workflow semantics never
//! reference it.

use std::sync::Arc;
use std::sync::Mutex;
use std::sync::MutexGuard;
use std::sync::PoisonError;

use codex_execution_contracts::Action;
use codex_execution_contracts::ActionResult;
use codex_execution_contracts::AdapterDescriptor;
use codex_execution_contracts::AdapterExecuteFuture;
use codex_execution_contracts::AdapterId;
use codex_execution_contracts::AdapterPrepareFuture;
use codex_execution_contracts::AdapterProbeFuture;
use codex_execution_contracts::BindingClass;
use codex_execution_contracts::BindingDiagnostic;
use codex_execution_contracts::CapabilityBindingId;
use codex_execution_contracts::DiagnosticCode;
use codex_execution_contracts::ExecutionEnvironment;
use codex_execution_contracts::ExecutionFailure;
use codex_execution_contracts::FailureKind;
use codex_execution_contracts::Observation;
use codex_execution_contracts::PrepareRequest;
use codex_execution_contracts::PreparedSession;
use codex_execution_contracts::ProbeReport;
use codex_execution_contracts::ProbeStatus;
use codex_execution_contracts::ProvidedCapability;
use codex_execution_contracts::SessionHandle;
use codex_workflow_contracts::CapabilityId;
use codex_workflow_contracts::EvidenceKind;
use codex_workflow_contracts::ResourceTypeId;

use crate::EnvAdapterError;
use crate::bridge::BridgeCallFailure;
use crate::bridge::normalized_failure;
use crate::evidence::AdapterEvidenceRecord;
use crate::evidence::EvidenceJournal;
use crate::mobile_device::MOBILE_DEVICE_ADAPTER_ID;
use crate::mobile_device::MOBILE_DEVICE_CAPABILITY;
use crate::mobile_device::MOBILE_DEVICE_RESOURCE;
use crate::mobile_device::MobileDeviceAction;
use crate::mobile_device::MobileDeviceBridge;
use crate::mobile_device::MobileDevicePolicy;
use crate::policy::AccessDecision;
use crate::session::BridgeSession;
use crate::session::OwnerToken;
use crate::session::TakeoverMode;
use crate::session::TakeoverReceipt;

/// The live mobile-device session state behind the adapter.
struct MobileRuntime {
    session: Option<BridgeSession>,
    journal: EvidenceJournal,
}

/// The mobile-device environment adapter.
///
/// Construction is lazy and side-effect free: nothing is probed,
/// attached, or bound until the registry asks. One adapter instance
/// serves the `control_mobile_device` capability and drives at most one
/// device session at a time, the same single-session model as the WO-007
/// desktop wiring.
pub struct MobileDeviceEnvironmentAdapter {
    descriptor: AdapterDescriptor,
    capability: CapabilityId,
    policy: MobileDevicePolicy,
    bridge: Arc<dyn MobileDeviceBridge>,
    inner: Mutex<MobileRuntime>,
}

impl MobileDeviceEnvironmentAdapter {
    /// Creates the adapter over a host-supplied device bridge.
    ///
    /// The descriptor registers behind the peer `Computer` environment
    /// class at the `Compatible` position of the frozen fallback chain:
    /// the device bridge is a compatible application-interaction
    /// environment, not a Codex-native runtime.
    pub fn new(
        policy: MobileDevicePolicy,
        bridge: Arc<dyn MobileDeviceBridge>,
    ) -> Result<Self, EnvAdapterError> {
        let capability = CapabilityId::parse(MOBILE_DEVICE_CAPABILITY)?;
        let descriptor = AdapterDescriptor {
            adapter: AdapterId::parse(MOBILE_DEVICE_ADAPTER_ID)?,
            environment: ExecutionEnvironment::Computer,
            class: BindingClass::Compatible,
            provides: vec![ProvidedCapability {
                capability: capability.clone(),
                resources: vec![ResourceTypeId::parse(MOBILE_DEVICE_RESOURCE)?],
            }],
        };
        descriptor.validate()?;
        Ok(Self {
            descriptor,
            capability,
            policy,
            bridge,
            inner: Mutex::new(MobileRuntime {
                session: None,
                journal: EvidenceJournal::new(MOBILE_DEVICE_ADAPTER_ID),
            }),
        })
    }

    /// A snapshot of the evidence records recorded so far (host
    /// harvesting point). Each record carries its locator and digest hex;
    /// the evidence plane promotes them into contract references.
    pub fn evidence_records(&self) -> Vec<AdapterEvidenceRecord> {
        self.lock_runtime().journal.records().to_vec()
    }

    /// The binding identity of the live session, when a session is open.
    pub fn session_binding(&self) -> Option<CapabilityBindingId> {
        self.lock_runtime()
            .session
            .as_ref()
            .map(|session| session.binding.clone())
    }

    /// Records a session takeover, when policy allows it.
    ///
    /// The receipt is journaled as recovery-class evidence and the session
    /// owner is displaced; the next registry dispatch re-prepares and
    /// re-acquires the session, so takeover displaces without wedging a
    /// run. Forced takeovers require a recorded reason.
    pub fn takeover_session(
        &self,
        new_owner: OwnerToken,
        mode: TakeoverMode,
        reason: Option<String>,
    ) -> Result<TakeoverReceipt, EnvAdapterError> {
        let mut runtime = self.lock_runtime();
        if self.policy.allow_session_takeover != Some(true) {
            return Err(EnvAdapterError::TakeoverDenied {
                reason: "mobile-device policy does not allow session takeover".to_string(),
            });
        }
        if mode == TakeoverMode::Forced && reason.is_none() {
            return Err(EnvAdapterError::TakeoverDenied {
                reason: "forced takeover requires a recorded reason".to_string(),
            });
        }
        let Some(session) = runtime.session.as_mut() else {
            return Err(EnvAdapterError::TakeoverDenied {
                reason: "no mobile-device session is open".to_string(),
            });
        };
        let receipt = TakeoverReceipt {
            binding: session.binding.clone(),
            target: session.target.clone(),
            previous_owner: session.owner.clone(),
            new_owner: new_owner.clone(),
            mode,
            reason,
        };
        session.owner = new_owner;
        let payload = serde_json::to_value(&receipt)?;
        runtime.journal.record(EvidenceKind::Recovery, &payload)?;
        Ok(receipt)
    }

    /// The deterministic binding identity this adapter serves.
    fn binding_id(&self) -> CapabilityBindingId {
        CapabilityBindingId::derive(&self.descriptor.adapter, &self.capability)
    }

    fn lock_runtime(&self) -> MutexGuard<'_, MobileRuntime> {
        self.inner.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl codex_execution_contracts::EnvironmentAdapter for MobileDeviceEnvironmentAdapter {
    fn descriptor(&self) -> &AdapterDescriptor {
        &self.descriptor
    }

    fn probe(&self) -> AdapterProbeFuture<'_> {
        Box::pin(async move {
            let report = match self.bridge.health() {
                Ok(_) => ProbeReport::ready(),
                Err(failure) => ProbeReport {
                    status: ProbeStatus::NotInstalled,
                    diagnostic: Some(BindingDiagnostic::new(
                        DiagnosticCode::NotInstalled,
                        format!(
                            "mobile-device bridge is not reachable: {}",
                            failure.message()
                        ),
                    )),
                },
            };
            Ok(report)
        })
    }

    fn prepare(&self, request: PrepareRequest) -> AdapterPrepareFuture<'_> {
        Box::pin(async move {
            let session_handle = match SessionHandle::parse(request.binding.as_ref()) {
                Ok(handle) => handle,
                Err(error) => {
                    return Err(normalized_failure(
                        FailureKind::Permanent,
                        error.to_string(),
                    ));
                }
            };
            let mut runtime = self.lock_runtime();
            // Fast path: the session for this binding is already open. It is
            // re-acquired for the dispatching owner if a takeover displaced
            // it, so displacement is observable but never wedges the run.
            if let Some(session) = runtime.session.as_mut()
                && session.binding == request.binding
            {
                session.owner = OwnerToken::new(session.binding.to_string());
                return Ok(PreparedSession {
                    binding: request.binding,
                    session: session_handle,
                });
            }
            // Slow path: derive the device from the attached resource, check
            // the adapter policy, and attach the bridge session.
            let device = match request
                .resources
                .iter()
                .find(|resource| resource.resource_type.as_ref() == MOBILE_DEVICE_RESOURCE)
            {
                Some(resource) => resource.resource.as_ref().to_string(),
                None => {
                    return Err(normalized_failure(
                        FailureKind::Permanent,
                        format!(
                            "mobile-device binding `{}` requires an attached \
                             `{MOBILE_DEVICE_RESOURCE}` resource",
                            request.binding
                        ),
                    ));
                }
            };
            match self.policy.device_access.decide(&device) {
                AccessDecision::Allowed => {}
                AccessDecision::Denied => {
                    return Err(normalized_failure(
                        FailureKind::PolicyDenied,
                        format!("mobile-device policy denies device `{device}`"),
                    ));
                }
                AccessDecision::Unspecified => {
                    return Err(normalized_failure(
                        FailureKind::PolicyDenied,
                        format!(
                            "mobile-device device `{device}` requires explicit host \
                             authorization"
                        ),
                    ));
                }
            }
            let observation = match self.bridge.attach(&device) {
                Ok(observation) => observation,
                Err(failure) => {
                    return Err(bridge_failure(&failure));
                }
            };
            let payload = match serde_json::to_value(&observation) {
                Ok(payload) => payload,
                Err(error) => {
                    return Err(normalized_failure(
                        FailureKind::Permanent,
                        error.to_string(),
                    ));
                }
            };
            if let Err(error) = runtime.journal.record(EvidenceKind::Observation, &payload) {
                return Err(normalized_failure(
                    FailureKind::Permanent,
                    error.to_string(),
                ));
            }
            runtime.session = Some(BridgeSession::new(request.binding.clone(), device));
            Ok(PreparedSession {
                binding: request.binding,
                session: session_handle,
            })
        })
    }

    fn execute<'a>(
        &'a self,
        session: &'a SessionHandle,
        action: Action,
    ) -> AdapterExecuteFuture<'a> {
        Box::pin(async move {
            let binding = self.binding_id();
            let action_payload = match serde_json::to_value(&action) {
                Ok(payload) => payload,
                Err(error) => {
                    return ActionResult::failed(
                        binding,
                        normalized_failure(FailureKind::Permanent, error.to_string()),
                    );
                }
            };
            let domain_action = match map_mobile_action(&action_payload) {
                Ok(domain_action) => domain_action,
                Err(reason) => {
                    return ActionResult::failed(
                        binding,
                        normalized_failure(FailureKind::Permanent, reason),
                    );
                }
            };
            let mut runtime = self.lock_runtime();
            let Some(active) = runtime.session.as_mut() else {
                return ActionResult::failed(
                    binding,
                    normalized_failure(
                        FailureKind::Unavailable,
                        format!("mobile-device session `{session}` is not prepared"),
                    ),
                );
            };
            if active.binding.as_ref() != session.as_ref() {
                return ActionResult::failed(
                    binding,
                    normalized_failure(
                        FailureKind::Unavailable,
                        format!("mobile-device session `{session}` is not prepared"),
                    ),
                );
            }
            if !active.owned_by_dispatch() {
                return ActionResult::failed(
                    binding,
                    normalized_failure(
                        FailureKind::Unavailable,
                        format!(
                            "mobile-device session `{session}` was taken over; \
                             re-prepare required"
                        ),
                    ),
                );
            }
            let device = active.target.clone();
            let outcome = self.bridge.execute(&device, domain_action);
            match outcome {
                Ok(outcome) => {
                    let mut observations = Vec::new();
                    if let Some(observation) = &outcome.observation {
                        let payload = match serde_json::to_value(observation) {
                            Ok(payload) => payload,
                            Err(error) => {
                                return ActionResult::failed(
                                    binding,
                                    normalized_failure(FailureKind::Permanent, error.to_string()),
                                );
                            }
                        };
                        if let Err(error) =
                            runtime.journal.record(EvidenceKind::Observation, &payload)
                        {
                            return ActionResult::failed(
                                binding,
                                normalized_failure(FailureKind::Permanent, error.to_string()),
                            );
                        }
                        observations.push(Observation::new(
                            ExecutionEnvironment::Computer,
                            self.descriptor.adapter.clone(),
                            payload,
                        ));
                    }
                    let trace = mobile_trace(
                        &action,
                        &device,
                        "succeeded",
                        outcome.detail.as_deref(),
                        None,
                    );
                    if let Err(error) = runtime.journal.record(EvidenceKind::Trace, &trace) {
                        return ActionResult::failed(
                            binding,
                            normalized_failure(FailureKind::Permanent, error.to_string()),
                        );
                    }
                    ActionResult::succeeded(binding)
                        .with_observations(observations)
                        .with_outputs(trace)
                }
                Err(failure) => {
                    let trace = mobile_trace(&action, &device, "failed", None, Some(&failure));
                    if let Err(error) = runtime.journal.record(EvidenceKind::Trace, &trace) {
                        return ActionResult::failed(
                            binding,
                            normalized_failure(FailureKind::Permanent, error.to_string()),
                        );
                    }
                    ActionResult::failed(binding, bridge_failure(&failure))
                }
            }
        })
    }
}

/// Maps a bridge call failure to a normalized execution failure.
fn bridge_failure(failure: &BridgeCallFailure) -> ExecutionFailure {
    normalized_failure(failure.failure_kind(), failure.message())
}

/// Builds the bounded action trace shared by the journal and the result
/// outputs.
fn mobile_trace(
    action: &Action,
    device: &str,
    outcome: &str,
    detail: Option<&str>,
    failure: Option<&BridgeCallFailure>,
) -> serde_json::Value {
    let failure_kind = failure.map(|failure| failure.kind);
    let failure_detail = failure.map(|failure| failure.detail.as_str());
    serde_json::json!({
        "environment": "mobile-device",
        "device": device,
        "operation": action.operation.as_ref(),
        "outcome": outcome,
        "detail": detail,
        "failureKind": failure_kind,
        "failureDetail": failure_detail,
    })
}

/// Maps the serialized action envelope onto the mobile vocabulary.
///
/// Input parameters are read from the canonical JSON serialization of the
/// action (the contract keeps the structured field private; the wire form
/// is the adapter's read path).
fn map_mobile_action(payload: &serde_json::Value) -> Result<MobileDeviceAction, String> {
    let operation = payload
        .get("operation")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    let inputs = payload
        .get("inputs")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    match operation {
        "tap" => Ok(MobileDeviceAction::Tap {
            x: inputs
                .get("x")
                .and_then(serde_json::Value::as_i64)
                .unwrap_or_default() as i32,
            y: inputs
                .get("y")
                .and_then(serde_json::Value::as_i64)
                .unwrap_or_default() as i32,
        }),
        "swipe" => Ok(MobileDeviceAction::Swipe {
            start_x: inputs
                .get("start_x")
                .and_then(serde_json::Value::as_i64)
                .unwrap_or_default() as i32,
            start_y: inputs
                .get("start_y")
                .and_then(serde_json::Value::as_i64)
                .unwrap_or_default() as i32,
            end_x: inputs
                .get("end_x")
                .and_then(serde_json::Value::as_i64)
                .unwrap_or_default() as i32,
            end_y: inputs
                .get("end_y")
                .and_then(serde_json::Value::as_i64)
                .unwrap_or_default() as i32,
            duration_ms: inputs
                .get("duration_ms")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or_default(),
        }),
        "type_text" => Ok(MobileDeviceAction::TypeText {
            text: inputs
                .get("text")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .to_string(),
        }),
        "press_key" => Ok(MobileDeviceAction::PressKey {
            key: inputs
                .get("key")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .to_string(),
        }),
        "launch_application" => Ok(MobileDeviceAction::LaunchApplication {
            app_id: inputs
                .get("app_id")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .to_string(),
        }),
        "screenshot" => Ok(MobileDeviceAction::Screenshot),
        "device_info" => Ok(MobileDeviceAction::DeviceInfo),
        other => Err(format!("unsupported mobile-device operation `{other}`")),
    }
}

#[cfg(test)]
#[path = "mobile_device_adapter_tests.rs"]
mod tests;
