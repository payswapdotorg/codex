//! Deterministic environment-adapter double for evaluation fixtures
//! (WO-013).
//!
//! [`ScriptedEnvironmentAdapter`] implements the frozen WO-005
//! [`EnvironmentAdapter`] boundary over a scripted turn list, so evaluation
//! runs exercise the real registry pipeline — registration, readiness,
//! authorization, binding, and single-action dispatch — without touching a
//! browser, a desktop, or any external environment.
//!
//! Every executed action is recorded as an
//! [`ActionFootprint`](crate::record::ActionFootprint) (operation, target,
//! canonical inputs, scripted duration), giving differential comparisons a
//! complete, ordered trace of what the environment was asked to do.

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
use codex_execution_contracts::CapabilityBindingId;
use codex_execution_contracts::EnvironmentAdapter;
use codex_execution_contracts::ExecutionContractError;
use codex_execution_contracts::ExecutionEnvironment;
use codex_execution_contracts::ExecutionFailure;
use codex_execution_contracts::FailureKind;
use codex_execution_contracts::Observation;
use codex_execution_contracts::PrepareRequest;
use codex_execution_contracts::PreparedSession;
use codex_execution_contracts::ProbeReport;
use codex_execution_contracts::ProvidedCapability;
use codex_execution_contracts::SessionHandle;
use codex_workflow_contracts::CapabilityId;

use crate::record::ActionFootprint;

/// The adapter identity of the scripted evaluation environment.
pub const EVAL_ADAPTER_ID: &str = "codex-eval-scripted-env";

/// The semantic capability the scripted evaluation environment provides.
pub const EVAL_CAPABILITY: &str = "inspect_environment";

/// The scripted environment class (a peer, non-reserved environment).
pub const EVAL_ENVIRONMENT: ExecutionEnvironment = ExecutionEnvironment::Api;

/// One scripted environment execution outcome.
#[derive(Clone, Debug, PartialEq)]
pub enum ScriptedEnvTurn {
    /// The action succeeds, emitting one observation and optional outputs.
    Succeed {
        /// The normalized observation payload recorded as evidence.
        observation: serde_json::Value,
        /// Bounded normalized outputs, when any.
        outputs: Option<serde_json::Value>,
        /// Scripted (logical) duration in milliseconds, when measured.
        duration_ms: Option<u64>,
    },
    /// The action fails with a pre-validated normalized failure.
    Fail {
        /// The normalized failure served to the recovery pipeline.
        failure: ExecutionFailure,
    },
}

impl ScriptedEnvTurn {
    /// A succeeding turn emitting `observation` and no outputs.
    pub fn succeed(observation: serde_json::Value) -> Self {
        Self::Succeed {
            observation,
            outputs: None,
            duration_ms: None,
        }
    }

    /// A succeeding turn emitting `observation`, `outputs`, and a logical
    /// duration.
    pub fn succeed_with(
        observation: serde_json::Value,
        outputs: serde_json::Value,
        duration_ms: u64,
    ) -> Self {
        Self::Succeed {
            observation,
            outputs: Some(outputs),
            duration_ms: Some(duration_ms),
        }
    }

    /// A failing turn with `kind` and `message`.
    ///
    /// The failure is validated at fixture construction so a malformed
    /// message (empty or unbounded) is rejected before any run starts,
    /// keeping the adapter boundary panic-free.
    pub fn fail(
        kind: FailureKind,
        message: impl Into<String>,
    ) -> Result<Self, ExecutionContractError> {
        Ok(Self::Fail {
            failure: ExecutionFailure::new(kind, message)?,
        })
    }

    /// The scripted (logical) duration of this turn, when measured.
    fn scripted_duration(&self) -> Option<u64> {
        match self {
            Self::Succeed { duration_ms, .. } => *duration_ms,
            Self::Fail { .. } => None,
        }
    }
}

/// A scripted environment adapter: serves turns in order, records every
/// executed action, and fails loudly when its script is exhausted.
#[derive(Debug)]
pub struct ScriptedEnvironmentAdapter {
    descriptor: AdapterDescriptor,
    capability: CapabilityId,
    script: Arc<Mutex<Vec<ScriptedEnvTurn>>>,
    executions: Arc<Mutex<Vec<ActionFootprint>>>,
}

impl ScriptedEnvironmentAdapter {
    /// Creates the adapter serving `script` in order.
    ///
    /// The adapter registers as a `Compatible` binding for
    /// [`EVAL_CAPABILITY`] in the [`EVAL_ENVIRONMENT`] class with no
    /// required resources, so instantiations need no resource bindings.
    pub fn new(script: Vec<ScriptedEnvTurn>) -> Result<Self, ExecutionContractError> {
        let adapter = AdapterId::parse(EVAL_ADAPTER_ID)?;
        let capability = CapabilityId::parse(EVAL_CAPABILITY)?;
        Ok(Self {
            descriptor: AdapterDescriptor {
                adapter,
                environment: EVAL_ENVIRONMENT,
                class: BindingClass::Compatible,
                provides: vec![ProvidedCapability {
                    capability: capability.clone(),
                    resources: Vec::new(),
                }],
            },
            capability,
            script: Arc::new(Mutex::new(script)),
            executions: Arc::new(Mutex::new(Vec::new())),
        })
    }

    /// Number of unscripted turns remaining.
    pub fn remaining(&self) -> usize {
        self.lock_script().len()
    }

    /// Snapshot of the actions executed so far, in dispatch order.
    pub fn executions(&self) -> Vec<ActionFootprint> {
        self.lock_executions().clone()
    }

    /// The binding identity this adapter serves.
    pub fn binding_id(&self) -> CapabilityBindingId {
        CapabilityBindingId::derive(&self.descriptor.adapter, &self.capability)
    }

    fn lock_script(&self) -> MutexGuard<'_, Vec<ScriptedEnvTurn>> {
        self.script.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn lock_executions(&self) -> MutexGuard<'_, Vec<ActionFootprint>> {
        self.executions
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }
}

impl EnvironmentAdapter for ScriptedEnvironmentAdapter {
    fn descriptor(&self) -> &AdapterDescriptor {
        &self.descriptor
    }

    fn probe(&self) -> AdapterProbeFuture<'_> {
        Box::pin(async { Ok(ProbeReport::ready()) })
    }

    fn prepare(&self, request: PrepareRequest) -> AdapterPrepareFuture<'_> {
        Box::pin(async move {
            // Binding identities are validated `<adapter>:<capability>`
            // pairs, so parsing cannot fail; an impossible failure surfaces
            // as a normalized (loud) preparation failure instead of a panic.
            let session = match SessionHandle::parse(request.binding.as_ref()) {
                Ok(handle) => handle,
                Err(error) => {
                    return Err(ExecutionFailure {
                        kind: FailureKind::Permanent,
                        message: format!("binding identity is not a valid handle: {error}"),
                    });
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
        _session: &'a SessionHandle,
        action: Action,
    ) -> AdapterExecuteFuture<'a> {
        let binding = self.binding_id();
        let adapter = self.descriptor.adapter.clone();
        let environment = self.descriptor.environment;
        let script = Arc::clone(&self.script);
        let executions = Arc::clone(&self.executions);
        Box::pin(async move {
            // Pop the turn first so the recorded footprint carries the
            // scripted (logical) duration of the attempt that follows.
            let turn = {
                let mut pending = script.lock().unwrap_or_else(PoisonError::into_inner);
                if pending.is_empty() {
                    None
                } else {
                    Some(pending.remove(0))
                }
            };
            let executed = {
                let mut recorded = executions.lock().unwrap_or_else(PoisonError::into_inner);
                recorded.push(ActionFootprint::of(
                    &action,
                    turn.as_ref().and_then(ScriptedEnvTurn::scripted_duration),
                ));
                recorded.len()
            };
            match turn {
                Some(ScriptedEnvTurn::Succeed {
                    observation,
                    outputs,
                    duration_ms,
                }) => {
                    let mut result =
                        ActionResult::succeeded(binding).with_observations(vec![Observation::new(
                            environment,
                            adapter,
                            observation,
                        )]);
                    if let Some(outputs) = outputs {
                        result = result.with_outputs(outputs);
                    }
                    if let Some(duration_ms) = duration_ms {
                        result = result.with_duration_ms(duration_ms);
                    }
                    result
                }
                Some(ScriptedEnvTurn::Fail { failure }) => ActionResult::failed(binding, failure),
                None => ActionResult::failed(binding, exhausted_failure(executed)),
            }
        })
    }
}

/// The failure served when the scripted environment runs dry.
///
/// The message is formatted from a bounded static template plus one number,
/// so it always satisfies the failure contract; the struct is built
/// directly (fields are public and fixture-bounded) instead of panicking
/// across the adapter boundary.
fn exhausted_failure(executions: usize) -> ExecutionFailure {
    ExecutionFailure {
        kind: FailureKind::Permanent,
        message: format!("scripted environment script exhausted after {executions} executions"),
    }
}
