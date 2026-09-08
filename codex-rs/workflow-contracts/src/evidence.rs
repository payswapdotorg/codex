//! Evidence references.
//!
//! Execution evidence (observations, artifacts, approvals, traces, test
//! results, recovery records) is owned by the evidence plane. Workflow
//! contracts reference evidence by identity and digest so a workflow
//! instance can be audited without embedding evidence payloads in workflow
//! semantics.
//!
//! External outputs are untrusted input by default; digests let auditors
//! verify that referenced evidence has not drifted.

use serde::Deserialize;
use serde::Serialize;

use crate::ContentDigest;

/// Kind of evidence a reference points at.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum EvidenceKind {
    /// A normalized observation of an execution environment.
    Observation,
    /// A durable artifact produced or captured during execution.
    Artifact,
    /// A recorded approval decision.
    Approval,
    /// A structured rollout trace (reusing Codex rollout trace systems).
    Trace,
    /// A test or verification result.
    TestResult,
    /// A recovery, retry, or takeover record.
    Recovery,
}

/// A reference from a workflow record to evidence held by the evidence
/// plane.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EvidenceReference {
    /// What kind of evidence is referenced.
    pub kind: EvidenceKind,
    /// Opaque locator allocated by the evidence plane, for example a trace
    /// or artifact identifier.
    pub locator: String,
    /// Digest of the referenced evidence for integrity checking.
    pub digest: ContentDigest,
}

#[cfg(test)]
#[path = "evidence_tests.rs"]
mod tests;
