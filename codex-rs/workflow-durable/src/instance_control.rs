//! Durable instance control: the legal `Paused -> Running` resume.
//!
//! [`DurableInstanceControl`] implements the WO-011 [`InstanceControl`]
//! port over a shared [`DurableInstanceStore`]: it performs exactly the
//! transition the in-memory double performs (only `Paused` instances
//! resume, with the same `InstanceUnavailable` / `IllegalResume`
//! errors), persists the updated record through the instance store's
//! atomic snapshot, and journals every received resume directive as an
//! append-only audit record that survives restart.

use std::path::Path;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::MutexGuard;
use std::sync::PoisonError;

use codex_workflow_app::WorkflowInstanceStore;
use codex_workflow_contracts::WorkflowInstance;
use codex_workflow_contracts::WorkflowInstanceStatus;
use codex_workflow_triggers::InstanceControl;
use codex_workflow_triggers::ResumeDirective;
use codex_workflow_triggers::WorkflowTriggerError;

use crate::DurableInstanceStore;
use crate::DurableStoreError;
use crate::journal::Journal;
use crate::journal::load_journal;

/// The owned backing of [`DurableInstanceControl`].
#[derive(Debug)]
struct ControlInner {
    instances: DurableInstanceStore,
    directives: Mutex<Vec<ResumeDirective>>,
    journal: Journal,
}

/// The durable resume seam into the control plane.
///
/// A shared-state handle like the in-memory double: clones observe the
/// same instance store and directive journal.
#[derive(Clone, Debug)]
pub struct DurableInstanceControl {
    inner: Arc<ControlInner>,
}

impl DurableInstanceControl {
    /// Creates a control plane backed by `instances` whose resume
    /// directives journal at `directives`, creating it as empty when
    /// absent.
    ///
    /// The instance store is shared: the lifecycle, the trigger plane,
    /// and this control observe and mutate the same durable instance
    /// records.
    pub fn open(
        instances: DurableInstanceStore,
        directives: impl AsRef<Path>,
    ) -> Result<Self, DurableStoreError> {
        let records: Vec<ResumeDirective> = load_journal(directives.as_ref())?;
        let journal = Journal::open(directives.as_ref())?;
        Ok(Self {
            inner: Arc::new(ControlInner {
                instances,
                directives: Mutex::new(records),
                journal,
            }),
        })
    }

    /// Snapshot of the resume directives received so far, in order.
    pub fn directives(&self) -> Vec<ResumeDirective> {
        self.lock().clone()
    }

    fn lock(&self) -> MutexGuard<'_, Vec<ResumeDirective>> {
        self.inner
            .directives
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }
}

impl InstanceControl for DurableInstanceControl {
    fn resume(
        &mut self,
        directive: &ResumeDirective,
    ) -> Result<WorkflowInstance, WorkflowTriggerError> {
        let mut instance = self
            .inner
            .instances
            .load(&directive.instance)?
            .ok_or_else(|| WorkflowTriggerError::InstanceUnavailable {
                instance: directive.instance.to_string(),
            })?;
        if instance.status != WorkflowInstanceStatus::Paused {
            return Err(WorkflowTriggerError::IllegalResume {
                instance: directive.instance.to_string(),
                current: format!("{:?}", instance.status),
            });
        }
        instance.status = WorkflowInstanceStatus::Running;
        // The transition is durable before it is observable: the
        // instance store's atomic snapshot write commits `Running`.
        // `put` is the store's interior-mutable save core (the mutex
        // serializes writers; the shared handle mirrors the in-memory
        // double's composition).
        self.inner.instances.put(instance.clone())?;
        // The directive is journaled after the transition it describes.
        // If journaling fails, the transition is rolled back so the
        // control plane stays all-or-nothing; a failure of that rare
        // rollback itself would heal at the next reload (the snapshot
        // keeps the last successfully flushed state).
        let mut directives = self.lock();
        directives.push(directive.clone());
        if let Err(error) = self
            .inner
            .journal
            .append(directive)
            .map_err(DurableStoreError::into_trigger_error)
        {
            directives.pop();
            let mut rolled_back = instance;
            rolled_back.status = WorkflowInstanceStatus::Paused;
            if let Err(rollback_error) = self.inner.instances.put(rolled_back) {
                return Err(WorkflowTriggerError::App(rollback_error));
            }
            return Err(error);
        }
        Ok(instance)
    }
}

#[cfg(test)]
#[path = "instance_control_tests.rs"]
mod tests;
