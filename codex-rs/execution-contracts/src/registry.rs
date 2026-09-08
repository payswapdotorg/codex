//! The capability and resource registry.
//!
//! [`CapabilityRegistry`] is the in-process binding decision authority of
//! the execution plane: it registers environment adapters, tracks binding
//! readiness through the frozen state machine, binds resources, resolves
//! capability requirements against policy with evidenced fallback, plans
//! multi-environment execution for a workflow IR, and dispatches single
//! actions to prepared sessions.
//!
//! ## What the registry is not
//!
//! - It is **not a workflow engine**: it never walks a graph, sequences
//!   steps, or owns durable workflow state. Graph traversal and durable
//!   lifecycle remain control-plane authority (roadmap M4, WO-010).
//! - It is **not durable**: it is an in-process decision surface. Durable
//!   instances, evidence storage, and scheduling are owned by later Work
//!   Orders.
//! - It is **not the policy authority**: policy arrives as
//!   [`crate::BindingPolicy`] inputs; the registry enforces, never
//!   invents.
//!
//! ## Reuse boundaries
//!
//! Adapters implementing [`crate::EnvironmentAdapter`] wrap existing Codex
//! primitives (Browser Use, Computer Use, terminal exec, tools, MCP,
//! human approval surfaces); approvals arrive as
//! [`crate::AuthorizationGrant`] references to Codex approval evidence;
//! sessions, rollout traces, and skills stay adapter-side concerns. The
//! registry itself never duplicates any of those mechanisms.

use std::collections::BTreeMap;
use std::sync::Arc;

use crate::Action;
use crate::ActionOutcome;
use crate::ActionResult;
use crate::AdapterId;
use crate::AuthorizationGrant;
use crate::BindingDecision;
use crate::BindingDiagnostic;
use crate::BindingPlan;
use crate::BindingPolicy;
use crate::CapabilityBinding;
use crate::CapabilityBindingId;
use crate::DiagnosticCode;
use crate::EnvironmentAdapter;
use crate::ExecutionContractError;
use crate::FallbackReason;
use crate::FallbackRecord;
use crate::PrepareRequest;
use crate::ProbeStatus;
use crate::ReadinessState;
use crate::ReadinessTracker;
use crate::ResourceBinding;
use crate::SelectedBinding;
use crate::StepBinding;
use crate::TransitionCause;
use crate::preparation_diagnostic;
use crate::probe_error_diagnostic;
use crate::resolution::compare_candidates;
use crate::resolution::not_installed;
use crate::resolution::record_skipped;
use crate::resolution::selected_of;
use crate::resolution::skipped_diagnostic;
use codex_workflow_contracts::CapabilityId;
use codex_workflow_contracts::CapabilityRequirement;
use codex_workflow_contracts::ResourceTypeId;
use codex_workflow_contracts::WorkflowIr;
use codex_workflow_contracts::WorkflowIrNode;

/// One registered capability binding with its live execution state.
#[derive(Clone, Debug)]
pub struct RegisteredBinding {
    /// The immutable binding record.
    pub binding: CapabilityBinding,
    /// The binding's readiness state machine.
    pub readiness: ReadinessTracker,
    /// The authorization grant that moved the binding to `AUTHORIZED`.
    pub authorization: Option<AuthorizationGrant>,
    /// The resource bindings attached when the binding became `BOUND`.
    pub attached: Vec<ResourceBinding>,
}

/// The capability and resource registry: the execution plane's in-process
/// binding decision authority.
#[derive(Default)]
pub struct CapabilityRegistry {
    adapters: BTreeMap<AdapterId, Arc<dyn EnvironmentAdapter>>,
    bindings: BTreeMap<CapabilityBindingId, RegisteredBinding>,
    resources: BTreeMap<ResourceTypeId, ResourceBinding>,
}

impl std::fmt::Debug for CapabilityRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CapabilityRegistry")
            .field("adapters", &self.adapters.len())
            .field("bindings", &self.bindings.len())
            .field("resources", &self.resources.len())
            .finish()
    }
}

impl CapabilityRegistry {
    /// Creates an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers an environment adapter, creating one `DECLARED` binding
    /// per provided capability.
    ///
    /// Registration validates the adapter descriptor (peer environment,
    /// non-empty unique capabilities) and rejects duplicate adapter ids.
    /// Returns the ids of the created bindings.
    pub fn register_adapter(
        &mut self,
        adapter: Arc<dyn EnvironmentAdapter>,
    ) -> Result<Vec<CapabilityBindingId>, ExecutionContractError> {
        let descriptor = adapter.descriptor();
        descriptor.validate()?;
        if self.adapters.contains_key(&descriptor.adapter) {
            return Err(ExecutionContractError::DuplicateAdapter {
                adapter: descriptor.adapter.clone(),
            });
        }
        let mut created = Vec::new();
        for provided in &descriptor.provides {
            let record = CapabilityBinding::from_parts(
                &descriptor.adapter,
                descriptor.environment,
                descriptor.class,
                provided,
            );
            record.validate()?;
            let binding_id = record.binding.clone();
            self.bindings.insert(
                binding_id.clone(),
                RegisteredBinding {
                    binding: record,
                    readiness: ReadinessTracker::new(),
                    authorization: None,
                    attached: Vec::new(),
                },
            );
            created.push(binding_id);
        }
        self.adapters.insert(descriptor.adapter.clone(), adapter);
        Ok(created)
    }

    /// Probes every adapter and drives its bindings through the readiness
    /// state machine.
    ///
    /// A `READY` probe advances `DECLARED`/`UNAVAILABLE` bindings to
    /// `AVAILABLE` and then `READY` (or recovers `FAILED` bindings). Bindings
    /// already at `READY` or beyond are not re-probed. Probe failures move
    /// bindings to `FAILED` or `UNAVAILABLE` with diagnostics, so an
    /// unavailable required capability is diagnosable after a refresh.
    pub async fn refresh_readiness(&mut self) -> Result<(), ExecutionContractError> {
        let adapters: Vec<Arc<dyn EnvironmentAdapter>> = self.adapters.values().cloned().collect();
        for adapter in adapters {
            let descriptor = adapter.descriptor().clone();
            let binding_ids: Vec<CapabilityBindingId> = self
                .bindings
                .values()
                .filter(|registered| registered.binding.adapter == descriptor.adapter)
                .map(|registered| registered.binding.binding.clone())
                .collect();
            let probe_outcome = adapter.probe().await;
            for binding_id in binding_ids {
                let registered = self.lookup_mut(&binding_id)?;
                let state = registered.readiness.state();
                if state.is_selectable() {
                    continue;
                }
                let environment = registered.binding.environment;
                match &probe_outcome {
                    Ok(report) => {
                        report.validate()?;
                        match report.status {
                            ProbeStatus::Ready => {
                                if state == ReadinessState::Declared {
                                    registered.readiness.advance(TransitionCause::Registered)?;
                                }
                                if state == ReadinessState::Unavailable {
                                    registered
                                        .readiness
                                        .advance(TransitionCause::Rediscovered)?;
                                }
                                registered
                                    .readiness
                                    .advance(TransitionCause::ProbeSucceeded)?;
                            }
                            ProbeStatus::Available => {
                                if state == ReadinessState::Declared {
                                    registered.readiness.advance(TransitionCause::Registered)?;
                                }
                                if matches!(
                                    state,
                                    ReadinessState::Failed | ReadinessState::Unavailable
                                ) {
                                    registered
                                        .readiness
                                        .advance(TransitionCause::Rediscovered)?;
                                }
                            }
                            ProbeStatus::NotInstalled => {
                                let diagnostic = report
                                    .diagnostic
                                    .clone()
                                    .unwrap_or_else(|| not_installed(&descriptor.adapter));
                                registered
                                    .readiness
                                    .advance(TransitionCause::CapabilityNotInstalled(diagnostic))?;
                            }
                        }
                    }
                    Err(failure) => {
                        let diagnostic = probe_error_diagnostic(environment, failure);
                        registered
                            .readiness
                            .advance(TransitionCause::ProbeError(diagnostic))?;
                    }
                }
            }
        }
        Ok(())
    }

    /// Applies a readiness transition cause to a binding.
    ///
    /// This is the generic, validated gate for state machine movement,
    /// including recovery (`Recovered`), environment loss
    /// (`EnvironmentLost`), and authorization
    /// (`Authorized`). The tracker rejects illegal transitions and
    /// diagnostic-less failures.
    pub fn advance(
        &mut self,
        binding: &CapabilityBindingId,
        cause: TransitionCause,
    ) -> Result<ReadinessState, ExecutionContractError> {
        let registered = self.lookup_mut(binding)?;
        if let TransitionCause::Authorized(grant) = &cause
            && grant.binding != *binding
        {
            return Err(ExecutionContractError::AuthorizationSubjectMismatch {
                granted: grant.binding.clone(),
                requested: binding.clone(),
            });
        }
        let state = registered.readiness.advance(cause.clone())?;
        if let TransitionCause::Authorized(grant) = cause {
            registered.authorization = Some(grant);
        }
        Ok(state)
    }

    /// Binds a resource type to a concrete resource instance.
    ///
    /// Rebinding a type replaces the active binding; bindings that already
    /// attached the previous instance keep their attached copy until they
    /// rebind. Durable rebinding semantics remain control-plane authority.
    pub fn bind_resource(
        &mut self,
        resource: ResourceBinding,
    ) -> Result<(), ExecutionContractError> {
        resource.validate()?;
        self.resources
            .insert(resource.resource_type.clone(), resource);
        Ok(())
    }

    /// Attaches the binding's required resources, moving it from
    /// `AUTHORIZED` to `BOUND`.
    ///
    /// Fails explicitly when a required resource type has no bound
    /// resource.
    pub fn bind(
        &mut self,
        binding: &CapabilityBindingId,
    ) -> Result<Vec<ResourceBinding>, ExecutionContractError> {
        let required = self.lookup(binding)?.binding.resources.clone();
        let mut attached = Vec::new();
        let mut missing = Vec::new();
        for resource_type in required {
            match self.resources.get(&resource_type) {
                Some(resource) => attached.push(resource.clone()),
                None => missing.push(resource_type),
            }
        }
        if !missing.is_empty() {
            return Err(ExecutionContractError::ResourceRequirementMissing {
                binding: binding.clone(),
                missing,
            });
        }
        let registered = self.lookup_mut(binding)?;
        registered
            .readiness
            .advance(TransitionCause::ResourcesAttached)?;
        registered.attached = attached.clone();
        Ok(attached)
    }

    /// Resolves one capability requirement to a selected binding under
    /// policy.
    ///
    /// Candidates are ordered by the frozen fallback chain (Codex-native
    /// first, compatible alternates second, human last), then by readiness
    /// (prefer bindings that already passed the gates), then by adapter
    /// identity. The first candidate that satisfies policy, readiness, and
    /// resource requirements is selected. When a preferred candidate was
    /// skipped, the decision carries a fallback record with the structured
    /// reason, and [`BindingDecision::to_evidence_reference`] produces the
    /// evidence the control plane must record.
    ///
    /// Unavailable required capabilities fail explicitly with a
    /// diagnostic naming the capability, the skipped candidates, and the
    /// reason.
    pub fn resolve(
        &self,
        requirement: &CapabilityRequirement,
        policy: &BindingPolicy,
    ) -> Result<BindingDecision, BindingDiagnostic> {
        if let Err(error) = policy.validate() {
            return Err(BindingDiagnostic::new(
                DiagnosticCode::PolicyDenied,
                error.to_string(),
            ));
        }
        let mut candidates: Vec<&RegisteredBinding> = self
            .bindings
            .values()
            .filter(|registered| registered.binding.capability == requirement.capability)
            .collect();
        if candidates.is_empty() {
            return Err(BindingDiagnostic::new(
                DiagnosticCode::NotRegistered,
                format!(
                    "no adapter provides capability `{}`",
                    requirement.capability.as_ref()
                ),
            ));
        }
        candidates.sort_by(|left, right| compare_candidates(left, right));

        let mut best_skipped: Option<(SelectedBinding, FallbackReason)> = None;
        for candidate in candidates {
            let binding = &candidate.binding;
            if !policy.allows_environment(binding.environment) {
                record_skipped(
                    &mut best_skipped,
                    candidate,
                    FallbackReason::PreferredOutsideEnvironmentScope,
                );
                continue;
            }
            if !policy.allows_class(binding.class) {
                record_skipped(
                    &mut best_skipped,
                    candidate,
                    FallbackReason::PreferredHumanFallbackForbidden,
                );
                continue;
            }
            let state = candidate.readiness.state();
            if !state.is_selectable() {
                record_skipped(
                    &mut best_skipped,
                    candidate,
                    FallbackReason::PreferredNotReady {
                        state,
                        diagnostic: candidate.readiness.diagnostic().cloned(),
                    },
                );
                continue;
            }
            let missing = self.missing_resources(binding);
            if !missing.is_empty() {
                record_skipped(
                    &mut best_skipped,
                    candidate,
                    FallbackReason::PreferredMissingResources { missing },
                );
                continue;
            }
            return Ok(BindingDecision {
                requirement: requirement.clone(),
                selected: selected_of(candidate),
                fallback: best_skipped.map(|(skipped, reason)| FallbackRecord { skipped, reason }),
            });
        }
        Err(skipped_diagnostic(&requirement.capability, best_skipped))
    }

    /// Plans the binding of every step of a workflow IR under policy.
    ///
    /// Each step node contributes one [`StepBinding`] with one decision per
    /// declared capability requirement, in declaration order. Because
    /// requirements bind independently, one plan routinely routes different
    /// steps — or different capabilities of a single step — to different
    /// environment classes: this is mixed-environment execution within one
    /// graph/run.
    ///
    /// The first resolution failure aborts the plan with its diagnostic,
    /// so an unavailable required capability fails the plan explicitly
    /// instead of producing a partially bound execution.
    pub fn plan(
        &self,
        ir: &WorkflowIr,
        policy: &BindingPolicy,
    ) -> Result<BindingPlan, BindingDiagnostic> {
        let mut steps = Vec::new();
        for (node_id, node) in &ir.nodes {
            if let WorkflowIrNode::Step(step) = node {
                let mut decisions = Vec::new();
                for requirement in &step.capabilities {
                    decisions.push(self.resolve(requirement, policy)?);
                }
                steps.push(StepBinding {
                    node: node_id.clone(),
                    decisions,
                });
            }
        }
        Ok(BindingPlan { steps })
    }

    /// Diagnoses why a capability cannot currently be bound.
    ///
    /// Returns one diagnostic per known obstacle: missing bindings,
    /// failed or unavailable bindings (with their diagnostics), unprobed
    /// bindings, and unbound resource requirements.
    pub fn diagnose(&self, capability: &CapabilityId) -> Vec<BindingDiagnostic> {
        let matching: Vec<&RegisteredBinding> = self
            .bindings
            .values()
            .filter(|registered| registered.binding.capability == *capability)
            .collect();
        if matching.is_empty() {
            return vec![BindingDiagnostic::new(
                DiagnosticCode::NotRegistered,
                format!("no adapter provides capability `{}`", capability.as_ref()),
            )];
        }
        let mut diagnostics = Vec::new();
        for registered in matching {
            let state = registered.readiness.state();
            if state.is_diagnostic() {
                let diagnostic = registered
                    .readiness
                    .diagnostic()
                    .cloned()
                    .unwrap_or_else(|| {
                        BindingDiagnostic::new(
                            DiagnosticCode::NotInstalled,
                            format!(
                                "binding `{}` is {state} without a diagnostic",
                                registered.binding.binding
                            ),
                        )
                    });
                diagnostics.push(diagnostic);
            } else if !state.is_selectable() {
                diagnostics.push(
                    BindingDiagnostic::new(
                        DiagnosticCode::NotInstalled,
                        format!(
                            "binding `{}` is {state}; probe it to move it to READY",
                            registered.binding.binding
                        ),
                    )
                    .in_environment(registered.binding.environment),
                );
            }
            let missing = self.missing_resources(&registered.binding);
            if !missing.is_empty() {
                diagnostics.push(
                    BindingDiagnostic::new(
                        DiagnosticCode::ResourceMissing,
                        format!(
                            "binding `{}` requires unbound resources: {}",
                            registered.binding.binding,
                            missing
                                .iter()
                                .map(AsRef::as_ref)
                                .collect::<Vec<_>>()
                                .join(", ")
                        ),
                    )
                    .in_environment(registered.binding.environment),
                );
            }
        }
        diagnostics
    }

    /// Dispatches one action to a bound binding.
    ///
    /// Dispatch prepares the adapter session for the binding's attached
    /// resources, moves the binding to `EXECUTING`, executes exactly one
    /// action, and records the outcome transition (`BOUND` on success,
    /// `FAILED` with a diagnostic on failure). This is single-action
    /// execution: graph traversal, sequencing, and durable state remain
    /// with the control plane.
    ///
    /// Contract violations (unknown binding, unbound binding, invalid
    /// action) return [`ExecutionContractError`]; action failures return a
    /// failed [`ActionResult`] so failure handling flows through the
    /// normalized recovery pipeline.
    pub async fn dispatch(
        &mut self,
        binding: &CapabilityBindingId,
        action: Action,
    ) -> Result<ActionResult, ExecutionContractError> {
        action.validate()?;
        let (adapter_id, capability, attached, state, environment) = {
            let registered = self.lookup(binding)?;
            (
                registered.binding.adapter.clone(),
                registered.binding.capability.clone(),
                registered.attached.clone(),
                registered.readiness.state(),
                registered.binding.environment,
            )
        };
        if !matches!(state, ReadinessState::Bound | ReadinessState::Executing) {
            return Err(ExecutionContractError::BindingNotReady {
                binding: binding.clone(),
                state,
                expected: ReadinessState::Bound,
            });
        }
        let adapter = self.adapters.get(&adapter_id).cloned().ok_or_else(|| {
            ExecutionContractError::InvalidRecord {
                reason: format!("adapter `{adapter_id}` of binding `{binding}` is not registered"),
            }
        })?;
        let request = PrepareRequest {
            binding: binding.clone(),
            capability,
            resources: attached,
        };
        let prepared = match adapter.prepare(request).await {
            Ok(prepared) => prepared,
            Err(failure) => {
                let diagnostic = preparation_diagnostic(environment, &failure);
                self.advance(binding, TransitionCause::PreparationFailed(diagnostic))?;
                return Ok(ActionResult::failed(binding.clone(), failure));
            }
        };
        self.advance(binding, TransitionCause::ActionDispatched)?;
        let result = adapter.execute(&prepared.session, action).await;
        result.validate()?;
        match result.outcome {
            ActionOutcome::Succeeded => {
                self.advance(binding, TransitionCause::ActionCompleted)?;
            }
            ActionOutcome::Failed => {
                let failure = result.failure.as_ref().ok_or_else(|| {
                    ExecutionContractError::InvalidRecord {
                        reason: format!(
                            "failed result from binding `{binding}` has no failure record"
                        ),
                    }
                })?;
                let diagnostic = BindingDiagnostic::new(
                    DiagnosticCode::ExecutionFailure,
                    failure.message.clone(),
                )
                .in_environment(environment);
                self.advance(binding, TransitionCause::ActionFailed(diagnostic))?;
            }
        }
        Ok(result)
    }

    /// Looks up one registered binding.
    pub fn binding(
        &self,
        binding: &CapabilityBindingId,
    ) -> Result<&RegisteredBinding, ExecutionContractError> {
        self.lookup(binding)
    }

    /// Iterates every registered binding in binding-id order.
    pub fn bindings(&self) -> impl Iterator<Item = &RegisteredBinding> {
        self.bindings.values()
    }

    /// The resource binding active for one resource type, when bound.
    pub fn resource_binding(&self, resource_type: &ResourceTypeId) -> Option<&ResourceBinding> {
        self.resources.get(resource_type)
    }

    fn lookup(
        &self,
        binding: &CapabilityBindingId,
    ) -> Result<&RegisteredBinding, ExecutionContractError> {
        self.bindings
            .get(binding)
            .ok_or_else(|| ExecutionContractError::UnknownBinding {
                binding: binding.clone(),
            })
    }

    fn lookup_mut(
        &mut self,
        binding: &CapabilityBindingId,
    ) -> Result<&mut RegisteredBinding, ExecutionContractError> {
        self.bindings
            .get_mut(binding)
            .ok_or_else(|| ExecutionContractError::UnknownBinding {
                binding: binding.clone(),
            })
    }

    fn missing_resources(&self, binding: &CapabilityBinding) -> Vec<ResourceTypeId> {
        binding
            .resources
            .iter()
            .filter(|resource_type| !self.resources.contains_key(*resource_type))
            .cloned()
            .collect()
    }
}

#[cfg(test)]
#[path = "registry_tests.rs"]
mod tests;
