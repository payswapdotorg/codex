//! The run path: bounded graph walk, per-step dispatch, and recovery.
//!
//! Running composes the frozen planes without adding a second engine:
//!
//! - the walk is the teaching compiler's frozen deterministic replay
//!   policy ([`crate::walk`], asserted equivalent to
//!   `codex_teaching_compiler::simulate_ir` by tests);
//! - every step dispatches **one action at a time** through
//!   [`CapabilityRegistry::dispatch`], which owns binding decisions,
//!   readiness transitions, and validation;
//! - failures flow through the frozen pipeline
//!   `classify -> recover (retry/rebind) -> verify -> continue or
//!   escalate`, recorded as [`Recovery`] evidence on the instance;
//! - observations, traces, approvals, and recovery records are promoted
//!   into contract [`EvidenceReference`]s through the evidence-plane seam.
//!
//! The run consumes the active instantiation: settled instances (also
//! paused ones, until a later scheduling Work Order owns resume) are read
//! back through [`WorkflowLifecycle::verify_run`], and cancelled instances
//! are terminal records no run ever settles
//! ([`WorkflowLifecycle::cancel`] owns the takeover).

use std::collections::BTreeMap;

use codex_execution_contracts::Action;
use codex_execution_contracts::ActionOutcome;
use codex_execution_contracts::BindingDecision;
use codex_execution_contracts::BindingPolicy;
use codex_execution_contracts::CapabilityBindingId;
use codex_execution_contracts::ExecutionEnvironment;
use codex_execution_contracts::ExecutionFailure;
use codex_execution_contracts::FailureKind;
use codex_execution_contracts::ReadinessState;
use codex_execution_contracts::Recovery;
use codex_execution_contracts::RecoveryOutcome;
use codex_execution_contracts::RecoveryStrategy;
use codex_execution_contracts::TransitionCause;
use codex_workflow_contracts::CapabilityId;
use codex_workflow_contracts::EvidenceKind;
use codex_workflow_contracts::IrNodeId;
use codex_workflow_contracts::StepNode;
use codex_workflow_contracts::WorkflowInstance;
use codex_workflow_contracts::WorkflowInstanceStatus;
use codex_workflow_contracts::WorkflowIr;
use codex_workflow_contracts::WorkflowIrNode;
use codex_workflow_contracts::WorkflowVersion;

use crate::WorkflowAppError;
use crate::WorkflowEvent;
use crate::lifecycle::WorkflowLifecycle;
use crate::port::ActionRequest;
use crate::walk::Walk;
use crate::walk::WalkConfig;
use crate::walk::WalkStep;
use crate::walk::WalkTerminal;

/// Why the run stopped.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum RunTerminal {
    /// Every step completed and the walk reached a terminal state.
    Completed,
    /// The run paused at a wait or human-gate node.
    Paused {
        /// The node the run paused at.
        node: IrNodeId,
        /// Why the run paused.
        reason: String,
    },
    /// The run settled with failure (escalated action, indeterminate
    /// continuation, or an internal seam failure).
    Failed {
        /// Why the run failed.
        reason: String,
    },
}

/// The outcome of one completed run.
#[derive(Clone, Debug)]
pub struct RunOutcome {
    /// The settled instance record (also persisted through the instance
    /// store).
    pub instance: WorkflowInstance,
    /// Why the run stopped.
    pub terminal: RunTerminal,
    /// The nodes the walk visited, in order.
    pub path: Vec<IrNodeId>,
}

/// The in-flight state of one instantiated workflow.
pub(crate) struct ActiveRun {
    /// The immutable version being executed.
    pub(crate) version: WorkflowVersion,
    /// The in-flight instance record; evidence is appended here and saved
    /// through the instance store.
    pub(crate) instance: WorkflowInstance,
    /// The binding policy in force for the run.
    pub(crate) policy: BindingPolicy,
    /// The resumable deterministic walk.
    pub(crate) walk: Walk,
    /// The walk budget.
    pub(crate) walk_config: WalkConfig,
    /// Binding decisions per step node, resolved at instantiation (and
    /// extended by rebinds).
    pub(crate) decisions: BTreeMap<IrNodeId, Vec<BindingDecision>>,
}

/// How a step settled.
enum StepFlow {
    /// Every capability requirement of the step completed.
    Completed,
    /// The step failed and was escalated.
    Failed(String),
}

/// How a recovery attempt settled.
enum RecoveryFlow {
    /// Execution continued after the verified recovery.
    Recovered(codex_execution_contracts::ActionResult),
    /// The failure was escalated to the control plane.
    Escalated(String),
}

impl WorkflowLifecycle {
    /// Runs the active instantiation to a terminal state.
    ///
    /// The instance record is settled (`Succeeded`, `Paused`, or
    /// `Failed`) and persisted before the outcome is returned. Internal
    /// seam failures settle the instance as `Failed` with the structured
    /// reason rather than leaving a phantom `Running` record behind.
    pub async fn run(&mut self) -> Result<RunOutcome, WorkflowAppError> {
        let mut active = self
            .active
            .take()
            .ok_or(WorkflowAppError::NoActiveWorkflow)?;
        let ir = active.version.definition.ir.clone();
        let walk_config = active.walk_config;
        let terminal = match execute_run(self, &mut active, &ir, walk_config).await {
            Ok(terminal) => terminal,
            Err(error) => RunTerminal::Failed {
                reason: error.to_string(),
            },
        };
        let instance_id = active.instance.instance_id;
        match &terminal {
            RunTerminal::Completed => {
                // The one status-mutation seam: the settlement transition
                // (Running -> Succeeded) is validated against the frozen
                // contract table before the record is persisted.
                self.transition_status(&mut active.instance, WorkflowInstanceStatus::Succeeded)?;
                self.events.record(WorkflowEvent::RunCompleted {
                    instance: instance_id,
                });
            }
            RunTerminal::Paused { node, reason } => {
                // The one status-mutation seam (Running -> Paused).
                self.transition_status(&mut active.instance, WorkflowInstanceStatus::Paused)?;
                self.events.record(WorkflowEvent::RunPaused {
                    instance: instance_id,
                    node: node.clone(),
                    reason: reason.clone(),
                });
            }
            RunTerminal::Failed { reason } => {
                // The one status-mutation seam (Running -> Failed).
                self.transition_status(&mut active.instance, WorkflowInstanceStatus::Failed)?;
                self.events.record(WorkflowEvent::RunFailed {
                    instance: instance_id,
                    reason: reason.clone(),
                });
            }
        }
        self.instances.save(active.instance.clone())?;
        let path = active.walk.path().to_vec();
        let instance = active.instance.clone();
        Ok(RunOutcome {
            instance,
            terminal,
            path,
        })
    }

    /// Proposes the action for one step capability through the action
    /// source seam (the Codex agent/model loop).
    fn propose_action(&mut self, request: &ActionRequest) -> Result<Action, WorkflowAppError> {
        self.actions.action_for(request)
    }

    /// Executes one step node: proposes one action per declared capability
    /// requirement and dispatches it through the registry.
    async fn execute_step(
        &mut self,
        active: &mut ActiveRun,
        node_id: &IrNodeId,
        _step: &StepNode,
    ) -> Result<StepFlow, WorkflowAppError> {
        let instance_id = active.instance.instance_id;
        self.events.record(WorkflowEvent::StepStarted {
            instance: instance_id,
            node: node_id.clone(),
        });
        let decisions = active.decisions.get(node_id).cloned().unwrap_or_default();
        for decision in decisions {
            let binding = decision.selected.binding.clone();
            let action = match self.propose_action(&ActionRequest {
                instance: instance_id,
                node: node_id.clone(),
                requirement: decision.requirement.clone(),
                selected: decision.selected.clone(),
            }) {
                Ok(action) => action,
                Err(error) => {
                    // No proposed action: escalate through the standard
                    // failure pipeline (never invent an action).
                    let reason = format!("no action available: {error}");
                    let failure = ExecutionFailure::new(FailureKind::Permanent, reason.clone())?;
                    let reason =
                        self.escalate_failure(active, node_id, &binding, failure, reason)?;
                    return Ok(StepFlow::Failed(reason));
                }
            };
            let result = self.registry.dispatch(&binding, action.clone()).await?;
            match result.outcome {
                ActionOutcome::Succeeded => {
                    self.record_action_success(active, node_id, &binding, &action, &result)?;
                }
                ActionOutcome::Failed => {
                    let failure = result.failure.clone().ok_or_else(|| {
                        WorkflowAppError::Execution(
                            codex_execution_contracts::ExecutionContractError::InvalidRecord {
                                reason: format!(
                                    "failed result from binding `{binding}` carries no failure"
                                ),
                            },
                        )
                    })?;
                    self.events.record(WorkflowEvent::ActionFailed {
                        instance: instance_id,
                        node: node_id.clone(),
                        binding: binding.clone(),
                        kind: failure.kind,
                        message: failure.message.clone(),
                    });
                    match self
                        .recover_failure(active, node_id, &decision, action.clone(), failure)
                        .await?
                    {
                        RecoveryFlow::Recovered(recovered) => {
                            self.record_action_success(
                                active,
                                node_id,
                                &recovered.binding,
                                &action,
                                &recovered,
                            )?;
                        }
                        RecoveryFlow::Escalated(reason) => {
                            return Ok(StepFlow::Failed(reason));
                        }
                    }
                }
            }
        }
        self.events.record(WorkflowEvent::StepCompleted {
            instance: instance_id,
            node: node_id.clone(),
        });
        Ok(StepFlow::Completed)
    }

    /// The frozen failure pipeline for one classified action failure.
    ///
    /// `Transient`/`Timeout` retry once on the same binding (with a fresh
    /// authorization); `Unavailable` attempts a policy-checked rebind to a
    /// compatible alternate binding; `Permanent`/`PolicyDenied` escalate
    /// immediately. Every attempt and escalation is recorded as recovery
    /// evidence.
    async fn recover_failure(
        &mut self,
        active: &mut ActiveRun,
        node_id: &IrNodeId,
        decision: &BindingDecision,
        action: Action,
        failure: ExecutionFailure,
    ) -> Result<RecoveryFlow, WorkflowAppError> {
        let instance_id = active.instance.instance_id;
        let binding = decision.selected.binding.clone();
        let capability = decision.requirement.capability.clone();
        match failure.kind {
            FailureKind::Transient | FailureKind::Timeout => {
                // Retry once after restoring the binding's readiness.
                let environment = decision.selected.environment;
                self.restore_binding(active, &binding, environment, capability)?;
                let retry = self.registry.dispatch(&binding, action.clone()).await?;
                if retry.outcome == ActionOutcome::Succeeded {
                    self.record_recovery(
                        active,
                        binding.clone(),
                        failure.clone(),
                        RecoveryStrategy::Retry,
                        RecoveryOutcome::Recovered,
                    )?;
                    self.events.record(WorkflowEvent::Recovered {
                        instance: instance_id,
                        node: node_id.clone(),
                        binding,
                        strategy: "retry".to_string(),
                    });
                    return Ok(RecoveryFlow::Recovered(retry));
                }
                let retry_detail = retry
                    .failure
                    .as_ref()
                    .map(|retry_failure| retry_failure.message.clone())
                    .unwrap_or_else(|| "no failure record".to_string());
                let reason = format!("retry of binding `{binding}` failed: {retry_detail}");
                self.record_retry_not_recovered(active, &binding, &failure)?;
                let reason = self.escalate_failure(active, node_id, &binding, failure, reason)?;
                Ok(RecoveryFlow::Escalated(reason))
            }
            FailureKind::Unavailable => {
                // Attempt a policy-checked rebind to a compatible binding.
                let rebind = match self.registry.resolve(&decision.requirement, &active.policy) {
                    Ok(new_decision) => new_decision,
                    Err(diagnostic) => {
                        let reason = format!(
                            "rebind for capability `{}` failed: {:?}: {}",
                            capability.as_ref(),
                            diagnostic.code,
                            diagnostic.message
                        );
                        let reason =
                            self.escalate_failure(active, node_id, &binding, failure, reason)?;
                        return Ok(RecoveryFlow::Escalated(reason));
                    }
                };
                if rebind.selected.binding == binding {
                    let reason =
                        format!("rebind for binding `{binding}` selected the same binding");
                    let reason =
                        self.escalate_failure(active, node_id, &binding, failure, reason)?;
                    return Ok(RecoveryFlow::Escalated(reason));
                }
                // Record the fallback decision as recovery evidence.
                let decision_payload = serde_json::to_value(&rebind)?;
                self.store_evidence(
                    &mut active.instance,
                    EvidenceKind::Recovery,
                    &decision_payload,
                )?;
                // Authorize and bind the alternate before dispatching.
                let new_binding = rebind.selected.binding.clone();
                let environment = rebind.selected.environment;
                let approval =
                    self.authorize_binding(&instance_id, &new_binding, environment, capability)?;
                active.instance.record_evidence(approval);
                self.registry.bind(&new_binding)?;
                let rebound = self.registry.dispatch(&new_binding, action.clone()).await?;
                if rebound.outcome == ActionOutcome::Succeeded {
                    self.record_recovery(
                        active,
                        binding.clone(),
                        failure.clone(),
                        RecoveryStrategy::Rebind(rebind),
                        RecoveryOutcome::Recovered,
                    )?;
                    self.events.record(WorkflowEvent::Recovered {
                        instance: instance_id,
                        node: node_id.clone(),
                        binding: new_binding,
                        strategy: "rebind".to_string(),
                    });
                    return Ok(RecoveryFlow::Recovered(rebound));
                }
                let rebound_detail = rebound
                    .failure
                    .as_ref()
                    .map(|rebound_failure| rebound_failure.message.clone())
                    .unwrap_or_else(|| "no failure record".to_string());
                let reason =
                    format!("rebound action on binding `{new_binding}` failed: {rebound_detail}");
                self.record_recovery(
                    active,
                    binding.clone(),
                    failure.clone(),
                    RecoveryStrategy::Rebind(rebind),
                    RecoveryOutcome::Failed,
                )?;
                let reason = self.escalate_failure(active, node_id, &binding, failure, reason)?;
                Ok(RecoveryFlow::Escalated(reason))
            }
            FailureKind::Permanent | FailureKind::PolicyDenied => {
                let reason = format!(
                    "binding `{binding}` failed permanently: {}",
                    failure.message
                );
                let reason = self.escalate_failure(active, node_id, &binding, failure, reason)?;
                Ok(RecoveryFlow::Escalated(reason))
            }
        }
    }

    /// Restores a failed binding to `BOUND` after a retryable failure:
    /// `FAILED -> READY` (recovered), a fresh authorization grant, and
    /// resource re-attachment. Approvals stay fresh: re-execution always
    /// re-crosses the approval gate.
    fn restore_binding(
        &mut self,
        active: &mut ActiveRun,
        binding: &CapabilityBindingId,
        environment: ExecutionEnvironment,
        capability: CapabilityId,
    ) -> Result<(), WorkflowAppError> {
        let state = self.registry.binding(binding)?.readiness.state();
        if state == ReadinessState::Failed {
            self.registry.advance(binding, TransitionCause::Recovered)?;
        }
        let approval = self.authorize_binding(
            &active.instance.instance_id,
            binding,
            environment,
            capability,
        )?;
        active.instance.record_evidence(approval);
        self.registry.bind(binding)?;
        Ok(())
    }

    /// Records an escalation: a `Recovery` evidence record with the
    /// `Escalate` strategy plus an `Escalated` event. Returns the
    /// escalation reason.
    fn escalate_failure(
        &mut self,
        active: &mut ActiveRun,
        node_id: &IrNodeId,
        binding: &CapabilityBindingId,
        failure: ExecutionFailure,
        reason: String,
    ) -> Result<String, WorkflowAppError> {
        self.record_recovery(
            active,
            binding.clone(),
            failure,
            RecoveryStrategy::Escalate,
            RecoveryOutcome::Escalated,
        )?;
        self.events.record(WorkflowEvent::Escalated {
            instance: active.instance.instance_id,
            node: node_id.clone(),
            binding: binding.clone(),
            reason: reason.clone(),
        });
        Ok(reason)
    }

    /// Records one recovery attempt as evidence on the instance.
    fn record_recovery(
        &mut self,
        active: &mut ActiveRun,
        binding: CapabilityBindingId,
        failure: ExecutionFailure,
        strategy: RecoveryStrategy,
        outcome: RecoveryOutcome,
    ) -> Result<(), WorkflowAppError> {
        let record = Recovery {
            binding,
            failure,
            strategy,
            outcome,
        };
        record.validate()?;
        let payload = serde_json::to_value(&record)?;
        self.store_evidence(&mut active.instance, EvidenceKind::Recovery, &payload)?;
        Ok(())
    }

    /// Records a retry attempt that did not recover.
    fn record_retry_not_recovered(
        &mut self,
        active: &mut ActiveRun,
        binding: &CapabilityBindingId,
        failure: &ExecutionFailure,
    ) -> Result<(), WorkflowAppError> {
        self.record_recovery(
            active,
            binding.clone(),
            failure.clone(),
            RecoveryStrategy::Retry,
            RecoveryOutcome::Failed,
        )
    }

    /// Promotes a successful (or recovered) dispatch into evidence: the
    /// normalized observations and a bounded dispatch trace.
    fn record_action_success(
        &mut self,
        active: &mut ActiveRun,
        node_id: &IrNodeId,
        binding: &CapabilityBindingId,
        action: &Action,
        result: &codex_execution_contracts::ActionResult,
    ) -> Result<(), WorkflowAppError> {
        for observation in &result.observations {
            let payload = serde_json::to_value(observation)?;
            self.store_evidence(&mut active.instance, EvidenceKind::Observation, &payload)?;
        }
        let payload = serde_json::json!({
            "node": node_id.to_string(),
            "binding": binding.to_string(),
            "operation": action.operation.as_ref(),
            "outcome": "succeeded",
            "outputs": result.outputs.clone(),
        });
        self.store_evidence(&mut active.instance, EvidenceKind::Trace, &payload)?;
        Ok(())
    }
}

/// Drives the walk to a terminal state, executing step nodes.
async fn execute_run(
    lifecycle: &mut WorkflowLifecycle,
    active: &mut ActiveRun,
    ir: &WorkflowIr,
    walk_config: WalkConfig,
) -> Result<RunTerminal, WorkflowAppError> {
    let terminal = loop {
        match active.walk.next(ir, walk_config)? {
            WalkStep::Visit(node_id) => {
                if let Some(WorkflowIrNode::Step(step)) = ir.nodes.get(&node_id).cloned() {
                    match lifecycle.execute_step(active, &node_id, &step).await? {
                        StepFlow::Completed => {}
                        StepFlow::Failed(reason) => break RunTerminal::Failed { reason },
                    }
                }
            }
            WalkStep::Terminal(WalkTerminal::Completed) => break RunTerminal::Completed,
            WalkStep::Terminal(WalkTerminal::Paused { node, reason }) => {
                break RunTerminal::Paused {
                    node,
                    reason: reason.to_string(),
                };
            }
            WalkStep::Terminal(WalkTerminal::Indeterminate { node, reason }) => {
                break RunTerminal::Failed {
                    reason: format!("run is indeterminate at node `{node}`: {reason}"),
                };
            }
        }
    };
    Ok(terminal)
}
