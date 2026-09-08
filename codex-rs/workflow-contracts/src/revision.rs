//! Revisions: immutable execution anchors versus movable development refs.
//!
//! These types enforce the architecture's separation between development
//! inputs and executable identity:
//!
//! - [`DevelopmentRef`] names a movable Git reference (branch or tag). It
//!   is development input and provenance only, and never participates in
//!   execution identity.
//! - [`ImmutableSourceRevision`] pins an exact commit. It is the only source
//!   anchor that participates in workflow execution identity.
//! - [`RevisionSha`] validates full commit SHAs, so an immutable revision
//!   can never be anchored to an abbreviated SHA.
//! - [`WorkflowVersionId`] is the digest identity of a published workflow
//!   version, derived from the execution identity tuple.

use std::fmt;
use std::hash::Hash;

use codex_protocol::protocol::GitSha;
use serde::Deserialize;
use serde::Serialize;

use crate::ContentDigest;
use crate::WorkflowContractError;

/// A movable Git reference used for workflow development.
///
/// Development refs are inputs to development workflows only. They never
/// appear in execution identity: publishing resolves a ref to a commit and
/// pins the commit.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub enum DevelopmentRef {
    /// A movable branch reference, for example `main` or `feature/retry`.
    Branch(String),
    /// A movable tag reference, for example `v1` or `weekly`.
    Tag(String),
}

/// An immutable source revision: the execution anchor of a published
/// workflow version.
///
/// The commit SHA is the authority. `resolved_from` records the development
/// ref the commit was resolved from when the version was published; it is
/// provenance only and may name a ref that later moves or disappears without
/// affecting the validity of the revision.
#[derive(Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ImmutableSourceRevision {
    /// Full commit SHA (40 or 64 lowercase hex characters).
    pub commit_sha: RevisionSha,
    /// Development ref the commit was resolved from, for provenance only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resolved_from: Option<DevelopmentRef>,
}

/// The immutable identity of a published workflow version.
///
/// The digest covers the execution identity tuple defined by the frozen
/// architecture: workflow identity, semantic version, repository identity,
/// immutable source revision, workflow definition digest, and dependency
/// lock digest. Provenance-only fields are deliberately excluded, so
/// republishing identical executable content with different publication
/// metadata yields the same version identity.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct WorkflowVersionId(ContentDigest);

/// A commit SHA accepted by workflow contracts.
///
/// Reuses `codex-protocol::GitSha` as the underlying value and additionally
/// requires the full lowercase-hex form (40 characters for SHA-1
/// repositories, 64 for SHA-256 repositories) so an immutable revision can
/// never be anchored to an abbreviated SHA.
#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct RevisionSha(GitSha);

impl Eq for RevisionSha {}

impl Hash for RevisionSha {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.0.hash(state);
    }
}

impl PartialOrd for RevisionSha {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for RevisionSha {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.0.cmp(&other.0.0)
    }
}

impl DevelopmentRef {
    /// Parses a branch reference name.
    pub fn branch(name: impl Into<String>) -> Result<Self, WorkflowContractError> {
        parse_ref_name(name, "branch name").map(Self::Branch)
    }

    /// Parses a tag reference name.
    pub fn tag(name: impl Into<String>) -> Result<Self, WorkflowContractError> {
        parse_ref_name(name, "tag name").map(Self::Tag)
    }

    /// The reference name without its kind.
    pub fn name(&self) -> &str {
        match self {
            Self::Branch(name) | Self::Tag(name) => name.as_str(),
        }
    }
}

impl TryFrom<String> for DevelopmentRef {
    type Error = WorkflowContractError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::branch(value)
    }
}

impl fmt::Display for DevelopmentRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Branch(name) => write!(f, "branch:{name}"),
            Self::Tag(name) => write!(f, "tag:{name}"),
        }
    }
}

impl ImmutableSourceRevision {
    /// Pins a commit, optionally recording the development ref it was
    /// resolved from.
    pub fn pin(commit_sha: RevisionSha, resolved_from: Option<DevelopmentRef>) -> Self {
        Self {
            commit_sha,
            resolved_from,
        }
    }

    /// Pins a commit provenance-free.
    pub fn pin_commit(commit_sha: RevisionSha) -> Self {
        Self::pin(commit_sha, /*resolved_from*/ None)
    }
}

impl fmt::Debug for ImmutableSourceRevision {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.resolved_from {
            Some(source) => write!(f, "{}@{}", self.commit_sha, source),
            None => f.write_str(self.commit_sha.as_str()),
        }
    }
}

impl RevisionSha {
    /// Validates a full commit SHA.
    ///
    /// Both SHA-1 (40 hex characters) and SHA-256 (64 hex characters)
    /// repositories are accepted; abbreviated SHAs are rejected because they
    /// are not immutable anchors.
    pub fn parse(sha: impl Into<String>) -> Result<Self, WorkflowContractError> {
        let sha = sha.into();
        let valid_length = sha.len() == 40 || sha.len() == 64;
        let valid_characters = sha
            .chars()
            .all(|character| character.is_ascii_hexdigit() && !character.is_ascii_uppercase());
        if valid_length && valid_characters {
            Ok(Self(GitSha::new(&sha)))
        } else {
            Err(WorkflowContractError::InvalidIdentifier {
                kind: "revision sha",
                value: sha,
                reason: "must be full lowercase hex (40 or 64 characters)",
            })
        }
    }

    /// The commit SHA as a string.
    pub fn as_str(&self) -> &str {
        self.0.0.as_str()
    }
}

impl TryFrom<String> for RevisionSha {
    type Error = WorkflowContractError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl TryFrom<&str> for RevisionSha {
    type Error = WorkflowContractError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::parse(value.to_owned())
    }
}

impl From<RevisionSha> for String {
    fn from(value: RevisionSha) -> Self {
        value.0.0
    }
}

impl From<RevisionSha> for GitSha {
    fn from(value: RevisionSha) -> Self {
        value.0
    }
}

impl fmt::Debug for RevisionSha {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl fmt::Display for RevisionSha {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl WorkflowVersionId {
    /// Wraps a digest as a workflow version identity.
    pub const fn from_digest(digest: ContentDigest) -> Self {
        Self(digest)
    }

    /// The underlying content digest.
    pub fn digest(&self) -> &ContentDigest {
        &self.0
    }
}

impl TryFrom<String> for WorkflowVersionId {
    type Error = WorkflowContractError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        ContentDigest::try_from(value).map(Self)
    }
}

impl TryFrom<&str> for WorkflowVersionId {
    type Error = WorkflowContractError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        ContentDigest::try_from(value).map(Self)
    }
}

impl From<WorkflowVersionId> for String {
    fn from(value: WorkflowVersionId) -> String {
        value.0.into()
    }
}

impl AsRef<str> for WorkflowVersionId {
    fn as_ref(&self) -> &str {
        self.0.as_str()
    }
}

impl fmt::Display for WorkflowVersionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0.as_str())
    }
}

/// Validates movable reference names.
fn parse_ref_name(
    value: impl Into<String>,
    kind: &'static str,
) -> Result<String, WorkflowContractError> {
    let value = value.into();
    let trimmed = value.trim();
    if trimmed.is_empty() {
        Err(WorkflowContractError::InvalidIdentifier {
            kind,
            value,
            reason: "must be non-empty",
        })
    } else if trimmed.len() != value.len() {
        Err(WorkflowContractError::InvalidIdentifier {
            kind,
            value,
            reason: "must not start or end with whitespace",
        })
    } else {
        Ok(value)
    }
}

#[cfg(test)]
#[path = "revision_tests.rs"]
mod tests;
