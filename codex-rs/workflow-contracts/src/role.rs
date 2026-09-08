//! Workflow roles.
//!
//! A `WorkflowRole` is a first-class contract naming who may participate in
//! a workflow: which capabilities the role may exercise and whether the role
//! requires approval. Roles are semantic orchestration concepts; concrete
//! agent, subagent, or human assignments are runtime bindings made by the
//! execution plane reusing Codex subagent orchestration.

use std::fmt;

use serde::Deserialize;
use serde::Serialize;

use crate::CapabilityId;
use crate::WorkflowContractError;

/// Stable identity of a role within a workflow definition.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct RoleId(String);

/// Whether exercising a role requires explicit approval.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub enum ApprovalRequirement {
    /// The role acts without an additional approval step.
    NotRequired,
    /// The role's actions require approval before taking effect.
    Required,
}

/// A role contract within a workflow definition.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkflowRole {
    /// The role's identity within the definition.
    pub role: RoleId,
    /// Human-facing name of the role.
    pub name: String,
    /// What the role is accountable for.
    pub objective: String,
    /// Semantic capabilities the role may exercise.
    ///
    /// Capability ids are semantic references; binding them to concrete
    /// skills, plugins, tools, or environments is a runtime decision.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub allowed_capabilities: Vec<CapabilityId>,
    /// Whether the role requires approval to act.
    pub approval: ApprovalRequirement,
}

impl RoleId {
    /// Parses a role identifier.
    pub fn parse(value: impl Into<String>) -> Result<Self, WorkflowContractError> {
        let value = value.into();
        if value.is_empty() {
            Err(WorkflowContractError::InvalidIdentifier {
                kind: "role id",
                value,
                reason: "must be non-empty",
            })
        } else {
            Ok(Self(value))
        }
    }
}

impl TryFrom<String> for RoleId {
    type Error = WorkflowContractError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl TryFrom<&str> for RoleId {
    type Error = WorkflowContractError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::parse(value.to_owned())
    }
}

impl From<RoleId> for String {
    fn from(value: RoleId) -> Self {
        value.0
    }
}

impl AsRef<str> for RoleId {
    fn as_ref(&self) -> &str {
        self.0.as_str()
    }
}

impl fmt::Display for RoleId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0.as_str())
    }
}

#[cfg(test)]
#[path = "role_tests.rs"]
mod tests;
