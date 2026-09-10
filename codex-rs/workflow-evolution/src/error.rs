//! The error channel of the workflow-evolution plane.
//!
//! Every refusal in this crate is a governed transition failure: the error
//! names the candidate, version, or record that was refused and why, so a
//! host can surface the decision to its control plane instead of guessing.

use codex_workflow_app::WorkflowAppError;
use codex_workflow_contracts::WorkflowContractError;
use codex_workflow_contracts::WorkflowDefinitionId;
use codex_workflow_contracts::WorkflowVersionId;
use codex_workflow_forge::WorkflowForgeError;
use codex_workflow_triggers::WorkflowTriggerError;
use thiserror::Error;

/// Errors produced by learning and governed evolution.
#[derive(Debug, Error)]
pub enum WorkflowEvolutionError {
    /// A frozen contract refused the operation.
    #[error("workflow contract refused: {0}")]
    Contract(#[from] WorkflowContractError),
    /// The forge layer refused the operation.
    #[error("workflow forge refused: {0}")]
    Forge(#[from] WorkflowForgeError),
    /// The application runtime refused the operation (for example a replay
    /// harness failure).
    #[error("workflow application refused: {0}")]
    App(#[from] WorkflowAppError),
    /// A schedule specification was invalid.
    #[error("schedule refused: {0}")]
    Trigger(#[from] WorkflowTriggerError),
    /// A record could not be serialized into its canonical form.
    #[error("serialization refused: {0}")]
    Serialization(#[from] serde_json::Error),
    /// The referenced candidate is unknown to the ledger.
    #[error("candidate `{candidate}` is not recorded")]
    CandidateNotFound {
        /// The candidate that was looked up.
        candidate: String,
    },
    /// A candidate with the same identity was already recorded; candidates
    /// are content-addressed, so a duplicate means the same proposal
    /// already exists.
    #[error("candidate `{candidate}` is already recorded")]
    CandidateAlreadyKnown {
        /// The duplicate candidate identity.
        candidate: String,
    },
    /// The candidate was generated against an older incumbent version and
    /// has been superseded by a newer installed version.
    #[error("candidate `{candidate}` is stale: generated for {expected}, installed is {actual}")]
    StaleCandidate {
        /// The stale candidate.
        candidate: String,
        /// The incumbent version the candidate was generated for.
        expected: WorkflowVersionId,
        /// The version actually installed now.
        actual: WorkflowVersionId,
    },
    /// The candidate is structurally invalid (for example a binding change
    /// that targets a node which is not a step, or a definition whose
    /// workflow identity differs from the incumbent).
    #[error("candidate `{candidate}` is invalid: {reason}")]
    InvalidCandidate {
        /// The invalid candidate.
        candidate: String,
        /// Why it was refused.
        reason: String,
    },
    /// The proposed change would reproduce the incumbent version content
    /// exactly, so there is nothing to evolve.
    #[error(
        "candidate `{candidate}` is a no-op: the applied change reproduces the incumbent content"
    )]
    NoopEvolution {
        /// The no-op candidate.
        candidate: String,
    },
    /// Credential-shaped content was detected; credentials never enter
    /// candidates, approvals, or lineage records.
    #[error("credential-shaped content was refused in {field} ({findings} findings)")]
    CredentialContamination {
        /// Which part of the record carried the finding.
        field: &'static str,
        /// How many findings the scrubber recorded.
        findings: usize,
    },
    /// The candidate's provenance exceeds the retention bound for evidence
    /// references.
    #[error(
        "candidate `{candidate}` retains {count} evidence references, above the policy cap of {cap}"
    )]
    ProvenanceTooLarge {
        /// The oversized candidate.
        candidate: String,
        /// The reference count it carried.
        count: usize,
        /// The configured maximum.
        cap: usize,
    },
    /// Validation failed: at least one gate did not pass. Publication is
    /// refused until every gate passes explicitly.
    #[error("candidate `{candidate}` failed validation: {failed}")]
    ValidationFailed {
        /// The candidate that failed.
        candidate: String,
        /// The gates that failed, by stage name.
        failed: String,
    },
    /// Publication was attempted before validation ran.
    #[error("candidate `{candidate}` has no validation report yet")]
    ValidationNotRun {
        /// The unvalidated candidate.
        candidate: String,
    },
    /// Publication requires an explicit approval record for the candidate.
    #[error("candidate `{candidate}` has no approval decision")]
    ApprovalRequired {
        /// The unapproved candidate.
        candidate: String,
    },
    /// The recorded approval decision rejected the candidate.
    #[error("candidate `{candidate}` was rejected by approver `{approver}`")]
    ApprovalRejected {
        /// The rejected candidate.
        candidate: String,
        /// The principal that rejected it.
        approver: String,
    },
    /// The approval record does not bind to this candidate's validation
    /// outcome; an approval for one validation never transfers to another.
    #[error("approval for candidate `{candidate}` does not match its validation outcome")]
    ApprovalMismatch {
        /// The mismatched candidate.
        candidate: String,
    },
    /// The candidate has already been promoted; lineage records are
    /// append-only history.
    #[error("candidate `{candidate}` was already promoted")]
    CandidateAlreadyPromoted {
        /// The already-promoted candidate.
        candidate: String,
    },
    /// The candidate has not been promoted yet, so it has no successor
    /// version to install or roll back to.
    #[error("candidate `{candidate}` has not been promoted")]
    CandidateNotPromoted {
        /// The unpublished candidate.
        candidate: String,
    },
    /// The workflow's installation (or its version record) is not
    /// available to the governor: governed evolution only operates on
    /// installed, verifiable incumbents.
    #[error("workflow `{workflow}` has no installed incumbent available")]
    IncumbentNotInstalled {
        /// The workflow without an available incumbent.
        workflow: WorkflowDefinitionId,
    },
    /// A lineage record for the successor version already exists; lineage
    /// is immutable history and cannot be overwritten.
    #[error("lineage for successor `{successor}` is already recorded")]
    LineageAlreadyRecorded {
        /// The successor version whose lineage exists.
        successor: WorkflowVersionId,
    },
    /// The workflow has no lineage to roll back to (it was never evolved
    /// through this plane).
    #[error("workflow `{workflow}` has no recorded lineage to roll back to")]
    NoPredecessorToRollback {
        /// The workflow without lineage.
        workflow: WorkflowDefinitionId,
    },
}
