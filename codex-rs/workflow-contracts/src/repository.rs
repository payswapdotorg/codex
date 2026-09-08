//! Workflow repositories, collaboration records, and the forge abstraction.
//!
//! A `WorkflowRepository` is the Git-native home of workflows: identity,
//! forge kind, fork lineage, default branch, manifests, and branch state.
//! Collaboration records (`WorkflowReview`, `WorkflowRelease`) model
//! fork/branch/review/release as forge-neutral concepts.
//!
//! **Forge abstraction boundary.** `WorkflowForge` is the only interface
//! through which the workflow platform talks to a hosting forge. The trait
//! is deliberately minimal and read-oriented: implementations resolve
//! development refs to immutable revisions and read repository metadata.
//! Write-side collaboration (proposing and merging reviews, publishing
//! releases) is owned by the workflow collaboration Work Order (WO-009),
//! which binds this trait to GitHub without leaking forge-specific types
//! into workflow semantics.
//!
//! Nothing in this module may reference a forge-specific API, URL shape, or
//! record model. GitHub is expected to be the first `ForgeKind` adapter,
//! not the semantic authority.

use serde::Deserialize;
use serde::Serialize;

use crate::DevelopmentRef;
use crate::ForgeKind;
use crate::ImmutableSourceRevision;
use crate::RevisionSha;
use crate::WorkflowContractError;
use crate::WorkflowDefinitionId;
use crate::WorkflowManifest;
use crate::WorkflowRepositoryId;
use crate::WorkflowVersionId;

/// Snapshot of one development branch's state.
///
/// The head SHA is development-time state: it moves. It is recorded for
/// development tooling and never participates in execution identity.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkflowBranchState {
    /// The development branch.
    pub branch: DevelopmentRef,
    /// Current head of the branch at snapshot time, when known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub head: Option<RevisionSha>,
}

/// Identity of a forge-neutral review (pull request equivalent).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct ReviewId(String);

/// Lifecycle state of a forge-neutral review.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum ReviewState {
    /// The review is open for discussion and iteration.
    Open,
    /// The review was merged; `merge_revision` is the immutable merge
    /// commit.
    Merged {
        /// The immutable merge commit.
        merge_revision: RevisionSha,
    },
    /// The review was closed without merging.
    Closed,
}

/// A forge-neutral review of proposed workflow changes.
///
/// Both base and head are immutable revisions: the review records where it
/// started and where the proposal stood, independent of branch movement.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkflowReview {
    /// The review's identity, allocated by the forge adapter.
    pub review: ReviewId,
    /// Repository the review belongs to.
    pub repository: WorkflowRepositoryId,
    /// Human-facing title.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Immutable revision the proposal is based on.
    pub base: ImmutableSourceRevision,
    /// Immutable revision the proposal heads at.
    pub head: ImmutableSourceRevision,
    /// Lifecycle state.
    pub state: ReviewState,
}

/// A forge-neutral release of workflow versions.
///
/// The tag name is a display/provenance handle: tags can be re-pointed by
/// forges. The release's authority is its immutable target revision and the
/// immutable version identities published from it.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkflowRelease {
    /// Release tag name (display handle, not execution authority).
    pub tag: String,
    /// Repository the release belongs to.
    pub repository: WorkflowRepositoryId,
    /// Immutable revision the release was cut at.
    pub target: ImmutableSourceRevision,
    /// Immutable version identities published by this release.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub versions: Vec<WorkflowVersionId>,
}

/// A workflow repository: the Git-native home of one or more workflows.
///
/// This record is a development-time snapshot. Fork lineage, branch state,
/// and manifests describe where development happens; execution always pins
/// immutable revisions through `WorkflowVersion`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkflowRepository {
    /// Canonical, credential-free repository identity.
    pub identity: WorkflowRepositoryId,
    /// Forge kind hosting the repository (opaque data, not semantics).
    pub forge: ForgeKind,
    /// Origin remote, when the repository was cloned or forked from one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub origin: Option<WorkflowRepositoryId>,
    /// Fork lineage, when the repository is a fork.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub forked_from: Option<crate::ForkLineage>,
    /// Default development branch.
    pub default_branch: DevelopmentRef,
    /// Known development branch states (snapshot, moves freely).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub branches: Vec<WorkflowBranchState>,
    /// Workflows declared by manifests in this repository.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub workflows: Vec<WorkflowManifest>,
    /// Maintainer attributions.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub maintainers: Vec<crate::Attribution>,
}

impl WorkflowRepository {
    /// Finds the manifest declaring `workflow`, when present.
    pub fn manifest_for(&self, workflow: &WorkflowDefinitionId) -> Option<&WorkflowManifest> {
        self.workflows
            .iter()
            .find(|manifest| &manifest.workflow == workflow)
    }
}

/// The forge abstraction: the boundary between workflow semantics and
/// hosting forges.
///
/// Implementations are provided by the workflow collaboration Work Order
/// (WO-009); GitHub is expected to be the first implementation. The trait
/// is intentionally read-oriented and forge-neutral:
///
/// - `resolve_ref` converts movable development refs into immutable
///   revisions, enforcing the development/executable identity separation;
/// - `repository_record` reads repository metadata, including manifests.
///
/// Implementations must:
///
/// - never surface forge-specific records or errors through this interface;
/// - resolve refs to full commit SHAs (never abbreviated SHAs);
/// - treat forge responses as untrusted input and validate them against
///   these contract types before returning.
pub trait WorkflowForge: Send + Sync {
    /// The forge kind this implementation services.
    fn kind(&self) -> &ForgeKind;

    /// Resolves a movable development ref to an immutable revision.
    fn resolve_ref(
        &self,
        repository: &WorkflowRepositoryId,
        reference: &DevelopmentRef,
    ) -> impl Future<Output = Result<ImmutableSourceRevision, WorkflowContractError>> + Send;

    /// Reads a repository's development-time record.
    fn repository_record(
        &self,
        repository: &WorkflowRepositoryId,
    ) -> impl Future<Output = Result<WorkflowRepository, WorkflowContractError>> + Send;
}

impl ReviewId {
    /// Parses a review identifier.
    pub fn parse(value: impl Into<String>) -> Result<Self, WorkflowContractError> {
        let value = value.into();
        if value.is_empty() {
            Err(WorkflowContractError::InvalidIdentifier {
                kind: "review id",
                value,
                reason: "must be non-empty",
            })
        } else {
            Ok(Self(value))
        }
    }
}

impl TryFrom<String> for ReviewId {
    type Error = WorkflowContractError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl TryFrom<&str> for ReviewId {
    type Error = WorkflowContractError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::parse(value.to_owned())
    }
}

impl From<ReviewId> for String {
    fn from(value: ReviewId) -> Self {
        value.0
    }
}

impl AsRef<str> for ReviewId {
    fn as_ref(&self) -> &str {
        self.0.as_str()
    }
}

#[cfg(test)]
#[path = "repository_tests.rs"]
mod tests;
