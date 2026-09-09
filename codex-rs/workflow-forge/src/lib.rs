//! WO-009: git-native workflow collaboration for the Codex universal
//! workflow platform.
//!
//! This crate makes workflows behave like software repositories —
//! collaborative, forkable, branchable, reviewable, composable,
//! version-pinned, publishable, and installable — while keeping semantic
//! authority in the workflow control plane.
//!
//! ## Position in the stack
//!
//! - The frozen contracts crate (`codex-workflow-contracts`) owns the
//!   universal record types: `WorkflowRepository`, `WorkflowReview`,
//!   `WorkflowRelease`, `WorkflowVersion`, `ExecutionVersionIdentity`,
//!   `ImmutableSourceRevision`, and the `WorkflowForge` boundary.
//! - **This crate** owns collaboration semantics *over* those records:
//!   fork / branch / review / merge / release transitions, workflow
//!   package manifest and dependency-lock semantics, subworkflow pins,
//!   repository discovery, and explicit install and update records.
//! - The **GitHub adapter** ([`GithubForge`]) is one implementation of the
//!   forge-neutral `WorkflowForge` boundary. GitHub-shaped records are
//!   pure data supplied by the host integration, which owns transport,
//!   authentication, and credentials.
//! - The **Codex application** orchestrates: it holds durable state, drives
//!   the record transitions defined here, and keeps the workflow control
//!   plane as the semantic authority.
//!
//! ## What this crate deliberately is not
//!
//! - It performs **no network I/O**: forge transport is supplied by the
//!   host through injected record sources ([`GithubRecordSource`]) and,
//!   for tests and seeding, the simulated forge ([`InMemoryForge`]).
//! - It is **not an agent runtime and not a workflow engine**: every
//!   operation is a pure record transition; execution, durable instance
//!   lifecycle, and legal transitions belong to the control plane.
//! - It holds **no durable collaboration state**: the registry types
//!   ([`InstallRegistry`]) are in-memory record collections whose
//!   persistence the application owns.
//!
//! ## Invariants held here
//!
//! - **Development refs are never authority.** Publication, pinning, and
//!   installation anchor to immutable commit SHAs; `resolved_from` fields
//!   are provenance only. Moving a branch can never change a published
//!   version identity.
//! - **Published revisions are immutable.** Version references recompute
//!   their digests ([`PublishedVersionRef::verify`]) before any release,
//!   pin, or install accepts them.
//! - **External outputs are untrusted.** GitHub records are validated
//!   (full SHAs, fork consistency, manifest ownership) before they become
//!   contract records; invalid records are rejected, not guessed at.
//! - **Credentials never enter this crate.** Repository identity is
//!   canonical and credential-free; only opaque references move through
//!   these APIs.
//! - **Forge semantics stay in the GitHub adapter.** Universal
//!   collaboration operations accept only frozen contract types.
//! - **The forge layer only moves content and identity.** Whether a
//!   candidate version *satisfies* a declared dependency pin is a
//!   control-plane decision; this crate records the resolved, immutable
//!   pin.
//!
//! ## Module map
//!
//! - `error` — the forge-layer error type and its projection onto the
//!   frozen contract error channel.
//! - `canonical` — internal canonical-identity string helpers that keep
//!   repository and workflow identity opaque.
//! - `collaboration` — pure review / merge / release / fork transitions
//!   over contract records.
//! - `github` — GitHub-shaped records, the record source boundary, and
//!   the `WorkflowForge` adapter.
//! - `package` — workflow package manifests and dependency locks with
//!   subworkflow pins to immutable revisions.
//! - `install` — installed workflow records and explicit, reviewable
//!   update plans and decisions.
//! - `discovery` — repository discovery queries over snapshots.
//! - [`memory`] — an in-memory forge simulation for tests and seeding.

#![deny(missing_docs)]

mod canonical;
mod collaboration;
mod discovery;
mod error;
mod github;
mod install;
pub mod memory;
mod package;

#[cfg(test)]
mod test_support;

pub use collaboration::{
    close_review, fork_repository, merge_review, propose_review, publish_release,
    upsert_branch_state, CommitSummary, PublishedVersionRef, ReviewApproval,
};
pub use discovery::{discover_repositories, RepositoryQuery};
pub use error::WorkflowForgeError;
pub use github::{
    map_branch_states, map_pull_request, map_release, map_repository, GithubForge, GithubHost,
    GithubProvenance, GithubPullRequestRecord, GithubPullRequestState, GithubRecordSource,
    GithubRefKind, GithubRefRecord, GithubReleaseRecord, GithubReleaseView, GithubRepoCoordinates,
    GithubRepositoryRecord,
};
pub use install::{
    InstallRegistry, InstalledWorkflow, UpdateDecision, WorkflowUpdatePlan, WorkflowUpdateRecord,
};
pub use memory::InMemoryForge;
pub use package::{
    SubworkflowPin, WorkflowPackageLock, WorkflowPackageManifest, PACKAGE_LOCK_FORMAT_VERSION,
    PACKAGE_MANIFEST_FORMAT_VERSION,
};
