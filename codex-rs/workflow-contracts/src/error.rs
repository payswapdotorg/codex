//! Errors surfaced by the workflow contract surface.
//!
//! These errors describe contract-level failures: malformed identities,
//! broken integrity, incomplete dependency locks, and forge interaction
//! failures. Durable control-plane and execution failures are owned by
//! later Work Orders and must surface through their own error types.

use thiserror::Error;

/// A failure produced while constructing, validating, or verifying workflow
/// contract records.
#[derive(Debug, Error)]
pub enum WorkflowContractError {
    /// A string identifier did not satisfy its format contract.
    #[error("invalid {kind} `{value}`: {reason}")]
    InvalidIdentifier {
        /// The kind of identifier that failed validation.
        kind: &'static str,
        /// The rejected value.
        value: String,
        /// Why the value was rejected.
        reason: &'static str,
    },

    /// A content digest was malformed or did not match the referenced bytes.
    #[error("invalid content digest: {reason}")]
    InvalidDigest {
        /// Why the digest was rejected.
        reason: String,
    },

    /// An immutable version record failed its integrity check.
    #[error("workflow version integrity failure: {reason}")]
    VersionIntegrity {
        /// Why the integrity check failed.
        reason: String,
    },

    /// A dependency lock does not cover the declared dependencies.
    #[error("incomplete dependency lock: {reason}")]
    IncompleteDependencyLock {
        /// Which declared dependency is missing or inconsistent.
        reason: String,
    },

    /// A workflow IR graph violated a structural invariant.
    #[error("malformed workflow IR: {reason}")]
    MalformedIr {
        /// Which structural invariant was violated.
        reason: String,
    },

    /// A repository-relative path escaped the repository root.
    #[error("invalid repository path `{value}`: {reason}")]
    InvalidRepositoryPath {
        /// The rejected path.
        value: String,
        /// Why the path was rejected.
        reason: &'static str,
    },

    /// A forge lookup failed.
    #[error("workflow forge lookup failed: {reason}")]
    ForgeLookup {
        /// Why the lookup failed.
        reason: String,
    },

    /// Serialization or deserialization of a contract record failed.
    #[error("workflow contract serialization failure: {source}")]
    Serialization {
        /// The underlying serde failure.
        #[source]
        source: serde_json::Error,
    },
}

impl From<serde_json::Error> for WorkflowContractError {
    fn from(source: serde_json::Error) -> Self {
        Self::Serialization { source }
    }
}

#[cfg(test)]
#[path = "error_tests.rs"]
mod tests;
