//! Workflow manifests.
//!
//! A `WorkflowManifest` is the repository document that declares one
//! workflow inside a workflow repository: its identity, where its
//! definition lives, and its provenance. Repositories may contain multiple
//! manifests. The manifest is development-time metadata; publishing a
//! version resolves the manifest path at an immutable revision and freezes
//! the resulting definition.

use serde::Deserialize;
use serde::Serialize;

use crate::WorkflowContractError;
use crate::WorkflowDefinitionId;
use crate::WorkflowProvenance;
use crate::WorkflowRepositoryId;

/// Current manifest serialization format version.
pub const MANIFEST_FORMAT_VERSION: u32 = 1;

/// A path inside a workflow repository, relative and without traversal.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct RepositoryRelativePath(String);

/// The manifest for one workflow inside a workflow repository.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkflowManifest {
    /// Manifest serialization format version.
    pub manifest_version: u32,
    /// The workflow this manifest declares.
    pub workflow: WorkflowDefinitionId,
    /// Human-facing display name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    /// Human-facing description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Repository containing the workflow.
    pub repository: WorkflowRepositoryId,
    /// Path of the workflow definition inside the repository.
    pub definition_path: RepositoryRelativePath,
    /// Provenance: attribution, license, upgrade policy.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provenance: Option<WorkflowProvenance>,
}

impl WorkflowManifest {
    /// Validates the manifest's format version.
    pub fn validate(&self) -> Result<(), WorkflowContractError> {
        if self.manifest_version != MANIFEST_FORMAT_VERSION {
            return Err(WorkflowContractError::InvalidIdentifier {
                kind: "manifest version",
                value: self.manifest_version.to_string(),
                reason: "unsupported manifest format version",
            });
        }
        Ok(())
    }
}

impl RepositoryRelativePath {
    /// Parses a repository-relative path.
    ///
    /// Paths must be relative, forward-slash separated, normalized (no `.`
    /// or `..` components), and non-empty, so a manifest can never point
    /// outside its repository.
    pub fn parse(value: impl Into<String>) -> Result<Self, WorkflowContractError> {
        let value = value.into();
        let invalid = value.is_empty()
            || value.contains('\\')
            || std::path::Path::new(&value).is_absolute()
            || value
                .split('/')
                .any(|segment| segment.is_empty() || segment == "." || segment == "..");
        if invalid {
            Err(WorkflowContractError::InvalidRepositoryPath {
                value,
                reason: "must be a normalized forward-slash relative path inside the repository",
            })
        } else {
            Ok(Self(value))
        }
    }
}

impl TryFrom<String> for RepositoryRelativePath {
    type Error = WorkflowContractError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl TryFrom<&str> for RepositoryRelativePath {
    type Error = WorkflowContractError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::parse(value.to_owned())
    }
}

impl From<RepositoryRelativePath> for String {
    fn from(value: RepositoryRelativePath) -> Self {
        value.0
    }
}

impl AsRef<str> for RepositoryRelativePath {
    fn as_ref(&self) -> &str {
        self.0.as_str()
    }
}

#[cfg(test)]
#[path = "manifest_tests.rs"]
mod tests;
