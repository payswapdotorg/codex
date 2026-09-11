//! Durable persisted run positions.
//!
//! [`DurableRunPositionStore`] implements the MWO-003
//! [`RunPositionStore`] port over one atomic snapshot file: the
//! position of one instance is current-state (the latest checkpoint
//! replaces the earlier one), so the whole `instance -> position` map
//! is serialized deterministically (ordered map, compact JSON) and
//! replaced with a temp→fsync→rename write — the same all-or-nothing
//! contract the instance store uses, so a crash leaves either the
//! previous checkpoint or the new one, never a partial record.

use std::collections::BTreeMap;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::MutexGuard;
use std::sync::PoisonError;

use codex_workflow_app::RunPosition;
use codex_workflow_app::RunPositionStore;
use codex_workflow_app::WorkflowAppError;
use codex_workflow_contracts::WorkflowInstanceId;

use crate::DurableStoreError;
use crate::atomic::read_snapshot;
use crate::atomic::write_state;

/// The backing state of [`DurableRunPositionStore`].
#[derive(Debug)]
struct PositionInner {
    state: Mutex<BTreeMap<WorkflowInstanceId, RunPosition>>,
    path: PathBuf,
}

/// Durable storage for the persisted run positions of active runs.
///
/// A shared-state handle like the in-memory double: clones observe the
/// same backing snapshot file. The port takes `&mut self`, so the host
/// serializes writers (see the crate-level single-writer contract);
/// every write rolls the in-memory state back on flush failure so the
/// observable state always matches the durable bytes.
#[derive(Clone, Debug)]
pub struct DurableRunPositionStore {
    inner: Arc<PositionInner>,
}

impl DurableRunPositionStore {
    /// Opens the run-position store at snapshot file `path`, creating
    /// it as empty when absent.
    ///
    /// The whole snapshot is parsed at open time; an unparseable file
    /// is a deterministic construction failure, never a silently empty
    /// store.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, DurableStoreError> {
        let path = path.as_ref().to_path_buf();
        let state = match read_snapshot(&path)? {
            Some(bytes) => serde_json::from_slice(&bytes)?,
            None => BTreeMap::new(),
        };
        Ok(Self {
            inner: Arc::new(PositionInner {
                state: Mutex::new(state),
                path,
            }),
        })
    }

    /// Snapshot of all persisted positions, ordered by instance id.
    pub fn records(&self) -> Vec<RunPosition> {
        self.lock().values().cloned().collect()
    }

    /// Number of persisted positions.
    pub fn len(&self) -> usize {
        self.lock().len()
    }

    /// Whether the store is empty.
    pub fn is_empty(&self) -> bool {
        self.lock().is_empty()
    }

    fn lock(&self) -> MutexGuard<'_, BTreeMap<WorkflowInstanceId, RunPosition>> {
        self.inner
            .state
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }
}

impl RunPositionStore for DurableRunPositionStore {
    fn save(&mut self, position: RunPosition) -> Result<(), WorkflowAppError> {
        let mut state = self.lock();
        let id = position.instance;
        let previous = state.insert(id, position);
        if let Err(error) = write_state(&self.inner.path, &*state) {
            // Roll the in-memory state back so it keeps matching the
            // durable bytes; the failed checkpoint stores nothing.
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

    fn load(&self, instance: &WorkflowInstanceId) -> Result<Option<RunPosition>, WorkflowAppError> {
        Ok(self.lock().get(instance).cloned())
    }

    fn discard(&mut self, instance: &WorkflowInstanceId) -> Result<(), WorkflowAppError> {
        let mut state = self.lock();
        // Discarding an instance with no position is an ordinary no-op
        // and performs no i/o.
        let Some(previous) = state.remove(instance) else {
            return Ok(());
        };
        if let Err(error) = write_state(&self.inner.path, &*state) {
            state.insert(*instance, previous);
            return Err(error.into_app_error());
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "run_position_store_tests.rs"]
mod tests;
