//! The computer environment wiring: WO-007 Computer Use behind the WO-005
//! `EnvironmentAdapter` boundary.
//!
//! [`ComputerUseEnvironmentAdapter`] composes the WO-007
//! [`ComputerUseWorkflowAdapter`] capability lifecycle with the
//! execution-contracts registry:
//!
//! - `probe` drives the WO-007 lazy readiness path
//!   (`Declared -> Available -> Ready` via bridge provisioning); missing
//!   provisioning surfaces as an explicit `NotInstalled` diagnostic;
//! - `prepare` authorizes the desktop application named by the attached
//!   `desktop_session` resource under the WO-007 policy (Codex approvals
//!   remain the authority for unspecified applications) and binds a
//!   session owned by the binding identity;
//! - `execute` maps the normalized [`Action`] envelope onto the WO-007
//!   normalized desktop-action vocabulary, executes one step plan, and
//!   returns the normalized [`ActionResult`] with pre/post screen
//!   observations as evidence.
//!
//! The operation vocabulary below is adapter-owned, exactly as the
//! execution contracts require: workflow semantics never reference it.

use std::sync::Arc;
use std::sync::Mutex;
use std::sync::MutexGuard;
use std::sync::PoisonError;

use codex_computer_use_adapter::AdapterEvidenceRecord;
use codex_computer_use_adapter::ApplicationIdentity;
use codex_computer_use_adapter::ApplicationPlatformIdentity;
use codex_computer_use_adapter::ComputerUseAdapterError;
use codex_computer_use_adapter::ComputerUseBridgeFactory;
use codex_computer_use_adapter::ComputerUsePolicy;
use codex_computer_use_adapter::ComputerUseSessionId;
use codex_computer_use_adapter::ComputerUseWorkflowAdapter;
use codex_computer_use_adapter::KeyModifier;
use codex_computer_use_adapter::MouseButton;
use codex_computer_use_adapter::NormalizedAction;
use codex_computer_use_adapter::OwnerToken;
use codex_computer_use_adapter::PointerTarget;
use codex_computer_use_adapter::RecoveryPolicy;
use codex_computer_use_adapter::StepExecutionStatus;
use codex_computer_use_adapter::StepPlan;
use codex_computer_use_adapter::UnixMsClock;
use codex_execution_contracts::Action;
use codex_execution_contracts::ActionResult;
use codex_execution_contracts::AdapterDescriptor;
use codex_execution_contracts::AdapterExecuteFuture;
use codex_execution_contracts::AdapterId;
use codex_execution_contracts::AdapterPrepareFuture;
use codex_execution_contracts::AdapterProbeFuture;
use codex_execution_contracts::BindingClass;
use codex_execution_contracts::CapabilityBindingId;
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

use crate::WorkflowAppError;

/// The adapter identity registered with the capability registry.
pub const COMPUTER_ENV_ADAPTER_ID: &str = "codex-computer-use-env";
/// The semantic capability this environment provides (the WO-007 adapter
/// identity `codex.computer-use` is its own, adapter-scoped string).
pub const COMPUTER_CAPABILITY: &str = "control_desktop_app";
/// The typed resource this environment requires before execution.
pub const DESKTOP_SESSION_RESOURCE: &str = "desktop_session";

/// The live computer-use session state behind the adapter.
struct ComputerRuntime {
    adapter: ComputerUseWorkflowAdapter,
    binding: Option<CapabilityBindingId>,
    owner: Option<OwnerToken>,
}

/// The computer environment adapter.
pub struct ComputerUseEnvironmentAdapter {
    descriptor: AdapterDescriptor,
    capability: CapabilityId,
    inner: Mutex<ComputerRuntime>,
}

impl ComputerUseEnvironmentAdapter {
    /// Creates the adapter with the system clock. Construction is lazy and
    /// side-effect free: no bridge is provisioned until readiness is
    /// probed.
    pub fn new(
        policy: ComputerUsePolicy,
        factory: Arc<dyn ComputerUseBridgeFactory>,
    ) -> Result<Self, WorkflowAppError> {
        Self::with_clock(policy, factory, codex_computer_use_adapter::system_clock())
    }

    /// Creates the adapter with an injected clock (deterministic tests).
    pub fn with_clock(
        policy: ComputerUsePolicy,
        factory: Arc<dyn ComputerUseBridgeFactory>,
        clock: UnixMsClock,
    ) -> Result<Self, WorkflowAppError> {
        let capability = CapabilityId::parse(COMPUTER_CAPABILITY)?;
        let descriptor = AdapterDescriptor {
            adapter: AdapterId::parse(COMPUTER_ENV_ADAPTER_ID)?,
            environment: ExecutionEnvironment::Computer,
            class: BindingClass::CodexNative,
            provides: vec![ProvidedCapability {
                capability: capability.clone(),
                resources: vec![codex_workflow_contracts::ResourceTypeId::parse(
                    DESKTOP_SESSION_RESOURCE,
                )?],
            }],
        };
        descriptor.validate()?;
        Ok(Self {
            descriptor,
            capability,
            inner: Mutex::new(ComputerRuntime {
                adapter: ComputerUseWorkflowAdapter::with_clock(policy, factory, clock),
                binding: None,
                owner: None,
            }),
        })
    }

    /// The WO-007 capability lifecycle state (for diagnostics and tests).
    pub fn capability_state(&self) -> codex_computer_use_adapter::CapabilityLifecycleState {
        self.lock_runtime().adapter.lifecycle_state()
    }

    /// A snapshot of the WO-007 evidence records emitted so far (host
    /// harvesting point).
    pub fn evidence_records(&self) -> Vec<AdapterEvidenceRecord> {
        self.lock_runtime().adapter.evidence_records().to_vec()
    }

    /// The bound WO-007 session id, when a session is bound.
    pub fn session_id(&self) -> Option<ComputerUseSessionId> {
        self.lock_runtime()
            .adapter
            .binding()
            .map(|binding| binding.session_id.clone())
    }

    /// The deterministic binding identity this adapter serves.
    fn binding_id(&self) -> CapabilityBindingId {
        CapabilityBindingId::derive(&self.descriptor.adapter, &self.capability)
    }

    fn lock_runtime(&self) -> MutexGuard<'_, ComputerRuntime> {
        self.inner.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl codex_execution_contracts::EnvironmentAdapter for ComputerUseEnvironmentAdapter {
    fn descriptor(&self) -> &AdapterDescriptor {
        &self.descriptor
    }

    fn probe(&self) -> AdapterProbeFuture<'_> {
        Box::pin(async move {
            let mut runtime = self.lock_runtime();
            if runtime.adapter.lifecycle_state()
                == codex_computer_use_adapter::CapabilityLifecycleState::Declared
                && let Err(error) = runtime.adapter.mark_available()
            {
                return Err(computer_failure(&error));
            }
            match runtime.adapter.ensure_ready() {
                Ok(_) => Ok(ProbeReport::ready()),
                Err(ComputerUseAdapterError::Readiness { diagnostic }) => Ok(ProbeReport {
                    status: ProbeStatus::NotInstalled,
                    diagnostic: Some(codex_execution_contracts::BindingDiagnostic::new(
                        codex_execution_contracts::DiagnosticCode::NotInstalled,
                        diagnostic,
                    )),
                }),
                Err(error) => Err(computer_failure(&error)),
            }
        })
    }

    fn prepare(&self, request: PrepareRequest) -> AdapterPrepareFuture<'_> {
        Box::pin(async move {
            let binding_handle = match SessionHandle::parse(request.binding.as_ref()) {
                Ok(handle) => handle,
                Err(error) => {
                    return Err(normalized_failure(
                        FailureKind::Permanent,
                        error.to_string(),
                    ));
                }
            };
            let mut runtime = self.lock_runtime();
            // Fast path: a session is already bound for this binding.
            if let Some(binding) = &runtime.binding
                && binding == &request.binding
                && runtime.adapter.lifecycle_state()
                    == codex_computer_use_adapter::CapabilityLifecycleState::Bound
            {
                return Ok(PreparedSession {
                    binding: request.binding,
                    session: binding_handle,
                });
            }
            // Slow path: drive readiness, authorize the target
            // application, and bind a fresh session.
            match runtime.adapter.lifecycle_state() {
                codex_computer_use_adapter::CapabilityLifecycleState::Declared => {
                    if let Err(error) = runtime.adapter.mark_available() {
                        return Err(computer_failure(&error));
                    }
                }
                codex_computer_use_adapter::CapabilityLifecycleState::Failed => {
                    return Err(normalized_failure(
                        FailureKind::Permanent,
                        format!(
                            "computer use capability for binding `{}` failed; reset required",
                            request.binding
                        ),
                    ));
                }
                _ => {}
            }
            if let Err(error) = runtime.adapter.ensure_ready() {
                return Err(computer_failure(&error));
            }
            let application = application_from_resources(&request.resources);
            if let Err(error) = runtime.adapter.authorize(&application, None) {
                return Err(computer_failure(&error));
            }
            let owner = OwnerToken::new(request.binding.to_string());
            if let Err(error) = runtime.adapter.bind(owner.clone()) {
                return Err(computer_failure(&error));
            }
            runtime.binding = Some(request.binding.clone());
            runtime.owner = Some(owner);
            Ok(PreparedSession {
                binding: request.binding,
                session: binding_handle,
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
            let mut runtime = self.lock_runtime();
            let owner = match (&runtime.binding, &runtime.owner) {
                (Some(bound), Some(owner)) if bound == &binding => owner.clone(),
                _ => {
                    return ActionResult::failed(
                        binding,
                        normalized_failure(
                            FailureKind::Unavailable,
                            format!("computer session `{session}` is not bound"),
                        ),
                    );
                }
            };
            let normalized = match map_computer_action(&action_payload) {
                Ok(normalized) => normalized,
                Err(reason) => {
                    return ActionResult::failed(
                        binding,
                        normalized_failure(FailureKind::Permanent, reason),
                    );
                }
            };
            // Targeted applications must be authorized before execution;
            // the WO-007 adapter refuses unauthorized targets.
            let targets = normalized
                .target_application()
                .map(|application| vec![application.clone()])
                .unwrap_or_default();
            for target in &targets {
                if let Err(error) = runtime.adapter.authorize(target, None) {
                    return ActionResult::failed(binding, computer_failure(&error));
                }
            }
            let step_ref = action
                .target
                .as_ref()
                .map(|target| target.as_ref().to_string())
                .unwrap_or_else(|| binding.to_string());
            let plan = StepPlan {
                step_ref,
                targets,
                actions: vec![normalized],
                capture_observations: true,
                recovery: RecoveryPolicy::FailFast,
            };
            let record = match runtime.adapter.execute_step(&owner, plan) {
                Ok(record) => record,
                Err(error) => {
                    return ActionResult::failed(binding, computer_failure(&error));
                }
            };
            let mut observations = Vec::new();
            for observation in [&record.pre_observation, &record.post_observation]
                .into_iter()
                .flatten()
            {
                match serde_json::to_value(observation) {
                    Ok(payload) => observations.push(Observation::new(
                        ExecutionEnvironment::Computer,
                        self.descriptor.adapter.clone(),
                        payload,
                    )),
                    Err(error) => {
                        return ActionResult::failed(
                            binding,
                            normalized_failure(FailureKind::Permanent, error.to_string()),
                        );
                    }
                }
            }
            match &record.status {
                StepExecutionStatus::Completed | StepExecutionStatus::CompletedWithRecovery => {
                    match serde_json::to_value(&record) {
                        Ok(outputs) => ActionResult::succeeded(binding)
                            .with_observations(observations)
                            .with_outputs(outputs),
                        Err(error) => ActionResult::failed(
                            binding,
                            normalized_failure(FailureKind::Permanent, error.to_string()),
                        ),
                    }
                }
                StepExecutionStatus::Failed { reason } => {
                    let kind = if reason.starts_with("bridge lost") {
                        FailureKind::Unavailable
                    } else {
                        FailureKind::Permanent
                    };
                    ActionResult::failed(binding, normalized_failure(kind, reason.clone()))
                        .with_observations(observations)
                }
            }
        })
    }
}

/// Maps a WO-007 adapter error to a normalized execution failure.
fn computer_failure(error: &ComputerUseAdapterError) -> ExecutionFailure {
    let kind = match error {
        ComputerUseAdapterError::Readiness { .. } | ComputerUseAdapterError::BridgeCall(_) => {
            FailureKind::Unavailable
        }
        ComputerUseAdapterError::PolicyDenied { .. }
        | ComputerUseAdapterError::AuthorizationRequired { .. }
        | ComputerUseAdapterError::TargetNotAuthorized { .. } => FailureKind::PolicyDenied,
        ComputerUseAdapterError::InvalidLifecycleTransition { .. }
        | ComputerUseAdapterError::OwnerMismatch { .. }
        | ComputerUseAdapterError::SessionBusy { .. }
        | ComputerUseAdapterError::WrongLifecycleState { .. }
        | ComputerUseAdapterError::FallbackRejected(_)
        | ComputerUseAdapterError::TakeoverReasonRequired
        | ComputerUseAdapterError::TakeoverPolicyDenied => FailureKind::Permanent,
    };
    normalized_failure(kind, error.to_string())
}

/// Builds a normalized failure with clamped, non-empty detail.
///
/// [`ExecutionFailure::new`] rejects empty and oversized messages; the
/// application adapter clamps instead of failing so a diagnostic is never
/// lost because its text was out of bounds.
fn normalized_failure(kind: FailureKind, message: impl Into<String>) -> ExecutionFailure {
    let mut message = message.into();
    if message.is_empty() {
        message = "unspecified computer-use failure".to_string();
    }
    if message.len() > codex_execution_contracts::FAILURE_MESSAGE_MAX_BYTES {
        message.truncate(codex_execution_contracts::FAILURE_MESSAGE_MAX_BYTES);
    }
    ExecutionFailure { kind, message }
}

/// Derives the desktop application identity from the attached
/// `desktop_session` resource: the resource names the application the
/// session drives, and the WO-007 policy gates it.
fn application_from_resources(
    resources: &[codex_execution_contracts::ResourceBinding],
) -> ApplicationIdentity {
    let name = resources
        .iter()
        .find(|resource| resource.resource_type.as_ref() == DESKTOP_SESSION_RESOURCE)
        .map(|resource| resource.resource.as_ref().to_string())
        .unwrap_or_else(|| "workflow-desktop".to_string());
    ApplicationIdentity {
        display_name: name.clone(),
        platform: ApplicationPlatformIdentity::Generic {
            binary_name: Some(name),
        },
    }
}

/// Maps the serialized action envelope onto the WO-007 action vocabulary.
///
/// Input parameters are read from the canonical JSON serialization of the
/// action (the contract keeps the structured field private; the wire form
/// is the adapter's read path).
fn map_computer_action(payload: &serde_json::Value) -> Result<NormalizedAction, String> {
    let operation = payload
        .get("operation")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    let inputs = payload
        .get("inputs")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let pointer = || {
        let window_ref = inputs
            .get("window_ref")
            .and_then(serde_json::Value::as_str)
            .map(str::to_string);
        PointerTarget {
            x: inputs
                .get("x")
                .and_then(serde_json::Value::as_i64)
                .unwrap_or_default() as i32,
            y: inputs
                .get("y")
                .and_then(serde_json::Value::as_i64)
                .unwrap_or_default() as i32,
            window_ref,
        }
    };
    match operation {
        "click" => {
            let button = match inputs
                .get("button")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("left")
            {
                "right" => MouseButton::Right,
                "middle" => MouseButton::Middle,
                _ => MouseButton::Left,
            };
            Ok(NormalizedAction::Click {
                target: pointer(),
                button,
                click_count: inputs
                    .get("count")
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or_default() as u8,
            })
        }
        "type_text" => Ok(NormalizedAction::TypeText {
            text: inputs
                .get("text")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .to_string(),
        }),
        "press_key" => {
            let modifiers = inputs
                .get("modifiers")
                .and_then(serde_json::Value::as_array)
                .map(|values| {
                    values
                        .iter()
                        .filter_map(|value| value.as_str())
                        .map(|modifier| match modifier {
                            "ctrl" => KeyModifier::Ctrl,
                            "alt" => KeyModifier::Alt,
                            "shift" => KeyModifier::Shift,
                            _ => KeyModifier::Meta,
                        })
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            Ok(NormalizedAction::PressKey {
                key: inputs
                    .get("key")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                modifiers,
            })
        }
        "scroll" => Ok(NormalizedAction::Scroll {
            target: pointer(),
            delta_y: inputs
                .get("delta_y")
                .and_then(serde_json::Value::as_i64)
                .unwrap_or_default() as i32,
        }),
        "focus_window" => Ok(NormalizedAction::FocusWindow {
            window_ref: inputs
                .get("window_ref")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .to_string(),
        }),
        "launch_application" => {
            let application = inputs
                .get("application")
                .cloned()
                .map(|value| application_from_json(&value))
                .unwrap_or_else(|| ApplicationIdentity {
                    display_name: "workflow-desktop".to_string(),
                    platform: ApplicationPlatformIdentity::Generic { binary_name: None },
                });
            Ok(NormalizedAction::LaunchApplication { application })
        }
        "screenshot" => Ok(NormalizedAction::Screenshot),
        "wait" => Ok(NormalizedAction::Wait {
            millis: inputs
                .get("millis")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or_default(),
        }),
        other => Err(format!("unsupported computer operation `{other}`")),
    }
}

/// Builds an application identity from action input JSON.
fn application_from_json(value: &serde_json::Value) -> ApplicationIdentity {
    let display_name = value
        .get("display_name")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("workflow-desktop")
        .to_string();
    let platform = match value.get("bundle_id").and_then(serde_json::Value::as_str) {
        Some(bundle_id) => ApplicationPlatformIdentity::MacosBundle {
            bundle_id: bundle_id.to_string(),
        },
        None => ApplicationPlatformIdentity::Generic {
            binary_name: value
                .get("binary_name")
                .and_then(serde_json::Value::as_str)
                .map(str::to_string),
        },
    };
    ApplicationIdentity {
        display_name,
        platform,
    }
}
