//! Durable installed configurations and rebind audits.
//!
//! [`DurableInstallationStore`] implements the WO-011
//! [`InstallationStore`] port over two files: an atomic snapshot for
//! the current `workflow -> configuration` map (upserts replace the
//! whole map deterministically) and an append-only JSONL journal for
//! the rebind audit history, which never rewrites. Both writes are
//! durable before their port methods return.

use std::collections::BTreeMap;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::MutexGuard;
use std::sync::PoisonError;

use codex_workflow_contracts::WorkflowDefinitionId;
use codex_workflow_triggers::InstallationStore;
use codex_workflow_triggers::InstalledConfiguration;
use codex_workflow_triggers::RebindRecord;
use codex_workflow_triggers::WorkflowTriggerError;

use crate::DurableStoreError;
use crate::atomic::read_snapshot;
use crate::atomic::write_state;
use crate::journal::Journal;
use crate::journal::load_journal;

/// The backing state of [`DurableInstallationStore`].
#[derive(Clone, Debug, Default)]
struct InstallationState {
    configurations: BTreeMap<WorkflowDefinitionId, InstalledConfiguration>,
    audits: Vec<RebindRecord>,
}

/// The owned backing of [`DurableInstallationStore`].
#[derive(Debug)]
struct InstallationInner {
    state: Mutex<InstallationState>,
    snapshot: PathBuf,
    audits: Journal,
}

/// Durable storage for installed configurations and rebind audits.
///
/// A shared-state handle like the in-memory double: clones observe the
/// same backing snapshot and audit journal.
#[derive(Clone, Debug)]
pub struct DurableInstallationStore {
    inner: Arc<InstallationInner>,
}

impl DurableInstallationStore {
    /// Opens the installation store with its configuration snapshot at
    /// `snapshot` and its rebind-audit journal at `audits`, creating
    /// both as empty when absent.
    ///
    /// The snapshot holds the current `workflow -> configuration` map;
    /// the audit history is replayed from the journal only.
    pub fn open(
        snapshot: impl AsRef<Path>,
        audits: impl AsRef<Path>,
    ) -> Result<Self, DurableStoreError> {
        let snapshot = snapshot.as_ref().to_path_buf();
        let audits_path = audits.as_ref();
        let configurations = match read_snapshot(&snapshot)? {
            Some(bytes) => serde_json::from_slice(&bytes)?,
            None => BTreeMap::new(),
        };
        let audits = load_journal(audits_path)?;
        let state = InstallationState {
            configurations,
            audits,
        };
        let journal = Journal::open(audits_path)?;
        Ok(Self {
            inner: Arc::new(InstallationInner {
                state: Mutex::new(state),
                snapshot,
                audits: journal,
            }),
        })
    }

    /// Snapshot of the rebind audit history, in order.
    pub fn audits(&self) -> Vec<RebindRecord> {
        self.lock().audits.clone()
    }

    fn lock(&self) -> MutexGuard<'_, InstallationState> {
        self.inner
            .state
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }
}

impl InstallationStore for DurableInstallationStore {
    fn save(&mut self, configuration: InstalledConfiguration) -> Result<(), WorkflowTriggerError> {
        let mut state = self.lock();
        let workflow = configuration.workflow.clone();
        let previous = state.configurations.insert(workflow.clone(), configuration);
        if let Err(error) = write_state(&self.inner.snapshot, &state.configurations) {
            // Roll the in-memory state back so it keeps matching the
            // durable snapshot; the failed save stores nothing.
            match previous {
                Some(previous) => {
                    state.configurations.insert(workflow, previous);
                }
                None => {
                    state.configurations.remove(&workflow);
                }
            }
            return Err(error.into_trigger_error());
        }
        Ok(())
    }

    fn load(
        &self,
        workflow: &WorkflowDefinitionId,
    ) -> Result<Option<InstalledConfiguration>, WorkflowTriggerError> {
        Ok(self.lock().configurations.get(workflow).cloned())
    }

    fn list(&self) -> Result<Vec<InstalledConfiguration>, WorkflowTriggerError> {
        Ok(self.lock().configurations.values().cloned().collect())
    }

    fn append_audit(&mut self, record: RebindRecord) -> Result<(), WorkflowTriggerError> {
        let mut state = self.lock();
        state.audits.push(record.clone());
        if let Err(error) = self
            .inner
            .audits
            .append(&record)
            .map_err(DurableStoreError::into_trigger_error)
        {
            // Roll the in-memory state back so it keeps matching the
            // durable journal; the failed append records nothing.
            state.audits.pop();
            return Err(error);
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "installation_store_tests.rs"]
mod tests;
