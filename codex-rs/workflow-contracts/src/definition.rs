//! Workflow definitions.
//!
//! A `WorkflowDefinition` is the complete semantic content of a workflow:
//! its IR graph, roles, triggers, and dependency declarations. The
//! definition is pure semantics — it deliberately carries no source
//! location, branch, or repository, so the same definition content digests
//! identically wherever it was authored. Source and revision identity is
//! attached by the immutable `WorkflowVersion` at publish time.

use std::collections::BTreeMap;

use serde::Deserialize;
use serde::Serialize;

use crate::ContentDigest;
use crate::RoleId;
use crate::WorkflowContractError;
use crate::WorkflowDefinitionId;
use crate::WorkflowDependencies;
use crate::WorkflowIr;
use crate::WorkflowRole;
use crate::WorkflowTrigger;

/// The complete semantic definition of a workflow.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkflowDefinition {
    /// Identity of the workflow within its repository.
    pub id: WorkflowDefinitionId,
    /// Human-facing description of the workflow.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// The workflow's execution graph.
    pub ir: WorkflowIr,
    /// Roles that participate in the workflow.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub roles: BTreeMap<RoleId, WorkflowRole>,
    /// Triggers that may start the workflow.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub triggers: Vec<WorkflowTrigger>,
    /// Dependency declarations: subworkflows, skills, plugins, MCP, and
    /// resources.
    #[serde(default, skip_serializing_if = "workflow_dependencies_is_empty")]
    pub dependencies: WorkflowDependencies,
}

impl WorkflowDefinition {
    /// Validates the definition's semantic invariants:
    ///
    /// - the IR graph is structurally valid;
    /// - every role referenced by the IR is declared;
    /// - every subworkflow dependency referenced by the IR is declared.
    pub fn validate(&self) -> Result<(), WorkflowContractError> {
        self.ir.validate()?;
        for (node_id, node) in &self.ir.nodes {
            if let crate::WorkflowIrNode::Step(step) = node {
                for role in &step.roles {
                    if !self.roles.contains_key(role) {
                        return Err(WorkflowContractError::MalformedIr {
                            reason: format!("node `{node_id}` references missing role `{role}`"),
                        });
                    }
                }
            }
        }
        for referenced in self.ir.referenced_subworkflow_dependencies() {
            let declared = self
                .dependencies
                .subworkflows
                .iter()
                .any(|dependency| dependency.dependency_id == referenced);
            if !declared {
                return Err(WorkflowContractError::MalformedIr {
                    reason: format!(
                        "subworkflow node references missing dependency `{referenced}`"
                    ),
                });
            }
        }
        Ok(())
    }

    /// Canonical digest of the definition's semantic content.
    ///
    /// The digest is part of workflow version identity. Because the
    /// definition carries no source or ref information, the digest is
    /// stable across branches and forks: identical semantics produce
    /// identical definition digests.
    pub fn digest(&self) -> Result<ContentDigest, WorkflowContractError> {
        ContentDigest::of(&self)
    }
}

/// Skips empty dependency sections during serialization.
fn workflow_dependencies_is_empty(dependencies: &WorkflowDependencies) -> bool {
    dependencies.skills.is_empty()
        && dependencies.plugins.is_empty()
        && dependencies.mcp.is_empty()
        && dependencies.subworkflows.is_empty()
        && dependencies.resources.is_empty()
}

#[cfg(test)]
#[path = "definition_tests.rs"]
mod tests;
