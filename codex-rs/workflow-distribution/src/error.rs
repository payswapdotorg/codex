//! Errors surfaced by the workflow distribution plane.
//!
//! Distribution failures name the plane that owns them: publication and
//! metadata integrity failures carry structured reasons, refusals carry
//! the decision data that produced them, and failures of the frozen
//! crates below (contracts, forge) project transparently. Nothing here
//! panics, invents semantics, or embeds credentials.

use codex_workflow_contracts::WorkflowContractError;
use codex_workflow_forge::WorkflowForgeError;
use thiserror::Error;

use crate::AccessDecision;
use crate::EntitlementDecision;

/// A failure produced while driving workflow distribution.
#[derive(Debug, Error)]
pub enum WorkflowDistributionError {
    /// The caller named a release this plane never published.
    #[error("no published release for workflow `{workflow}` at version `{version}`")]
    UnknownRelease {
        /// The workflow that was requested.
        workflow: String,
        /// The version identity that was requested.
        version: String,
    },
    /// The caller named a workflow this plane never installed.
    #[error("workflow `{workflow}` is not installed")]
    NotInstalled {
        /// The workflow that was requested.
        workflow: String,
    },
    /// The workflow is already installed: distribution installs are
    /// pinned and changes must flow through the explicit, reviewable
    /// upgrade path.
    #[error("workflow `{workflow}` is already installed at version `{version}`")]
    AlreadyInstalled {
        /// The workflow that is already installed.
        workflow: String,
        /// The immutable version it is pinned to.
        version: String,
    },
    /// The same immutable version was submitted for publication twice:
    /// published releases are never silently overwritten.
    #[error("version `{version}` of workflow `{workflow}` is already published")]
    AlreadyPublished {
        /// The workflow whose release already exists.
        workflow: String,
        /// The immutable version identity already published.
        version: String,
    },
    /// The requesting principal cannot see the release (visibility
    /// scope), so the release does not exist for them.
    #[error(
        "release `{version}` of workflow `{workflow}` is not visible to the requesting principal"
    )]
    ReleaseNotVisible {
        /// The workflow whose release is invisible.
        workflow: String,
        /// The immutable version identity whose entry is out of scope.
        version: String,
    },
    /// An install was refused by the access policy (license or sharing
    /// terms). The denial is decision data, never a semantic change:
    /// no record was mutated.
    #[error("install of `{workflow}` refused by the access policy")]
    InstallRefused {
        /// The workflow whose install was refused.
        workflow: String,
        /// The access decision that refused it.
        access: AccessDecision,
        /// The entitlement decision that refused it, when the release
        /// carries commercial terms.
        entitlement: Option<EntitlementDecision>,
    },
    /// A publication metadata document failed validation or integrity.
    #[error("invalid publication metadata: {reason}")]
    InvalidMetadata {
        /// Why the document was rejected.
        reason: String,
    },
    /// Commercial terms failed validation or integrity.
    #[error("invalid commercial terms: {reason}")]
    InvalidCommercialTerms {
        /// Why the terms were rejected.
        reason: String,
    },
    /// A distribution-state transition is illegal for the record's
    /// current state.
    #[error("illegal distribution transition `{transition}` from `{from}`")]
    IllegalTransition {
        /// The transition that was attempted.
        transition: String,
        /// The state the record is in.
        from: String,
    },
    /// An upgrade proposal no longer matches the installation: it was
    /// superseded and must be re-evaluated and re-decided.
    #[error("stale upgrade proposal for `{workflow}`: expected `{expected}`, installed `{actual}`")]
    StaleUpgradeProposal {
        /// The workflow whose upgrade was proposed.
        workflow: String,
        /// The from-version the proposal was built against.
        expected: String,
        /// The version actually installed now.
        actual: String,
    },
    /// An upgrade approval was refused because the new release failed
    /// an install gate (access or entitlement). The refusal is
    /// decision data, never a semantic change: the installation keeps
    /// its pinned version and nothing was recorded.
    #[error("upgrade of `{workflow}` refused by an install gate")]
    UpgradeRefused {
        /// The workflow whose upgrade was refused.
        workflow: String,
        /// The access decision that refused it.
        access: AccessDecision,
        /// The entitlement decision that refused it, when the new
        /// release carries commercial terms.
        entitlement: Option<EntitlementDecision>,
    },
    /// A structurally invalid record reached this plane (for example a
    /// fork request whose repository collides with its upstream).
    #[error("invalid distribution record: {reason}")]
    InvalidRecord {
        /// Why the record was rejected.
        reason: String,
    },
    /// A frozen workflow contract failed underneath this plane.
    #[error(transparent)]
    Contract(#[from] WorkflowContractError),
    /// The forge layer failed underneath this plane.
    #[error(transparent)]
    Forge(#[from] WorkflowForgeError),
}
