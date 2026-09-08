//! Errors surfaced by the execution contract surface.
//!
//! These errors describe contract-level failures: malformed identities,
//! oversized payloads, illegal readiness transitions, policy misuse, and
//! registry operation violations. Diagnosable *runtime* availability and
//! binding failures are modeled as [`crate::BindingDiagnostic`] records and
//! surface through resolution results and readiness state, not as panics or
//! opaque errors. Durable control-plane failures remain owned by later Work
//! Orders.

use thiserror::Error;

use crate::AdapterId;
use crate::CapabilityBindingId;
use crate::ExecutionEnvironment;
use crate::ReadinessState;
use codex_workflow_contracts::EvidenceKind;
use codex_workflow_contracts::ResourceTypeId;
use codex_workflow_contracts::WorkflowContractError;

/// A failure produced while constructing, validating, or operating
/// execution-plane contract records.
#[derive(Debug, Error)]
pub enum ExecutionContractError {
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

    /// A payload exceeded its contract-level size or depth bound.
    #[error("{field} exceeds the {limit} byte contract bound")]
    PayloadTooLarge {
        /// Which payload field was oversized.
        field: &'static str,
        /// The maximum allowed serialized size in bytes.
        limit: usize,
    },

    /// A structured payload nested deeper than the contract allows.
    #[error("{field} nests deeper than {limit} levels")]
    PayloadTooDeep {
        /// Which payload field was too deeply nested.
        field: &'static str,
        /// The maximum allowed nesting depth.
        limit: u8,
    },

    /// A readiness transition was illegal from the binding's current state.
    #[error("illegal readiness transition: cannot apply `{cause}` from {current}")]
    IllegalTransition {
        /// The state the binding is currently in.
        current: ReadinessState,
        /// The transition cause that was rejected.
        cause: &'static str,
    },

    /// An operation targeted a reserved execution environment.
    #[error("execution environment {environment} is reserved: {reason}")]
    ReservedEnvironment {
        /// The reserved environment that was used.
        environment: ExecutionEnvironment,
        /// Why the environment cannot be used yet.
        reason: &'static str,
    },

    /// An adapter identity was registered twice.
    #[error("duplicate adapter registration: {adapter}")]
    DuplicateAdapter {
        /// The adapter that was already registered.
        adapter: AdapterId,
    },

    /// A capability binding identity is not registered.
    #[error("unknown capability binding `{binding}`")]
    UnknownBinding {
        /// The binding that was looked up.
        binding: CapabilityBindingId,
    },

    /// A binding was not in the state the operation requires.
    #[error("binding `{binding}` is {state}, expected {expected}")]
    BindingNotReady {
        /// The binding that was operated on.
        binding: CapabilityBindingId,
        /// The binding's current readiness state.
        state: ReadinessState,
        /// The state the operation requires.
        expected: ReadinessState,
    },

    /// An authorization grant was issued for a different binding.
    #[error("authorization grant subject `{granted}` does not match binding `{requested}`")]
    AuthorizationSubjectMismatch {
        /// The binding the grant was issued for.
        granted: CapabilityBindingId,
        /// The binding the grant was presented to.
        requested: CapabilityBindingId,
    },

    /// Evidence that must reference an approval decision did not.
    #[error("approval evidence is required, got evidence kind {actual:?}")]
    ExpectedApprovalEvidence {
        /// The evidence kind that was supplied instead.
        actual: EvidenceKind,
    },

    /// Required resource types have no bound resource.
    #[error("binding `{binding}` is missing required resources: {}", missing.iter().map(AsRef::as_ref).collect::<Vec<_>>().join(", "))]
    ResourceRequirementMissing {
        /// The binding whose resource requirements are unmet.
        binding: CapabilityBindingId,
        /// The resource types with no binding.
        missing: Vec<ResourceTypeId>,
    },

    /// A contract record violated its consistency rules.
    #[error("invalid execution contract record: {reason}")]
    InvalidRecord {
        /// Which consistency rule was violated.
        reason: String,
    },

    /// A binding policy was structurally invalid.
    #[error("invalid binding policy: {reason}")]
    InvalidPolicy {
        /// Why the policy was rejected.
        reason: String,
    },

    /// Serialization or deserialization of a contract record failed.
    #[error("execution contract serialization failure: {source}")]
    Serialization {
        /// The underlying serde failure.
        #[source]
        source: serde_json::Error,
    },

    /// A workflow contract operation failed while integrating with the
    /// WO-003 contract surface (for example digesting a record).
    #[error("workflow contract failure: {source}")]
    WorkflowContract {
        /// The underlying workflow contract failure.
        #[source]
        source: WorkflowContractError,
    },
}

impl From<serde_json::Error> for ExecutionContractError {
    fn from(source: serde_json::Error) -> Self {
        Self::Serialization { source }
    }
}

impl From<WorkflowContractError> for ExecutionContractError {
    fn from(source: WorkflowContractError) -> Self {
        Self::WorkflowContract { source }
    }
}

#[cfg(test)]
#[path = "error_tests.rs"]
mod tests;
