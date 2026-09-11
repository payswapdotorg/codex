//! Provider-side evidence journal for the resource provider plane
//! (WO-016).
//!
//! Every provider/resource fact — lifecycle transitions, provider
//! health, capacity events, rebind minting — is journaled as a
//! digest-bearing record, following the evidence identity convention of
//! the WO-006/WO-007/WO-015 adapters: the digest input bytes are the
//! canonical JSON serialization of the payload (ordered maps, so
//! identical payloads always digest identically), the record exposes its
//! digest hex, and the evidence plane promotes records into contract
//! `EvidenceReference` values from exactly those bytes. Locators are
//! deterministic (`codex-resource-providers/<provider>/<sequence>`), so
//! journal output is reproducible under test and replay.
//!
//! Evidence **distinguishes provider/resource facts from workflow
//! semantic facts** structurally: locators are provider-scoped and
//! payloads carry provider identity, resource class, and resource-plane
//! transitions — never workflow versions, nodes, or semantic state.
//! Credential material is absent by construction: the journal records
//! typed infrastructure facts only.
//!
//! The journal is in-memory and append-only: it is a harvesting point
//! for the host, not the evidence plane itself.

use sha2::Digest;
use sha2::Sha256;

use codex_workflow_contracts::EvidenceKind;

use crate::ResourceProviderError;

/// The locator prefix every WO-016 provider evidence record uses.
pub const EVIDENCE_LOCATOR_PREFIX: &str = "codex-resource-providers";

/// One provider-emitted evidence record with digest identity.
///
/// Records carry the payload itself (bounded by construction: transitions
/// and health answers are small provider-built objects) plus the digest
/// of the payload's canonical serialization.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ResourceEvidenceRecord {
    /// Deterministic locator:
    /// `codex-resource-providers/<provider>/<sequence>`.
    pub locator: String,
    /// SHA-256 hex digest of the canonical payload serialization.
    pub digest_hex: String,
    /// The evidence kind this record carries.
    pub kind: EvidenceKind,
    /// The payload, untrusted-derived and bounded by construction.
    pub payload: serde_json::Value,
}

/// Append-only evidence journal for one provider instance.
#[derive(Clone, Debug)]
pub struct ResourceEvidenceJournal {
    provider: String,
    next: u64,
    records: Vec<ResourceEvidenceRecord>,
}

impl ResourceEvidenceJournal {
    /// Creates an empty journal for the provider named `provider`.
    pub fn new(provider: impl Into<String>) -> Self {
        Self {
            provider: provider.into(),
            next: 0,
            records: Vec::new(),
        }
    }

    /// Records one payload under `kind`.
    ///
    /// The payload is serialized canonically and digested with SHA-256;
    /// the record and its locator are deterministic functions of the
    /// journal position, so the same run always produces the same
    /// evidence identity.
    pub fn record(
        &mut self,
        kind: EvidenceKind,
        payload: &serde_json::Value,
    ) -> Result<ResourceEvidenceRecord, ResourceProviderError> {
        let bytes = serde_json::to_vec(payload)?;
        let digest_hex = sha256_hex(&bytes);
        let locator = format!("{EVIDENCE_LOCATOR_PREFIX}/{}/{}", self.provider, self.next);
        self.next += 1;
        let record = ResourceEvidenceRecord {
            locator,
            digest_hex,
            kind,
            payload: payload.clone(),
        };
        self.records.push(record.clone());
        Ok(record)
    }

    /// The records recorded so far, oldest first.
    pub fn records(&self) -> &[ResourceEvidenceRecord] {
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
