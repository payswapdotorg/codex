//! The single boundary between adapter evidence and the
//! `codex-workflow-contracts` evidence/instance model.
//!
//! [`EvidenceRecord`] carries the contract [`EvidenceKind`], a
//! deterministic credential-free locator, the SHA-256 hex over its
//! canonical payload, and the payload bytes. The evidence plane allocates
//! its own `ContentDigest` from [`EvidenceRecord::digest_input_bytes`] —
//! this crate never constructs evidence-plane digest types itself — and
//! [`EvidenceRecord::into_reference`] or [`attach_evidence`] completes the
//! contract reference. [`EvidenceSink`] is implemented for
//! `WorkflowInstance`, bridging onto its append-only `record_evidence`.

use codex_workflow_contracts::{ContentDigest, EvidenceKind, EvidenceReference, WorkflowInstance};
use serde::Serialize;

/// A normalized, digest-bearing evidence record.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct EvidenceRecord {
    /// Contract evidence kind.
    pub kind: EvidenceKind,
    /// Deterministic, credential-free locator allocated by the adapter.
    pub locator: String,
    /// Lowercase hex SHA-256 over [`Self::digest_input_bytes`].
    pub digest_hex: String,
    /// Canonical payload text; the evidence plane stores this as payload.
    canonical_payload: String,
}

impl EvidenceRecord {
    /// Builds a record by canonically serializing and digesting `payload`.
    pub fn from_payload<T: Serialize + ?Sized>(
        kind: EvidenceKind,
        locator: String,
        payload: &T,
    ) -> Self {
        let (canonical_payload, digest_hex) = crate::canonical::canonical_digest(payload);
        Self {
            kind,
            locator,
            digest_hex,
            canonical_payload,
        }
    }

    /// Canonical payload bytes. The evidence plane must allocate its
    /// `ContentDigest` from exactly these bytes so both sides agree on
    /// digest identity.
    pub fn digest_input_bytes(&self) -> &[u8] {
        self.canonical_payload.as_bytes()
    }

    /// Canonical payload text (audit/debug convenience).
    pub fn payload_canonical(&self) -> &str {
        &self.canonical_payload
    }

    /// Converts into a contract evidence reference. `digest` is allocated
    /// by the evidence plane from [`Self::digest_input_bytes`].
    pub fn into_reference(self, digest: ContentDigest) -> EvidenceReference {
        EvidenceReference {
            kind: self.kind,
            locator: self.locator,
            digest,
        }
    }
}

/// A sink for contract evidence references; implemented for
/// `WorkflowInstance` (append-only, per the contract).
pub trait EvidenceSink {
    /// Appends one evidence reference.
    fn record_evidence(&mut self, reference: EvidenceReference);
}

impl EvidenceSink for WorkflowInstance {
    fn record_evidence(&mut self, reference: EvidenceReference) {
        WorkflowInstance::record_evidence(self, reference);
    }
}

/// Attaches normalized records to `sink`, pairing each record with the
/// `ContentDigest` the evidence plane allocated from the record's
/// [`EvidenceRecord::digest_input_bytes`]. Append order is preserved.
pub fn attach_evidence<S: EvidenceSink>(
    sink: &mut S,
    pairs: impl IntoIterator<Item = (EvidenceRecord, ContentDigest)>,
) {
    for (record, digest) in pairs {
        sink.record_evidence(record.into_reference(digest));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Serialize)]
    struct Payload {
        alpha: u32,
        beta: String,
    }

    #[test]
    fn record_digest_matches_canonical_payload() {
        let record = EvidenceRecord::from_payload(
            EvidenceKind::Observation,
            "codex-browser-use/abc/observation/0001".to_string(),
            &Payload {
                alpha: 7,
                beta: "x".to_string(),
            },
        );
        assert_eq!(
            record.digest_hex,
            crate::canonical::sha256_hex(record.digest_input_bytes())
        );
        assert_eq!(record.payload_canonical(), r#"{"alpha":7,"beta":"x"}"#);
    }
}
