//! Errors surfaced by the workflow forge layer.
//!
//! [`WorkflowForgeError`] is the error channel for collaboration
//! transitions, packaging, installation, and adapter mapping. The frozen
//! `WorkflowForge` trait returns `WorkflowContractError`, so
//! [`WorkflowForgeError::as_contract_error`] projects this type onto the
//! frozen contract error enum at that boundary.

use codex_workflow_contracts::DevelopmentRef;
use codex_workflow_contracts::ReviewId;
use codex_workflow_contracts::RevisionSha;
use codex_workflow_contracts::SubworkflowDependencyId;
use codex_workflow_contracts::WorkflowContractError;
use codex_workflow_contracts::WorkflowDefinitionId;
use codex_workflow_contracts::WorkflowRepositoryId;
use codex_workflow_contracts::WorkflowVersionId;

/// Errors produced by workflow-forge collaboration, packaging,
/// installation, and adapter logic.
///
/// The type is exhaustive within this crate but marked
/// `#[non_exhaustive]` so later work orders can extend the failure
/// surface without breaking downstream matches.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum WorkflowForgeError {
    /// A frozen contract invariant was violated while building or
    /// validating a forge-layer record.
    #[error("workflow contract error: {0}")]
    Contract(#[from] WorkflowContractError),

    /// A record could not be deserialized. External outputs are
    /// untrusted: deserialization failures are surfaced, never guessed
    /// around.
    #[error("record deserialization failed: {0}")]
    Serde(#[from] serde_json::Error),

    /// A record supplied to the forge layer failed validation. External
    /// outputs are untrusted: mapping rejects invalid records instead of
    /// repairing them.
    #[error("invalid forge-layer record: {reason}")]
    InvalidRecord {
        /// Why the record was rejected.
        reason: String,
    },

    /// A release tag failed validation.
    #[error("invalid release tag {tag:?}: {reason}")]
    InvalidTag {
        /// The offending tag.
        tag: String,
        /// Why the tag was rejected.
        reason: &'static str,
    },

    /// The repository is unknown to the forge layer.
    #[error("unknown repository {repository:?}")]
    UnknownRepository {
        /// The repository that was looked up.
        repository: WorkflowRepositoryId,
    },

    /// The development ref is unknown on the repository.
    #[error("unknown {branch} on repository {repository:?}")]
    UnknownBranch {
        /// The repository that was looked up.
        repository: WorkflowRepositoryId,
        /// The branch or tag that could not be resolved.
        branch: DevelopmentRef,
    },

    /// The commit is unknown to the forge layer.
    #[error("unknown commit {commit} on repository {repository:?}")]
    UnknownCommit {
        /// The repository that was looked up.
        repository: WorkflowRepositoryId,
        /// The commit SHA that is not present.
        commit: RevisionSha,
    },

    /// A fork was requested with an identity that collides with its
    /// origin.
    #[error("fork identity {identity:?} collides with its origin")]
    ForkIdentityCollision {
        /// The colliding fork identity.
        identity: WorkflowRepositoryId,
    },

    /// A review was proposed whose head equals its base.
    #[error("review {review:?} head {head} equals base {base}")]
    ReviewHeadEqualsBase {
        /// The review that was rejected.
        review: ReviewId,
        /// The immutable base revision.
        base: RevisionSha,
        /// The immutable head revision, equal to the base.
        head: RevisionSha,
    },

    /// A merge was attempted on a review that is already merged.
    #[error("review {review:?} is already merged at {merge_revision}")]
    ReviewAlreadyMerged {
        /// The review that was already merged.
        review: ReviewId,
        /// The immutable merge commit of the earlier merge.
        merge_revision: RevisionSha,
    },

    /// A close or merge was attempted on a review that is already closed.
    #[error("review {review:?} is already closed")]
    ReviewAlreadyClosed {
        /// The review that was already closed.
        review: ReviewId,
    },

    /// A merge was attempted without an approval recorded at the review's
    /// current head.
    #[error("review {review:?} has no approval at head {head}")]
    MissingApprovalAtHead {
        /// The review that lacks a fresh approval.
        review: ReviewId,
        /// The head revision that needed an approval.
        head: RevisionSha,
    },

    /// The supplied merge revision is not a new immutable revision
    /// distinct from the review's base and head.
    #[error("invalid merge revision {merge_revision} for review {review:?}")]
    InvalidMergeRevision {
        /// The review being merged.
        review: ReviewId,
        /// The rejected merge revision.
        merge_revision: RevisionSha,
    },

    /// A release tag already exists for the repository. Re-publishing a
    /// tag is an explicit control-plane operation, never a silent
    /// overwrite.
    #[error("release tag {tag:?} already exists for repository {repository:?}")]
    DuplicateReleaseTag {
        /// The repository the tag belongs to.
        repository: WorkflowRepositoryId,
        /// The duplicated tag.
        tag: String,
    },

    /// A release would publish two versions of the same workflow, which
    /// would make the release ambiguous.
    #[error("release tag {tag:?} would publish workflow {workflow:?} twice")]
    DuplicateReleaseWorkflow {
        /// The workflow that appears twice.
        workflow: WorkflowDefinitionId,
        /// The release tag that was rejected.
        tag: String,
    },

    /// A version reference's recorded id does not match the recomputed
    /// digest of its execution identity tuple.
    #[error("version identity mismatch for workflow {workflow:?}: recorded {recorded}, recomputed {recomputed}")]
    VersionIdentityMismatch {
        /// The workflow whose version failed verification.
        workflow: WorkflowDefinitionId,
        /// The recorded version identity.
        recorded: WorkflowVersionId,
        /// The recomputed version identity.
        recomputed: WorkflowVersionId,
    },

    /// A version is not anchored at the immutable revision it is being
    /// published or pinned from.
    #[error("version for workflow {workflow:?} is anchored at {actual}, expected {expected}")]
    VersionNotAnchoredAtTarget {
        /// The workflow whose version was rejected.
        workflow: WorkflowDefinitionId,
        /// The immutable revision the version should be anchored at.
        expected: RevisionSha,
        /// The revision the version is actually anchored at.
        actual: RevisionSha,
    },

    /// A version being used to pin a subworkflow dependency is not part
    /// of the referenced release.
    #[error("dependency {dependency} cannot pin version {version}: not published by the release")]
    VersionNotInRelease {
        /// The subworkflow dependency being pinned.
        dependency: SubworkflowDependencyId,
        /// The version that is not part of the release.
        version: WorkflowVersionId,
    },

    /// A subworkflow pin's recorded fields disagree with its pinned
    /// version.
    #[error("subworkflow pin for {dependency} is inconsistent: {reason}")]
    PinInconsistent {
        /// The subworkflow dependency whose pin is inconsistent.
        dependency: SubworkflowDependencyId,
        /// What disagreed.
        reason: &'static str,
    },

    /// A dependency lock pins the same subworkflow dependency twice.
    #[error("duplicate subworkflow pin for {dependency}")]
    DuplicateSubworkflowPin {
        /// The dependency that is pinned twice.
        dependency: SubworkflowDependencyId,
    },

    /// A record declared an unsupported serialization format version.
    #[error("unsupported {kind} format version {version}")]
    UnsupportedFormatVersion {
        /// The kind of record, for example `"workflow package lock"`.
        kind: &'static str,
        /// The unsupported version that was found.
        version: u32,
    },

    /// A manifest declares a repository other than the package or
    /// repository that supplied it.
    #[error(
        "manifest belongs to repository {manifest_repository:?}, expected {expected_repository:?}"
    )]
    ManifestRepositoryMismatch {
        /// The repository the manifest declares.
        manifest_repository: WorkflowRepositoryId,
        /// The repository the manifest was supplied for.
        expected_repository: WorkflowRepositoryId,
    },

    /// The workflow is already installed; upgrades must go through the
    /// explicit update path.
    #[error("workflow {workflow:?} is already installed")]
    AlreadyInstalled {
        /// The workflow that is already installed.
        workflow: WorkflowDefinitionId,
    },

    /// The workflow is not installed, so no update can be proposed or
    /// decided.
    #[error("workflow {workflow:?} is not installed")]
    NotInstalled {
        /// The workflow that is not installed.
        workflow: WorkflowDefinitionId,
    },

    /// An update was proposed to the version that is already installed.
    #[error("update for workflow {workflow:?} is a no-op at version {version}")]
    UpdateIsNoOp {
        /// The workflow whose update is a no-op.
        workflow: WorkflowDefinitionId,
        /// The version that is already installed.
        version: WorkflowVersionId,
    },

    /// An update plan no longer matches the installed version, because a
    /// different update was decided in the meantime. Stale plans must be
    /// re-proposed and re-reviewed, never applied blindly.
    #[error("stale update plan for workflow {workflow:?}: expected from {expected_from}, installed {actual_from}")]
    StaleUpdatePlan {
        /// The workflow whose update plan is stale.
        workflow: WorkflowDefinitionId,
        /// The version the plan expected to upgrade from.
        expected_from: WorkflowVersionId,
        /// The version actually installed.
        actual_from: WorkflowVersionId,
    },
}

impl WorkflowForgeError {
    /// Projects this error onto the frozen contract error enum.
    ///
    /// `WorkflowForge` — the forge-neutral boundary frozen in
    /// `codex-workflow-contracts` — returns
    /// [`WorkflowContractError`](WorkflowContractError), so adapters must
    /// translate forge-layer failures before they cross the boundary.
    /// This projection is deliberately lossless for contract errors and
    /// digest-integrity failures, and collapses the remaining forge-layer
    /// detail into a descriptive identifier error.
    pub fn as_contract_error(self) -> WorkflowContractError {
        match self {
            Self::Contract(error) => error,
            Self::VersionIdentityMismatch { workflow, .. } => {
                WorkflowContractError::VersionIntegrity {
                    reason: format!(
                        "version id does not match the execution identity tuple for workflow {workflow:?}"
                    ),
                }
            }
            Self::Serde(error) => WorkflowContractError::InvalidIdentifier {
                kind: "forge record",
                value: error.to_string(),
                reason: "record deserialization failed",
            },
            other => WorkflowContractError::InvalidIdentifier {
                kind: "forge record",
                value: other.to_string(),
                reason: "the forge layer rejected the request or record",
            },
        }
    }
}
