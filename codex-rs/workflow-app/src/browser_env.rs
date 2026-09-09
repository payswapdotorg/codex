//! The browser environment wiring: WO-006 Browser Use behind the WO-005
//! `EnvironmentAdapter` boundary.
//!
//! [`BrowserUseEnvironmentAdapter`] composes the WO-006
//! [`BrowserUseAdapter`] capability lifecycle with the execution-contracts
//! registry:
//!
//! - `probe` reports the host-supplied [`BridgeProbe`] (no browser is ever
//!   launched here — that would be a second runtime);
//! - `prepare` drives the WO-006 lifecycle `Declared -> Available -> Ready
//!   -> Authorized` (requirement-vs-config pre-flight included) and binds
//!   a session identity derived from the attached `browser_profile`
//!   resource;
//! - `execute` maps the normalized [`Action`] envelope onto the WO-006
//!   browser action vocabulary, delegates the actual browser interaction
//!   to the host-supplied [`BrowserActionExecutor`] (the native Codex
//!   Browser Use integration point), records the turn as WO-006 evidence,
//!   and returns a normalized [`ActionResult`] with observation evidence.
//!
//! The operation vocabulary below is adapter-owned, exactly as the
//! execution contracts require: workflow semantics never reference it.

use std::sync::Arc;
use std::sync::Mutex;
use std::sync::MutexGuard;
use std::sync::PoisonError;

use codex_browser_use_adapter::AdapterError;
use codex_browser_use_adapter::BridgeProbe;
use codex_browser_use_adapter::BrowserAction;
use codex_browser_use_adapter::BrowserActionKind;
use codex_browser_use_adapter::BrowserActionOutcome;
use codex_browser_use_adapter::BrowserActionResult;
use codex_browser_use_adapter::BrowserSessionIdentity;
use codex_browser_use_adapter::BrowserUseAdapter;
use codex_browser_use_adapter::BrowserUseConfigSnapshot;
use codex_browser_use_adapter::BrowserUseRequirement;
use codex_browser_use_adapter::CapabilityLifecycleState;
use codex_browser_use_adapter::EvidenceRecord;
use codex_execution_contracts::ACTION_OUTPUTS_MAX_BYTES;
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
use codex_execution_contracts::FAILURE_MESSAGE_MAX_BYTES;
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
pub const BROWSER_ENV_ADAPTER_ID: &str = "codex-browser-use-env";
/// The semantic capability this environment provides (the WO-006 adapter
/// identity `codex.browser-use` is its own, adapter-scoped string).
pub const BROWSER_CAPABILITY: &str = "navigate_web";
/// The typed resource this environment requires before execution.
pub const BROWSER_PROFILE_RESOURCE: &str = "browser_profile";

/// One browser turn executed by the host integration.
///
/// `result` and `observation` are the normalized WO-006 records; the
/// optional [`FailureKind`] lets the host classify failures precisely
/// (the host's classification stays authoritative over text matching).
#[derive(Clone, Debug)]
pub struct BrowserTurn {
    /// The normalized result of the action.
    pub result: BrowserActionResult,
    /// The normalized observation captured with the action, when any.
    pub observation: Option<codex_browser_use_adapter::BrowserObservation>,
    /// The host's failure classification, when the action failed.
    pub failure_kind: Option<FailureKind>,
}

/// The host-supplied executor of browser actions against the native Codex
/// Browser Use runtime.
///
/// This is where the existing runtime is reached: the adapter never
/// launches browsers or daemons. Implementations drive the native Browser
/// Use capability for the bound session and return the normalized turn.
pub trait BrowserActionExecutor: Send + Sync {
    /// Executes one browser action in the given session.
    fn execute(&self, session: &BrowserSessionIdentity, action: &BrowserAction) -> BrowserTurn;
}

/// The live browser session state behind the adapter.
struct BrowserRuntime {
    adapter: BrowserUseAdapter,
    active: Option<ActiveBrowserSession>,
}

/// One prepared browser session: the execution-contracts binding and the
/// WO-006 execution session recording evidence.
struct ActiveBrowserSession {
    binding: CapabilityBindingId,
    session: codex_browser_use_adapter::BrowserExecutionSession,
}

/// The browser environment adapter.
pub struct BrowserUseEnvironmentAdapter {
    descriptor: AdapterDescriptor,
    capability: CapabilityId,
    probe: BridgeProbe,
    origin_scope: Vec<String>,
    executor: Arc<dyn BrowserActionExecutor>,
    inner: Mutex<BrowserRuntime>,
}

impl BrowserUseEnvironmentAdapter {
    /// Creates the adapter. Construction is lazy and side-effect free:
    /// nothing is probed, prepared, or bound until the registry asks.
    pub fn new(
        requirement: BrowserUseRequirement,
        probe: BridgeProbe,
        config: BrowserUseConfigSnapshot,
        executor: Arc<dyn BrowserActionExecutor>,
    ) -> Result<Self, WorkflowAppError> {
        let capability = CapabilityId::parse(BROWSER_CAPABILITY)?;
        let descriptor = AdapterDescriptor {
            adapter: AdapterId::parse(BROWSER_ENV_ADAPTER_ID)?,
            environment: ExecutionEnvironment::Browser,
            class: BindingClass::CodexNative,
            provides: vec![ProvidedCapability {
                capability: capability.clone(),
                resources: vec![codex_workflow_contracts::ResourceTypeId::parse(
                    BROWSER_PROFILE_RESOURCE,
                )?],
            }],
        };
        descriptor.validate()?;
        let origin_scope = requirement.origins.keys().cloned().collect::<Vec<_>>();
        Ok(Self {
            descriptor,
            capability,
            probe: probe.clone(),
            origin_scope,
            executor,
            inner: Mutex::new(BrowserRuntime {
                adapter: BrowserUseAdapter::new(requirement, probe, config),
                active: None,
            }),
        })
    }

    /// The WO-006 capability lifecycle state (for diagnostics and tests).
    pub fn capability_state(&self) -> CapabilityLifecycleState {
        self.lock_runtime().adapter.state()
    }

    /// A snapshot of the WO-006 evidence records recorded by the active
    /// execution session (host harvesting point). Each record carries its
    /// locator and digest hex; the evidence plane promotes them into
    /// contract references.
    pub fn session_records(&self) -> Vec<EvidenceRecord> {
        match &self.lock_runtime().active {
            Some(active) => active.session.records().to_vec(),
            None => Vec::new(),
        }
    }

    /// The deterministic binding identity this adapter serves.
    fn binding_id(&self) -> CapabilityBindingId {
        CapabilityBindingId::derive(&self.descriptor.adapter, &self.capability)
    }

    fn lock_runtime(&self) -> MutexGuard<'_, BrowserRuntime> {
        self.inner.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl codex_execution_contracts::EnvironmentAdapter for BrowserUseEnvironmentAdapter {
    fn descriptor(&self) -> &AdapterDescriptor {
        &self.descriptor
    }

    fn probe(&self) -> AdapterProbeFuture<'_> {
        Box::pin(async move {
            let report = if self.probe.native_bridge_present {
                if self.probe.native_runtime_provisioned {
                    ProbeReport::ready()
                } else {
                    ProbeReport {
                        status: ProbeStatus::Available,
                        diagnostic: None,
                    }
                }
            } else {
                ProbeReport {
                    status: ProbeStatus::NotInstalled,
                    diagnostic: Some(codex_execution_contracts::BindingDiagnostic::new(
                        codex_execution_contracts::DiagnosticCode::NotInstalled,
                        "native Codex Browser Use bridge not found; refusing to start a second \
                         browser agent",
                    )),
                }
            };
            Ok(report)
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
            match runtime.adapter.state() {
                CapabilityLifecycleState::Declared
                | CapabilityLifecycleState::Available
                | CapabilityLifecycleState::Ready => {
                    // Drive the WO-006 lifecycle through readiness and the
                    // requirement-vs-config authorization pre-flight.
                    if let Err(error) = runtime.adapter.prepare() {
                        return Err(browser_failure(&error));
                    }
                }
                CapabilityLifecycleState::Authorized => {
                    // Recovered path: readiness resolved, bind below.
                }
                CapabilityLifecycleState::Bound | CapabilityLifecycleState::Executing => {
                    // Fast path: the session for this binding is live.
                    if let Some(active) = &runtime.active
                        && active.binding == request.binding
                    {
                        return Ok(PreparedSession {
                            binding: request.binding,
                            session: binding_handle,
                        });
                    }
                    return Err(normalized_failure(
                        FailureKind::Unavailable,
                        format!(
                            "browser binding `{}` cannot be re-prepared while a session is live",
                            request.binding
                        ),
                    ));
                }
                CapabilityLifecycleState::Interrupted | CapabilityLifecycleState::Released => {
                    return Err(normalized_failure(
                        FailureKind::Unavailable,
                        format!(
                            "browser binding `{}` is interrupted; remediate before re-preparing",
                            request.binding
                        ),
                    ));
                }
            }
            // Bind a session identity derived from the attached browser
            // profile resource and the binding identity.
            let profile = request
                .resources
                .iter()
                .find(|resource| resource.resource_type.as_ref() == BROWSER_PROFILE_RESOURCE)
                .map(|resource| resource.resource.to_string());
            let session = BrowserSessionIdentity {
                profile,
                session_id: format!("workflow-app-{}", request.binding),
                tabs: Vec::new(),
                takeover_of: None,
            };
            if let Err(error) = runtime.adapter.bind(session, self.origin_scope.clone()) {
                return Err(browser_failure(&error));
            }
            let execution = match runtime.adapter.begin_execution() {
                Ok(execution) => execution,
                Err(error) => return Err(browser_failure(&error)),
            };
            runtime.active = Some(ActiveBrowserSession {
                binding: request.binding.clone(),
                session: execution,
            });
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
            let active = match &mut runtime.active {
                Some(active) if active.binding == binding => active,
                _ => {
                    return ActionResult::failed(
                        binding,
                        normalized_failure(
                            FailureKind::Unavailable,
                            format!("browser session `{session}` is not prepared"),
                        ),
                    );
                }
            };
            let browser_action = match map_browser_action(&action_payload) {
                Ok(browser_action) => browser_action,
                Err(reason) => {
                    return ActionResult::failed(
                        binding,
                        normalized_failure(FailureKind::Permanent, reason),
                    );
                }
            };
            let session_identity = active.session.binding().session.clone();
            let turn = self.executor.execute(&session_identity, &browser_action);
            if let Some(observation) = &turn.observation {
                active.session.record_observation(observation);
            }
            active.session.record_action(&browser_action, &turn.result);
            let mut observations = Vec::new();
            if let Some(observation) = &turn.observation {
                match serde_json::to_value(observation) {
                    Ok(payload) => observations.push(Observation::new(
                        ExecutionEnvironment::Browser,
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
            match &turn.result.outcome {
                BrowserActionOutcome::Succeeded => match serde_json::to_value(&turn.result) {
                    Ok(outputs) => ActionResult::succeeded(binding)
                        .with_observations(observations)
                        .with_outputs(outputs),
                    Err(error) => ActionResult::failed(
                        binding,
                        normalized_failure(FailureKind::Permanent, error.to_string()),
                    ),
                },
                BrowserActionOutcome::Failed { reason } => ActionResult::failed(
                    binding,
                    normalized_failure(
                        turn.failure_kind.unwrap_or(FailureKind::Permanent),
                        reason.clone(),
                    ),
                )
                .with_observations(observations),
                BrowserActionOutcome::BlockedByPolicy { policy } => ActionResult::failed(
                    binding,
                    normalized_failure(
                        FailureKind::PolicyDenied,
                        format!("blocked by browser policy: {policy}"),
                    ),
                )
                .with_observations(observations),
                BrowserActionOutcome::DeniedByApproval => ActionResult::failed(
                    binding,
                    normalized_failure(
                        FailureKind::PolicyDenied,
                        "denied by the browser approval flow",
                    ),
                )
                .with_observations(observations),
            }
        })
    }
}

/// Maps a WO-006 adapter error to a normalized execution failure.
fn browser_failure(error: &AdapterError) -> ExecutionFailure {
    let kind = match error {
        AdapterError::Unavailable { .. } => FailureKind::Unavailable,
        AdapterError::Unauthorized { .. } => FailureKind::PolicyDenied,
        AdapterError::NotPrepared
        | AdapterError::NotBound
        | AdapterError::IllegalTransition { .. } => FailureKind::Permanent,
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
        message = "unspecified browser failure".to_string();
    }
    let limit = FAILURE_MESSAGE_MAX_BYTES.min(ACTION_OUTPUTS_MAX_BYTES);
    if message.len() > limit {
        message.truncate(limit);
    }
    ExecutionFailure { kind, message }
}

/// Maps the serialized action envelope onto the WO-006 action vocabulary.
///
/// Input parameters are read from the canonical JSON serialization of the
/// action (the contract keeps the structured field private; the wire form
/// is the adapter's read path).
fn map_browser_action(payload: &serde_json::Value) -> Result<BrowserAction, String> {
    let operation = payload
        .get("operation")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    let kind = match operation {
        "navigate" => BrowserActionKind::Navigate,
        "click" => BrowserActionKind::Click,
        "input" => BrowserActionKind::Input,
        "scroll" => BrowserActionKind::Scroll,
        "key_press" => BrowserActionKind::KeyPress,
        "wait" => BrowserActionKind::Wait,
        "snapshot" => BrowserActionKind::Snapshot,
        "upload" => BrowserActionKind::Upload,
        "download" => BrowserActionKind::Download,
        other => {
            return Err(format!("unsupported browser operation `{other}`"));
        }
    };
    let inputs = payload
        .get("inputs")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let target = payload
        .get("target")
        .and_then(serde_json::Value::as_str)
        .map(str::to_string);
    let origin = inputs
        .get("origin")
        .and_then(serde_json::Value::as_str)
        .map(str::to_string);
    let detail = inputs
        .get("text")
        .or_else(|| inputs.get("detail"))
        .and_then(serde_json::Value::as_str)
        .map(str::to_string);
    Ok(BrowserAction {
        kind,
        origin,
        target,
        detail,
    })
}
