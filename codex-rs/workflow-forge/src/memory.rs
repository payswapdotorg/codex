//! An in-memory forge simulation: the git object and ref layer as pure
//! data.
//!
//! [`InMemoryForge`] exists for unit tests and host seeding. It models
//! what a hosting forge's git layer provides — commits, movable refs,
//! and repository snapshots — with **no I/O, no execution, and no
//! collaboration semantics of its own**: review, merge, release, and
//! install semantics are the pure functions in [`crate::collaboration`],
//! [`crate::package`], and [`crate::install`], which tests drive over
//! this simulation.
//!
//! Modelling choices that mirror real forges:
//!
//! - Commits are minted deterministically from the forge salt, a
//!   monotonically increasing counter, the parents, the message, and the
//!   author, formatted as full 40-hex SHAs. Identical sequences on
//!   identically-salted forges produce identical SHAs; differently-salted
//!   forges never collide.
//! - The commit object store is **forge-wide**, like a GitHub fork
//!   network's shared object database: a commit merged across repositories
//!   of one forge is visible to all of them.
//! - `clone_repository_from`, `fetch_repository_from`, and
//!   `push_branch_to` simulate git transport between two forges, copying
//!   exactly the objects reachable from the relevant refs.
//! - Branch heads move freely; nothing here is an execution anchor.

use std::collections::HashMap;
use std::future::ready;
use std::future::Future;

use codex_workflow_contracts::Attribution;
use codex_workflow_contracts::DevelopmentRef;
use codex_workflow_contracts::ForgeKind;
use codex_workflow_contracts::ForkLineage;
use codex_workflow_contracts::ImmutableSourceRevision;
use codex_workflow_contracts::RevisionSha;
use codex_workflow_contracts::WorkflowBranchState;
use codex_workflow_contracts::WorkflowContractError;
use codex_workflow_contracts::WorkflowForge;
use codex_workflow_contracts::WorkflowManifest;
use codex_workflow_contracts::WorkflowRepository;
use codex_workflow_contracts::WorkflowRepositoryId;

use crate::collaboration::fork_repository;
use crate::collaboration::CommitSummary;
use crate::WorkflowForgeError;

/// FNV-1a offset basis used for deterministic commit minting.
const FNV_OFFSET: u64 = 0xcbf29ce484222325;
/// FNV-1a prime used for deterministic commit minting.
const FNV_PRIME: u64 = 0x100000001b3;

/// Per-repository state on the simulated forge: the repository record
/// and its movable refs.
struct MemoryRepository {
    record: WorkflowRepository,
    refs: HashMap<DevelopmentRef, RevisionSha>,
}

/// An in-memory forge: a pure-data simulation of a hosting forge's git
/// object and ref layer.
///
/// The forge implements the frozen [`WorkflowForge`] boundary, so
/// collaboration code cannot tell a simulated forge from a real adapter
/// — an invariant the unit tests rely on when they exercise the full
/// fork / review / merge / publish / install flow.
pub struct InMemoryForge {
    kind: ForgeKind,
    salt: u64,
    repos: HashMap<WorkflowRepositoryId, MemoryRepository>,
    commits: HashMap<RevisionSha, CommitSummary>,
    counter: u64,
}

impl InMemoryForge {
    /// Creates an empty simulated forge with the default salt.
    pub fn new(kind: ForgeKind) -> Self {
        Self::with_salt(kind, 0)
    }

    /// Creates an empty simulated forge with an explicit salt.
    ///
    /// The salt separates object identity across simulated forges: two
    /// forges with different salts never mint the same commit SHA, which
    /// keeps deterministic tests collision-free while clone, fetch, and
    /// push still share objects as git does.
    pub fn with_salt(kind: ForgeKind, salt: u64) -> Self {
        Self {
            kind,
            salt,
            repos: HashMap::new(),
            commits: HashMap::new(),
            counter: 0,
        }
    }

    /// The forge kind this simulation services.
    pub fn kind(&self) -> &ForgeKind {
        &self.kind
    }

    /// Registers a new repository on the forge with an empty ref set.
    ///
    /// Manifests must validate and declare exactly this repository —
    /// external outputs stay untrusted even in the simulation.
    pub fn seed_repository(
        &mut self,
        identity: WorkflowRepositoryId,
        default_branch: DevelopmentRef,
        maintainers: Vec<Attribution>,
        workflows: Vec<WorkflowManifest>,
    ) -> Result<(), WorkflowForgeError> {
        if self.repos.contains_key(&identity) {
            return Err(WorkflowForgeError::InvalidRecord {
                reason: format!("repository {identity:?} is already seeded"),
            });
        }
        for manifest in &workflows {
            manifest.validate()?;
            if manifest.repository != identity {
                return Err(WorkflowForgeError::ManifestRepositoryMismatch {
                    manifest_repository: manifest.repository.clone(),
                    expected_repository: identity.clone(),
                });
            }
        }
        let record = WorkflowRepository {
            identity: identity.clone(),
            forge: self.kind.clone(),
            origin: None,
            forked_from: None,
            default_branch,
            branches: Vec::new(),
            workflows,
            maintainers,
        };
        self.repos.insert(
            identity,
            MemoryRepository {
                record,
                refs: HashMap::new(),
            },
        );
        Ok(())
    }

    /// Creates a development branch at an existing commit.
    ///
    /// Branches cannot be re-created: moving them happens through
    /// commits and merges.
    pub fn create_branch(
        &mut self,
        repository: &WorkflowRepositoryId,
        branch: DevelopmentRef,
        from: &RevisionSha,
    ) -> Result<(), WorkflowForgeError> {
        if !self.commits.contains_key(from) {
            return Err(WorkflowForgeError::UnknownCommit {
                repository: repository.clone(),
                commit: from.clone(),
            });
        }
        let state = self.repository_state_mut(repository)?;
        if state.refs.contains_key(&branch) {
            return Err(WorkflowForgeError::InvalidRecord {
                reason: format!("branch {branch} already exists"),
            });
        }
        state.refs.insert(branch, from.clone());
        Ok(())
    }

    /// Resolves a branch head, allowing an unborn default branch.
    ///
    /// A freshly seeded repository has no refs yet; its first commit on
    /// the default branch is the genesis commit (no parents). Every
    /// other unborn branch stays `UnknownBranch`.
    fn genesis_or_head(
        &self,
        repository: &WorkflowRepositoryId,
        branch: &DevelopmentRef,
    ) -> Result<Option<RevisionSha>, WorkflowForgeError> {
        let state = self.repository_state(repository)?;
        match state.refs.get(branch) {
            Some(sha) => Ok(Some(sha.clone())),
            None if state.record.default_branch == *branch && state.refs.is_empty() => Ok(None),
            None => Err(WorkflowForgeError::UnknownBranch {
                repository: repository.clone(),
                branch: branch.clone(),
            }),
        }
    }

    /// Commits onto a branch, advancing its head.
    ///
    /// The commit has the branch's current head as its single parent
    /// (or no parent, for a repository's first commit).
    pub fn commit(
        &mut self,
        repository: &WorkflowRepositoryId,
        branch: &DevelopmentRef,
        message: impl Into<String>,
        author: impl Into<String>,
    ) -> Result<CommitSummary, WorkflowForgeError> {
        let (message, author) = (message.into(), author.into());
        // The branch's current head is the commit's single parent; an
        // unborn default branch takes its first commit with no parent.
        let head = self.genesis_or_head(repository, branch)?;
        let parents = head.map(|sha| vec![sha]).unwrap_or_default();
        let sha = self.mint_commit(&parents, &message, &author);
        let summary = CommitSummary {
            revision: ImmutableSourceRevision::pin_commit(sha.clone()),
            message,
            author,
            parents,
        };
        self.repository_state_mut(repository)?
            .refs
            .insert(branch.clone(), sha);
        self.store_commit(summary.clone());
        Ok(summary)
    }

    /// Creates a merge commit of `head_to_merge` into a branch,
    /// advancing the branch head to the merge commit.
    ///
    /// The merge commit's parents are `[current branch head,
    /// head_to_merge]`. Because the object store is forge-wide, merging
    /// a commit from another repository of the same forge (a fork
    /// network) works exactly like integrating upstream.
    pub fn create_merge_commit(
        &mut self,
        repository: &WorkflowRepositoryId,
        into_branch: &DevelopmentRef,
        head_to_merge: &RevisionSha,
        message: impl Into<String>,
        author: impl Into<String>,
    ) -> Result<CommitSummary, WorkflowForgeError> {
        let (message, author) = (message.into(), author.into());
        let base = self.head(repository, into_branch)?;
        if !self.commits.contains_key(head_to_merge) {
            return Err(WorkflowForgeError::UnknownCommit {
                repository: repository.clone(),
                commit: head_to_merge.clone(),
            });
        }
        if head_to_merge == &base {
            return Err(WorkflowForgeError::InvalidRecord {
                reason: "a merge commit must merge a different revision into the branch head"
                    .to_owned(),
            });
        }
        let parents = vec![base, head_to_merge.clone()];
        let sha = self.mint_commit(&parents, &message, &author);
        let summary = CommitSummary {
            revision: ImmutableSourceRevision::pin_commit(sha.clone()),
            message,
            author,
            parents,
        };
        self.repository_state_mut(repository)?
            .refs
            .insert(into_branch.clone(), sha);
        self.store_commit(summary.clone());
        Ok(summary)
    }

    /// The current head of a development branch.
    pub fn head(
        &self,
        repository: &WorkflowRepositoryId,
        branch: &DevelopmentRef,
    ) -> Result<RevisionSha, WorkflowForgeError> {
        let state = self.repository_state(repository)?;
        state
            .refs
            .get(branch)
            .cloned()
            .ok_or_else(|| WorkflowForgeError::UnknownBranch {
                repository: repository.clone(),
                branch: branch.clone(),
            })
    }

    /// The branch states of a repository, sorted by ref for
    /// deterministic snapshots.
    pub fn branch_states(
        &self,
        repository: &WorkflowRepositoryId,
    ) -> Result<Vec<WorkflowBranchState>, WorkflowForgeError> {
        let state = self.repository_state(repository)?;
        let mut branches: Vec<WorkflowBranchState> = state
            .refs
            .iter()
            .map(|(branch, head)| WorkflowBranchState {
                branch: branch.clone(),
                head: Some(head.clone()),
            })
            .collect();
        branches.sort_by(|left, right| left.branch.cmp(&right.branch));
        Ok(branches)
    }

    /// The commit summary for a SHA, when the object is known.
    pub fn commit_summary(
        &self,
        repository: &WorkflowRepositoryId,
        commit: &RevisionSha,
    ) -> Result<Option<CommitSummary>, WorkflowForgeError> {
        let _ = self.repository_state(repository)?;
        Ok(self.commits.get(commit).cloned())
    }

    /// The commits reachable from a repository's refs, sorted by SHA.
    pub fn commit_summaries(
        &self,
        repository: &WorkflowRepositoryId,
    ) -> Result<Vec<CommitSummary>, WorkflowForgeError> {
        let state = self.repository_state(repository)?;
        let roots: Vec<RevisionSha> = state.refs.values().cloned().collect();
        let mut summaries: Vec<CommitSummary> = reachable_commits(&self.commits, &roots)
            .into_values()
            .collect();
        summaries.sort_by(|left, right| left.revision.commit_sha.cmp(&right.revision.commit_sha));
        Ok(summaries)
    }

    /// Blocking form of ref resolution.
    ///
    /// Resolves a movable development ref to an immutable revision
    /// anchored at the ref's current head, recording the ref as
    /// `resolved_from` provenance.
    pub fn resolve_ref_blocking(
        &self,
        repository: &WorkflowRepositoryId,
        reference: &DevelopmentRef,
    ) -> Result<ImmutableSourceRevision, WorkflowForgeError> {
        let state = self.repository_state(repository)?;
        match state.refs.get(reference) {
            Some(sha) => Ok(ImmutableSourceRevision::pin(
                sha.clone(),
                Some(reference.clone()),
            )),
            None => Err(WorkflowForgeError::UnknownBranch {
                repository: repository.clone(),
                branch: reference.clone(),
            }),
        }
    }

    /// Blocking form of the repository record read: a snapshot with
    /// current branch states.
    pub fn repository_record_blocking(
        &self,
        repository: &WorkflowRepositoryId,
    ) -> Result<WorkflowRepository, WorkflowForgeError> {
        let state = self.repository_state(repository)?;
        let branches = self.branch_states(repository)?;
        Ok(WorkflowRepository {
            branches,
            ..state.record.clone()
        })
    }

    /// Folds a repository on this forge: the fork record derives from
    /// the origin's snapshot with lineage and attribution supplied by
    /// the caller, and shares the forge's object store, exactly like a
    /// server-side fork in a fork network.
    pub fn fork(
        &mut self,
        origin: &WorkflowRepositoryId,
        fork_identity: WorkflowRepositoryId,
        lineage: ForkLineage,
        maintainer: Attribution,
    ) -> Result<WorkflowRepository, WorkflowForgeError> {
        if self.repos.contains_key(&fork_identity) {
            return Err(WorkflowForgeError::InvalidRecord {
                reason: format!("fork identity {fork_identity:?} already exists on this forge"),
            });
        }
        let state = self.repository_state(origin)?;
        let origin_record = state.record.clone();
        let refs = state.refs.clone();
        let fork_record =
            fork_repository(&origin_record, fork_identity.clone(), lineage, maintainer)?;
        self.repos.insert(
            fork_identity,
            MemoryRepository {
                record: fork_record.clone(),
                refs,
            },
        );
        Ok(fork_record)
    }

    /// Simulates a clone: this forge starts tracking the remote
    /// repository, copying its record, refs, and reachable objects.
    pub fn clone_repository_from(
        &mut self,
        remote: &InMemoryForge,
        repository: &WorkflowRepositoryId,
    ) -> Result<WorkflowRepository, WorkflowForgeError> {
        if self.repos.contains_key(repository) {
            return Err(WorkflowForgeError::InvalidRecord {
                reason: format!("local forge already tracks repository {repository:?}"),
            });
        }
        let state = remote.repository_state(repository)?;
        let record = state.record.clone();
        let refs = state.refs.clone();
        let roots: Vec<RevisionSha> = refs.values().cloned().collect();
        let objects = reachable_commits(&remote.commits, &roots);
        self.repos
            .insert(repository.clone(), MemoryRepository { record, refs });
        self.commits.extend(objects);
        self.repository_record_blocking(repository)
    }

    /// Simulates a fetch: refreshes this forge's record and refs for a
    /// repository from the remote, unioning in the reachable objects.
    pub fn fetch_repository_from(
        &mut self,
        remote: &InMemoryForge,
        repository: &WorkflowRepositoryId,
    ) -> Result<(), WorkflowForgeError> {
        let state = remote.repository_state(repository)?;
        let record = state.record.clone();
        let refs = state.refs.clone();
        let roots: Vec<RevisionSha> = refs.values().cloned().collect();
        let objects = reachable_commits(&remote.commits, &roots);
        let local = self.repository_state_mut(repository)?;
        local.record = record;
        local.refs = refs;
        self.commits.extend(objects);
        Ok(())
    }

    /// Simulates a push: publishes a local branch head to the remote
    /// forge, copying the objects reachable from the head.
    pub fn push_branch_to(
        &self,
        remote: &mut InMemoryForge,
        repository: &WorkflowRepositoryId,
        branch: &DevelopmentRef,
    ) -> Result<RevisionSha, WorkflowForgeError> {
        let local = self.repository_state(repository)?;
        let head =
            local
                .refs
                .get(branch)
                .cloned()
                .ok_or_else(|| WorkflowForgeError::UnknownBranch {
                    repository: repository.clone(),
                    branch: branch.clone(),
                })?;
        let objects = reachable_commits(&self.commits, std::slice::from_ref(&head));
        let remote_state = remote.repository_state_mut(repository)?;
        remote_state.refs.insert(branch.clone(), head.clone());
        remote.commits.extend(objects);
        Ok(head)
    }

    /// Looks up a repository's state.
    fn repository_state(
        &self,
        repository: &WorkflowRepositoryId,
    ) -> Result<&MemoryRepository, WorkflowForgeError> {
        self.repos
            .get(repository)
            .ok_or_else(|| WorkflowForgeError::UnknownRepository {
                repository: repository.clone(),
            })
    }

    /// Looks up a repository's state mutably.
    fn repository_state_mut(
        &mut self,
        repository: &WorkflowRepositoryId,
    ) -> Result<&mut MemoryRepository, WorkflowForgeError> {
        self.repos
            .get_mut(repository)
            .ok_or_else(|| WorkflowForgeError::UnknownRepository {
                repository: repository.clone(),
            })
    }

    /// Mints a deterministic full commit SHA.
    fn mint_commit(&mut self, parents: &[RevisionSha], message: &str, author: &str) -> RevisionSha {
        self.counter += 1;
        let mut hash = FNV_OFFSET ^ self.salt;
        absorb(&mut hash, &self.counter.to_le_bytes());
        for parent in parents {
            absorb(&mut hash, parent.as_str().as_bytes());
        }
        absorb(&mut hash, message.as_bytes());
        absorb(&mut hash, author.as_bytes());
        RevisionSha::parse(format!("{hash:040x}"))
            .expect("formatted hash is a full 40-hex commit sha")
    }

    /// Stores a commit object in the forge-wide object store.
    fn store_commit(&mut self, commit: CommitSummary) {
        self.commits
            .insert(commit.revision.commit_sha.clone(), commit);
    }
}

impl WorkflowForge for InMemoryForge {
    fn kind(&self) -> &ForgeKind {
        &self.kind
    }

    fn resolve_ref(
        &self,
        repository: &WorkflowRepositoryId,
        reference: &DevelopmentRef,
    ) -> impl Future<Output = Result<ImmutableSourceRevision, WorkflowContractError>> + Send {
        ready(
            self.resolve_ref_blocking(repository, reference)
                .map_err(WorkflowForgeError::as_contract_error),
        )
    }

    fn repository_record(
        &self,
        repository: &WorkflowRepositoryId,
    ) -> impl Future<Output = Result<WorkflowRepository, WorkflowContractError>> + Send {
        ready(
            self.repository_record_blocking(repository)
                .map_err(WorkflowForgeError::as_contract_error),
        )
    }
}

/// Absorbs bytes into a running FNV-1a hash, with a field separator.
fn absorb(hash: &mut u64, bytes: &[u8]) {
    for byte in bytes {
        *hash ^= u64::from(*byte);
        *hash = hash.wrapping_mul(FNV_PRIME);
    }
    *hash ^= 0x1f;
    *hash = hash.wrapping_mul(FNV_PRIME);
}

/// Collects the commits reachable from the given roots.
fn reachable_commits(
    commits: &HashMap<RevisionSha, CommitSummary>,
    roots: &[RevisionSha],
) -> HashMap<RevisionSha, CommitSummary> {
    let mut reachable = HashMap::new();
    let mut stack: Vec<RevisionSha> = roots.to_vec();
    while let Some(sha) = stack.pop() {
        if reachable.contains_key(&sha) {
            continue;
        }
        if let Some(commit) = commits.get(&sha) {
            stack.extend(commit.parents.iter().cloned());
            reachable.insert(sha.clone(), commit.clone());
        }
    }
    reachable
}

#[cfg(test)]
mod tests {
    use codex_workflow_contracts::DevelopmentRef;
    use codex_workflow_contracts::ImmutableSourceRevision;
    use codex_workflow_contracts::ReviewId;
    use codex_workflow_contracts::ReviewState;
    use codex_workflow_contracts::WorkflowForge;

    use crate::collaboration::merge_review;
    use crate::collaboration::propose_review;
    use crate::collaboration::publish_release;
    use crate::collaboration::ReviewApproval;
    use crate::install::InstallRegistry;
    use crate::package::SubworkflowPin;
    use crate::package::WorkflowPackageLock;
    use crate::test_support::attribution;
    use crate::test_support::block_on_ready;
    use crate::test_support::fork_lineage;
    use crate::test_support::github_kind;
    use crate::test_support::manifest;
    use crate::test_support::published_version;
    use crate::test_support::repo_id;
    use crate::test_support::sha;
    use crate::test_support::workflow_id;
    use crate::InMemoryForge;
    use crate::UpdateDecision;
    use crate::WorkflowForgeError;

    const ORIGIN: &str = "github.com/acme/ops-workflows";

    fn seeded_forge(salt: u64) -> InMemoryForge {
        let mut forge = InMemoryForge::with_salt(github_kind(), salt);
        forge
            .seed_repository(
                repo_id(ORIGIN),
                DevelopmentRef::branch("main").unwrap(),
                vec![attribution("acme-ops")],
                vec![manifest("deploy", ORIGIN, "workflows/deploy.toml")],
            )
            .unwrap();
        forge
    }

    #[test]
    fn seeds_commits_and_resolves_refs() {
        let mut forge = seeded_forge(0x3);
        let origin = repo_id(ORIGIN);
        let main = DevelopmentRef::branch("main").unwrap();
        let commit = forge
            .commit(&origin, &main, "initial workflow", "acme-bot")
            .unwrap();
        assert!(commit.parents.is_empty());

        // Ref resolution and repository records go through the frozen,
        // forge-neutral trait boundary.
        let resolved = block_on_ready(forge.resolve_ref(&origin, &main)).unwrap();
        assert_eq!(resolved.commit_sha, commit.revision.commit_sha);
        assert_eq!(resolved.resolved_from.as_ref(), Some(&main));

        let record = block_on_ready(forge.repository_record(&origin)).unwrap();
        assert_eq!(record.branches.len(), 1);
        assert_eq!(
            record.branches[0].head.as_ref(),
            Some(&commit.revision.commit_sha)
        );
        assert!(record.manifest_for(&workflow_id("deploy")).is_some());
        assert_eq!(forge.commit_summaries(&origin).unwrap().len(), 1);

        // Unknown repositories and branches fail.
        let unknown_repository = repo_id("github.com/acme/absent");
        assert!(matches!(
            forge.repository_record_blocking(&unknown_repository),
            Err(WorkflowForgeError::UnknownRepository { .. })
        ));
        let unknown_branch = DevelopmentRef::branch("absent").unwrap();
        assert!(matches!(
            forge.resolve_ref_blocking(&origin, &unknown_branch),
            Err(WorkflowForgeError::UnknownBranch { .. })
        ));
    }

    #[test]
    fn branches_and_merges_validate_their_inputs() {
        let mut forge = seeded_forge(0x5);
        let origin = repo_id(ORIGIN);
        let main = DevelopmentRef::branch("main").unwrap();
        let base = forge.commit(&origin, &main, "initial", "acme-bot").unwrap();
        let base_sha = base.revision.commit_sha.clone();

        let feature = DevelopmentRef::branch("feature/x").unwrap();
        let unknown = sha(999);
        assert!(matches!(
            forge.create_branch(&origin, feature.clone(), &unknown),
            Err(WorkflowForgeError::UnknownCommit { .. })
        ));
        forge
            .create_branch(&origin, feature.clone(), &base_sha)
            .unwrap();
        assert!(matches!(
            forge.create_branch(&origin, feature.clone(), &base_sha),
            Err(WorkflowForgeError::InvalidRecord { .. })
        ));

        let head = forge
            .commit(&origin, &feature, "work on feature", "alice")
            .unwrap();
        // Merging a branch into its own head, or an unknown commit, is
        // rejected; a real merge carries both parents.
        assert!(matches!(
            forge.create_merge_commit(&origin, &main, &base_sha, "noop", "bot"),
            Err(WorkflowForgeError::InvalidRecord { .. })
        ));
        assert!(matches!(
            forge.create_merge_commit(&origin, &main, &unknown, "merge", "bot"),
            Err(WorkflowForgeError::UnknownCommit { .. })
        ));
        let merge = forge
            .create_merge_commit(
                &origin,
                &main,
                &head.revision.commit_sha,
                "merge feature",
                "acme-bot",
            )
            .unwrap();
        assert_eq!(
            merge.parents,
            vec![base_sha, head.revision.commit_sha.clone()]
        );
        assert_eq!(
            forge.head(&origin, &main).unwrap(),
            merge.revision.commit_sha
        );
    }

    #[test]
    fn clones_fetches_and_pushes_between_forges() {
        let mut remote = seeded_forge(0x1);
        let origin = repo_id(ORIGIN);
        let main = DevelopmentRef::branch("main").unwrap();
        let first = remote
            .commit(&origin, &main, "initial", "acme-bot")
            .unwrap();
        let second = remote.commit(&origin, &main, "second", "acme-bot").unwrap();

        // Clone: the local forge sees the remote's refs and history.
        let mut local = InMemoryForge::with_salt(github_kind(), 0x2);
        let record = local.clone_repository_from(&remote, &origin).unwrap();
        assert_eq!(
            record.branches[0].head.as_ref(),
            Some(&second.revision.commit_sha)
        );
        assert!(local
            .commit_summary(&origin, &first.revision.commit_sha)
            .unwrap()
            .is_some());
        assert!(local.clone_repository_from(&remote, &origin).is_err());

        // Push: local development reaches the remote.
        let feature = DevelopmentRef::branch("feature/local").unwrap();
        local
            .create_branch(&origin, feature.clone(), &first.revision.commit_sha)
            .unwrap();
        let pushed = local
            .commit(&origin, &feature, "local work", "alice")
            .unwrap();
        let pushed_head = local
            .push_branch_to(&mut remote, &origin, &feature)
            .unwrap();
        assert_eq!(pushed_head, pushed.revision.commit_sha);
        assert_eq!(
            remote
                .resolve_ref_blocking(&origin, &feature)
                .unwrap()
                .commit_sha,
            pushed.revision.commit_sha
        );

        // Fetch: remote progress reaches the local clone.
        let third = remote.commit(&origin, &main, "third", "acme-bot").unwrap();
        local.fetch_repository_from(&remote, &origin).unwrap();
        assert_eq!(
            local.head(&origin, &main).unwrap(),
            third.revision.commit_sha
        );
    }

    #[test]
    fn mints_deterministic_and_distinct_commit_ids() {
        let main = DevelopmentRef::branch("main").unwrap();
        let manifest_entry = manifest("deploy", ORIGIN, "workflows/deploy.toml");
        let origin = repo_id(ORIGIN);

        let mut first = InMemoryForge::with_salt(github_kind(), 0x7);
        first
            .seed_repository(
                origin.clone(),
                main.clone(),
                vec![attribution("acme")],
                vec![manifest_entry.clone()],
            )
            .unwrap();
        let first_commit = first.commit(&origin, &main, "initial", "acme-bot").unwrap();

        let mut second = InMemoryForge::with_salt(github_kind(), 0x7);
        second
            .seed_repository(
                origin.clone(),
                main.clone(),
                vec![attribution("acme")],
                vec![manifest_entry.clone()],
            )
            .unwrap();
        let second_commit = second
            .commit(&origin, &main, "initial", "acme-bot")
            .unwrap();
        assert_eq!(
            first_commit.revision.commit_sha,
            second_commit.revision.commit_sha
        );

        let mut third = InMemoryForge::with_salt(github_kind(), 0x8);
        third
            .seed_repository(
                origin,
                main,
                vec![attribution("acme")],
                vec![manifest_entry],
            )
            .unwrap();
        let third_commit = third
            .commit(
                &repo_id(ORIGIN),
                &DevelopmentRef::branch("main").unwrap(),
                "initial",
                "acme-bot",
            )
            .unwrap();
        assert_ne!(
            first_commit.revision.commit_sha,
            third_commit.revision.commit_sha
        );
    }

    #[test]
    fn published_versions_survive_branch_movement() {
        let mut forge = seeded_forge(0x4);
        let origin = repo_id(ORIGIN);
        let main = DevelopmentRef::branch("main").unwrap();
        let first = forge.commit(&origin, &main, "initial", "acme-bot").unwrap();

        let released = published_version(
            "deploy",
            ORIGIN,
            &first.revision.commit_sha,
            (1, 0, 0),
            "definition",
            "lock",
        );
        let release = publish_release(
            &origin,
            &[],
            "v1.0.0",
            &ImmutableSourceRevision::pin(
                first.revision.commit_sha.clone(),
                Some(DevelopmentRef::tag("v1.0.0").unwrap()),
            ),
            std::slice::from_ref(&released),
        )
        .unwrap();

        // The branch moves after publication.
        let second = forge
            .commit(&origin, &main, "post-release", "acme-bot")
            .unwrap();
        assert_eq!(
            forge
                .resolve_ref_blocking(&origin, &main)
                .unwrap()
                .commit_sha,
            second.revision.commit_sha
        );

        // The published version and release stay anchored to the
        // immutable commit: development refs are not authority.
        assert!(released.verify().is_ok());
        assert_eq!(
            released.identity.source_revision.commit_sha,
            first.revision.commit_sha
        );
        assert_eq!(release.target.commit_sha, first.revision.commit_sha);

        // New content at the moved head is a different identity, even
        // with an identical semantic version.
        let next = published_version(
            "deploy",
            ORIGIN,
            &second.revision.commit_sha,
            (1, 0, 0),
            "definition",
            "lock",
        );
        assert_ne!(next.version_id, released.version_id);
    }

    /// The acceptance-criteria flow: two users independently change a
    /// workflow through branches and forks, review and merge both
    /// changes, publish an immutable release, install and explicitly
    /// upgrade it without touching the running version, and pin the
    /// release as a subworkflow dependency at an immutable revision.
    #[test]
    fn two_users_fork_review_merge_publish_install_and_upgrade() {
        let mut remote = seeded_forge(0xacce);
        let origin = repo_id(ORIGIN);
        let main = DevelopmentRef::branch("main").unwrap();
        let base = remote
            .commit(&origin, &main, "initial workflow", "acme-bot")
            .unwrap();
        let base_sha = base.revision.commit_sha.clone();

        // Two users fork the repository on the forge, carrying lineage
        // and attribution as data.
        let alice_fork = repo_id("github.com/alice/ops-workflows");
        remote
            .fork(
                &origin,
                alice_fork.clone(),
                fork_lineage(ORIGIN),
                attribution("alice"),
            )
            .unwrap();
        let bob_fork = repo_id("github.com/bob/ops-workflows");
        remote
            .fork(
                &origin,
                bob_fork.clone(),
                fork_lineage(ORIGIN),
                attribution("bob"),
            )
            .unwrap();

        // Alice clones her fork locally, develops on a branch, pushes.
        let mut alice_local = InMemoryForge::with_salt(github_kind(), 0xa11ce);
        alice_local
            .clone_repository_from(&remote, &alice_fork)
            .unwrap();
        let retry = DevelopmentRef::branch("feature/retry").unwrap();
        alice_local
            .create_branch(&alice_fork, retry.clone(), &base_sha)
            .unwrap();
        let head_a = alice_local
            .commit(&alice_fork, &retry, "add retry step to deploy", "alice")
            .unwrap();
        alice_local
            .push_branch_to(&mut remote, &alice_fork, &retry)
            .unwrap();

        // Alice proposes a review against the upstream default branch:
        // base and head are immutable revisions.
        let base_revision = ImmutableSourceRevision::pin(base_sha.clone(), Some(main.clone()));
        let head_revision = remote.resolve_ref_blocking(&alice_fork, &retry).unwrap();
        let review = propose_review(
            &origin,
            ReviewId::parse("101").unwrap(),
            Some("Add retry step".to_owned()),
            &base_revision,
            &head_revision,
        )
        .unwrap();

        // Bob approves at the proposal's head; the forge mints the
        // upstream merge commit; the review records it immutably.
        let approvals = vec![ReviewApproval {
            review: review.review.clone(),
            reviewer: "bob".to_owned(),
            approved_head: head_a.revision.commit_sha.clone(),
        }];
        let merge = remote
            .create_merge_commit(
                &origin,
                &main,
                &head_a.revision.commit_sha,
                "Merge feature/retry",
                "acme-bot",
            )
            .unwrap();
        let merged = merge_review(&review, &approvals, merge.revision.commit_sha.clone()).unwrap();
        let merge_sha = match merged.state {
            ReviewState::Merged { merge_revision } => merge_revision,
            _ => panic!("the first review must merge"),
        };
        assert_eq!(merge_sha, merge.revision.commit_sha);

        // Bob independently developed off the original base; he
        // integrates the merged upstream main before proposing, so his
        // review builds on Alice's merged work.
        let notify = DevelopmentRef::branch("feature/notify").unwrap();
        remote
            .create_branch(&bob_fork, notify.clone(), &base_sha)
            .unwrap();
        remote
            .commit(&bob_fork, &notify, "notify on failure", "bob")
            .unwrap();
        let integrated = remote
            .create_merge_commit(&bob_fork, &notify, &merge_sha, "merge upstream main", "bob")
            .unwrap();
        let base_two = ImmutableSourceRevision::pin(merge_sha.clone(), Some(main.clone()));
        let head_two = remote.resolve_ref_blocking(&bob_fork, &notify).unwrap();
        let review_two = propose_review(
            &origin,
            ReviewId::parse("102").unwrap(),
            Some("Notify on failure".to_owned()),
            &base_two,
            &head_two,
        )
        .unwrap();
        let approvals_two = vec![ReviewApproval {
            review: review_two.review.clone(),
            reviewer: "alice".to_owned(),
            approved_head: integrated.revision.commit_sha.clone(),
        }];
        let merge_two = remote
            .create_merge_commit(
                &origin,
                &main,
                &integrated.revision.commit_sha,
                "Merge feature/notify",
                "acme-bot",
            )
            .unwrap();
        let merged_two = merge_review(
            &review_two,
            &approvals_two,
            merge_two.revision.commit_sha.clone(),
        )
        .unwrap();
        let release_sha = match merged_two.state {
            ReviewState::Merged { merge_revision } => merge_revision,
            _ => panic!("the second review must merge"),
        };
        assert_eq!(remote.head(&origin, &main).unwrap(), release_sha);

        // The maintainer publishes an immutable release from merged main.
        let version = published_version(
            "deploy",
            ORIGIN,
            &release_sha,
            (1, 2, 0),
            "deploy-definition-v1",
            "deploy-lock-v1",
        );
        let release = publish_release(
            &origin,
            &[],
            "v1.2.0",
            &ImmutableSourceRevision::pin(
                release_sha.clone(),
                Some(DevelopmentRef::tag("v1.2.0").unwrap()),
            ),
            std::slice::from_ref(&version),
        )
        .unwrap();
        assert_eq!(release.versions, vec![version.version_id.clone()]);
        assert_eq!(release.target.commit_sha, release_sha);

        // A consumer installs the release; the running instance pins
        // the installed version identity.
        let mut installs = InstallRegistry::new();
        let installed = installs.install(&version).unwrap();
        assert_eq!(installed.installed.version_id, version.version_id);
        let running = installs
            .installed(&workflow_id("deploy"))
            .unwrap()
            .installed
            .version_id
            .clone();

        // Development continues after the release; a new version is
        // published at the new head.
        let post_release = remote
            .commit(&origin, &main, "post-release hardening", "acme-bot")
            .unwrap();
        let version_two = published_version(
            "deploy",
            ORIGIN,
            &post_release.revision.commit_sha,
            (1, 3, 0),
            "deploy-definition-v2",
            "deploy-lock-v2",
        );
        let release_two = publish_release(
            &origin,
            std::slice::from_ref(&release),
            "v1.3.0",
            &ImmutableSourceRevision::pin_commit(post_release.revision.commit_sha.clone()),
            std::slice::from_ref(&version_two),
        )
        .unwrap();
        assert_eq!(
            release_two.target.commit_sha,
            post_release.revision.commit_sha
        );

        // Nothing about the installed workflow changed silently: the
        // installation still pins the old version, and the first
        // release target is untouched by branch movement.
        assert_eq!(
            installs
                .installed(&workflow_id("deploy"))
                .unwrap()
                .installed
                .version_id,
            running
        );
        assert_eq!(release.target.commit_sha, release_sha);

        // The upgrade is proposed, decided explicitly, and recorded.
        let plan = installs
            .propose_update(
                &workflow_id("deploy"),
                &version_two,
                Some("upgrade to v1.3.0".to_owned()),
            )
            .unwrap();
        let record = installs
            .decide_update(&plan, UpdateDecision::Apply)
            .unwrap();
        assert!(record.applied);
        assert_eq!(record.from, running);
        assert_ne!(
            installs
                .installed(&workflow_id("deploy"))
                .unwrap()
                .installed
                .version_id,
            running
        );
        // The previously-running version identity remains valid and
        // verifiable — existing running instances are unaffected.
        assert!(version.verify().is_ok());
        assert_eq!(version.version_id, running);
        assert_eq!(installs.history().len(), 1);

        // A downstream workflow pins this release as a subworkflow
        // dependency at an immutable revision and version.
        let dependency =
            codex_workflow_contracts::SubworkflowDependencyId::parse("deploy_needs_notify")
                .unwrap();
        let pin = SubworkflowPin::from_release(dependency.clone(), &release, &version).unwrap();
        assert_eq!(pin.version_id(), Some(&version.version_id));
        assert_eq!(pin.source_revision.commit_sha, release_sha);
        let lock = WorkflowPackageLock::new(vec![pin]).unwrap();
        assert!(lock.verify().is_ok());
        assert!(lock.digest().is_ok());
    }

    #[test]
    fn review_approvals_must_match_the_proposed_head() {
        // Approving an outdated head, then merging a new head, must fail
        // — approvals cannot be reused after the proposal moves. The
        // flow-level assertion lives in the two-user test; this pins
        // the forge-level inputs used there.
        let mut forge = seeded_forge(0x6);
        let origin = repo_id(ORIGIN);
        let main = DevelopmentRef::branch("main").unwrap();
        let base = forge.commit(&origin, &main, "initial", "acme-bot").unwrap();
        let feature = DevelopmentRef::branch("feature/fresh").unwrap();
        forge
            .create_branch(&origin, feature.clone(), &base.revision.commit_sha)
            .unwrap();
        let first = forge
            .commit(&origin, &feature, "first proposal", "alice")
            .unwrap();
        let second = forge
            .commit(&origin, &feature, "revised proposal", "alice")
            .unwrap();
        let stale_approval = ReviewApproval {
            review: ReviewId::parse("201").unwrap(),
            reviewer: "bob".to_owned(),
            approved_head: first.revision.commit_sha.clone(),
        };
        let review = propose_review(
            &origin,
            ReviewId::parse("201").unwrap(),
            None,
            &ImmutableSourceRevision::pin(base.revision.commit_sha.clone(), Some(main.clone())),
            &ImmutableSourceRevision::pin_commit(second.revision.commit_sha.clone()),
        )
        .unwrap();
        assert!(matches!(
            merge_review(
                &review,
                &[stale_approval],
                second.revision.commit_sha.clone()
            ),
            Err(WorkflowForgeError::MissingApprovalAtHead { .. })
        ));
    }
}
