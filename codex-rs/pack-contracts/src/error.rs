//! Errors surfaced by the Pack contract surface.
//!
//! These errors describe contract-level failures: malformed identities,
//! broken revision integrity, illegal state transitions, immutability
//! violations, and policy conflicts. Durable control-plane decisions
//! (promotion, rollback) and execution failures are owned by later Pack work
//! orders and must surface through their own error types.

use thiserror::Error;

/// A failure produced while constructing, validating, or verifying Pack
/// contract records.
#[derive(Debug, Error)]
pub enum PackContractError {
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

    /// An immutable pack revision record failed its integrity check.
    #[error("pack revision integrity failure: {reason}")]
    RevisionIntegrity {
        /// Why the integrity check failed.
        reason: String,
    },

    /// A pack state transition violated the governed lifecycle.
    #[error("invalid pack state transition: {reason}")]
    InvalidStateTransition {
        /// Why the transition is illegal.
        reason: String,
    },

    /// An operation attempted to mutate state that is immutable by contract.
    #[error("immutability violation: {reason}")]
    ImmutabilityViolation {
        /// Which immutable contract was violated.
        reason: String,
    },

    /// A policy requirement conflicted with another policy or with platform
    /// invariants.
    #[error("pack policy conflict: {reason}")]
    PolicyConflict {
        /// Why the policies conflict.
        reason: String,
    },

    /// Serialization or deserialization of a contract record failed.
    #[error("pack contract serialization failure: {source}")]
    Serialization {
        /// The underlying serde failure.
        #[source]
        source: serde_json::Error,
    },
}

impl From<serde_json::Error> for PackContractError {
    fn from(source: serde_json::Error) -> Self {
        Self::Serialization { source }
    }
}

#[cfg(test)]
#[path = "error_tests.rs"]
mod tests;
