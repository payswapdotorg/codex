//! Errors surfaced by the teaching compiler.
//!
//! Errors are data: every failure names the exact invariant that was
//! violated so operators can act on it. The compiler never invents retry
//! semantics; recovery decisions belong to the session host and the
//! control plane.

use codex_workflow_contracts::WorkflowContractError;
use thiserror::Error;

/// Errors produced while recording, compiling, validating, simulating,
/// optimizing, approving, or finalizing taught workflows.
#[derive(Debug, Error)]
pub enum TeachingCompilerError {
    /// A lifecycle transition was attempted from an illegal state.
    #[error("invalid state: current `{current}`, requires {required}")]
    InvalidState {
        /// The state the candidate or session is actually in.
        current: String,
        /// The state the operation requires.
        required: String,
    },
    /// An approval input was structurally invalid.
    #[error("invalid approval: {reason}")]
    InvalidApproval {
        /// Why the approval was rejected.
        reason: String,
    },
    /// The approval record was bound to a different content epoch.
    #[error(
        "approval epoch mismatch: approval was recorded at epoch {approval_epoch}, \
         candidate is at epoch {candidate_epoch}"
    )]
    ApprovalEpochMismatch {
        /// Content epoch the approval was recorded against.
        approval_epoch: u64,
        /// Current content epoch of the candidate.
        candidate_epoch: u64,
    },
    /// Finalization requires a simulation of the current content epoch.
    #[error("no simulation of the current content epoch is available")]
    MissingSimulation,
    /// The simulation was run against a different content epoch.
    #[error(
        "simulation epoch mismatch: simulation was run at epoch {simulation_epoch}, \
         candidate is at epoch {candidate_epoch}"
    )]
    SimulationEpochMismatch {
        /// Content epoch the simulation was run against.
        simulation_epoch: u64,
        /// Current content epoch of the candidate.
        candidate_epoch: u64,
    },
    /// A trajectory event failed session-level consistency or bound checks.
    #[error("invalid trajectory event: {reason}")]
    InvalidEvent {
        /// Why the event was rejected.
        reason: String,
    },
    /// An evidence reference failed its structural validation.
    #[error("invalid evidence: {reason}")]
    InvalidEvidence {
        /// Why the evidence reference was rejected.
        reason: String,
    },
    /// The session contained no records to compile.
    #[error("the teaching session is empty")]
    EmptySession,
    /// Compilation requires the session to be closed first.
    #[error("the teaching session is still open; close it before compiling")]
    SessionNotClosed,
    /// The session's trajectory bound was exceeded.
    #[error("trajectory bound exceeded: at most {limit} records are accepted")]
    TrajectoryBoundExceeded {
        /// The configured maximum number of records.
        limit: usize,
    },
    /// The requested optimization is forbidden: policy disabled it, or it
    /// would not preserve step evidence.
    #[error("optimization forbidden: {reason}")]
    OptimizationForbidden {
        /// Why the optimization was rejected.
        reason: String,
    },
    /// The optimization proposal does not match the candidate content.
    #[error(
        "stale optimization proposal: proposal was built at epoch {proposal_epoch}, \
         candidate is at epoch {candidate_epoch}"
    )]
    StaleOptimizationProposal {
        /// Content epoch the proposal was computed against.
        proposal_epoch: u64,
        /// Current content epoch of the candidate.
        candidate_epoch: u64,
    },
    /// A workflow contract validation failed.
    #[error("workflow contract error: {0}")]
    Contract(#[from] WorkflowContractError),
    /// The simulation exceeded its step budget, indicating a graph that
    /// may not terminate.
    #[error("simulation exceeded its step budget of {budget}; the graph may not terminate")]
    SimulationStepBudgetExceeded {
        /// The configured step budget.
        budget: u64,
    },
    /// A node id required by the compiler is invalid or missing.
    #[error("invalid node id `{node_id}`: {reason}")]
    InvalidNodeId {
        /// The rejected node id.
        node_id: String,
        /// Why it was rejected.
        reason: String,
    },
    /// The session mode does not accept the recorded origin.
    #[error("mode `{mode}` does not accept {origin} records")]
    ModeOriginMismatch {
        /// The session's teaching mode.
        mode: String,
        /// The rejected record origin.
        origin: String,
    },
    /// The teaching input produced no compilable steps.
    #[error("teaching input produced no steps: {reason}")]
    NoCompilableSteps {
        /// Why no steps could be derived.
        reason: String,
    },
}
