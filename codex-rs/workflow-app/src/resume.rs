//! Persisted run position, resume continuation, and startup
//! reconciliation (MWO-003).
//!
//! The run path keeps its control-plane position durable behind the
//! [`RunPositionStore`](crate::port::RunPositionStore) seam at the
//! documented checkpoints: instantiation (the run is resumable from
//! the moment it starts), every step completion (the position advances
//! past the settled step), every pause (the position holds the wait
//! node's pending re-entry point), and terminal settlement or
//! cancellation (the position is discarded — a settled record never
//! looks resumable).
//!
//! This module owns the three operations the M4 remediation mandated:
//!
//! - [`WorkflowLifecycle::with_positions`]: constructs the lifecycle
//!   with the position seam attached. A lifecycle constructed through
//!   [`WorkflowLifecycle::new`] keeps the pre-MWO-003 behavior exactly
//!   (no persistence, no rehydration) — that is the ordinary-Codex
//!   no-op default path.
//! - [`WorkflowLifecycle::resume_run`]: rehydrates a resumed instance's
//!   active run from the durable records and continues the walk from
//!   the persisted step through the ordinary
//!   [`WorkflowLifecycle::run`] path (the WO-011 report explicitly
//!   deferred this; resume marked `Paused -> Running` and nothing ran).
//! - [`WorkflowLifecycle::reconcile_startup`]: the startup sweep that
//!   finds `Running` instances with no live run — the post-crash
//!   signature — and transitions them through the MWO-002 seam to the
//!   documented recovery state: `Paused`, with recovery evidence,
//!   awaiting an explicit resume. Never silently `Running`, never
//!   auto-executing, and idempotent.

use std::collections::BTreeMap;

use codex_execution_contracts::BindingDecision;
use codex_execution_contracts::CapabilityBindingId;
use codex_execution_contracts::ExecutionEnvironment;
use codex_execution_contracts::ReadinessState;
use codex_workflow_contracts::CapabilityId;
use codex_workflow_contracts::EvidenceKind;
use codex_workflow_contracts::IrNodeId;
use codex_workflow_contracts::WorkflowInstance;
use codex_workflow_contracts::WorkflowInstanceId;
use codex_workflow_contracts::WorkflowInstanceStatus;

use crate::LifecycleDeps;
use crate::RunOutcome;
use crate::WorkflowAppError;
use crate::WorkflowEvent;
use crate::lifecycle::WorkflowLifecycle;
use crate::port::RunPosition;
use crate::port::RunPositionStore;
use crate::run::ActiveRun;
use crate::walk::Walk;
use crate::walk::WalkConfig;

/// The operator-visible reason recorded on every reconciliation.
const RECONCILE_REASON: &str =
    "instance was `Running` with no live run at startup; awaiting an explicit resume";

impl WorkflowLifecycle {
    /// Creates the lifecycle from its seams, registry, and the persisted
    /// run-position seam.
    ///
    /// Identical to [`WorkflowLifecycle::new`] except that the run path
    /// persists its position at the documented checkpoints, a resumed
    /// instance can be continued through [`Self::resume_run`], and the
    /// stores can be swept at startup through
    /// [`Self::reconcile_startup`]. A lifecycle constructed through
    /// `new` keeps the pre-MWO-003 behavior exactly, which is the
    /// ordinary-Codex no-op default path.
    pub fn with_positions(deps: LifecycleDeps, positions: Box<dyn RunPositionStore>) -> Self {
        let mut lifecycle = WorkflowLifecycle::new(deps);
        lifecycle.positions = Some(positions);
        lifecycle
    }

    /// Persists the active run's position, when a run-position store is
    /// attached.
    ///
    /// This is the checkpoint write the run path performs; a lifecycle
    /// without the seam performs none (the pre-MWO-003 behavior).
    pub(crate) fn persist_position(&mut self, active: &ActiveRun) -> Result<(), WorkflowAppError> {
        let Some(store) = self.positions.as_mut() else {
            return Ok(());
        };
        store.save(RunPosition {
            instance: active.instance.instance_id,
            version: active.instance.version.clone(),
            walk: active.walk.position(),
            decisions: active.decisions.clone(),
            policy: active.policy.clone(),
            max_steps: active.walk_config.max_steps,
            resources: active.resources.clone(),
        })
    }

    /// Discards the persisted run position of `instance`, when a
    /// run-position store is attached; discarding an instance with no
    /// position is an ordinary no-op.
    pub(crate) fn discard_position(
        &mut self,
        instance: &WorkflowInstanceId,
    ) -> Result<(), WorkflowAppError> {
        match self.positions.as_mut() {
            Some(store) => store.discard(instance),
            None => Ok(()),
        }
    }

    /// Continues a resumed instance's run from its persisted position.
    ///
    /// The instance must already have been resumed through the control
    /// plane — the explicit `Paused -> Running` transition the host's
    /// `InstanceControl` seam performs. This method never performs that
    /// transition itself: auto-resume without an explicit control-plane
    /// transition is forbidden, and a record that is not `Running` is
    /// refused with [`WorkflowAppError::InstanceNotRunning`].
    ///
    /// Rehydration composes the frozen planes and mutates no workflow
    /// semantics:
    ///
    /// - the instance record is loaded from the instance store and its
    ///   pinned version from the version store, re-verified end to end
    ///   (`WorkflowVersion::verify_integrity`) before it executes;
    /// - the persisted position must exist and must pin the same version
    ///   as the instance record ([`WorkflowAppError::RunPositionUnavailable`]
    ///   otherwise), and only ever describes control-plane state;
    /// - execution state is re-established exactly like the original
    ///   run established it: adapters are probed
    ///   (`refresh_readiness`), the persisted resource identities are
    ///   re-attached, and every distinct binding the persisted decisions
    ///   route through re-crosses the approval gate with a fresh grant
    ///   (a binding already `BOUND` in this process is left as is);
    /// - the walk is restored through [`Walk::resume`] and continues
    ///   through the ordinary [`Self::run`] path — the same
    ///   checkpoints, recovery pipeline, evidence, and settlement
    ///   transitions apply.
    pub async fn resume_run(
        &mut self,
        instance: &WorkflowInstanceId,
    ) -> Result<RunOutcome, WorkflowAppError> {
        // A run this lifecycle still holds cannot be replaced by a
        // rehydration: dropping it silently would strand a `Running`
        // record exactly like a crash.
        if let Some(active) = self.active.as_ref() {
            return Err(WorkflowAppError::ActiveRunHeld {
                instance: active.instance.instance_id.to_string(),
            });
        }
        // The durable instance record: the continuation OF the explicit
        // control-plane transition, never a way to perform one.
        let mut record = self.instances.load(instance)?.ok_or_else(|| {
            WorkflowAppError::InstanceUnavailable {
                instance: instance.to_string(),
            }
        })?;
        if record.status != WorkflowInstanceStatus::Running {
            return Err(WorkflowAppError::InstanceNotRunning {
                instance: instance.to_string(),
                current: record.status,
            });
        }
        let position = self.load_position(instance, &record)?;
        // The pinned version, re-verified end to end before it executes.
        let version = self.versions.load(&record.version)?.ok_or_else(|| {
            WorkflowAppError::VersionUnavailable {
                version: record.version.to_string(),
            }
        })?;
        version
            .verify_integrity()
            .map_err(|error| WorkflowAppError::VersionIntegrity {
                version: record.version.to_string(),
                reason: error.to_string(),
            })?;
        self.reestablish_execution(&mut record, &position).await?;
        let walk = Walk::resume(&version.definition.ir, position.walk.clone())?;
        self.events.record(WorkflowEvent::RunResumed {
            instance: *instance,
            node: position.walk.current.clone(),
        });
        self.active = Some(ActiveRun {
            version,
            instance: record,
            policy: position.policy,
            walk,
            walk_config: WalkConfig {
                max_steps: position.max_steps,
            },
            decisions: position.decisions,
            resources: position.resources,
        });
        self.run().await
    }

    /// The startup reconciliation sweep for post-crash recovery.
    ///
    /// The caller enumerates the instance store (both the in-memory
    /// double and the durable store expose a `records()` audit handle)
    /// and hands the records to the sweep. Every record in `Running`
    /// with no live run in this lifecycle — the post-crash signature —
    /// is transitioned through the one status-mutation seam to the
    /// documented recovery state: `Paused`, with the recovery recorded
    /// as trace evidence and an
    /// [`WorkflowEvent::InstanceReconciled`] event, awaiting an
    /// explicit resume. Never silently `Running`, never auto-executing.
    ///
    /// The sweep is idempotent: a second sweep finds the reconciled
    /// records `Paused` and is a no-op. Records the store no longer
    /// reports as `Running`, and a run this lifecycle still holds (a
    /// live run, not an orphan), are never reconciled.
    pub fn reconcile_startup(
        &mut self,
        records: &[WorkflowInstance],
    ) -> Result<Vec<WorkflowInstanceId>, WorkflowAppError> {
        let mut reconciled = Vec::new();
        for candidate in records
            .iter()
            .filter(|candidate| candidate.status == WorkflowInstanceStatus::Running)
        {
            // The store is authoritative: re-load each `Running`
            // candidate through the port before touching it.
            let Some(mut recovered) = self.instances.load(&candidate.instance_id)? else {
                continue;
            };
            if recovered.status != WorkflowInstanceStatus::Running {
                continue;
            }
            // A run this lifecycle still holds is live, not orphaned.
            if self
                .active
                .as_ref()
                .is_some_and(|run| run.instance.instance_id == recovered.instance_id)
            {
                continue;
            }
            // The one status-mutation seam: Running -> Paused (the
            // documented recovery state, awaiting an explicit resume).
            self.transition_status(&mut recovered, WorkflowInstanceStatus::Paused)?;
            let payload = serde_json::json!({
                "instance": recovered.instance_id.to_string(),
                "operation": "reconcile-startup",
                "reason": RECONCILE_REASON,
            });
            self.store_evidence(&mut recovered, EvidenceKind::Trace, &payload)?;
            self.events.record(WorkflowEvent::InstanceReconciled {
                instance: recovered.instance_id,
                reason: RECONCILE_REASON.to_string(),
            });
            reconciled.push(recovered.instance_id);
        }
        Ok(reconciled)
    }

    /// Loads the persisted position for a resumed instance, verifying the
    /// position is present and pins the instance record's version.
    fn load_position(
        &self,
        instance: &WorkflowInstanceId,
        record: &WorkflowInstance,
    ) -> Result<RunPosition, WorkflowAppError> {
        let store =
            self.positions
                .as_ref()
                .ok_or_else(|| WorkflowAppError::RunPositionUnavailable {
                    instance: instance.to_string(),
                    reason: "the lifecycle has no run-position store attached".to_string(),
                })?;
        let position =
            store
                .load(instance)?
                .ok_or_else(|| WorkflowAppError::RunPositionUnavailable {
                    instance: instance.to_string(),
                    reason: "no run position is persisted for the instance".to_string(),
                })?;
        if position.version != record.version {
            return Err(WorkflowAppError::RunPositionUnavailable {
                instance: instance.to_string(),
                reason: format!(
                    "the persisted position pins version `{}` while the instance record pins `{}`",
                    position.version, record.version
                ),
            });
        }
        Ok(position)
    }

    /// Re-establishes execution state for a rehydrated run exactly like
    /// the original run established it: adapters are probed, the
    /// persisted resource identities re-attached, and every distinct
    /// binding the persisted decisions route through re-crosses the
    /// approval gate with a fresh grant (a binding already `BOUND` in
    /// this process is left as is; approvals stay fresh, mirroring the
    /// retry path's `restore_binding` precedent).
    async fn reestablish_execution(
        &mut self,
        record: &mut WorkflowInstance,
        position: &RunPosition,
    ) -> Result<(), WorkflowAppError> {
        self.registry.refresh_readiness().await?;
        for resource in &position.resources {
            self.registry.bind_resource(resource.clone())?;
        }
        let instance = record.instance_id;
        let mut reattached = false;
        for (binding, environment, capability) in distinct_decision_bindings(&position.decisions) {
            if self.registry.binding(&binding)?.readiness.state() == ReadinessState::Bound {
                continue;
            }
            let approval = self.authorize_binding(&instance, &binding, environment, capability)?;
            record.record_evidence(approval);
            let attached = self.registry.bind(&binding)?;
            self.events.record(WorkflowEvent::ResourcesBound {
                instance,
                binding,
                resources: attached
                    .into_iter()
                    .map(|resource| resource.resource_type)
                    .collect(),
            });
            reattached = true;
        }
        if reattached {
            self.instances.save(record.clone())?;
        }
        Ok(())
    }
}

/// The distinct bindings of one persisted decision map, with their
/// environment and capability, ordered by binding identity for
/// determinism — the same shape [`crate::lifecycle`] derives from a
/// fresh binding plan, applied to the persisted decisions instead.
fn distinct_decision_bindings(
    decisions: &BTreeMap<IrNodeId, Vec<BindingDecision>>,
) -> Vec<(CapabilityBindingId, ExecutionEnvironment, CapabilityId)> {
    let mut distinct: BTreeMap<CapabilityBindingId, (ExecutionEnvironment, CapabilityId)> =
        BTreeMap::new();
    for decision in decisions.values().flatten() {
        distinct
            .entry(decision.selected.binding.clone())
            .or_insert((
                decision.selected.environment,
                decision.requirement.capability.clone(),
            ));
    }
    distinct
        .into_iter()
        .map(|(binding, (environment, capability))| (binding, environment, capability))
        .collect()
}

#[cfg(test)]
#[path = "resume_tests.rs"]
mod tests;
