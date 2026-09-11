//! Durable workflow instance records.

use std::collections::BTreeMap;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::MutexGuard;
use std::sync::PoisonError;

use codex_workflow_app::WorkflowAppError;
use codex_workflow_app::WorkflowInstanceStore;
use codex_workflow_contracts::WorkflowInstance;
use codex_workflow_contracts::WorkflowInstanceId;

use crate::DurableStoreError;
use crate::atomic::read_snapshot;
use crate::atomic::write_state;

/// The backing state of [`DurableInstanceStore`].
#[derive(Debug)]
struct InstanceInner {
    state: Mutex<BTreeMap<WorkflowInstanceId, WorkflowInstance>>,
    path: PathBuf,
}

/// Durable storage for workflow instance records.
///
/// A shared-state handle like the in-memory double: clones observe the
/// same backing snapshot file, and the [`crate::DurableInstanceControl`]
/// composition relies on that sharing. `create` rejects duplicate
/// identities exactly like the in-memory double; `save` persists status
/// changes and appended evidence through the same atomic snapshot
/// write the version store uses.
#[derive(Clone, Debug)]
pub struct DurableInstanceStore {
    inner: Arc<InstanceInner>,
}

impl DurableInstanceStore {
    /// Opens the instance store at snapshot file `path`, creating it as
    /// empty when absent.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, DurableStoreError> {
        let path = path.as_ref().to_path_buf();
        let state = match read_snapshot(&path)? {
            Some(bytes) => serde_json::from_slice(&bytes)?,
            None => BTreeMap::new(),
        };
        Ok(Self {
            inner: Arc::new(InstanceInner {
                state: Mutex::new(state),
                path,
            }),
        })
    }

    /// Snapshot of all instance records, ordered by instance id.
    pub fn records(&self) -> Vec<WorkflowInstance> {
        self.lock().values().cloned().collect()
    }

    /// The interior-mutable core of [`WorkflowInstanceStore::save`].
    ///
    /// The mutex already serializes writers (the shared-state handle
    /// mirrors the in-memory double); this core exists so crate-internal
    /// compositions — [`crate::DurableInstanceControl`] — can persist
    /// through a shared handle. It is not part of the public surface.
    pub(crate) fn put(&self, instance: WorkflowInstance) -> Result<(), WorkflowAppError> {
        let mut state = self.lock();
        let id = instance.instance_id;
        let previous = state.insert(id, instance);
        if let Err(error) = write_state(&self.inner.path, &*state) {
            match previous {
                Some(previous) => {
                    state.insert(id, previous);
                }
                None => {
                    state.remove(&id);
                }
            }
            return Err(error.into_app_error());
        }
        Ok(())
    }

    /// Number of stored instances.
    pub fn len(&self) -> usize {
        self.lock().len()
    }

    /// Whether the store is empty.
    pub fn is_empty(&self) -> bool {
        self.lock().is_empty()
    }

    fn lock(&self) -> MutexGuard<'_, BTreeMap<WorkflowInstanceId, WorkflowInstance>> {
        self.inner
            .state
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }
}

impl WorkflowInstanceStore for DurableInstanceStore {
    fn create(&mut self, instance: WorkflowInstance) -> Result<(), WorkflowAppError> {
        let mut state = self.lock();
        if state.contains_key(&instance.instance_id) {
            return Err(WorkflowAppError::InstanceAlreadyExists {
                instance: instance.instance_id.to_string(),
            });
        }
        let id = instance.instance_id;
        state.insert(id, instance);
        if let Err(error) = write_state(&self.inner.path, &*state) {
            state.remove(&id);
            return Err(error.into_app_error());
        }
        Ok(())
    }

    fn save(&mut self, instance: WorkflowInstance) -> Result<(), WorkflowAppError> {
        self.put(instance)
    }

    fn load(
        &self,
        instance: &WorkflowInstanceId,
    ) -> Result<Option<WorkflowInstance>, WorkflowAppError> {
        Ok(self.lock().get(instance).cloned())
    }
}

#[cfg(test)]
#[path = "instance_store_tests.rs"]
mod tests;
