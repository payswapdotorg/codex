//! Durable workflow version records.
//!
//! [`DurableVersionStore`] implements the WO-010
//! [`WorkflowVersionStore`] port over one atomic snapshot file: the
//! whole `id -> record` map is serialized deterministically (ordered
//! map, compact JSON) and replaced with a temp→fsync→rename write, so a
//! crash leaves either the previous snapshot or the new one — never a
//! partial file.
//!
//! On load, every record is re-verified with
//! [`WorkflowVersion::verify_integrity`]: a tampered or corrupted
//! version surfaces as
//! [`WorkflowAppError::VersionIntegrity`] and never becomes
//! executable.

use std::collections::BTreeMap;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::MutexGuard;
use std::sync::PoisonError;

use codex_workflow_app::WorkflowAppError;
use codex_workflow_app::WorkflowVersionStore;
use codex_workflow_contracts::WorkflowVersion;
use codex_workflow_contracts::WorkflowVersionId;

use crate::DurableStoreError;
use crate::atomic::read_snapshot;
use crate::atomic::write_state;

/// The backing state of [`DurableVersionStore`].
#[derive(Debug)]
struct VersionInner {
    state: Mutex<BTreeMap<WorkflowVersionId, WorkflowVersion>>,
    path: PathBuf,
}

/// Durable storage for immutable workflow version records.
///
/// A shared-state handle like the in-memory double: clones observe the
/// same backing snapshot file. The port takes `&mut self`, so the host
/// serializes writers (see the crate-level single-writer contract).
#[derive(Clone, Debug)]
pub struct DurableVersionStore {
    inner: Arc<VersionInner>,
}

impl DurableVersionStore {
    /// Opens the version store at snapshot file `path`, creating it as
    /// empty when absent.
    ///
    /// The whole snapshot is parsed at open time; an unparseable file
    /// is a deterministic construction failure, never a silent empty
    /// store.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, DurableStoreError> {
        let path = path.as_ref().to_path_buf();
        let state = match read_snapshot(&path)? {
            Some(bytes) => serde_json::from_slice(&bytes)?,
            None => BTreeMap::new(),
        };
        Ok(Self {
            inner: Arc::new(VersionInner {
                state: Mutex::new(state),
                path,
            }),
        })
    }

    /// Number of stored versions.
    pub fn len(&self) -> usize {
        self.lock().len()
    }

    /// Whether the store is empty.
    pub fn is_empty(&self) -> bool {
        self.lock().is_empty()
    }

    fn lock(&self) -> MutexGuard<'_, BTreeMap<WorkflowVersionId, WorkflowVersion>> {
        self.inner
            .state
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }
}

impl WorkflowVersionStore for DurableVersionStore {
    fn publish(&mut self, version: WorkflowVersion) -> Result<WorkflowVersionId, WorkflowAppError> {
        let id = version.version_id.clone();
        let mut state = self.lock();
        let previous = state.insert(id.clone(), version);
        if let Err(error) = write_state(&self.inner.path, &*state) {
            // Roll the in-memory state back so it keeps matching the
            // durable bytes; the failed publish stores nothing.
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
        Ok(id)
    }

    fn load(
        &self,
        version: &WorkflowVersionId,
    ) -> Result<Option<WorkflowVersion>, WorkflowAppError> {
        let state = self.lock();
        let Some(record) = state.get(version) else {
            return Ok(None);
        };
        record
            .verify_integrity()
            .map_err(|error| WorkflowAppError::VersionIntegrity {
                version: version.to_string(),
                reason: error.to_string(),
            })?;
        Ok(Some(record.clone()))
    }
}

#[cfg(test)]
#[path = "version_store_tests.rs"]
mod tests;
