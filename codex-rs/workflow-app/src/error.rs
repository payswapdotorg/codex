//! Errors surfaced by the workflow application layer.
//!
//! Application-layer failures are either control-plane contract failures
//! (versions, instances, approvals), execution-plane diagnostics surfaced
//! as records ([`BindingDiagnostic`]), or composition failures of this
//! crate's own seams. Nothing here panics or invents semantics: every
//! variant names the plane that owns the failure.

use codex_browser_use_adapter::AdapterError;
use codex_computer_use_adapter::ComputerUseAdapterError;
use codex_execution_contracts::DiagnosticCode;
use codex_execution_contracts::ExecutionContractError;
use codex_teaching_compiler::TeachingCompilerError;
use codex_workflow_contracts::WorkflowContractError;
use codex_workflow_contracts::WorkflowInstanceStatus;
use codex_workflow_forge::WorkflowForgeError;
use thiserror::Error;

/// A failure produced while driving the workflow application lifecycle.
#[derive(Debug, Error)]
pub enum WorkflowAppError {
    /// No workflow version has been selected, so the requested operation
    /// has nothing to act on. This is the ordinary-Codex no-op path: the
    /// application performs no workflow work at all.
    #[error("no workflow is active; select a version before running workflow operations")]
    NoActiveWorkflow,
    /// The requested workflow version is not present in the version store.
    #[error("workflow version `{version}` is not available in the version store")]
    VersionUnavailable {
        /// The version identity that was requested.
        version: String,
    },
    /// A workflow version failed end-to-end integrity verification.
    ///
    /// Published versions are immutable; a record whose digests do not
    /// recompute is tampered or corrupted and must never execute.
    #[error("workflow version `{version}` failed integrity verification: {reason}")]
    VersionIntegrity {
        /// The version identity that failed verification.
        version: String,
        /// Why verification failed.
        reason: String,
    },
    /// A workflow instance record could not be found.
    #[error("workflow instance `{instance}` is not present in the instance store")]
    InstanceUnavailable {
        /// The instance identity that was requested.
        instance: String,
    },
    /// A workflow instance record already exists.
    #[error("workflow instance `{instance}` already exists in the instance store")]
    InstanceAlreadyExists {
        /// The duplicate instance identity.
        instance: String,
    },
    /// An instance status transition the frozen control-plane table
    /// forbids: the record's current status cannot legally move to the
    /// requested target (a double-cancel, a cancel of an already-settled
    /// instance, or any settlement the legal-transition table refuses).
    /// Never a silent no-op.
    #[error("workflow instance `{instance}` cannot transition from `{current:?}` to `{target:?}`")]
    IllegalStatusTransition {
        /// The instance whose transition was refused.
        instance: String,
        /// The instance's current status.
        current: WorkflowInstanceStatus,
        /// The refused target status.
        target: WorkflowInstanceStatus,
    },
    /// Capability resolution or planning failed with a structured
    /// execution-plane diagnostic.
    #[error("binding plan failed: {code:?}: {message}")]
    BindingPlanFailed {
        /// The structured diagnostic code.
        code: DiagnosticCode,
        /// The diagnostic message.
        message: String,
    },
    /// The approval plane refused to authorize a binding.
    #[error("approval for binding `{binding}` was denied: {reason}")]
    ApprovalDenied {
        /// The binding that was refused.
        binding: String,
        /// Why the approval was denied.
        reason: String,
    },
    /// The action source could not propose an action for a step.
    #[error("no action available for step `{node}` (capability `{capability}`): {reason}")]
    ActionUnavailable {
        /// The step node the action was requested for.
        node: String,
        /// The capability the action was requested for.
        capability: String,
        /// Why no action is available.
        reason: String,
    },
    /// A binding resolution skipped the preferred candidate without
    /// selecting a replacement.
    #[error("binding resolution failed for capability `{capability}`: {reason}")]
    ResolutionFailed {
        /// The capability that could not be bound.
        capability: String,
        /// Why resolution failed.
        reason: String,
    },
    /// An execution-contracts failure.
    #[error(transparent)]
    Execution(#[from] ExecutionContractError),
    /// A workflow-contracts failure.
    #[error(transparent)]
    Contract(#[from] WorkflowContractError),
    /// A workflow-forge failure.
    #[error(transparent)]
    Forge(#[from] WorkflowForgeError),
    /// A teaching-compiler failure.
    #[error(transparent)]
    Teaching(#[from] TeachingCompilerError),
    /// Serializing an evidence or adapter payload failed.
    #[error("serialization failure: {0}")]
    Serialization(#[from] serde_json::Error),
    /// A browser-use adapter failure.
    #[error(transparent)]
    BrowserAdapter(#[from] AdapterError),
    /// A computer-use adapter failure.
    #[error(transparent)]
    ComputerAdapter(#[from] ComputerUseAdapterError),
}
