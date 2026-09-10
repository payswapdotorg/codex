//! Errors surfaced by the WO-015 environment adapters.

use codex_execution_contracts::ExecutionContractError;
use codex_workflow_contracts::WorkflowContractError;
use thiserror::Error;

/// A failure produced while constructing or operating one of the WO-015
/// environment adapters.
#[derive(Debug, Error)]
pub enum EnvAdapterError {
    /// An execution contract operation failed: identifier parsing,
    /// descriptor validation, or record validation.
    #[error("execution contract failure: {source}")]
    Contract {
        /// The underlying contract failure.
        #[source]
        source: ExecutionContractError,
    },
    /// A workflow contract operation failed while parsing the WO-003
    /// identifiers the adapters build on (capabilities, resource types).
    #[error("workflow contract failure: {source}")]
    WorkflowContract {
        /// The underlying workflow contract failure.
        #[source]
        source: WorkflowContractError,
    },
    /// Serializing an evidence payload for the adapter journal failed.
    #[error("evidence serialization failure: {source}")]
    Evidence {
        /// The underlying serde failure.
        #[source]
        source: serde_json::Error,
    },
    /// A session takeover was refused by policy or by the request shape.
    #[error("session takeover denied: {reason}")]
    TakeoverDenied {
        /// Why the takeover was refused.
        reason: String,
    },
}

impl From<ExecutionContractError> for EnvAdapterError {
    fn from(source: ExecutionContractError) -> Self {
        Self::Contract { source }
    }
}

impl From<WorkflowContractError> for EnvAdapterError {
    fn from(source: WorkflowContractError) -> Self {
        Self::WorkflowContract { source }
    }
}

impl From<serde_json::Error> for EnvAdapterError {
    fn from(source: serde_json::Error) -> Self {
        Self::Evidence { source }
    }
}
