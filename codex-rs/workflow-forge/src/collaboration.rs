//! Pure collaboration transitions over frozen contract records.
//!
//! Every operation in this module is a *pure record transition*: it reads
//! the records it is given, validates the work order's invariants, and
//! returns a new contract record. No operation mutates hidden state, talks
//! to a forge, or executes anything. The Codex application owns
//! orchestration and durable state; this module defines the semantics
//! those orchestrations must obey.
//!
//! Invariants enforced here:
//!
//! - Reviews record immutable base and head revisions — never moving
//!   branch state.
//! - Merges require the review to be open and to carry an approval at the
//!   review's *current* head, and produce a new immutable merge revision.
//! - Releases are cut at an immutable revision, and every version they
//!   publish must be anchored at exactly that revision and must verify
//!   against its recomputed identity digest.
//! - Forks copy development state and carry fork lineage and attribution
//!   as data.

use codex_workflow_contracts::Attribution;
use codex_workflow_contracts::DevelopmentRef;
use codex_workflow_contracts::ExecutionVersionIdentity;
use codex_workflow_contracts::ForkLineage;
use codex_workflow_contracts::ImmutableSourceRevision;
use codex_workflow_contracts::ReviewId;
use codex_workflow_contracts::ReviewState;
use codex_workflow_contracts::RevisionSha;
use codex_workflow_contracts::WorkflowBranchState;
use codex_workflow_contracts::WorkflowRelease;
use codex_workflow_contracts::WorkflowRepository;
use codex_workflow_contracts::WorkflowRepositoryId;
use codex_workflow_contracts::WorkflowReview;
use codex_workflow_contracts::WorkflowVersion;
use codex_workflow_contracts::WorkflowVersionId;
use serde::Deserialize;
use serde::Serialize;

use crate::WorkflowForgeError;

/// A reference to an immutable published workflow version: the execution
/// identity tuple and its digest, without carrying the full definition.
///
/// The identity digest is recomputable from the tuple
/// ([`ExecutionVersionIdentity::version_id`]), so a reference can always
/// be verified ([`PublishedVersionRef::verify`]). This is the check that
/// makes published versions effectively immutable: a tampered record
/// fails recomputation.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PublishedVersionRef {
    /// Identity tuple of the published version.
    pub identity: ExecutionVersionIdentity,
    /// Version identity digest recomputed from the tuple.
    pub version_id: WorkflowVersionId,
}

impl PublishedVersionRef {
    /// Projects a full version record into a reference.
    pub fn of(version: &WorkflowVersion) -> Self {
        Self {
            identity: version.identity.clone(),
            version_id: version.version_id.clone(),
        }
    }

    /// Verifies that the recorded version id equals the digest of the
    /// execution identity tuple.
    ///
    /// Every publish, pin, and install path calls this first: published
    /// revisions are immutable, and a record whose identity digest does
    /// not recompute is rejected.
    pub fn verify(&self) -> Result<(), WorkflowForgeError> {
        let recomputed = self.identity.version_id()?;
        if recomputed == self.version_id {
            Ok(())
        } else {
            Err(WorkflowForgeError::VersionIdentityMismatch {
                workflow: self.identity.workflow.clone(),
                recorded: self.version_id.clone(),
                recomputed,
            })
        }
    }
}

/// A review approval recorded at a specific immutable head revision.
///
/// Approving a head, then merging a *different* head, does not satisfy
/// [`merge_review`]: approvals must be fresh at the revision actually
/// being merged. Who is *allowed* to approve is a workflow-role decision
/// owned by the workflow control plane; this record only carries the
/// approval as data.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReviewApproval {
    /// The review being approved.
    pub review: ReviewId,
    /// Display name of the approving contributor. Attribution only;
    /// credentials never enter this crate.
    pub reviewer: String,
    /// The immutable head revision the approval was recorded at.
    pub approved_head: RevisionSha,
}

/// A forge-neutral commit summary: display and provenance metadata for
/// an immutable revision.
///
/// The commit SHA (inside `revision`) is the authority; message, author,
/// and parents are development-time metadata for tooling and review.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CommitSummary {
    /// The immutable revision this commit summarizes.
    pub revision: ImmutableSourceRevision,
    /// Commit message (display only).
    pub message: String,
    /// Commit author display name (attribution only).
    pub author: String,
    /// Parent commits, oldest context first. Merge commits have two.
    pub parents: Vec<RevisionSha>,
}

/// Validates a release tag name: non-empty, without surrounding
/// whitespace.
pub(crate) fn validate_tag(tag: &str) -> Result<(), WorkflowForgeError> {
    let trimmed = tag.trim();
    if trimmed.is_empty() {
        Err(WorkflowForgeError::InvalidTag {
            tag: tag.to_owned(),
            reason: "release tags must be non-empty",
        })
    } else if trimmed.len() != tag.len() {
        Err(WorkflowForgeError::InvalidTag {
            tag: tag.to_owned(),
            reason: "release tags must not start or end with whitespace",
        })
    } else {
        Ok(())
    }
}

/// Proposes a review of workflow changes (the pull-request equivalent).
///
/// Both `base` and `head` are immutable revisions: the review records
/// where the proposal started and where it stood, independent of branch
/// movement afterwards. The head must differ from the base — a no-change
/// review is rejected.
///
/// Whether the proposal may be merged, and by whom, is decided by review
/// approvals and workflow roles; this function only constructs the
/// contract record.
pub fn propose_review(
    repository: &WorkflowRepositoryId,
    review: ReviewId,
    title: Option<String>,
    base: &ImmutableSourceRevision,
    head: &ImmutableSourceRevision,
) -> Result<WorkflowReview, WorkflowForgeError> {
    if base.commit_sha == head.commit_sha {
        return Err(WorkflowForgeError::ReviewHeadEqualsBase {
            review,
            base: base.commit_sha.clone(),
            head: head.commit_sha.clone(),
        });
    }
    Ok(WorkflowReview {
        review,
        repository: repository.clone(),
        title,
        base: base.clone(),
        head: head.clone(),
        state: ReviewState::Open,
    })
}

/// Merges an open review.
///
/// The merge:
///
/// - requires the review to be open (merged reviews cannot merge again,
///   closed reviews must be re-proposed);
/// - requires at least one approval recorded at the review's *current*
///   head, so approvals cannot be reused after the proposal moves;
/// - requires a merge revision that is a *new* immutable revision,
///   distinct from both base and head. Forge-side fast-forwards are
///   therefore out of model: the host merges with an explicit merge
///   commit (merge commit, squash, or rebase-and-merge semantics).
///
/// The returned record carries `ReviewState::Merged` with the immutable
/// merge revision. Advancing the target development branch to that
/// revision is the host's ref operation; this crate only validates and
/// records the semantics.
pub fn merge_review(
    review: &WorkflowReview,
    approvals: &[ReviewApproval],
    merge_revision: RevisionSha,
) -> Result<WorkflowReview, WorkflowForgeError> {
    match &review.state {
        ReviewState::Open => {}
        ReviewState::Merged { merge_revision } => {
            return Err(WorkflowForgeError::ReviewAlreadyMerged {
                review: review.review.clone(),
                merge_revision: merge_revision.clone(),
            });
        }
        ReviewState::Closed => {
            return Err(WorkflowForgeError::ReviewAlreadyClosed {
                review: review.review.clone(),
            });
        }
    }
    let approved_at_head = approvals.iter().any(|approval| {
        approval.review == review.review && approval.approved_head == review.head.commit_sha
    });
    if !approved_at_head {
        return Err(WorkflowForgeError::MissingApprovalAtHead {
            review: review.review.clone(),
            head: review.head.commit_sha.clone(),
        });
    }
    if merge_revision == review.head.commit_sha || merge_revision == review.base.commit_sha {
        return Err(WorkflowForgeError::InvalidMergeRevision {
            review: review.review.clone(),
            merge_revision,
        });
    }
    Ok(WorkflowReview {
        state: ReviewState::Merged {
            merge_revision: merge_revision.clone(),
        },
        ..review.clone()
    })
}

/// Closes an open review without merging. Merged reviews are immutable
/// history and cannot be closed.
pub fn close_review(review: &WorkflowReview) -> Result<WorkflowReview, WorkflowForgeError> {
    match &review.state {
        ReviewState::Open => Ok(WorkflowReview {
            state: ReviewState::Closed,
            ..review.clone()
        }),
        ReviewState::Merged { merge_revision } => Err(WorkflowForgeError::ReviewAlreadyMerged {
            review: review.review.clone(),
            merge_revision: merge_revision.clone(),
        }),
        ReviewState::Closed => Err(WorkflowForgeError::ReviewAlreadyClosed {
            review: review.review.clone(),
        }),
    }
}

/// Publishes a release: cuts an immutable tag at `target` and lists the
/// workflow versions published from it.
///
/// Every version must:
///
/// - verify against its recomputed identity digest (immutability);
/// - be anchored exactly at `target`'s commit SHA — versions built from a
///   different revision, or from a *moving branch* that has since
///   advanced, are rejected;
/// - appear at most once per workflow in the release.
///
/// Tags must be unique per repository: re-publishing under an existing
/// tag is an explicit control-plane operation, never a silent overwrite.
/// The tag name remains a display handle; the release's authority is its
/// immutable target revision and the version identities it publishes.
pub fn publish_release(
    repository: &WorkflowRepositoryId,
    existing: &[WorkflowRelease],
    tag: &str,
    target: &ImmutableSourceRevision,
    versions: &[PublishedVersionRef],
) -> Result<WorkflowRelease, WorkflowForgeError> {
    validate_tag(tag)?;
    if existing
        .iter()
        .any(|release| &release.repository == repository && release.tag == tag)
    {
        return Err(WorkflowForgeError::DuplicateReleaseTag {
            repository: repository.clone(),
            tag: tag.to_owned(),
        });
    }
    let mut published = Vec::with_capacity(versions.len());
    let mut workflows = Vec::with_capacity(versions.len());
    for version in versions {
        version.verify()?;
        if version.identity.source_revision.commit_sha != target.commit_sha {
            return Err(WorkflowForgeError::VersionNotAnchoredAtTarget {
                workflow: version.identity.workflow.clone(),
                expected: target.commit_sha.clone(),
                actual: version.identity.source_revision.commit_sha.clone(),
            });
        }
        if workflows.contains(&version.identity.workflow) {
            return Err(WorkflowForgeError::DuplicateReleaseWorkflow {
                workflow: version.identity.workflow.clone(),
                tag: tag.to_owned(),
            });
        }
        workflows.push(version.identity.workflow.clone());
        published.push(version.version_id.clone());
    }
    Ok(WorkflowRelease {
        tag: tag.to_owned(),
        repository: repository.clone(),
        target: target.clone(),
        versions: published,
    })
}

/// Creates a fork repository record derived from an origin repository.
///
/// The fork copies the origin's development snapshot — default branch,
/// branch states, and workflow manifests — so work can continue
/// immediately, and carries fork lineage and maintainer attribution as
/// data. Lineage and attribution are supplied by the host (translated
/// from forge records); the forge layer never invents provenance.
///
/// The fork's identity must differ from the origin's.
pub fn fork_repository(
    origin: &WorkflowRepository,
    fork_identity: WorkflowRepositoryId,
    lineage: ForkLineage,
    maintainer: Attribution,
) -> Result<WorkflowRepository, WorkflowForgeError> {
    if origin.identity == fork_identity {
        return Err(WorkflowForgeError::ForkIdentityCollision {
            identity: fork_identity,
        });
    }
    Ok(WorkflowRepository {
        identity: fork_identity,
        forge: origin.forge.clone(),
        origin: Some(origin.identity.clone()),
        forked_from: Some(lineage),
        default_branch: origin.default_branch.clone(),
        branches: origin.branches.clone(),
        workflows: origin.workflows.clone(),
        maintainers: vec![maintainer],
    })
}

/// Upserts a development branch state into a branch-state list.
///
/// Branch states are development-time snapshots: their heads move freely
/// and never participate in execution identity.
pub fn upsert_branch_state(
    branches: &[WorkflowBranchState],
    branch: DevelopmentRef,
    head: RevisionSha,
) -> Vec<WorkflowBranchState> {
    let mut updated: Vec<WorkflowBranchState> = branches
        .iter()
        .filter(|state| state.branch != branch)
        .cloned()
        .collect();
    updated.push(WorkflowBranchState {
        branch,
        head: Some(head),
    });
    updated
}

#[cfg(test)]
mod tests {
    use codex_workflow_contracts::ReviewState;

    use crate::test_support::published_version;
    use crate::test_support::repo_id;
    use crate::test_support::sha;
    use crate::test_support::workflow_id;

    use super::close_review;
    use super::fork_repository;
    use super::merge_review;
    use super::propose_review;
    use super::publish_release;
    use super::upsert_branch_state;
    use super::ReviewApproval;
    use crate::WorkflowForgeError;

    fn sample_review() -> codex_workflow_contracts::WorkflowReview {
        let repository = repo_id("github.com/acme/ops-workflows");
        let base = sha(1);
        let head = sha(2);
        propose_review(
            &repository,
            codex_workflow_contracts::ReviewId::parse("101").unwrap(),
            Some("Add retry step".to_owned()),
            &codex_workflow_contracts::ImmutableSourceRevision::pin_commit(base),
            &codex_workflow_contracts::ImmutableSourceRevision::pin_commit(head),
        )
        .unwrap()
    }

    #[test]
    fn proposes_reviews_with_immutable_base_and_head() {
        let review = sample_review();
        assert_eq!(review.state, ReviewState::Open);
        assert_eq!(review.base.commit_sha, sha(1));
        assert_eq!(review.head.commit_sha, sha(2));
        assert_eq!(review.repository, repo_id("github.com/acme/ops-workflows"));
        assert_eq!(review.title.as_deref(), Some("Add retry step"));
    }

    #[test]
    fn rejects_reviews_where_head_equals_base() {
        let repository = repo_id("github.com/acme/ops-workflows");
        let revision = codex_workflow_contracts::ImmutableSourceRevision::pin_commit(sha(7));
        let result = propose_review(
            &repository,
            codex_workflow_contracts::ReviewId::parse("102").unwrap(),
            None,
            &revision,
            &revision,
        );
        assert!(matches!(
            result,
            Err(WorkflowForgeError::ReviewHeadEqualsBase { .. })
        ));
    }

    #[test]
    fn merging_requires_an_open_review_and_a_fresh_approval() {
        let review = sample_review();
        let merge_sha = sha(9);
        let approvals = vec![ReviewApproval {
            review: review.review.clone(),
            reviewer: "bob".to_owned(),
            approved_head: sha(1), // approval recorded at a stale head
        }];
        let stale = merge_review(&review, &approvals, merge_sha.clone());
        assert!(matches!(
            stale,
            Err(WorkflowForgeError::MissingApprovalAtHead { .. })
        ));

        let fresh = vec![ReviewApproval {
            review: review.review.clone(),
            reviewer: "bob".to_owned(),
            approved_head: sha(2),
        }];
        // A merge revision equal to base or head is not a new revision.
        let reused_head = merge_review(&review, &fresh, sha(2));
        assert!(matches!(
            reused_head,
            Err(WorkflowForgeError::InvalidMergeRevision { .. })
        ));

        let merged = merge_review(&review, &fresh, merge_sha.clone()).unwrap();
        assert_eq!(
            merged.state,
            ReviewState::Merged {
                merge_revision: merge_sha
            }
        );
        // The merged record is immutable history: it cannot merge again.
        let again = merge_review(&merged, &fresh, sha(10));
        assert!(matches!(
            again,
            Err(WorkflowForgeError::ReviewAlreadyMerged { .. })
        ));
    }

    #[test]
    fn closes_only_open_reviews() {
        let review = sample_review();
        let closed = close_review(&review).unwrap();
        assert_eq!(closed.state, ReviewState::Closed);
        assert!(matches!(
            close_review(&closed),
            Err(WorkflowForgeError::ReviewAlreadyClosed { .. })
        ));
    }

    #[test]
    fn publishing_requires_anchored_and_verified_versions() {
        let repository = repo_id("github.com/acme/ops-workflows");
        let target = sha(30);
        let anchored = published_version(
            "deploy",
            "github.com/acme/ops-workflows",
            &target,
            (1, 2, 0),
            "d1",
            "l1",
        );
        let target_revision =
            codex_workflow_contracts::ImmutableSourceRevision::pin_commit(target.clone());

        let release = publish_release(
            &repository,
            &[],
            "v1.2.0",
            &target_revision,
            std::slice::from_ref(&anchored),
        )
        .unwrap();
        assert_eq!(release.versions, vec![anchored.version_id.clone()]);
        assert_eq!(release.target.commit_sha, target);
        assert_eq!(release.tag, "v1.2.0");

        // A version anchored at a different revision is rejected.
        let unanchored = published_version(
            "deploy",
            "github.com/acme/ops-workflows",
            &sha(31),
            (1, 2, 1),
            "d1",
            "l1",
        );
        assert!(matches!(
            publish_release(&repository, &[], "v1.2.1", &target_revision, &[unanchored]),
            Err(WorkflowForgeError::VersionNotAnchoredAtTarget { .. })
        ));

        // A tampered version reference fails digest recomputation.
        let mut tampered = anchored.clone();
        tampered.version_id = published_version(
            "deploy",
            "github.com/acme/ops-workflows",
            &target,
            (1, 3, 0),
            "d1",
            "l1",
        )
        .version_id;
        assert!(matches!(
            publish_release(
                &repository,
                &[],
                "v1.2.2",
                &target_revision,
                &[tampered.clone()]
            ),
            Err(WorkflowForgeError::VersionIdentityMismatch { .. })
        ));
        assert!(matches!(
            tampered.verify(),
            Err(WorkflowForgeError::VersionIdentityMismatch { .. })
        ));

        // Duplicate tags are refused, and duplicate workflow versions in
        // one release are ambiguous.
        assert!(matches!(
            publish_release(
                &repository,
                &[release],
                "v1.2.0",
                &target_revision,
                std::slice::from_ref(&anchored)
            ),
            Err(WorkflowForgeError::DuplicateReleaseTag { .. })
        ));
        let second = published_version(
            "deploy",
            "github.com/acme/ops-workflows",
            &target,
            (1, 2, 0),
            "d2",
            "l1",
        );
        assert!(matches!(
            publish_release(
                &repository,
                &[],
                "v1.2.3",
                &target_revision,
                &[anchored, second]
            ),
            Err(WorkflowForgeError::DuplicateReleaseWorkflow { .. })
        ));
    }

    #[test]
    fn forking_copies_development_state_with_lineage_and_attribution() {
        use crate::test_support::attribution;
        use crate::test_support::fork_lineage;
        use crate::test_support::manifest;

        let origin_id = repo_id("github.com/acme/ops-workflows");
        let origin = codex_workflow_contracts::WorkflowRepository {
            identity: origin_id.clone(),
            forge: crate::test_support::github_kind(),
            origin: None,
            forked_from: None,
            default_branch: codex_workflow_contracts::DevelopmentRef::branch("main").unwrap(),
            branches: vec![codex_workflow_contracts::WorkflowBranchState {
                branch: codex_workflow_contracts::DevelopmentRef::branch("main").unwrap(),
                head: Some(sha(1)),
            }],
            workflows: vec![manifest(
                "deploy",
                "github.com/acme/ops-workflows",
                "workflows/deploy.toml",
            )],
            maintainers: vec![attribution("acme-ops")],
        };
        let fork_id = repo_id("github.com/alice/ops-workflows");
        let lineage = fork_lineage("github.com/acme/ops-workflows");
        let fork = fork_repository(
            &origin,
            fork_id.clone(),
            lineage.clone(),
            attribution("alice"),
        )
        .unwrap();
        assert_eq!(fork.identity, fork_id);
        assert_eq!(fork.origin.as_ref(), Some(&origin_id));
        assert_eq!(fork.forked_from.as_ref(), Some(&lineage));
        assert_eq!(fork.workflows, origin.workflows);
        assert_eq!(fork.default_branch, origin.default_branch);
        assert_eq!(fork.maintainers, vec![attribution("alice")]);

        let collision = fork_repository(&origin, origin_id.clone(), lineage, attribution("alice"));
        assert!(matches!(
            collision,
            Err(WorkflowForgeError::ForkIdentityCollision { .. })
        ));
    }

    #[test]
    fn upserts_branch_states_without_duplicating_branches() {
        use codex_workflow_contracts::DevelopmentRef;
        use codex_workflow_contracts::WorkflowBranchState;

        let main = DevelopmentRef::branch("main").unwrap();
        let mut branches = vec![WorkflowBranchState {
            branch: main.clone(),
            head: Some(sha(1)),
        }];
        branches = upsert_branch_state(&branches, main.clone(), sha(2));
        assert_eq!(branches.len(), 1);
        assert_eq!(branches[0].head.as_ref(), Some(&sha(2)));

        let feature = DevelopmentRef::branch("feature/retry").unwrap();
        branches = upsert_branch_state(&branches, feature.clone(), sha(3));
        assert_eq!(branches.len(), 2);
        assert_eq!(
            branches
                .iter()
                .find(|state| state.branch == feature)
                .and_then(|state| state.head.as_ref()),
            Some(&sha(3))
        );
    }

    #[test]
    fn version_references_project_from_full_records_and_verify() {
        // `PublishedVersionRef::of` reads the known fields of a full
        // contract version record; verification recomputes the digest.
        let version = published_version(
            "deploy",
            "github.com/acme/ops-workflows",
            &sha(40),
            (2, 0, 0),
            "definition",
            "lock",
        );
        assert_eq!(version.identity.workflow, workflow_id("deploy"));
        assert!(version.verify().is_ok());
    }
}
