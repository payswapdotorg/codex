//! Durable evidence payloads.
//!
//! [`DurableEvidenceStore`] implements the WO-010 [`EvidenceStore`]
//! port over one append-only JSONL journal: every stored payload is one
//! committed line (sequence, kind, locator, digest, payload), fsynced
//! before `store` returns. The locator and digest minting mirrors the
//! in-memory double exactly — locators are
//! `workflow-app/<kind-slug>/<sequence>` and digests are
//! [`ContentDigest::of`] the payload — so a workflow that stores the
//! same payloads through either store observes identical evidence
//! references.
//!
//! `promote` validates an adapter-supplied digest and mints a reference
//! without persisting anything, exactly like the in-memory double: the
//! promoted payload stays owned by the adapter's own journal.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::MutexGuard;
use std::sync::PoisonError;

use codex_workflow_app::EvidenceStore;
use codex_workflow_app::WorkflowAppError;
use codex_workflow_contracts::ContentDigest;
use codex_workflow_contracts::EvidenceKind;
use codex_workflow_contracts::EvidenceReference;

use crate::DurableStoreError;
use crate::journal::Journal;
use crate::journal::load_journal;

/// The backing state of [`DurableEvidenceStore`].
#[derive(Clone, Debug, Default)]
struct EvidenceState {
    next_sequence: u64,
    payloads: BTreeMap<String, (EvidenceKind, serde_json::Value)>,
}

/// One committed evidence record in the journal.
///
/// The record is the durable mirror of one `store` call; the sequence
/// allocates the locator and is recovered on load so locators never
/// repeat across restarts.
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct EvidenceJournalRecord {
    sequence: u64,
    kind: EvidenceKind,
    locator: String,
    digest: String,
    payload: serde_json::Value,
}

/// Durable evidence plane: stores payloads and allocates the same
/// deterministic locators and digests as the in-memory double.
///
/// A shared-state handle: clones observe the same backing journal.
#[derive(Clone, Debug)]
pub struct DurableEvidenceStore {
    inner: Arc<EvidenceInner>,
}

/// The owned backing of [`DurableEvidenceStore`].
#[derive(Debug)]
struct EvidenceInner {
    state: Mutex<EvidenceState>,
    journal: Journal,
}

impl DurableEvidenceStore {
    /// Opens the evidence store at journal file `path`, creating it as
    /// empty when absent.
    ///
    /// Committed lines are replayed in order; a torn tail is repaired
    /// by the documented journal policy, and the sequence counter
    /// resumes from the highest committed sequence so locators stay
    /// unique across restarts.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, DurableStoreError> {
        let records: Vec<EvidenceJournalRecord> = load_journal(path.as_ref())?;
        let mut state = EvidenceState::default();
        for record in records {
            state.next_sequence = state.next_sequence.max(record.sequence);
            state
                .payloads
                .insert(record.locator, (record.kind, record.payload));
        }
        let journal = Journal::open(path.as_ref())?;
        Ok(Self {
            inner: Arc::new(EvidenceInner {
                state: Mutex::new(state),
                journal,
            }),
        })
    }

    /// Number of stored payloads.
    pub fn len(&self) -> usize {
        self.lock().payloads.len()
    }

    /// Whether the store is empty.
    pub fn is_empty(&self) -> bool {
        self.lock().payloads.is_empty()
    }

    /// Verifies one stored payload against its reference digest.
    ///
    /// Returns `Ok(false)` when the locator is unknown — the
    /// evidence-plane "verify" step, identical to the in-memory double.
    pub fn verify(&self, reference: &EvidenceReference) -> Result<bool, WorkflowAppError> {
        match self.lock().payloads.get(&reference.locator) {
            Some((_, payload)) => Ok(ContentDigest::of(payload)? == reference.digest),
            None => Ok(false),
        }
    }

    /// The stored payload for `locator`, when present (audit
    /// convenience, mirroring the in-memory double).
    pub fn payload(&self, locator: &str) -> Option<serde_json::Value> {
        self.lock()
            .payloads
            .get(locator)
            .map(|(_, payload)| payload.clone())
    }

    fn lock(&self) -> MutexGuard<'_, EvidenceState> {
        self.inner
            .state
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }
}

impl EvidenceStore for DurableEvidenceStore {
    fn store(
        &mut self,
        kind: EvidenceKind,
        payload: &serde_json::Value,
    ) -> Result<EvidenceReference, WorkflowAppError> {
        let digest = ContentDigest::of(payload)?;
        let mut state = self.lock();
        state.next_sequence += 1;
        let locator = format!("workflow-app/{}/{}", slug(kind), state.next_sequence);
        let record = EvidenceJournalRecord {
            sequence: state.next_sequence,
            kind,
            locator: locator.clone(),
            digest: digest.to_string(),
            payload: payload.clone(),
        };
        state
            .payloads
            .insert(locator.clone(), (kind, payload.clone()));
        if let Err(error) = self
            .inner
            .journal
            .append(&record)
            .map_err(DurableStoreError::into_app_error)
        {
            // Roll the in-memory state back so it keeps matching the
            // durable journal; the failed store records nothing.
            state.payloads.remove(&locator);
            state.next_sequence -= 1;
            return Err(error);
        }
        Ok(EvidenceReference {
            kind,
            locator,
            digest,
        })
    }

    fn promote(
        &mut self,
        kind: EvidenceKind,
        locator: &str,
        digest_hex: &str,
    ) -> Result<EvidenceReference, WorkflowAppError> {
        let digest = ContentDigest::try_from(format!("sha256:{digest_hex}"))?;
        Ok(EvidenceReference {
            kind,
            locator: locator.to_string(),
            digest,
        })
    }
}

/// The locator slug for one evidence kind.
///
/// This replicates the in-memory double's private slug table verbatim:
/// the locator format is the observable minting contract both stores
/// must agree on.
fn slug(kind: EvidenceKind) -> &'static str {
    match kind {
        EvidenceKind::Observation => "observation",
        EvidenceKind::Artifact => "artifact",
        EvidenceKind::Approval => "approval",
        EvidenceKind::Trace => "trace",
        EvidenceKind::TestResult => "test-result",
        EvidenceKind::Recovery => "recovery",
    }
}

#[cfg(test)]
#[path = "evidence_store_tests.rs"]
mod tests;
