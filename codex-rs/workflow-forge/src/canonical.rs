//! Canonical identity string helpers.
//!
//! The frozen contracts treat repository and workflow identity as opaque
//! canonical values. This crate never inspects or constructs those types
//! through their private details: it extracts and reconstructs them
//! through their serde forms, so a change to the frozen identity types
//! cannot silently change universal semantics.
//!
//! The GitHub adapter defines GitHub's canonical identity *format*
//! (`host/owner/repository`, see [`crate::github`]); universal
//! collaboration semantics never inspect identity structure — they only
//! round-trip it through these helpers.

#[cfg(test)]
use codex_workflow_contracts::WorkflowDefinitionId;
use codex_workflow_contracts::WorkflowRepositoryId;
use serde_json::Value;

use crate::WorkflowForgeError;

/// Reconstructs a repository identity from its canonical string form.
///
/// Repository identity is credential-free and canonical by construction
/// in the frozen contracts, so this is a pure serde round-trip. If the
/// frozen type is not string-shaped the error names the offending shape
/// instead of guessing.
pub(crate) fn repository_id_from_canonical(
    canonical: &str,
) -> Result<WorkflowRepositoryId, WorkflowForgeError> {
    serde_json::from_value(Value::String(canonical.to_owned())).map_err(|error| {
        WorkflowForgeError::InvalidRecord {
            reason: format!("canonical repository identity is not a string newtype: {error}"),
        }
    })
}

/// Extracts the canonical string form of a repository identity.
pub(crate) fn canonical_repository_id(
    identity: &WorkflowRepositoryId,
) -> Result<String, WorkflowForgeError> {
    match serde_json::to_value(identity) {
        Ok(Value::String(canonical)) => Ok(canonical),
        Ok(other) => Err(WorkflowForgeError::InvalidRecord {
            reason: format!(
                "repository identity did not serialize to a canonical string: {other:?}"
            ),
        }),
        Err(error) => Err(WorkflowForgeError::Serde(error)),
    }
}

/// Reconstructs a workflow identity from its canonical string form.
#[cfg(test)]
pub(crate) fn workflow_id_from_name(
    name: &str,
) -> Result<WorkflowDefinitionId, WorkflowForgeError> {
    serde_json::from_value(Value::String(name.to_owned())).map_err(|error| {
        WorkflowForgeError::InvalidRecord {
            reason: format!("workflow identity is not a string newtype: {error}"),
        }
    })
}

/// Extracts the canonical string form of a workflow identity.
#[cfg(test)]
pub(crate) fn canonical_workflow_id(
    workflow: &WorkflowDefinitionId,
) -> Result<String, WorkflowForgeError> {
    match serde_json::to_value(workflow) {
        Ok(Value::String(name)) => Ok(name),
        Ok(other) => Err(WorkflowForgeError::InvalidRecord {
            reason: format!("workflow identity did not serialize to a string: {other:?}"),
        }),
        Err(error) => Err(WorkflowForgeError::Serde(error)),
    }
}

#[cfg(test)]
mod tests {
    use super::canonical_repository_id;
    use super::canonical_workflow_id;
    use super::repository_id_from_canonical;
    use super::workflow_id_from_name;

    #[test]
    fn repository_and_workflow_ids_round_trip_through_canonical_strings() {
        let repository = repository_id_from_canonical("github.com/acme/ops-workflows").unwrap();
        assert_eq!(
            canonical_repository_id(&repository).unwrap(),
            "github.com/acme/ops-workflows"
        );
        let workflow = workflow_id_from_name("deploy-service").unwrap();
        assert_eq!(canonical_workflow_id(&workflow).unwrap(), "deploy-service");
        // Round-trips are stable and identity-preserving.
        assert_eq!(
            repository_id_from_canonical("github.com/acme/ops-workflows").unwrap(),
            repository
        );
    }
}
