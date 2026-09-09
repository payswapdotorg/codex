//! GitHub-shaped forge records and the GitHub adapter.
//!
//! This module is the **only** place GitHub concepts exist in the
//! workflow platform. GitHub records are pure data supplied by the host
//! integration through [`GithubRecordSource`]; this crate performs no
//! network I/O and holds no credentials. Mapping validates every record
//! against the frozen contract types before it is returned — external
//! outputs are untrusted — so nothing GitHub-shaped ever leaks into the
//! universal collaboration semantics, which accept only contract types.
//!
//! The adapter defines GitHub's canonical repository identity format,
//! `host/owner/repository` (for example `github.com/acme/ops-workflows`).
//! That format is adapter-owned: universal semantics treat repository
//! identity as opaque and round-trip it through canonical helpers.
//!
//! Write-side collaboration (proposing and merging reviews, publishing
//! releases) is performed by the host against the GitHub API and then
//! mapped through [`map_pull_request`] and [`map_release`] into contract
//! records; the pure semantics live in [`crate::collaboration`].

use std::future::ready;
use std::future::Future;
use std::sync::Arc;

use codex_workflow_contracts::Attribution;
use codex_workflow_contracts::DevelopmentRef;
use codex_workflow_contracts::ForgeKind;
use codex_workflow_contracts::ForkLineage;
use codex_workflow_contracts::ImmutableSourceRevision;
use codex_workflow_contracts::ReviewId;
use codex_workflow_contracts::ReviewState;
use codex_workflow_contracts::RevisionSha;
use codex_workflow_contracts::WorkflowForge;
use codex_workflow_contracts::WorkflowManifest;
use codex_workflow_contracts::WorkflowRelease;
use codex_workflow_contracts::WorkflowRepository;
use codex_workflow_contracts::WorkflowRepositoryId;
use codex_workflow_contracts::WorkflowReview;
use codex_workflow_contracts::WorkflowVersionId;
use serde::Deserialize;
use serde::Serialize;

use crate::canonical::canonical_repository_id;
use crate::canonical::repository_id_from_canonical;
use crate::collaboration::validate_tag;
use crate::WorkflowForgeError;

/// A GitHub instance host, for example `github.com` or a GitHub
/// Enterprise Server hostname.
///
/// Hosts are normalized to lowercase. The host participates in the
/// adapter's canonical repository identity, keeping different GitHub
/// instances distinct.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct GithubHost(String);

impl GithubHost {
    /// Parses and normalizes a GitHub host name.
    ///
    /// Hosts are hostnames: non-empty, without scheme, slash, or colon,
    /// and without surrounding whitespace.
    pub fn parse(value: impl Into<String>) -> Result<Self, WorkflowForgeError> {
        let value = value.into();
        let normalized = value.trim().to_ascii_lowercase();
        let valid = !normalized.is_empty()
            && normalized.len() == value.len()
            && !normalized.contains('/')
            && !normalized.contains(':');
        if valid {
            Ok(Self(normalized))
        } else {
            Err(WorkflowForgeError::InvalidRecord {
                reason: format!(
                    "invalid github host {value:?}: hosts are hostnames such as github.com"
                ),
            })
        }
    }

    /// The normalized host name.
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

/// `owner/repository` coordinates on a GitHub host.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GithubRepoCoordinates {
    /// Host the repository lives on.
    pub host: GithubHost,
    /// Repository owner (user or organization).
    pub owner: String,
    /// Repository name.
    pub repository: String,
}

impl GithubRepoCoordinates {
    /// Validates and creates repository coordinates.
    ///
    /// Owner and repository names are non-empty, slash-free GitHub name
    /// segments (alphanumerics, `.`, `-`, `_`).
    pub fn new(
        host: GithubHost,
        owner: impl Into<String>,
        repository: impl Into<String>,
    ) -> Result<Self, WorkflowForgeError> {
        let owner = owner.into();
        let repository = repository.into();
        if !valid_name_segment(&owner) || !valid_name_segment(&repository) {
            return Err(WorkflowForgeError::InvalidRecord {
                reason: format!(
                    "invalid github repository coordinates {owner}/{repository}: names are alphanumeric . - _ segments"
                ),
            });
        }
        Ok(Self {
            host,
            owner,
            repository,
        })
    }

    /// The adapter's canonical repository identity string:
    /// `host/owner/repository`.
    pub fn canonical(&self) -> String {
        format!("{}/{}/{}", self.host.as_str(), self.owner, self.repository)
    }

    /// Parses canonical coordinates produced by [`Self::canonical`].
    pub fn parse_canonical(canonical: &str) -> Result<Self, WorkflowForgeError> {
        let parts: Vec<&str> = canonical.split('/').collect();
        if parts.len() != 3 {
            return Err(WorkflowForgeError::InvalidRecord {
                reason: format!(
                    "canonical repository identity {canonical:?} is not host/owner/repository"
                ),
            });
        }
        Self::new(GithubHost::parse(parts[0])?, parts[1], parts[2])
    }
}

/// Validates a GitHub owner or repository name segment.
fn valid_name_segment(segment: &str) -> bool {
    !segment.is_empty()
        && !segment.contains('/')
        && segment.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '.' | '-' | '_')
        })
}

/// The kind of a movable GitHub ref.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub enum GithubRefKind {
    /// A movable branch ref.
    Branch,
    /// A movable tag ref.
    Tag,
}

/// A movable GitHub ref with its current head, as supplied by the host
/// integration.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GithubRefRecord {
    /// Whether the ref is a branch or a tag.
    pub kind: GithubRefKind,
    /// The ref name, for example `main` or `v1`.
    pub name: String,
    /// Full commit SHA the ref currently points at. Abbreviated SHAs are
    /// rejected during mapping: refs are development state, never
    /// execution anchors.
    pub head_sha: String,
}

/// A GitHub repository record: the metadata subset the workflow
/// platform needs, as supplied by the host integration.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GithubRepositoryRecord {
    /// Repository coordinates (host, owner, name).
    pub coordinates: GithubRepoCoordinates,
    /// Default development branch name.
    pub default_branch: String,
    /// Whether GitHub reports this repository as a fork.
    pub fork: bool,
    /// The upstream repository coordinates, present exactly when this
    /// repository is a fork.
    pub parent: Option<GithubRepoCoordinates>,
    /// Whether the repository is archived. Archived repositories remain
    /// readable; collaboration policy is the control plane's decision.
    pub archived: bool,
}

/// Lifecycle state of a GitHub pull request record.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum GithubPullRequestState {
    /// The pull request is open.
    Open,
    /// The pull request was merged at the given merge commit.
    Merged {
        /// Full SHA of the merge commit.
        merge_commit_sha: String,
    },
    /// The pull request was closed without merging.
    Closed,
}

/// A GitHub pull request record: the review-shaped proposal record as
/// supplied by the host integration.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GithubPullRequestRecord {
    /// GitHub pull request number. The adapter allocates the review id
    /// from this number.
    pub number: u64,
    /// Pull request title, when present.
    pub title: Option<String>,
    /// Full commit SHA of the immutable base revision.
    pub base_sha: String,
    /// Full commit SHA of the immutable head revision.
    pub head_sha: String,
    /// Lifecycle state.
    pub state: GithubPullRequestState,
}

/// A GitHub release record: the tag-side display record as supplied by
/// the host integration.
///
/// `target_commitish` is whatever GitHub recorded — a branch name, a tag
/// name, or a full SHA. Because tags and branches move, the host must
/// resolve the target to an immutable revision (via
/// [`GithubForge::resolve_ref_blocking`]) before mapping; [`map_release`]
/// verifies a SHA-shaped commitish against that resolution.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GithubReleaseRecord {
    /// Release tag name (display handle, not execution authority).
    pub tag_name: String,
    /// The commitish GitHub recorded for the release.
    pub target_commitish: String,
    /// Whether the release is a draft. Drafts are not published versions
    /// and are rejected during mapping.
    pub draft: bool,
}

/// Contract-side provenance translated by the host integration for one
/// GitHub repository.
///
/// Provenance translation stays with the host so this crate never
/// guesses attribution or lineage shapes from raw forge payloads.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct GithubProvenance {
    /// Fork lineage, present exactly when the repository is a fork.
    pub fork_lineage: Option<ForkLineage>,
    /// Maintainer attributions.
    pub maintainers: Vec<Attribution>,
}

/// A GitHub release paired with its host-resolved immutable target and
/// the published workflow version identities it carries.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GithubReleaseView {
    /// The raw GitHub release record.
    pub record: GithubReleaseRecord,
    /// The immutable revision the release was resolved to. The release's
    /// authority is this revision and its version identities — the tag
    /// name is a display handle.
    pub target: ImmutableSourceRevision,
    /// Immutable version identities published by this release.
    pub versions: Vec<WorkflowVersionId>,
}

/// Transport-free source of GitHub records.
///
/// The host integration implements this trait with real GitHub
/// transport. Implementations must return validated, complete records;
/// mapping in this crate re-validates everything (external outputs are
/// untrusted) before contract records are produced. Credentials live
/// only in the host implementation — only opaque record data crosses
/// this boundary.
pub trait GithubRecordSource: Send + Sync {
    /// Reads the repository metadata record.
    fn repository(
        &self,
        coordinates: &GithubRepoCoordinates,
    ) -> Result<GithubRepositoryRecord, WorkflowForgeError>;

    /// Reads the movable refs (branches and tags) with their current
    /// heads.
    fn refs(
        &self,
        coordinates: &GithubRepoCoordinates,
    ) -> Result<Vec<GithubRefRecord>, WorkflowForgeError>;

    /// Reads the workflow manifests discovered in the repository
    /// content. Content transport (clone/fetch) is host-owned; only the
    /// parsed contract manifests cross this boundary.
    fn workflow_manifests(
        &self,
        coordinates: &GithubRepoCoordinates,
    ) -> Result<Vec<WorkflowManifest>, WorkflowForgeError>;

    /// Reads the contract-side provenance translation for the repository.
    fn provenance(
        &self,
        coordinates: &GithubRepoCoordinates,
    ) -> Result<GithubProvenance, WorkflowForgeError>;

    /// Reads the pull requests relevant to workflow collaboration.
    fn pull_requests(
        &self,
        coordinates: &GithubRepoCoordinates,
    ) -> Result<Vec<GithubPullRequestRecord>, WorkflowForgeError>;

    /// Reads the releases with their host-resolved immutable targets and
    /// published version identities.
    fn releases(
        &self,
        coordinates: &GithubRepoCoordinates,
    ) -> Result<Vec<GithubReleaseView>, WorkflowForgeError>;
}

/// The GitHub adapter: implements the frozen, forge-neutral
/// [`WorkflowForge`] boundary over a [`GithubRecordSource`].
///
/// The adapter is one implementation of the boundary, not the semantic
/// authority: it resolves movable refs to immutable revisions and reads
/// repository records. All records it returns are frozen contract types.
pub struct GithubForge {
    kind: ForgeKind,
    source: Arc<dyn GithubRecordSource>,
}

impl GithubForge {
    /// Creates the adapter. The forge kind is injected by the host (for
    /// example the GitHub variant of the frozen `ForgeKind`) so this
    /// crate never hard-codes forge enum construction.
    pub fn new(kind: ForgeKind, source: Arc<dyn GithubRecordSource>) -> Self {
        Self { kind, source }
    }

    /// The record source backing this adapter.
    pub fn source(&self) -> &dyn GithubRecordSource {
        self.source.as_ref()
    }

    /// Reverses a canonical repository identity into GitHub coordinates.
    fn coordinates(
        &self,
        repository: &WorkflowRepositoryId,
    ) -> Result<GithubRepoCoordinates, WorkflowForgeError> {
        let canonical = canonical_repository_id(repository)?;
        GithubRepoCoordinates::parse_canonical(&canonical)
    }

    /// Blocking form of ref resolution, for hosts that prefer direct
    /// calls over the async trait boundary.
    ///
    /// Resolves a movable development ref (branch or tag) to an
    /// immutable revision anchored at the ref's current full commit SHA.
    /// The `resolved_from` field records the ref as provenance only.
    pub fn resolve_ref_blocking(
        &self,
        repository: &WorkflowRepositoryId,
        reference: &DevelopmentRef,
    ) -> Result<ImmutableSourceRevision, WorkflowForgeError> {
        let coordinates = self.coordinates(repository)?;
        let refs = self.source.refs(&coordinates)?;
        let expected_kind = match reference {
            DevelopmentRef::Branch(_) => GithubRefKind::Branch,
            DevelopmentRef::Tag(_) => GithubRefKind::Tag,
        };
        let found = refs
            .iter()
            .find(|record| record.kind == expected_kind && record.name == reference.name())
            .ok_or_else(|| WorkflowForgeError::UnknownBranch {
                repository: repository.clone(),
                branch: reference.clone(),
            })?;
        let commit_sha = RevisionSha::parse(found.head_sha.clone())?;
        Ok(ImmutableSourceRevision::pin(
            commit_sha,
            Some(reference.clone()),
        ))
    }

    /// Blocking form of the repository record read.
    pub fn repository_record_blocking(
        &self,
        repository: &WorkflowRepositoryId,
    ) -> Result<WorkflowRepository, WorkflowForgeError> {
        let coordinates = self.coordinates(repository)?;
        let record = self.source.repository(&coordinates)?;
        let provenance = self.source.provenance(&coordinates)?;
        let manifests = self.source.workflow_manifests(&coordinates)?;
        let mut neutral = map_repository(&self.kind, &record, &provenance, manifests)?;
        let refs = self.source.refs(&coordinates)?;
        neutral.branches = map_branch_states(&refs)?;
        Ok(neutral)
    }

    /// Reads and maps the pull requests of a repository into neutral
    /// review records.
    pub fn pull_request_reviews(
        &self,
        repository: &WorkflowRepositoryId,
    ) -> Result<Vec<WorkflowReview>, WorkflowForgeError> {
        let coordinates = self.coordinates(repository)?;
        let records = self.source.pull_requests(&coordinates)?;
        let mut reviews = Vec::with_capacity(records.len());
        for record in &records {
            reviews.push(map_pull_request(repository, record)?);
        }
        Ok(reviews)
    }

    /// Reads and maps the releases of a repository into neutral release
    /// records.
    pub fn workflow_releases(
        &self,
        repository: &WorkflowRepositoryId,
    ) -> Result<Vec<WorkflowRelease>, WorkflowForgeError> {
        let coordinates = self.coordinates(repository)?;
        let views = self.source.releases(&coordinates)?;
        let mut releases = Vec::with_capacity(views.len());
        for view in &views {
            releases.push(map_release(
                repository,
                &view.record,
                &view.target,
                view.versions.clone(),
            )?);
        }
        Ok(releases)
    }
}

impl WorkflowForge for GithubForge {
    fn kind(&self) -> &ForgeKind {
        &self.kind
    }

    fn resolve_ref(
        &self,
        repository: &WorkflowRepositoryId,
        reference: &DevelopmentRef,
    ) -> impl Future<
        Output = Result<ImmutableSourceRevision, codex_workflow_contracts::WorkflowContractError>,
    > + Send {
        ready(
            self.resolve_ref_blocking(repository, reference)
                .map_err(WorkflowForgeError::as_contract_error),
        )
    }

    fn repository_record(
        &self,
        repository: &WorkflowRepositoryId,
    ) -> impl Future<
        Output = Result<WorkflowRepository, codex_workflow_contracts::WorkflowContractError>,
    > + Send {
        ready(
            self.repository_record_blocking(repository)
                .map_err(WorkflowForgeError::as_contract_error),
        )
    }
}

/// Maps and validates a GitHub repository record into the forge-neutral
/// repository record.
///
/// Validation (external outputs are untrusted):
///
/// - fork records must carry a parent and fork lineage, and non-fork
///   records must carry neither;
/// - the default branch must be a valid development ref name;
/// - every manifest must validate and declare exactly this repository.
///
/// Branch states are supplied separately by the adapter through
/// [`map_branch_states`], because ref heads move independently of
/// repository metadata.
pub fn map_repository(
    kind: &ForgeKind,
    record: &GithubRepositoryRecord,
    provenance: &GithubProvenance,
    manifests: Vec<WorkflowManifest>,
) -> Result<WorkflowRepository, WorkflowForgeError> {
    let identity = repository_id_from_canonical(&record.coordinates.canonical())?;
    let origin = match (record.fork, &record.parent) {
        (true, Some(parent)) => Some(repository_id_from_canonical(&parent.canonical())?),
        (true, None) => {
            return Err(WorkflowForgeError::InvalidRecord {
                reason: "fork record carries no parent repository".to_owned(),
            });
        }
        (false, Some(_)) => {
            return Err(WorkflowForgeError::InvalidRecord {
                reason: "non-fork record carries a parent repository".to_owned(),
            });
        }
        (false, None) => None,
    };
    let forked_from = match (record.fork, &provenance.fork_lineage) {
        (true, Some(lineage)) => Some(lineage.clone()),
        (true, None) => {
            return Err(WorkflowForgeError::InvalidRecord {
                reason: "fork record carries no fork lineage attribution".to_owned(),
            });
        }
        (false, Some(_)) => {
            return Err(WorkflowForgeError::InvalidRecord {
                reason: "non-fork record carries fork lineage".to_owned(),
            });
        }
        (false, None) => None,
    };
    let default_branch = DevelopmentRef::branch(record.default_branch.clone())?;
    for manifest in &manifests {
        manifest.validate()?;
        if manifest.repository != identity {
            return Err(WorkflowForgeError::ManifestRepositoryMismatch {
                manifest_repository: manifest.repository.clone(),
                expected_repository: identity.clone(),
            });
        }
    }
    Ok(WorkflowRepository {
        identity,
        forge: kind.clone(),
        origin,
        forked_from,
        default_branch,
        branches: Vec::new(),
        workflows: manifests,
        maintainers: provenance.maintainers.clone(),
    })
}

/// Maps GitHub ref records into development branch states.
///
/// Branches only: tag refs are provenance handles for releases and are
/// resolved through ref resolution instead. Head SHAs must be full
/// lowercase hex — abbreviated SHAs are rejected, because branch states
/// are snapshots of development state, never execution anchors.
pub fn map_branch_states(
    refs: &[GithubRefRecord],
) -> Result<Vec<codex_workflow_contracts::WorkflowBranchState>, WorkflowForgeError> {
    let mut branches = Vec::new();
    for reference in refs {
        if reference.kind != GithubRefKind::Branch {
            continue;
        }
        let branch = DevelopmentRef::branch(reference.name.clone())?;
        let head = RevisionSha::parse(reference.head_sha.clone()).map_err(|error| {
            WorkflowForgeError::InvalidRecord {
                reason: format!(
                    "github branch {} carries a non-full head sha: {error}",
                    reference.name
                ),
            }
        })?;
        branches.push(codex_workflow_contracts::WorkflowBranchState {
            branch,
            head: Some(head),
        });
    }
    Ok(branches)
}

/// Maps and validates a GitHub pull request record into a neutral review
/// record.
///
/// The adapter allocates the review id from the pull request number.
/// Base and head SHAs must be full commit SHAs (abbreviated SHAs are
/// rejected) and must differ — reviews always describe a change.
pub fn map_pull_request(
    repository: &WorkflowRepositoryId,
    record: &GithubPullRequestRecord,
) -> Result<WorkflowReview, WorkflowForgeError> {
    let base = RevisionSha::parse(record.base_sha.clone())?;
    let head = RevisionSha::parse(record.head_sha.clone())?;
    if base == head {
        return Err(WorkflowForgeError::InvalidRecord {
            reason: format!(
                "pull request {} has identical base and head {base}",
                record.number
            ),
        });
    }
    let state = match &record.state {
        GithubPullRequestState::Open => ReviewState::Open,
        GithubPullRequestState::Merged { merge_commit_sha } => ReviewState::Merged {
            merge_revision: RevisionSha::parse(merge_commit_sha.clone())?,
        },
        GithubPullRequestState::Closed => ReviewState::Closed,
    };
    Ok(WorkflowReview {
        review: ReviewId::parse(record.number.to_string())?,
        repository: repository.clone(),
        title: record.title.clone(),
        base: ImmutableSourceRevision::pin_commit(base),
        head: ImmutableSourceRevision::pin_commit(head),
        state,
    })
}

/// Maps and validates a GitHub release record into a neutral release
/// record.
///
/// The immutable target revision is host-resolved (GitHub release
/// commitishes can be moving branch or tag names, and this crate never
/// treats a moving ref as a published version). If the recorded
/// commitish is itself a full SHA, it must agree with the resolved
/// target. Draft releases are not published versions and are rejected.
pub fn map_release(
    repository: &WorkflowRepositoryId,
    record: &GithubReleaseRecord,
    target: &ImmutableSourceRevision,
    versions: Vec<WorkflowVersionId>,
) -> Result<WorkflowRelease, WorkflowForgeError> {
    if record.draft {
        return Err(WorkflowForgeError::InvalidRecord {
            reason: "draft releases are not published workflow releases".to_owned(),
        });
    }
    validate_tag(&record.tag_name)?;
    if let Ok(commitish) = RevisionSha::parse(record.target_commitish.clone()) {
        if commitish != target.commit_sha {
            return Err(WorkflowForgeError::InvalidRecord {
                reason: format!(
                    "release target_commitish {commitish} disagrees with the resolved immutable revision {}",
                    target.commit_sha
                ),
            });
        }
    }
    Ok(WorkflowRelease {
        tag: record.tag_name.clone(),
        repository: repository.clone(),
        target: target.clone(),
        versions,
    })
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use codex_workflow_contracts::ImmutableSourceRevision;
    use codex_workflow_contracts::ReviewState;
    use codex_workflow_contracts::WorkflowForge;
    use codex_workflow_contracts::WorkflowManifest;

    use crate::canonical::canonical_repository_id;
    use crate::test_support::attribution;
    use crate::test_support::block_on_ready;
    use crate::test_support::fork_lineage;
    use crate::test_support::github_kind;
    use crate::test_support::manifest;
    use crate::test_support::published_version;
    use crate::test_support::repo_id;
    use crate::test_support::sha;

    use super::map_pull_request;
    use super::map_release;
    use super::map_repository;
    use super::GithubForge;
    use super::GithubHost;
    use super::GithubProvenance;
    use super::GithubPullRequestRecord;
    use super::GithubPullRequestState;
    use super::GithubRefKind;
    use super::GithubRefRecord;
    use super::GithubReleaseRecord;
    use super::GithubReleaseView;
    use super::GithubRepoCoordinates;
    use super::GithubRepositoryRecord;
    use crate::WorkflowForgeError;

    fn coordinates() -> GithubRepoCoordinates {
        GithubRepoCoordinates::new(
            GithubHost::parse("github.com").unwrap(),
            "acme",
            "ops-workflows",
        )
        .unwrap()
    }

    /// A canned record source standing in for the host's GitHub
    /// transport. External outputs are untrusted, so tests also mutate
    /// these records to check validation.
    struct StubSource {
        refs: Vec<GithubRefRecord>,
        manifests: Vec<WorkflowManifest>,
        provenance: GithubProvenance,
        pulls: Vec<GithubPullRequestRecord>,
        releases: Vec<GithubReleaseView>,
    }

    impl StubSource {
        fn repository_record(&self) -> GithubRepositoryRecord {
            GithubRepositoryRecord {
                coordinates: coordinates(),
                default_branch: "main".to_owned(),
                fork: false,
                parent: None,
                archived: false,
            }
        }
    }

    impl super::GithubRecordSource for StubSource {
        fn repository(
            &self,
            requested: &GithubRepoCoordinates,
        ) -> Result<GithubRepositoryRecord, WorkflowForgeError> {
            if requested == &coordinates() {
                Ok(self.repository_record())
            } else {
                Err(WorkflowForgeError::InvalidRecord {
                    reason: format!("stub source does not know {requested:?}"),
                })
            }
        }

        fn refs(
            &self,
            requested: &GithubRepoCoordinates,
        ) -> Result<Vec<GithubRefRecord>, WorkflowForgeError> {
            if requested == &coordinates() {
                Ok(self.refs.clone())
            } else {
                Err(WorkflowForgeError::InvalidRecord {
                    reason: format!("stub source does not know {requested:?}"),
                })
            }
        }

        fn workflow_manifests(
            &self,
            requested: &GithubRepoCoordinates,
        ) -> Result<Vec<WorkflowManifest>, WorkflowForgeError> {
            if requested == &coordinates() {
                Ok(self.manifests.clone())
            } else {
                Err(WorkflowForgeError::InvalidRecord {
                    reason: format!("stub source does not know {requested:?}"),
                })
            }
        }

        fn provenance(
            &self,
            requested: &GithubRepoCoordinates,
        ) -> Result<GithubProvenance, WorkflowForgeError> {
            if requested == &coordinates() {
                Ok(self.provenance.clone())
            } else {
                Err(WorkflowForgeError::InvalidRecord {
                    reason: format!("stub source does not know {requested:?}"),
                })
            }
        }

        fn pull_requests(
            &self,
            requested: &GithubRepoCoordinates,
        ) -> Result<Vec<GithubPullRequestRecord>, WorkflowForgeError> {
            if requested == &coordinates() {
                Ok(self.pulls.clone())
            } else {
                Err(WorkflowForgeError::InvalidRecord {
                    reason: format!("stub source does not know {requested:?}"),
                })
            }
        }

        fn releases(
            &self,
            requested: &GithubRepoCoordinates,
        ) -> Result<Vec<GithubReleaseView>, WorkflowForgeError> {
            if requested == &coordinates() {
                Ok(self.releases.clone())
            } else {
                Err(WorkflowForgeError::InvalidRecord {
                    reason: format!("stub source does not know {requested:?}"),
                })
            }
        }
    }

    fn stub_source() -> StubSource {
        StubSource {
            refs: vec![
                GithubRefRecord {
                    kind: GithubRefKind::Branch,
                    name: "main".to_owned(),
                    head_sha: sha(1).to_string(),
                },
                GithubRefRecord {
                    kind: GithubRefKind::Tag,
                    name: "v1".to_owned(),
                    head_sha: sha(2).to_string(),
                },
            ],
            manifests: vec![manifest(
                "deploy",
                "github.com/acme/ops-workflows",
                "workflows/deploy.toml",
            )],
            provenance: GithubProvenance {
                fork_lineage: None,
                maintainers: vec![attribution("acme-ops")],
            },
            pulls: vec![GithubPullRequestRecord {
                number: 101,
                title: Some("Add retry step".to_owned()),
                base_sha: sha(1).to_string(),
                head_sha: sha(3).to_string(),
                state: GithubPullRequestState::Open,
            }],
            releases: vec![GithubReleaseView {
                record: GithubReleaseRecord {
                    tag_name: "v1.2.0".to_owned(),
                    target_commitish: sha(1).to_string(),
                    draft: false,
                },
                target: ImmutableSourceRevision::pin_commit(sha(1)),
                versions: vec![
                    published_version(
                        "deploy",
                        "github.com/acme/ops-workflows",
                        &sha(1),
                        (1, 2, 0),
                        "definition",
                        "lock",
                    )
                    .version_id,
                ],
            }],
        }
    }

    fn forge() -> GithubForge {
        GithubForge::new(github_kind(), Arc::new(stub_source()))
    }

    #[test]
    fn maps_repository_records_to_neutral_contracts() {
        let forge = forge();
        let repository = repo_id("github.com/acme/ops-workflows");
        // Through the frozen, forge-neutral trait boundary.
        let record: codex_workflow_contracts::WorkflowRepository =
            block_on_ready(forge.repository_record(&repository)).unwrap();
        assert_eq!(
            canonical_repository_id(&record.identity).unwrap(),
            "github.com/acme/ops-workflows"
        );
        assert_eq!(record.origin, None);
        assert_eq!(record.forked_from, None);
        assert_eq!(
            record.default_branch,
            codex_workflow_contracts::DevelopmentRef::branch("main").unwrap()
        );
        assert_eq!(record.branches.len(), 1);
        assert_eq!(record.branches[0].head.as_ref(), Some(&sha(1)));
        assert_eq!(record.maintainers, vec![attribution("acme-ops")]);
        assert_eq!(
            record
                .manifest_for(&crate::test_support::workflow_id("deploy"))
                .map(|manifest| manifest.definition_path.as_ref()),
            Some("workflows/deploy.toml")
        );
    }

    #[test]
    fn repository_records_reject_abbreviated_head_shas() {
        let mut source = stub_source();
        source.refs[0].head_sha = "abc123".to_owned();
        let forge = GithubForge::new(github_kind(), Arc::new(source));
        let repository = repo_id("github.com/acme/ops-workflows");
        let result = forge.repository_record_blocking(&repository);
        assert!(matches!(
            result,
            Err(WorkflowForgeError::InvalidRecord { .. })
        ));
    }

    #[test]
    fn fork_records_require_parent_and_lineage() {
        let kind = github_kind();
        let mut record = stub_source().repository_record();
        let provenance = GithubProvenance {
            fork_lineage: None,
            maintainers: vec![attribution("acme-ops")],
        };

        // Fork without parent or lineage is rejected.
        record.fork = true;
        assert!(matches!(
            map_repository(&kind, &record, &provenance, vec![]),
            Err(WorkflowForgeError::InvalidRecord { .. })
        ));

        // Fork with parent but no lineage is rejected.
        record.parent = Some(coordinates());
        assert!(matches!(
            map_repository(&kind, &record, &provenance, vec![]),
            Err(WorkflowForgeError::InvalidRecord { .. })
        ));

        // Non-fork with parent or lineage is rejected.
        record.fork = false;
        assert!(matches!(
            map_repository(&kind, &record, &provenance, vec![]),
            Err(WorkflowForgeError::InvalidRecord { .. })
        ));
        record.parent = None;
        let with_lineage = GithubProvenance {
            fork_lineage: Some(fork_lineage("github.com/acme/ops-workflows")),
            maintainers: vec![attribution("acme-ops")],
        };
        assert!(matches!(
            map_repository(&kind, &record, &with_lineage, vec![]),
            Err(WorkflowForgeError::InvalidRecord { .. })
        ));

        // A consistent fork maps with origin and lineage as data.
        record.fork = true;
        record.parent = Some(coordinates());
        let fork = map_repository(&kind, &record, &with_lineage, vec![]).unwrap();
        assert_eq!(fork.origin, Some(repo_id("github.com/acme/ops-workflows")));
        assert_eq!(
            fork.forked_from,
            Some(fork_lineage("github.com/acme/ops-workflows"))
        );
    }

    #[test]
    fn manifests_must_belong_to_the_mapped_repository() {
        let kind = github_kind();
        let record = stub_source().repository_record();
        let provenance = GithubProvenance {
            fork_lineage: None,
            maintainers: vec![attribution("acme-ops")],
        };
        // A manifest declaring a different repository is untrusted input.
        let foreign = manifest("deploy", "github.com/other/repo", "workflows/deploy.toml");
        assert!(matches!(
            map_repository(&kind, &record, &provenance, vec![foreign]),
            Err(WorkflowForgeError::ManifestRepositoryMismatch { .. })
        ));
    }

    #[test]
    fn maps_pull_requests_with_neutral_states() {
        let repository = repo_id("github.com/acme/ops-workflows");
        let open = GithubPullRequestRecord {
            number: 101,
            title: Some("Add retry step".to_owned()),
            base_sha: sha(1).to_string(),
            head_sha: sha(3).to_string(),
            state: GithubPullRequestState::Open,
        };
        let review = map_pull_request(&repository, &open).unwrap();
        assert_eq!(review.review.as_ref(), "101");
        assert_eq!(review.state, ReviewState::Open);
        assert_eq!(review.base.commit_sha, sha(1));
        assert_eq!(review.head.commit_sha, sha(3));

        let merged = GithubPullRequestRecord {
            state: GithubPullRequestState::Merged {
                merge_commit_sha: sha(4).to_string(),
            },
            ..open.clone()
        };
        let merged_review = map_pull_request(&repository, &merged).unwrap();
        assert_eq!(
            merged_review.state,
            ReviewState::Merged {
                merge_revision: sha(4)
            }
        );

        let closed = GithubPullRequestRecord {
            state: GithubPullRequestState::Closed,
            ..open.clone()
        };
        assert_eq!(
            map_pull_request(&repository, &closed).unwrap().state,
            ReviewState::Closed
        );

        // Abbreviated SHAs and empty diffs are rejected.
        let abbreviated = GithubPullRequestRecord {
            base_sha: "abc123".to_owned(),
            ..open.clone()
        };
        assert!(matches!(
            map_pull_request(&repository, &abbreviated),
            Err(WorkflowForgeError::Contract(_))
        ));
        let empty_diff = GithubPullRequestRecord {
            head_sha: sha(1).to_string(),
            ..open.clone()
        };
        assert!(matches!(
            map_pull_request(&repository, &empty_diff),
            Err(WorkflowForgeError::InvalidRecord { .. })
        ));
    }

    #[test]
    fn maps_releases_only_when_resolved_to_immutable_targets() {
        let repository = repo_id("github.com/acme/ops-workflows");
        let target = ImmutableSourceRevision::pin_commit(sha(10));
        let record = GithubReleaseRecord {
            tag_name: "v1.2.0".to_owned(),
            target_commitish: sha(10).to_string(),
            draft: false,
        };
        let version_id = published_version(
            "deploy",
            "github.com/acme/ops-workflows",
            &sha(10),
            (1, 2, 0),
            "definition",
            "lock",
        )
        .version_id;
        let release = map_release(&repository, &record, &target, vec![version_id.clone()]).unwrap();
        assert_eq!(release.tag, "v1.2.0");
        assert_eq!(release.target.commit_sha, sha(10));
        assert_eq!(release.versions, vec![version_id]);

        // Drafts are not published versions.
        let draft = GithubReleaseRecord {
            draft: true,
            ..record.clone()
        };
        assert!(matches!(
            map_release(&repository, &draft, &target, vec![]),
            Err(WorkflowForgeError::InvalidRecord { .. })
        ));

        // A SHA-shaped commitish must agree with the resolved target.
        let mismatched = GithubReleaseRecord {
            target_commitish: sha(11).to_string(),
            ..record
        };
        assert!(matches!(
            map_release(&repository, &mismatched, &target, vec![]),
            Err(WorkflowForgeError::InvalidRecord { .. })
        ));
    }

    #[test]
    fn resolves_refs_through_the_neutral_trait() {
        let forge = forge();
        let repository = repo_id("github.com/acme/ops-workflows");
        let main = codex_workflow_contracts::DevelopmentRef::branch("main").unwrap();
        let revision = block_on_ready(forge.resolve_ref(&repository, &main)).unwrap();
        assert_eq!(revision.commit_sha, sha(1));
        assert_eq!(revision.resolved_from.as_ref(), Some(&main));

        let tag = codex_workflow_contracts::DevelopmentRef::tag("v1").unwrap();
        let tagged = block_on_ready(forge.resolve_ref(&repository, &tag)).unwrap();
        assert_eq!(tagged.commit_sha, sha(2));

        let unknown = codex_workflow_contracts::DevelopmentRef::branch("nope").unwrap();
        let result = block_on_ready(forge.resolve_ref(&repository, &unknown));
        assert!(result.is_err());
    }

    #[test]
    fn adapter_outputs_are_forge_neutral_contract_records() {
        let forge = forge();
        let repository = repo_id("github.com/acme/ops-workflows");
        // Everything the adapter returns is a frozen contract record:
        // no GitHub-shaped type crosses the boundary.
        let record: codex_workflow_contracts::WorkflowRepository =
            forge.repository_record_blocking(&repository).unwrap();
        let reviews: Vec<codex_workflow_contracts::WorkflowReview> =
            forge.pull_request_reviews(&repository).unwrap();
        let releases: Vec<codex_workflow_contracts::WorkflowRelease> =
            forge.workflow_releases(&repository).unwrap();
        assert_eq!(reviews.len(), 1);
        assert_eq!(releases.len(), 1);
        assert_eq!(releases[0].versions.len(), 1);
        assert_eq!(
            record
                .manifest_for(&crate::test_support::workflow_id("deploy"))
                .map(|m| m.workflow.clone()),
            Some(crate::test_support::workflow_id("deploy"))
        );
    }
}
