//! The workflow application lifecycle service.
//!
//! [`WorkflowLifecycle`] composes the frozen planes behind control-plane
//! seams: it selects immutable versions from the [`WorkflowVersionStore`],
//! instantiates them (validate -> approve -> bind) through the capability
//! registry, runs them across environment classes ([`crate::run`]),
//! cancels them through the control plane ([`WorkflowLifecycle::cancel`]),
//! and reconciles the durable records afterwards
//! ([`WorkflowLifecycle::verify_run`]). It owns no durable state: every
//! record crosses a [`crate::port`] seam.
//!
//! Every instance status mutation routes through one seam,
//! `WorkflowLifecycle::transition_status`, which validates the
//! transition against the frozen legal-transition table the workflow
//! contracts declare; a status is never a bare field assignment here.
//!
//! Instantiation enforces the frozen gate order:
//!
//! ```text
//! validate: version integrity + adapter readiness + binding plan
//! approve:  one AuthorizationGrant per selected binding, resting on
//!           recorded approval evidence (Codex approvals stay the
//!           authority)
//! bind:     resource bindings attached, binding AUTHORIZED -> BOUND
//! ```
//!
//! A binding never reaches dispatch without passing every gate in order.

use std::collections::BTreeMap;

use codex_execution_contracts::AuthorizationGrant;
use codex_execution_contracts::BindingPlan;
use codex_execution_contracts::BindingPolicy;
use codex_execution_contracts::CapabilityBindingId;
use codex_execution_contracts::CapabilityRegistry;
use codex_execution_contracts::ExecutionEnvironment;
use codex_execution_contracts::ResourceBinding;
use codex_execution_contracts::TransitionCause;
use codex_workflow_contracts::CapabilityId;
use codex_workflow_contracts::EvidenceKind;
use codex_workflow_contracts::EvidenceReference;
use codex_workflow_contracts::TriggerSource;
use codex_workflow_contracts::WorkflowDefinitionId;
use codex_workflow_contracts::WorkflowInstance;
use codex_workflow_contracts::WorkflowInstanceId;
use codex_workflow_contracts::WorkflowInstanceStatus;
use codex_workflow_contracts::WorkflowVersion;
use codex_workflow_contracts::WorkflowVersionId;

use crate::WorkflowAppError;
use crate::WorkflowEvent;
use crate::approval::ApprovalRequest;
use crate::port::ApprovalSource;
use crate::port::EventSink;
use crate::port::EvidenceStore;
use crate::port::RunPositionStore;
use crate::port::StepActionSource;
use crate::port::WorkflowInstanceStore;
use crate::port::WorkflowVersionStore;
use crate::run::ActiveRun;
use crate::walk::Walk;
use crate::walk::WalkConfig;

/// The seams and registry a lifecycle is constructed from.
pub struct LifecycleDeps {
    /// Durable workflow version records.
    pub versions: Box<dyn WorkflowVersionStore>,
    /// Durable workflow instance records.
    pub instances: Box<dyn WorkflowInstanceStore>,
    /// The evidence plane.
    pub evidence: Box<dyn EvidenceStore>,
    /// The approval plane.
    pub approvals: Box<dyn ApprovalSource>,
    /// The action proposal seam (the Codex agent/model loop).
    pub actions: Box<dyn StepActionSource>,
    /// The lifecycle event observer.
    pub events: Box<dyn EventSink>,
    /// The capability registry with the host's environment adapters
    /// registered.
    pub registry: CapabilityRegistry,
}

/// A request to instantiate the selected workflow version.
#[derive(Clone, Debug, Default)]
pub struct InstantiateRequest {
    /// The accepted trigger that starts the instance, when recorded.
    ///
    /// External trigger payloads are untrusted input; only this
    /// normalized control-plane source is recorded.
    pub trigger: Option<TriggerSource>,
    /// The binding policy applied during capability resolution.
    ///
    /// Defaults to the conservative execution-contracts policy (all peer
    /// environments in scope, human fallback forbidden).
    pub policy: BindingPolicy,
    /// The concrete resource bindings to attach.
    ///
    /// Resource bindings are opaque, credential-free instance identities;
    /// the adapters resolve them to secret material internally.
    pub resources: Vec<ResourceBinding>,
    /// The walk budget for the run.
    pub walk: WalkConfig,
}

/// The result of reconciling a finished run against the durable records.
#[derive(Clone, Debug)]
pub struct VerifiedRun {
    /// The durable instance record as stored.
    pub instance: WorkflowInstance,
    /// The workflow the instance executed.
    pub workflow: WorkflowDefinitionId,
    /// The immutable version the instance pinned.
    pub version: WorkflowVersionId,
    /// The evidence references recorded on the instance.
    pub evidence: Vec<EvidenceReference>,
}

/// The application-facing workflow lifecycle service.
pub struct WorkflowLifecycle {
    pub(crate) versions: Box<dyn WorkflowVersionStore>,
    pub(crate) instances: Box<dyn WorkflowInstanceStore>,
    pub(crate) evidence: Box<dyn EvidenceStore>,
    pub(crate) approvals: Box<dyn ApprovalSource>,
    pub(crate) actions: Box<dyn StepActionSource>,
    pub(crate) events: Box<dyn EventSink>,
    pub(crate) registry: CapabilityRegistry,
    pub(crate) selected: Option<WorkflowVersion>,
    pub(crate) active: Option<ActiveRun>,
    /// The persisted-run-position seam, when the host attached one
    /// ([`WorkflowLifecycle::with_positions`]); `None` keeps the
    /// pre-MWO-003 behavior exactly.
    pub(crate) positions: Option<Box<dyn RunPositionStore>>,
}

impl WorkflowLifecycle {
    /// Creates the lifecycle from its seams and registry.
    ///
    /// Nothing is probed or executed at construction: adapters register
    /// lazily-probing descriptors, and no workflow is active until
    /// [`Self::select_version`] succeeds. This is the ordinary-Codex
    /// no-op default path.
    pub fn new(deps: LifecycleDeps) -> Self {
        Self {
            versions: deps.versions,
            instances: deps.instances,
            evidence: deps.evidence,
            approvals: deps.approvals,
            actions: deps.actions,
            events: deps.events,
            registry: deps.registry,
            selected: None,
            active: None,
            positions: None,
        }
    }

    /// Whether any workflow state is held (a selected version or an
    /// active run).
    ///
    /// When this is `false`, no workflow code path has executed anything:
    /// ordinary Codex behavior is untouched.
    pub fn is_active(&self) -> bool {
        self.selected.is_some() || self.active.is_some()
    }

    /// The capability registry backing this lifecycle.
    pub fn registry(&self) -> &CapabilityRegistry {
        &self.registry
    }

    /// Selects an immutable workflow version and verifies its integrity.
    ///
    /// The version is loaded from the store and must pass
    /// [`WorkflowVersion::verify_integrity`]: a tampered record never
    /// becomes executable. Selection alone instantiates nothing and
    /// touches no adapter.
    pub fn select_version(
        &mut self,
        version: &WorkflowVersionId,
    ) -> Result<WorkflowDefinitionId, WorkflowAppError> {
        let record =
            self.versions
                .load(version)?
                .ok_or_else(|| WorkflowAppError::VersionUnavailable {
                    version: version.to_string(),
                })?;
        record
            .verify_integrity()
            .map_err(|error| WorkflowAppError::VersionIntegrity {
                version: version.to_string(),
                reason: error.to_string(),
            })?;
        let workflow = record.definition.id.clone();
        self.selected = Some(record);
        self.events.record(WorkflowEvent::VersionSelected {
            version: version.clone(),
        });
        Ok(workflow)
    }

    /// Instantiates the selected version: validate -> approve -> bind.
    ///
    /// On any phase failure the durable instance record is settled as
    /// `Failed` with the structured reason before the original error is
    /// returned, so approval denials and binding failures are observable
    /// after the fact.
    pub async fn instantiate(
        &mut self,
        request: InstantiateRequest,
    ) -> Result<WorkflowInstance, WorkflowAppError> {
        let version = self
            .selected
            .clone()
            .ok_or(WorkflowAppError::NoActiveWorkflow)?;
        let instance_id = WorkflowInstanceId::generate();
        let mut instance = WorkflowInstance::new(
            instance_id,
            version.definition.id.clone(),
            version.version_id.clone(),
            WorkflowInstanceStatus::Pending,
        );
        instance.trigger = request.trigger;
        self.instances.create(instance.clone())?;
        self.events.record(WorkflowEvent::InstanceCreated {
            instance: instance_id,
            workflow: version.definition.id.clone(),
            version: version.version_id.clone(),
            status: WorkflowInstanceStatus::Pending,
        });

        // Phase 1: validate — integrity, readiness, resources, plan.
        version
            .verify_integrity()
            .map_err(|error| self.fail_instance(&mut instance, error.into()))?;
        self.registry
            .refresh_readiness()
            .await
            .map_err(|error| self.fail_instance(&mut instance, error.into()))?;
        for resource in &request.resources {
            self.registry
                .bind_resource(resource.clone())
                .map_err(|error| self.fail_instance(&mut instance, error.into()))?;
        }
        let plan = self
            .registry
            .plan(&version.definition.ir, &request.policy)
            .map_err(|error| {
                self.fail_instance(
                    &mut instance,
                    WorkflowAppError::BindingPlanFailed {
                        message: error.message,
                        code: error.code,
                    },
                )
            })?;
        let selected = distinct_bindings(&plan);

        // Phase 2: approve — one grant per distinct selected binding,
        // each resting on recorded approval evidence.
        for (binding, environment, capability) in &selected {
            let approval = self
                .authorize_binding(&instance_id, binding, *environment, capability.clone())
                .map_err(|error| self.fail_instance(&mut instance, error))?;
            instance.record_evidence(approval);
        }

        // Phase 3: bind — attach each binding's required resources
        // (AUTHORIZED -> BOUND).
        for (binding, _, _) in selected {
            let attached = self
                .registry
                .bind(&binding)
                .map_err(|error| self.fail_instance(&mut instance, error.into()))?;
            self.events.record(WorkflowEvent::ResourcesBound {
                instance: instance_id,
                binding,
                resources: attached
                    .into_iter()
                    .map(|resource| resource.resource_type)
                    .collect(),
            });
        }

        let walk = Walk::new(&version.definition.ir)?;
        let decisions = plan
            .steps
            .iter()
            .map(|step| (step.node.clone(), step.decisions.clone()))
            .collect::<BTreeMap<_, _>>();
        // The one status-mutation seam: the start transition
        // (Pending -> Running) is validated against the frozen contract
        // table before the record is persisted and the run is activated.
        self.transition_status(&mut instance, WorkflowInstanceStatus::Running)?;
        self.instances.save(instance.clone())?;
        self.events.record(WorkflowEvent::InstanceStarted {
            instance: instance_id,
        });
        let settled = instance.clone();
        let run = ActiveRun {
            version,
            instance: instance.clone(),
            policy: request.policy,
            walk,
            walk_config: request.walk,
            decisions,
            resources: request.resources,
        };
        // Checkpoint: the run is resumable from the moment it starts, so
        // a crash between instantiation and the first step completion
        // still reconciles and resumes. A checkpoint-seam failure
        // settles the record `Failed`, exactly like the run path's
        // internal seam failures — never a phantom `Running` record.
        if let Err(error) = self.persist_position(&run) {
            return Err(self.fail_instance(&mut instance, error));
        }
        self.active = Some(run);
        Ok(settled)
    }

    /// Reconciles a run's durable records: the instance is loaded from the
    /// store, its pinned version is loaded from the version store, and the
    /// version must still verify end to end.
    pub fn verify_run(
        &self,
        instance_id: &WorkflowInstanceId,
    ) -> Result<VerifiedRun, WorkflowAppError> {
        let instance = self.instances.load(instance_id)?.ok_or_else(|| {
            WorkflowAppError::InstanceUnavailable {
                instance: instance_id.to_string(),
            }
        })?;
        let version = self.versions.load(&instance.version)?.ok_or_else(|| {
            WorkflowAppError::VersionUnavailable {
                version: instance.version.to_string(),
            }
        })?;
        version
            .verify_integrity()
            .map_err(|error| WorkflowAppError::VersionIntegrity {
                version: instance.version.to_string(),
                reason: error.to_string(),
            })?;
        Ok(VerifiedRun {
            workflow: instance.workflow.clone(),
            version: instance.version.clone(),
            evidence: instance.evidence.clone(),
            instance,
        })
    }

    /// Cancels an instance through the control plane: `Pending`, `Running`,
    /// or `Paused` transitions to `Cancelled`.
    ///
    /// Cancellation is an operator-visible takeover: the reason is
    /// recorded as evidence through the evidence plane (the trace of the
    /// control-plane operation, following the existing evidence
    /// conventions), a [`WorkflowEvent::InstanceCancelled`] event is
    /// emitted, and the updated record persists through the normal
    /// instance-store path. An in-flight run state held for the cancelled
    /// instance is dropped, so a later [`Self::run`] is an explicit
    /// no-op instead of settling the now-terminal record.
    ///
    /// Double-cancel and cancel of an already-settled instance are
    /// explicit [`WorkflowAppError::IllegalStatusTransition`] errors,
    /// never silent no-ops, and are refused before any durable side
    /// effect is recorded. Cancellation never mutates workflow
    /// semantics: the version pin, integrity digests, and dependency
    /// identities are untouched.
    pub fn cancel(
        &mut self,
        instance: &WorkflowInstanceId,
        reason: impl Into<String>,
    ) -> Result<WorkflowInstance, WorkflowAppError> {
        let mut record = self.instances.load(instance)?.ok_or_else(|| {
            WorkflowAppError::InstanceUnavailable {
                instance: instance.to_string(),
            }
        })?;
        // Fail fast: the frozen legal table refuses the transition before
        // any evidence, event, or store write happens.
        self.transition_status(&mut record, WorkflowInstanceStatus::Cancelled)?;
        // Take over the in-flight run state, when one is held for this
        // instance: no later run may settle the terminal record.
        if self
            .active
            .as_ref()
            .is_some_and(|run| run.instance.instance_id == *instance)
        {
            self.active = None;
        }
        // The persisted position of the cancelled instance goes with the
        // in-flight run: a terminal record must never look resumable.
        self.discard_position(instance)?;
        let reason = reason.into();
        // The operator-visible reason is recorded through the evidence
        // plane as the trace of the control-plane cancellation.
        let payload = serde_json::json!({
            "instance": instance.to_string(),
            "operation": "cancel",
            "reason": &reason,
        });
        self.store_evidence(&mut record, EvidenceKind::Trace, &payload)?;
        self.events.record(WorkflowEvent::InstanceCancelled {
            instance: *instance,
            reason,
        });
        Ok(record)
    }

    /// Transitions the instance's status through the frozen legal table.
    ///
    /// This is the application layer's one status-mutation seam: every
    /// control-plane transition of a workflow instance (the instantiation
    /// start, the run settlement, the failure settlement, and
    /// cancellation) routes through here and is validated against
    /// [`WorkflowInstanceStatus::legal_transitions`] before the record is
    /// touched. A transition the table forbids returns
    /// [`WorkflowAppError::IllegalStatusTransition`] naming the instance
    /// and both statuses: never a silent no-op, never a bare field
    /// assignment. The seam mutates the in-memory record only; each
    /// caller persists through the normal instance-store save path.
    pub(crate) fn transition_status(
        &mut self,
        instance: &mut WorkflowInstance,
        target: WorkflowInstanceStatus,
    ) -> Result<(), WorkflowAppError> {
        if !instance.status.can_transition_to(target) {
            return Err(WorkflowAppError::IllegalStatusTransition {
                instance: instance.instance_id.to_string(),
                current: instance.status,
                target,
            });
        }
        instance.status = target;
        Ok(())
    }

    /// Records one evidence payload through the evidence plane and appends
    /// the reference to the in-flight instance.
    pub(crate) fn store_evidence(
        &mut self,
        instance: &mut WorkflowInstance,
        kind: EvidenceKind,
        payload: &serde_json::Value,
    ) -> Result<EvidenceReference, WorkflowAppError> {
        let reference = self.evidence.store(kind, payload)?;
        instance.record_evidence(reference.clone());
        self.instances.save(instance.clone())?;
        Ok(reference)
    }

    /// Authorizes one binding through the approval plane.
    ///
    /// The grant rests on recorded approval evidence of kind `Approval`;
    /// the registry enforces the `READY -> AUTHORIZED` transition.
    pub(crate) fn authorize_binding(
        &mut self,
        instance: &WorkflowInstanceId,
        binding: &CapabilityBindingId,
        environment: ExecutionEnvironment,
        capability: CapabilityId,
    ) -> Result<EvidenceReference, WorkflowAppError> {
        let request = ApprovalRequest {
            instance: *instance,
            binding: binding.clone(),
            environment,
            capability,
        };
        let approval = self.approvals.approve(&request)?;
        let grant = AuthorizationGrant::new(binding.clone(), approval.clone())?;
        self.registry
            .advance(binding, TransitionCause::Authorized(grant))?;
        self.events.record(WorkflowEvent::BindingAuthorized {
            instance: *instance,
            binding: binding.clone(),
            environment,
        });
        Ok(approval)
    }

    /// Settles a failed instantiation: the durable instance record becomes
    /// `Failed` through the status-mutation seam, a `RunFailed` event is
    /// emitted, and the original error is returned for the caller.
    ///
    /// A settlement the frozen legal table refuses (an already-terminal
    /// record) surfaces the refusal instead of silently dropping it.
    fn fail_instance(
        &mut self,
        instance: &mut WorkflowInstance,
        error: WorkflowAppError,
    ) -> WorkflowAppError {
        if let Err(refused) = self.transition_status(instance, WorkflowInstanceStatus::Failed) {
            return refused;
        }
        let save = self.instances.save(instance.clone());
        self.events.record(WorkflowEvent::RunFailed {
            instance: instance.instance_id,
            reason: error.to_string(),
        });
        match save {
            Ok(()) => error,
            Err(store_error) => store_error,
        }
    }
}

/// The distinct bindings selected by a plan, with their environment and
/// capability, ordered by binding identity for determinism.
fn distinct_bindings(
    plan: &BindingPlan,
) -> Vec<(CapabilityBindingId, ExecutionEnvironment, CapabilityId)> {
    let mut distinct: BTreeMap<CapabilityBindingId, (ExecutionEnvironment, CapabilityId)> =
        BTreeMap::new();
    for step in &plan.steps {
        for decision in &step.decisions {
            distinct
                .entry(decision.selected.binding.clone())
                .or_insert((
                    decision.selected.environment,
                    decision.requirement.capability.clone(),
                ));
        }
    }
    distinct
        .into_iter()
        .map(|(binding, (environment, capability))| (binding, environment, capability))
        .collect()
}

#[cfg(test)]
#[path = "lifecycle_tests.rs"]
mod tests;
