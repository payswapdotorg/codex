//! Errors surfaced by the execution resource provider plane (WO-016).
//!
//! [`ResourceProviderError`] is the provider-plane failure vocabulary. It
//! wraps the WO-003/WO-005 contract errors where relevant, keeps
//! unsupported operations explicitly unsupported, and maps every variant
//! onto the execution-contract [`FailureKind`] family
//! (`Unavailable`/`Timeout`/`PolicyDenied`/`Permanent`) through
//! [`ResourceProviderError::failure_kind`] and
//! [`ResourceProviderError::normalized_failure`], so provider failure
//! flows through the **existing** recovery model (environment loss ->
//! `Unavailable` -> rebind) without mutating workflow meaning.

use codex_execution_contracts::ExecutionContractError;
use codex_execution_contracts::ExecutionFailure;
use codex_execution_contracts::FAILURE_MESSAGE_MAX_BYTES;
use codex_execution_contracts::FailureKind;
use codex_workflow_contracts::WorkflowContractError;
use thiserror::Error;

use crate::model::ProviderId;
use crate::model::ResourceId;
use crate::provider::LifecycleOp;

/// A failure produced while operating one execution resource provider.
#[derive(Debug, Error)]
pub enum ResourceProviderError {
    /// An execution contract operation failed: identifier parsing or
    /// record validation at the WO-005 boundary.
    #[error("execution contract failure: {source}")]
    Contract {
        /// The underlying contract failure.
        #[source]
        source: ExecutionContractError,
    },
    /// A workflow contract operation failed while parsing the WO-003
    /// identifiers the bind surface mints (logical resource types).
    #[error("workflow contract failure: {source}")]
    WorkflowContract {
        /// The underlying contract failure.
        #[source]
        source: WorkflowContractError,
    },
    /// The provider does not support the requested lifecycle operation.
    ///
    /// Unsupported operations stay explicitly unsupported: the error
    /// names both the provider and the operation, never a silent no-op.
    #[error("provider `{provider}` does not support operation `{operation}`")]
    Unsupported {
        /// The provider that rejected the operation.
        provider: ProviderId,
        /// The unsupported operation.
        operation: LifecycleOp,
    },
    /// The resource is gone: never known to this provider, released, or
    /// lost. Foreign handles (another provider's id) are rejected the
    /// same way, so cross-provider handle confusion is diagnosable.
    #[error("provider `{provider}` lost resource `{resource}`")]
    ResourceLost {
        /// The provider that no longer holds the resource.
        provider: ProviderId,
        /// The resource that is gone.
        resource: ResourceId,
    },
    /// The provider has no capacity left for a new resource.
    #[error("provider `{provider}` has no capacity: {detail}")]
    CapacityExhausted {
        /// The provider that is exhausted.
        provider: ProviderId,
        /// Bounded detail about the exhaustion.
        detail: String,
    },
    /// The provider endpoint cannot be reached at all.
    #[error("provider `{provider}` is unreachable: {detail}")]
    Unreachable {
        /// The provider that could not be reached.
        provider: ProviderId,
        /// Bounded detail about the outage.
        detail: String,
    },
    /// A resource spec, resize request, or record is structurally
    /// invalid — including specs that request features the provider does
    /// not declare.
    #[error("invalid resource record: {reason}")]
    InvalidSpec {
        /// Why the record was rejected.
        reason: String,
    },
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
    /// The resource is not in a state where the operation applies (for
    /// example minting a binding for a resource that is still
    /// provisioning).
    #[error(
        "resource `{resource}` of provider `{provider}` is {status}, so the operation does not apply"
    )]
    ResourceNotBindable {
        /// The provider that owns the resource.
        provider: ProviderId,
        /// The resource whose status blocks the operation.
        resource: ResourceId,
        /// The blocking resource-plane status.
        status: crate::model::ResourceStatus,
    },
    /// Serializing an evidence payload for the provider journal failed.
    #[error("evidence serialization failure: {source}")]
    Evidence {
        /// The underlying serde failure.
        #[source]
        source: serde_json::Error,
    },
}

impl ResourceProviderError {
    /// Maps the failure onto the execution-contract failure family.
    ///
    /// The mapping follows the frozen recovery pipeline: resource loss,
    /// capacity exhaustion, and outages are `Unavailable` (rebind the
    /// resource or the binding); unsupported operations and invalid
    /// records are `Permanent` (the same request will not succeed
    /// unchanged).
    pub fn failure_kind(&self) -> FailureKind {
        match self {
            Self::ResourceLost { .. }
            | Self::CapacityExhausted { .. }
            | Self::Unreachable { .. } => FailureKind::Unavailable,
            Self::Unsupported { .. }
            | Self::InvalidSpec { .. }
            | Self::InvalidIdentifier { .. }
            | Self::ResourceNotBindable { .. } => FailureKind::Permanent,
            Self::Contract { .. } | Self::WorkflowContract { .. } | Self::Evidence { .. } => {
                FailureKind::Permanent
            }
        }
    }

    /// Whether re-binding (a replacement resource or an alternate
    /// provider) is the recovery path this failure advises.
    ///
    /// This documents the environment-loss path
    /// (`ResourceLost`/`CapacityExhausted`/`Unreachable` ->
    /// `Unavailable` -> rebind) without fabricating a binding decision;
    /// the actual rebind decision stays with the WO-005 registry under
    /// policy.
    pub fn rebind_advisable(&self) -> bool {
        matches!(self.failure_kind(), FailureKind::Unavailable)
    }

    /// Builds the normalized, bounded execution failure for the contract
    /// recovery pipeline.
    ///
    /// Messages are clamped at a UTF-8 character boundary so a diagnostic
    /// is never lost because untrusted provider output was out of
    /// bounds.
    pub fn normalized_failure(&self) -> ExecutionFailure {
        ExecutionFailure {
            kind: self.failure_kind(),
            message: clamp_message(self.to_string()),
        }
    }
}

impl From<ExecutionContractError> for ResourceProviderError {
    fn from(source: ExecutionContractError) -> Self {
        Self::Contract { source }
    }
}

impl From<WorkflowContractError> for ResourceProviderError {
    fn from(source: WorkflowContractError) -> Self {
        Self::WorkflowContract { source }
    }
}

impl From<serde_json::Error> for ResourceProviderError {
    fn from(source: serde_json::Error) -> Self {
        Self::Evidence { source }
    }
}

/// Bounds a failure message, truncating at a UTF-8 character boundary and
/// marking the truncation, mirroring the WO-005 diagnostic clamp.
///
/// Unlike the diagnostic clamp (whose bound is a soft payload guide), a
/// clamped failure message must stay constructible through
/// [`ExecutionFailure::new`], which enforces
/// `FAILURE_MESSAGE_MAX_BYTES` as a hard limit — so the truncation marker
/// reserves room inside the bound instead of overflowing it.
fn clamp_message(message: String) -> String {
    const TRUNCATION_MARKER: &str = "...[truncated]";
    if message.len() <= FAILURE_MESSAGE_MAX_BYTES {
        return message;
    }
    let mut end = FAILURE_MESSAGE_MAX_BYTES - TRUNCATION_MARKER.len();
    while !message.is_char_boundary(end) {
        end -= 1;
    }
    let mut truncated = message;
    truncated.truncate(end);
    truncated.push_str(TRUNCATION_MARKER);
    truncated
}

#[cfg(test)]
#[path = "error_tests.rs"]
mod tests;
