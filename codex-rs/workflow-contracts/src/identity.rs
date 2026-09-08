//! Identity types for the workflow contract surface.
//!
//! Repository identity is forge-neutral and reuses Codex's canonical remote
//! identity (`codex-git-utils::canonicalize_git_remote_url`): a
//! transport-independent, credential-free `host/owner/repository` form. No
//! forge-specific type appears in these contracts.
//!
//! Source-revision identity — the separation between movable development
//! refs and immutable commit anchors — lives in [`crate::revision`].

use std::fmt;

use codex_git_utils::canonicalize_git_remote_url;
use semver::Version;
use serde::Deserialize;
use serde::Serialize;

use crate::WorkflowContractError;

/// Identity of one workflow inside a repository.
///
/// The identifier is a stable, repository-scoped name (for example
/// `release-notes`). It is semantic identity, not a filesystem path or a Git
/// ref.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct WorkflowDefinitionId(String);

/// Forge-neutral identity of a workflow repository.
///
/// The identity reuses Codex's canonical remote form
/// (`canonicalize_git_remote_url`): transport-independent, lowercase-host,
/// `.git`-stripped, and credential-free by construction, for example
/// `github.com/acme/release-bot`. Forks compare by canonical identity, and
/// the same repository reached over HTTPS or SSH is the same workflow
/// repository.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct WorkflowRepositoryId(String);

/// Neutral label for the forge hosting a workflow repository (for example
/// `github`, `gitlab`, `forgejo`, `local`).
///
/// The kind is opaque data. It never changes workflow semantics; it only
/// tells the workflow platform which forge adapter (bound in a later Work
/// Order) can service the repository.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct ForgeKind(String);

/// Identity of one execution instance of a published workflow version.
///
/// Instance identifiers are allocated by the workflow control plane (a later
/// Work Order owns durable instances). This contract only fixes the shape.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct WorkflowInstanceId(uuid::Uuid);

/// Semantic version of a workflow, reusing `semver::Version` for parsing and
/// ordering semantics.
pub type SemanticVersion = Version;

impl WorkflowDefinitionId {
    /// Parses and validates a workflow definition identifier.
    pub fn parse(value: impl Into<String>) -> Result<Self, WorkflowContractError> {
        parse_non_empty(value, "workflow definition id").map(Self)
    }
}

impl TryFrom<String> for WorkflowDefinitionId {
    type Error = WorkflowContractError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl TryFrom<&str> for WorkflowDefinitionId {
    type Error = WorkflowContractError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::parse(value.to_owned())
    }
}

impl From<WorkflowDefinitionId> for String {
    fn from(value: WorkflowDefinitionId) -> Self {
        value.0
    }
}

impl AsRef<str> for WorkflowDefinitionId {
    fn as_ref(&self) -> &str {
        self.0.as_str()
    }
}

impl fmt::Display for WorkflowDefinitionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0.as_str())
    }
}

impl WorkflowRepositoryId {
    /// Parses a remote URL into a canonical, credential-free repository
    /// identity.
    ///
    /// Reuses `codex-git-utils::canonicalize_git_remote_url`, which accepts
    /// URL-like, SCP-like, and `host/path` remotes, strips userinfo and the
    /// `.git` suffix, normalizes hosts, and rejects non-remote strings —
    /// so credentials and malformed remotes can never become repository
    /// identity.
    pub fn parse(url: impl AsRef<str>) -> Result<Self, WorkflowContractError> {
        let canonical = canonicalize_git_remote_url(url.as_ref().trim()).ok_or_else(|| {
            WorkflowContractError::InvalidIdentifier {
                kind: "workflow repository id",
                value: url.as_ref().to_owned(),
                reason: "must be a valid Git remote (URL, SCP-like, or host/path form)",
            }
        })?;
        Ok(Self(canonical))
    }

    /// Returns the canonical, credential-free repository identity.
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl TryFrom<String> for WorkflowRepositoryId {
    type Error = WorkflowContractError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(&value)
    }
}

impl TryFrom<&str> for WorkflowRepositoryId {
    type Error = WorkflowContractError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl From<WorkflowRepositoryId> for String {
    fn from(value: WorkflowRepositoryId) -> Self {
        value.0
    }
}

impl fmt::Display for WorkflowRepositoryId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0.as_str())
    }
}

impl ForgeKind {
    /// Parses a forge kind label.
    pub fn parse(value: impl Into<String>) -> Result<Self, WorkflowContractError> {
        parse_non_empty(value, "forge kind").map(Self)
    }

    /// The well-known kind label for GitHub-hosted repositories.
    ///
    /// This is a convenience constant for adapters; the contracts themselves
    /// treat the kind as opaque data.
    pub const GITHUB: &'static str = "github";
}

impl TryFrom<String> for ForgeKind {
    type Error = WorkflowContractError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl TryFrom<&str> for ForgeKind {
    type Error = WorkflowContractError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::parse(value.to_owned())
    }
}

impl From<ForgeKind> for String {
    fn from(value: ForgeKind) -> Self {
        value.0
    }
}

impl AsRef<str> for ForgeKind {
    fn as_ref(&self) -> &str {
        self.0.as_str()
    }
}

impl fmt::Display for ForgeKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0.as_str())
    }
}

impl WorkflowInstanceId {
    /// Allocates a fresh instance identifier.
    ///
    /// Control-plane implementations own durable allocation; this
    /// constructor only provides the contract-level shape for tests and
    /// in-memory use.
    pub fn generate() -> Self {
        Self(uuid::Uuid::new_v4())
    }

    /// The underlying UUID.
    pub const fn as_uuid(&self) -> uuid::Uuid {
        self.0
    }
}

impl From<uuid::Uuid> for WorkflowInstanceId {
    fn from(value: uuid::Uuid) -> Self {
        Self(value)
    }
}

impl From<WorkflowInstanceId> for uuid::Uuid {
    fn from(value: WorkflowInstanceId) -> uuid::Uuid {
        value.0
    }
}

impl fmt::Display for WorkflowInstanceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Shared validation for non-empty textual identifiers.
fn parse_non_empty(
    value: impl Into<String>,
    kind: &'static str,
) -> Result<String, WorkflowContractError> {
    let value = value.into();
    let trimmed = value.trim();
    if trimmed.is_empty() {
        Err(WorkflowContractError::InvalidIdentifier {
            kind,
            value: value.clone(),
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
#[path = "identity_tests.rs"]
mod tests;
