//! Shared construction helpers for workflow-forge unit tests.
//!
//! The frozen `codex-workflow-contracts` crate is authoritative. For the
//! modules included verbatim in the WO-009 packet (repository, revision,
//! manifest, version) these helpers use the frozen constructors
//! directly. For contract types whose bodies were only summarized in the
//! packet (`identity`, `provenance`), values are built through serde
//! with the shapes the packet documents; every such helper carries an
//! explicit `expect` message naming the assumed shape, so any drift from
//! the real tree fails loudly in exactly one place and nothing outside
//! this module depends on those shapes.

use std::future::Future;
use std::task::Context;
use std::task::Poll;
use std::task::RawWaker;
use std::task::RawWakerVTable;
use std::task::Waker;

use codex_workflow_contracts::Attribution;
use codex_workflow_contracts::ContentDigest;
use codex_workflow_contracts::ExecutionVersionIdentity;
use codex_workflow_contracts::ForgeKind;
use codex_workflow_contracts::ForkLineage;
use codex_workflow_contracts::ImmutableSourceRevision;
use codex_workflow_contracts::RepositoryRelativePath;
use codex_workflow_contracts::RevisionSha;
use codex_workflow_contracts::SemanticVersion;
use codex_workflow_contracts::WorkflowDefinitionId;
use codex_workflow_contracts::WorkflowManifest;
use codex_workflow_contracts::WorkflowRepositoryId;
use codex_workflow_contracts::MANIFEST_FORMAT_VERSION;

use crate::canonical::repository_id_from_canonical;
use crate::canonical::workflow_id_from_name;
use crate::collaboration::PublishedVersionRef;

/// Builds a repository identity from its canonical string form, using
/// the frozen serde round-trip.
pub(crate) fn repo_id(canonical: &str) -> WorkflowRepositoryId {
    repository_id_from_canonical(canonical)
        .expect("repository identity deserializes from a canonical string")
}

/// Builds a workflow identity from its canonical name.
pub(crate) fn workflow_id(name: &str) -> WorkflowDefinitionId {
    workflow_id_from_name(name).expect("workflow identity deserializes from a canonical string")
}

/// Builds the GitHub forge kind.
///
/// The production code never constructs forge kinds — hosts inject them
/// into `GithubForge` and `InMemoryForge` — so this guess is confined to
/// tests.
pub(crate) fn github_kind() -> ForgeKind {
    serde_json::from_value(serde_json::json!("github"))
        .expect("ForgeKind deserializes from a unit-variant string")
}

/// Builds a semantic version from its components.
pub(crate) fn semantic_version(major: u64, minor: u64, patch: u64) -> SemanticVersion {
    SemanticVersion::new(major, minor, patch)
}

/// Builds a maintainer attribution from a display name.
pub(crate) fn attribution(name: &str) -> Attribution {
    serde_json::from_value(serde_json::json!({ "name": name }))
        .expect("Attribution deserializes from a name field")
}

/// Builds fork lineage rooted at the given canonical repository.
pub(crate) fn fork_lineage(root: &str) -> ForkLineage {
    serde_json::from_value(serde_json::json!({ "upstream": repo_id(root) }))
        .expect("ForkLineage deserializes from an upstream field")
}

/// Builds a full 40-hex commit SHA from a numeric seed.
///
/// Distinct seeds always produce distinct, valid SHAs; tests use them
/// for base, head, merge, and target revisions.
pub(crate) fn sha(seed: u64) -> RevisionSha {
    RevisionSha::parse(format!("{seed:040x}")).expect("seeded SHAs are full 40-hex")
}

/// Builds a validated contract manifest for one workflow.
pub(crate) fn manifest(workflow: &str, repository: &str, path: &str) -> WorkflowManifest {
    WorkflowManifest {
        manifest_version: MANIFEST_FORMAT_VERSION,
        workflow: workflow_id(workflow),
        display_name: Some(workflow.to_owned()),
        description: None,
        repository: repo_id(repository),
        definition_path: RepositoryRelativePath::parse(path).expect("test paths are valid"),
        provenance: None,
    }
}

/// Builds a published-version reference with a **real** identity digest:
/// the version id is computed by the frozen
/// `ExecutionVersionIdentity::version_id`, so integrity checks in the
/// tests exercise genuine digest recomputation.
pub(crate) fn published_version(
    workflow: &str,
    repository: &str,
    commit: &RevisionSha,
    version: (u64, u64, u64),
    definition_seed: &str,
    lock_seed: &str,
) -> PublishedVersionRef {
    let identity = ExecutionVersionIdentity {
        workflow: workflow_id(workflow),
        semantic_version: semantic_version(version.0, version.1, version.2),
        repository: repo_id(repository),
        source_revision: ImmutableSourceRevision::pin_commit(commit.clone()),
        definition_digest: ContentDigest::of(&format!("definition:{definition_seed}"))
            .expect("definition digests serialize"),
        dependency_lock_digest: ContentDigest::of(&format!("lock:{lock_seed}"))
            .expect("lock digests serialize"),
    };
    let version_id = identity
        .version_id()
        .expect("the frozen identity tuple digests");
    PublishedVersionRef {
        identity,
        version_id,
    }
}

/// Polls an immediately-ready future once, without an executor.
///
/// `WorkflowForge` methods return `impl Future` values that the
/// adapters complete eagerly with `std::future::ready`; this helper
/// lets tests call the frozen trait boundary without pulling in an
/// async runtime dependency.
pub(crate) fn block_on_ready<F: Future>(future: F) -> F::Output {
    unsafe fn clone_waker(_: *const ()) -> RawWaker {
        RawWaker::new(std::ptr::null(), &VTABLE)
    }
    unsafe fn noop_waker(_: *const ()) {}
    static VTABLE: RawWakerVTable =
        RawWakerVTable::new(clone_waker, noop_waker, noop_waker, noop_waker);
    let waker = unsafe { Waker::from_raw(RawWaker::new(std::ptr::null(), &VTABLE)) };
    let mut future = std::pin::pin!(future);
    let mut context = Context::from_waker(&waker);
    match future.as_mut().poll(&mut context) {
        Poll::Ready(output) => output,
        Poll::Pending => panic!("expected an immediately-ready future"),
    }
}
