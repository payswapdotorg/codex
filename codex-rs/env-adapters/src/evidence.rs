//! Adapter-side evidence journal for the WO-015 environment adapters.
//!
//! Every adapter action, observation, failure trace, and takeover receipt
//! is journaled as a digest-bearing record, following the evidence identity
//! convention of the WO-006/WO-007 adapters: the digest input bytes are the
//! canonical JSON serialization of the payload (ordered maps, so identical
//! payloads always digest identically), the record exposes its digest hex,
//! and the evidence plane promotes records into contract
//! `EvidenceReference` values from exactly those bytes. Locators are
//! deterministic (`codex-env-adapters/<adapter>/<sequence>`), so journal
//! output is reproducible under test and replay.
//!
//! The journal is in-memory and append-only: it is a harvesting point for
//! the host, not the evidence plane itself.

use sha2::Digest;
use sha2::Sha256;

use codex_workflow_contracts::EvidenceKind;

use crate::EnvAdapterError;

/// The locator prefix every WO-015 adapter evidence record uses.
pub const EVIDENCE_LOCATOR_PREFIX: &str = "codex-env-adapters";

/// One adapter-emitted evidence record with digest identity.
///
/// Records carry the payload itself (bounded by construction: observations
/// reuse the payloads the execution contracts already bound, traces are
/// small adapter-built objects) plus the digest of the payload's canonical
/// serialization.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct AdapterEvidenceRecord {
    /// Deterministic locator: `codex-env-adapters/<adapter>/<sequence>`.
    pub locator: String,
    /// SHA-256 hex digest of the canonical payload serialization.
    pub digest_hex: String,
    /// The evidence kind this record carries.
    pub kind: EvidenceKind,
    /// The payload, untrusted-derived and bounded by construction.
    pub payload: serde_json::Value,
}

/// Append-only evidence journal for one adapter instance.
#[derive(Clone, Debug)]
pub struct EvidenceJournal {
    adapter: String,
    next: u64,
    records: Vec<AdapterEvidenceRecord>,
}

impl EvidenceJournal {
    /// Creates an empty journal for the adapter named `adapter`.
    pub fn new(adapter: impl Into<String>) -> Self {
        Self {
            adapter: adapter.into(),
            next: 0,
            records: Vec::new(),
        }
    }

    /// Records one payload under `kind`.
    ///
    /// The payload is serialized canonically and digested with SHA-256; the
    /// record and its locator are deterministic functions of the journal
    /// position, so the same run always produces the same evidence
    /// identity.
    pub fn record(
        &mut self,
        kind: EvidenceKind,
        payload: &serde_json::Value,
    ) -> Result<AdapterEvidenceRecord, EnvAdapterError> {
        let bytes = serde_json::to_vec(payload)?;
        let digest_hex = sha256_hex(&bytes);
        let locator = format!("{EVIDENCE_LOCATOR_PREFIX}/{}/{}", self.adapter, self.next);
        self.next += 1;
        let record = AdapterEvidenceRecord {
            locator,
            digest_hex,
            kind,
            payload: payload.clone(),
        };
        self.records.push(record.clone());
        Ok(record)
    }

    /// The records recorded so far, oldest first.
    pub fn records(&self) -> &[AdapterEvidenceRecord] {
        &self.records
    }

    /// Number of recorded records.
    pub fn len(&self) -> usize {
        self.records.len()
    }

    /// Whether no record has been recorded.
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }
}

/// The SHA-256 hex digest of `bytes`.
fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut hex = String::with_capacity(digest.len() * 2);
    for byte in digest {
        hex.push_str(&format!("{byte:02x}"));
    }
    hex
}

#[cfg(test)]
#[path = "evidence_tests.rs"]
mod tests;
