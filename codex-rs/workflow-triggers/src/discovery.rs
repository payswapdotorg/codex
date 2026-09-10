//! Workflow package discovery over the host catalog seam.
//!
//! Discovery reuses the WO-009 repository discovery semantics: the host's
//! transport (a forge's search API) supplies repository snapshots through
//! the [`crate::PackageCatalog`] seam, and the plane applies the frozen
//! [`RepositoryQuery`] filtering plus per-workflow published-version
//! listings. Workflow and plugin distribution remain distinct semantic
//! layers; nothing here conflates them.

use codex_workflow_contracts::ForgeKind;
use codex_workflow_contracts::WorkflowDefinitionId;
use codex_workflow_contracts::WorkflowRepository;
use codex_workflow_contracts::WorkflowVersion;
use codex_workflow_contracts::WorkflowVersionId;
use codex_workflow_forge::PublishedVersionRef;
use codex_workflow_forge::RepositoryQuery;
use serde::Deserialize;
use serde::Serialize;

use crate::WorkflowTriggerError;
use crate::WorkflowTriggerPlane;

/// A workflow package discovery query. Unset fields match anything.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct PackageQuery {
    /// Match packages publishing this workflow.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflow: Option<WorkflowDefinitionId>,
    /// Match repositories hosted on this forge kind.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub forge: Option<ForgeKind>,
    /// Match repositories whose canonical identity or workflow display
    /// names contain this fragment (case-insensitive).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name_contains: Option<String>,
}

/// One discovered workflow package: a workflow declared by a repository,
/// with its published versions.
///
/// The listing is a discovery *view*: the repository record is a
/// development-time snapshot and the version references are immutable
/// published identities. Installing requires fetching the full sealed
/// version record (see [`WorkflowTriggerPlane::fetch_version`]).
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkflowPackageListing {
    /// The repository declaring the workflow.
    pub repository: WorkflowRepository,
    /// The workflow the package publishes.
    pub workflow: WorkflowDefinitionId,
    /// The workflow's display name, when declared.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    /// The published versions of the workflow.
    pub versions: Vec<PublishedVersionRef>,
}

impl WorkflowTriggerPlane {
    /// Discovers workflow packages matching `query`.
    ///
    /// The repository filter reuses the WO-009 discovery semantics over
    /// the host-supplied catalog snapshots; each declared workflow of a
    /// matching repository becomes a listing with its published
    /// versions.
    pub fn discover(
        &self,
        query: &PackageQuery,
    ) -> Result<Vec<WorkflowPackageListing>, WorkflowTriggerError> {
        let repository_query = RepositoryQuery {
            workflow: query.workflow.clone(),
            forge: query.forge.clone(),
            forked_from: None,
            name_contains: query.name_contains.clone(),
        };
        let repositories = self.catalog.search_repositories(&repository_query)?;
        let mut listings = Vec::new();
        for repository in repositories {
            for manifest in &repository.workflows {
                if let Some(workflow) = &query.workflow
                    && &manifest.workflow != workflow
                {
                    continue;
                }
                let versions = self.catalog.published_versions(&manifest.workflow)?;
                listings.push(WorkflowPackageListing {
                    repository: repository.clone(),
                    workflow: manifest.workflow.clone(),
                    display_name: manifest.display_name.clone(),
                    versions,
                });
            }
        }
        Ok(listings)
    }

    /// Fetches the full sealed version record for a discovered reference,
    /// when the catalog holds it.
    ///
    /// The record re-verifies at install time and at every fire; the
    /// catalog is a transport, never an integrity authority.
    pub fn fetch_version(
        &self,
        version: &WorkflowVersionId,
    ) -> Result<Option<WorkflowVersion>, WorkflowTriggerError> {
        self.catalog.fetch_version(version)
    }
}
