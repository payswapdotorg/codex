//! Repository discovery over host-supplied snapshots.
//!
//! Discovery transport (a forge's search API) is host-owned: the host
//! queries, maps candidate repositories into contract records, and then
//! applies these pure queries. Queries match on forge-neutral fields
//! only — the workflows a repository declares, the forge kind, the fork
//! origin, and name fragments.

use codex_workflow_contracts::ForgeKind;
use codex_workflow_contracts::WorkflowDefinitionId;
use codex_workflow_contracts::WorkflowRepository;
use codex_workflow_contracts::WorkflowRepositoryId;
use serde::Deserialize;
use serde::Serialize;

use crate::canonical::canonical_repository_id;
use crate::WorkflowForgeError;

/// A repository discovery query. Unset fields match anything.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct RepositoryQuery {
    /// Match repositories that declare this workflow.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflow: Option<WorkflowDefinitionId>,
    /// Match repositories hosted on this forge kind.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub forge: Option<ForgeKind>,
    /// Match repositories whose direct fork origin is this repository.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub forked_from: Option<WorkflowRepositoryId>,
    /// Match repositories whose canonical identity or workflow display
    /// names contain this fragment (case-insensitive).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name_contains: Option<String>,
}

/// Discovers repositories matching a query.
///
/// Pure filtering over host-supplied snapshots: candidates are cloned
/// when they match, so the caller keeps ownership of the search corpus.
/// Fork-lineage traversal beyond the direct origin requires the frozen
/// lineage shape and stays with the control plane; this query matches
/// the recorded origin.
pub fn discover_repositories(
    candidates: &[WorkflowRepository],
    query: &RepositoryQuery,
) -> Result<Vec<WorkflowRepository>, WorkflowForgeError> {
    let mut matched = Vec::new();
    for candidate in candidates {
        let matches_workflow = match &query.workflow {
            Some(workflow) => candidate.manifest_for(workflow).is_some(),
            None => true,
        };
        let matches_forge = match &query.forge {
            Some(forge) => &candidate.forge == forge,
            None => true,
        };
        let matches_fork = match &query.forked_from {
            Some(origin) => candidate.origin.as_ref() == Some(origin),
            None => true,
        };
        let matches_name = match &query.name_contains {
            Some(fragment) => name_matches(candidate, fragment)?,
            None => true,
        };
        if matches_workflow && matches_forge && matches_fork && matches_name {
            matched.push(candidate.clone());
        }
    }
    Ok(matched)
}

/// Case-insensitive fragment match against the repository's canonical
/// identity and its workflows' display names.
fn name_matches(
    repository: &WorkflowRepository,
    fragment: &str,
) -> Result<bool, WorkflowForgeError> {
    let fragment = fragment.to_ascii_lowercase();
    let canonical = canonical_repository_id(&repository.identity)?.to_ascii_lowercase();
    if canonical.contains(&fragment) {
        return Ok(true);
    }
    for manifest in &repository.workflows {
        let display = manifest
            .display_name
            .as_deref()
            .unwrap_or_default()
            .to_ascii_lowercase();
        if display.contains(&fragment) {
            return Ok(true);
        }
    }
    Ok(false)
}

#[cfg(test)]
mod tests {
    use codex_workflow_contracts::WorkflowBranchState;

    use crate::test_support::attribution;
    use crate::test_support::github_kind;
    use crate::test_support::manifest;
    use crate::test_support::repo_id;
    use crate::test_support::sha;
    use crate::test_support::workflow_id;

    use super::discover_repositories;
    use super::RepositoryQuery;

    fn repository(
        canonical: &str,
        origin: Option<&str>,
    ) -> codex_workflow_contracts::WorkflowRepository {
        codex_workflow_contracts::WorkflowRepository {
            identity: repo_id(canonical),
            forge: github_kind(),
            origin: origin.map(repo_id),
            forked_from: None,
            default_branch: codex_workflow_contracts::DevelopmentRef::branch("main").unwrap(),
            branches: vec![WorkflowBranchState {
                branch: codex_workflow_contracts::DevelopmentRef::branch("main").unwrap(),
                head: Some(sha(1)),
            }],
            workflows: vec![manifest("deploy", canonical, "workflows/deploy.toml")],
            maintainers: vec![attribution("acme-ops")],
        }
    }

    #[test]
    fn discovers_by_workflow_manifest() {
        let upstream = repository("github.com/acme/ops-workflows", None);
        let other = repository("github.com/acme/other", None);
        let corpus = vec![upstream.clone(), other];
        let results = discover_repositories(
            &corpus,
            &RepositoryQuery {
                workflow: Some(workflow_id("deploy")),
                ..RepositoryQuery::default()
            },
        )
        .unwrap();
        assert_eq!(results.len(), 2);
        let absent = discover_repositories(
            &corpus,
            &RepositoryQuery {
                workflow: Some(workflow_id("absent")),
                ..RepositoryQuery::default()
            },
        )
        .unwrap();
        assert!(absent.is_empty());
    }

    #[test]
    fn discovers_by_fork_origin() {
        let upstream = repository("github.com/acme/ops-workflows", None);
        let fork = repository(
            "github.com/alice/ops-workflows",
            Some("github.com/acme/ops-workflows"),
        );
        let corpus = vec![upstream.clone(), fork.clone()];
        let results = discover_repositories(
            &corpus,
            &RepositoryQuery {
                forked_from: Some(repo_id("github.com/acme/ops-workflows")),
                ..RepositoryQuery::default()
            },
        )
        .unwrap();
        assert_eq!(results, vec![fork]);
    }

    #[test]
    fn discovers_by_name_fragments() {
        let corpus = vec![
            repository("github.com/acme/ops-workflows", None),
            repository("github.com/acme/tooling", None),
        ];
        let results = discover_repositories(
            &corpus,
            &RepositoryQuery {
                name_contains: Some("OPS".to_owned()), // case-insensitive
                ..RepositoryQuery::default()
            },
        )
        .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(
            results[0].identity,
            repo_id("github.com/acme/ops-workflows")
        );
    }

    #[test]
    fn combined_queries_narrow_results() {
        let corpus = vec![
            repository("github.com/acme/ops-workflows", None),
            repository(
                "github.com/alice/ops-workflows",
                Some("github.com/acme/ops-workflows"),
            ),
        ];
        let results = discover_repositories(
            &corpus,
            &RepositoryQuery {
                workflow: Some(workflow_id("deploy")),
                forked_from: Some(repo_id("github.com/acme/ops-workflows")),
                name_contains: Some("alice".to_owned()),
                ..RepositoryQuery::default()
            },
        )
        .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(
            results[0].identity,
            repo_id("github.com/alice/ops-workflows")
        );
    }
}
