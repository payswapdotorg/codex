//! Workflow dependency declarations.
//!
//! Three dependency families are modeled and kept deliberately separate:
//!
//! - **Subworkflow dependencies** compose workflows with other workflows.
//!   They are workflow-to-workflow semantic edges and are pinned to
//!   immutable version identities at publish time.
//! - **Implementation dependencies** (`SkillDependency`, `PluginDependency`,
//!   `McpDependency`) bind reusable Codex capability packages to a workflow.
//!   They are resolved to immutable digests at publish time but never become
//!   part of the workflow's orchestration semantics.
//! - **Capability and resource requirements** express what a workflow needs
//!   semantically (for example `navigate_web` or a `browser_profile`
//!   resource). Bindings between requirements and concrete implementations
//!   are made by the execution plane, not by these contracts.
//!
//! Resolution of these declarations into immutable identities is modeled in
//! [`crate::DependencyLock`].

use std::fmt;

use semver::VersionReq;
use serde::Deserialize;
use serde::Serialize;

use crate::ContentDigest;
use crate::SemanticVersion;
use crate::WorkflowContractError;
use crate::WorkflowDefinitionId;
use crate::WorkflowRepositoryId;
use crate::WorkflowVersionId;

/// Semantic capability identifier (for example `navigate_web`).
///
/// Capabilities are stable semantic contracts. A workflow says what
/// capability it requires; the runtime decides which compatible
/// Codex-native adapter executes it.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct CapabilityId(String);

/// Typed resource identifier (for example `browser_profile`,
/// `github_account`, `human_approver`).
///
/// Resources are typed execution dependencies. Workflow semantics refer to
/// logical resource types; concrete resource bindings (and any credentials
/// they carry) are resolved during installation or execution and never enter
/// workflow source.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct ResourceTypeId(String);

/// Local reference name for a declared subworkflow dependency.
///
/// `WorkflowIr` subworkflow nodes reference dependencies through this id,
/// keeping the graph free of inline dependency details.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct SubworkflowDependencyId(String);

/// Identifier of a Codex skill dependency.
///
/// Skill identity follows Codex skill conventions: a stable name, optionally
/// qualified by a providing plugin.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SkillName {
    /// Stable skill name, for example `browser-use`.
    pub name: String,
    /// Plugin that provides the skill, when the skill is plugin-scoped.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plugin: Option<String>,
}

/// Identifier of a Codex plugin dependency.
///
/// Follows Codex plugin naming rules: ASCII letters, digits, `_`, `-`, and
/// `.` separating non-empty segments.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct PluginName(String);

/// Identifier of a required MCP capability (for example
/// `github_issue_tracker`).
///
/// MCP capabilities are referenced semantically. Concrete MCP server names
/// and transports are runtime bindings, not workflow semantics.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct McpCapabilityId(String);

/// A capability the workflow requires.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CapabilityRequirement {
    /// The required semantic capability.
    pub capability: CapabilityId,
    /// Human-readable purpose of the requirement within this workflow.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub purpose: Option<String>,
}

/// A typed resource the workflow requires.
///
/// The requirement never carries credentials or concrete bindings; it
/// declares the logical resource type the workflow needs.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResourceRequirement {
    /// The required resource type, for example `browser_profile`.
    pub resource_type: ResourceTypeId,
    /// Human-readable purpose of the requirement within this workflow.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub purpose: Option<String>,
}

/// A skill the workflow requires.
///
/// Skills provide reusable capability and instructions; they never define
/// workflow orchestration semantics.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SkillDependency {
    /// The required skill.
    pub skill: SkillName,
    /// Compatible version range, when the skill is distributed in versions.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version_requirement: Option<VersionReq>,
}

/// A plugin the workflow requires.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PluginDependency {
    /// The required plugin.
    pub plugin: PluginName,
    /// Compatible version range, when the plugin is distributed in versions.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version_requirement: Option<VersionReq>,
}

/// An MCP capability the workflow requires.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct McpDependency {
    /// The required MCP capability.
    pub capability: McpCapabilityId,
    /// Human-readable purpose of the requirement within this workflow.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub purpose: Option<String>,
}

/// How a subworkflow dependency pins its target version.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub enum SubworkflowVersionPin {
    /// Compatible semantic version range, resolved at publish time.
    Range(VersionReq),
    /// Exact semantic version, for example `1.4.2`.
    Exact(SemanticVersion),
    /// Exact immutable version identity, for example
    /// `commit:<digest-pinned version id>`.
    VersionId(WorkflowVersionId),
}

/// A workflow-to-workflow composition dependency.
///
/// The dependency is explicit, version-pinned, and resolved to an immutable
/// version identity in the dependency lock. Upgrading a dependency always
/// creates a new candidate version; it can never silently change a published
/// workflow.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SubworkflowDependency {
    /// Local reference id used by IR subworkflow nodes.
    pub dependency_id: SubworkflowDependencyId,
    /// Repository containing the subworkflow.
    pub repository: WorkflowRepositoryId,
    /// The subworkflow's definition identity.
    pub workflow: WorkflowDefinitionId,
    /// The declared version pin.
    pub pin: SubworkflowVersionPin,
}

/// The complete dependency declarations of a workflow definition.
///
/// Implementation dependencies (skills, plugins, MCP) and composition
/// dependencies (subworkflows) are stored in separate fields so consumers
/// can reason about them independently, per the frozen architecture.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields, default)]
pub struct WorkflowDependencies {
    /// Codex skills the workflow requires.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub skills: Vec<SkillDependency>,
    /// Codex plugins the workflow requires.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub plugins: Vec<PluginDependency>,
    /// MCP capabilities the workflow requires.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub mcp: Vec<McpDependency>,
    /// Subworkflows the workflow composes.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub subworkflows: Vec<SubworkflowDependency>,
    /// Typed resources the workflow requires.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub resources: Vec<ResourceRequirement>,
}

impl CapabilityId {
    /// Parses a capability identifier.
    ///
    /// Capability ids use lowercase snake_case (`navigate_web`) so they stay
    /// stable across serializations and language bindings.
    pub fn parse(value: impl Into<String>) -> Result<Self, WorkflowContractError> {
        parse_snake_case(value, "capability id").map(Self)
    }
}

impl TryFrom<String> for CapabilityId {
    type Error = WorkflowContractError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl TryFrom<&str> for CapabilityId {
    type Error = WorkflowContractError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::parse(value.to_owned())
    }
}

impl From<CapabilityId> for String {
    fn from(value: CapabilityId) -> Self {
        value.0
    }
}

impl AsRef<str> for CapabilityId {
    fn as_ref(&self) -> &str {
        self.0.as_str()
    }
}

impl ResourceTypeId {
    /// Parses a resource type identifier using capability id rules.
    pub fn parse(value: impl Into<String>) -> Result<Self, WorkflowContractError> {
        parse_snake_case(value, "resource type id").map(Self)
    }
}

impl TryFrom<String> for ResourceTypeId {
    type Error = WorkflowContractError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl TryFrom<&str> for ResourceTypeId {
    type Error = WorkflowContractError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::parse(value.to_owned())
    }
}

impl From<ResourceTypeId> for String {
    fn from(value: ResourceTypeId) -> Self {
        value.0
    }
}

impl AsRef<str> for ResourceTypeId {
    fn as_ref(&self) -> &str {
        self.0.as_str()
    }
}

impl SubworkflowDependencyId {
    /// Parses a subworkflow dependency reference.
    pub fn parse(value: impl Into<String>) -> Result<Self, WorkflowContractError> {
        parse_snake_case(value, "subworkflow dependency id").map(Self)
    }
}

impl TryFrom<String> for SubworkflowDependencyId {
    type Error = WorkflowContractError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl TryFrom<&str> for SubworkflowDependencyId {
    type Error = WorkflowContractError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::parse(value.to_owned())
    }
}

impl From<SubworkflowDependencyId> for String {
    fn from(value: SubworkflowDependencyId) -> Self {
        value.0
    }
}

impl AsRef<str> for SubworkflowDependencyId {
    fn as_ref(&self) -> &str {
        self.0.as_str()
    }
}

impl fmt::Display for SubworkflowDependencyId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0.as_str())
    }
}

impl PluginName {
    /// Parses a plugin name.
    ///
    /// Follows Codex plugin naming rules: non-empty ASCII letters, digits,
    /// `_`, `-`, and `.` separating non-empty segments.
    pub fn parse(value: impl Into<String>) -> Result<Self, WorkflowContractError> {
        let value = value.into();
        validate_name_segment(&value, "plugin name")?;
        Ok(Self(value))
    }
}

impl TryFrom<String> for PluginName {
    type Error = WorkflowContractError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl TryFrom<&str> for PluginName {
    type Error = WorkflowContractError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::parse(value.to_owned())
    }
}

impl From<PluginName> for String {
    fn from(value: PluginName) -> Self {
        value.0
    }
}

impl AsRef<str> for PluginName {
    fn as_ref(&self) -> &str {
        self.0.as_str()
    }
}

impl McpCapabilityId {
    /// Parses an MCP capability identifier.
    pub fn parse(value: impl Into<String>) -> Result<Self, WorkflowContractError> {
        parse_snake_case(value, "mcp capability id").map(Self)
    }
}

impl TryFrom<String> for McpCapabilityId {
    type Error = WorkflowContractError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl TryFrom<&str> for McpCapabilityId {
    type Error = WorkflowContractError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::parse(value.to_owned())
    }
}

impl From<McpCapabilityId> for String {
    fn from(value: McpCapabilityId) -> Self {
        value.0
    }
}

impl AsRef<str> for McpCapabilityId {
    fn as_ref(&self) -> &str {
        self.0.as_str()
    }
}

impl SkillName {
    /// Parses a standalone skill name.
    ///
    /// Skill identity follows Codex skill conventions: a stable, non-empty,
    /// path-segment-safe token (ASCII letters, digits, `_`, `-`, `.`).
    pub fn parse(name: impl Into<String>) -> Result<Self, WorkflowContractError> {
        let name = name.into();
        validate_name_segment(&name, "skill name")?;
        Ok(Self { name, plugin: None })
    }

    /// Qualifies the skill with the plugin that provides it.
    pub fn provided_by(mut self, plugin: impl Into<String>) -> Self {
        self.plugin = Some(plugin.into());
        self
    }
}

/// Validates snake_case identifiers shared by capability, resource, MCP, and
/// dependency ids.
fn parse_snake_case(
    value: impl Into<String>,
    kind: &'static str,
) -> Result<String, WorkflowContractError> {
    let value = value.into();
    let valid = !value.is_empty()
        && value.chars().all(|character| {
            character.is_ascii_lowercase() || character.is_ascii_digit() || character == '_'
        })
        && !value.starts_with('_')
        && !value.ends_with('_');
    if valid {
        Ok(value)
    } else {
        Err(WorkflowContractError::InvalidIdentifier {
            kind,
            value,
            reason: "must be non-empty lowercase snake_case",
        })
    }
}

/// Validates path-segment-safe name tokens shared by skill and plugin names.
fn validate_name_segment(value: &str, kind: &'static str) -> Result<(), WorkflowContractError> {
    let characters_ok = !value.is_empty()
        && value.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | '.')
        });
    let dots_ok = !value.starts_with('.') && !value.ends_with('.') && !value.contains("..");
    if characters_ok && dots_ok {
        Ok(())
    } else {
        Err(WorkflowContractError::InvalidIdentifier {
            kind,
            value: value.to_owned(),
            reason: "must be a non-empty path-segment-safe token",
        })
    }
}

#[cfg(test)]
#[path = "dependency_tests.rs"]
mod tests;
