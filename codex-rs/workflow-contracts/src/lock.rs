//! Dependency locks: resolved, immutable dependency identities.
//!
//! A [`DependencyLock`] resolves every dependency a workflow declares to an
//! immutable identity: skills and plugins to implementation digests, MCP
//! capabilities to bound server identities, and subworkflows to immutable
//! published version identities. The lock participates in workflow version
//! identity, so dependencies are locked independently of workflow semantics
//! and can never silently change under a published version.

use std::collections::BTreeMap;

use serde::Deserialize;
use serde::Serialize;

use crate::ContentDigest;
use crate::McpCapabilityId;
use crate::PluginName;
use crate::SemanticVersion;
use crate::SkillName;
use crate::SubworkflowDependencyId;
use crate::WorkflowContractError;
use crate::WorkflowDefinitionId;
use crate::WorkflowDependencies;
use crate::WorkflowRepositoryId;
use crate::WorkflowVersionId;

/// The key of one resolved lock entry.
///
/// Serializes as a compact string key (see [`DependencyKey::as_key`]) so
/// lock entries can be stored in a JSON map with deterministic ordering.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DependencyKey {
    /// A resolved skill dependency.
    Skill {
        /// The declared skill.
        skill: SkillName,
    },
    /// A resolved plugin dependency.
    Plugin {
        /// The declared plugin.
        plugin: PluginName,
    },
    /// A resolved MCP capability dependency.
    Mcp {
        /// The declared MCP capability.
        capability: McpCapabilityId,
    },
    /// A resolved subworkflow dependency.
    Subworkflow {
        /// The declared subworkflow dependency reference.
        dependency_id: SubworkflowDependencyId,
    },
}

/// Where and under which terms a resolved dependency was obtained.
///
/// Provenance is recorded so dependency identity is auditable. It contains
/// no credentials and no commercial settlement data.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DependencyProvenance {
    /// Source the dependency was resolved from, when applicable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    /// License terms of the resolved dependency.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
}

/// The immutable identity a dependency resolved to.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum ResolvedDependencyIdentity {
    /// A resolved skill: an implementation digest, optionally versioned.
    Skill {
        /// Content digest of the resolved skill implementation.
        digest: ContentDigest,
        /// Version of the resolved skill, when versioned.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        version: Option<String>,
    },
    /// A resolved plugin: an implementation digest, optionally versioned.
    Plugin {
        /// Content digest of the resolved plugin bundle.
        digest: ContentDigest,
        /// Version of the resolved plugin, when versioned.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        version: Option<String>,
    },
    /// A resolved MCP capability: the bound server identity and an optional
    /// configuration digest.
    Mcp {
        /// Identity of the MCP server that provides the capability.
        server: String,
        /// Digest of the server configuration the capability was resolved
        /// against, when applicable.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        config_digest: Option<ContentDigest>,
    },
    /// A resolved subworkflow pinned to an immutable published version.
    Subworkflow {
        /// Repository of the pinned subworkflow version.
        repository: WorkflowRepositoryId,
        /// Definition identity of the pinned subworkflow.
        workflow: WorkflowDefinitionId,
        /// Immutable identity of the pinned subworkflow version.
        version_id: WorkflowVersionId,
        /// Semantic version of the pinned subworkflow version.
        semantic_version: SemanticVersion,
    },
}

impl ResolvedDependencyIdentity {
    /// The digest that integrity-checks this resolution, when present.
    ///
    /// Subworkflow resolutions are integrity-checked by their version id.
    pub fn integrity_digest(&self) -> Option<ContentDigest> {
        match self {
            Self::Skill { digest, .. } | Self::Plugin { digest, .. } => Some(digest.clone()),
            Self::Mcp { config_digest, .. } => config_digest.clone(),
            Self::Subworkflow { version_id, .. } => Some(version_id.digest().clone()),
        }
    }
}

/// One resolved dependency entry inside a lock.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResolvedDependency {
    /// The declared dependency this entry resolves.
    pub key: DependencyKey,
    /// The immutable identity the dependency resolved to.
    pub resolved: ResolvedDependencyIdentity,
    /// Provenance of the resolution.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provenance: Option<DependencyProvenance>,
}

/// The resolved, immutable dependency identities of a published workflow
/// version.
///
/// The lock is part of workflow version identity: every declared dependency
/// resolves to exactly one immutable identity, so dependencies can be
/// integrity-checked and locked independently of workflow semantics.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields, default)]
pub struct DependencyLock {
    /// Resolved entries, keyed by declared dependency.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub entries: BTreeMap<DependencyKey, ResolvedDependency>,
}

impl DependencyLock {
    /// Inserts a resolved entry, replacing any previous resolution of the
    /// same key.
    pub fn insert(&mut self, entry: ResolvedDependency) {
        self.entries.insert(entry.key.clone(), entry);
    }

    /// Verifies the lock covers every dependency declared by `dependencies`.
    ///
    /// Completeness is a hard invariant of published workflow versions: a
    /// version whose lock does not resolve all declared dependencies is not
    /// publishable.
    pub fn verify_covers(
        &self,
        dependencies: &WorkflowDependencies,
    ) -> Result<(), WorkflowContractError> {
        for skill in &dependencies.skills {
            let key = DependencyKey::Skill {
                skill: skill.skill.clone(),
            };
            if !self.entries.contains_key(&key) {
                return Err(WorkflowContractError::IncompleteDependencyLock {
                    reason: format!(
                        "declared skill dependency `{}` is not locked",
                        skill.skill.name
                    ),
                });
            }
        }
        for plugin in &dependencies.plugins {
            let key = DependencyKey::Plugin {
                plugin: plugin.plugin.clone(),
            };
            if !self.entries.contains_key(&key) {
                return Err(WorkflowContractError::IncompleteDependencyLock {
                    reason: format!(
                        "declared plugin dependency `{}` is not locked",
                        plugin.plugin.as_ref()
                    ),
                });
            }
        }
        for mcp in &dependencies.mcp {
            let key = DependencyKey::Mcp {
                capability: mcp.capability.clone(),
            };
            if !self.entries.contains_key(&key) {
                return Err(WorkflowContractError::IncompleteDependencyLock {
                    reason: format!(
                        "declared MCP dependency `{}` is not locked",
                        mcp.capability.as_ref()
                    ),
                });
            }
        }
        for subworkflow in &dependencies.subworkflows {
            let key = DependencyKey::Subworkflow {
                dependency_id: subworkflow.dependency_id.clone(),
            };
            let Some(entry) = self.entries.get(&key) else {
                return Err(WorkflowContractError::IncompleteDependencyLock {
                    reason: format!(
                        "declared subworkflow dependency `{}` is not locked",
                        subworkflow.dependency_id.as_ref()
                    ),
                });
            };
            let ResolvedDependencyIdentity::Subworkflow {
                repository,
                workflow,
                version_id: _,
                semantic_version: _,
            } = &entry.resolved
            else {
                return Err(WorkflowContractError::IncompleteDependencyLock {
                    reason: format!(
                        "subworkflow dependency `{}` resolved to a non-subworkflow identity",
                        subworkflow.dependency_id.as_ref()
                    ),
                });
            };
            if repository != &subworkflow.repository || workflow != &subworkflow.workflow {
                return Err(WorkflowContractError::IncompleteDependencyLock {
                    reason: format!(
                        "subworkflow dependency `{}` resolved to a different target",
                        subworkflow.dependency_id.as_ref()
                    ),
                });
            }
        }
        Ok(())
    }

    /// Canonical digest of the lock, used inside workflow version identity.
    pub fn digest(&self) -> Result<ContentDigest, WorkflowContractError> {
        ContentDigest::of(&self.entries)
    }
}

impl DependencyKey {
    /// Compact string form used as the lock's serialization key, for
    /// example `skill:browser-use@acme.plugins`, `plugin:acme-connector`,
    /// `mcp:github_issue_tracker`, or `subworkflow:publish_notes`.
    ///
    /// The four kinds use disjoint prefixes and every component is a
    /// validated path-segment-safe token, so the string form round-trips
    /// losslessly.
    pub fn as_key(&self) -> String {
        match self {
            Self::Skill { skill } => match &skill.plugin {
                Some(plugin) => format!("skill:{}@{plugin}", skill.name),
                None => format!("skill:{}", skill.name),
            },
            Self::Plugin { plugin } => format!("plugin:{}", plugin.as_ref()),
            Self::Mcp { capability } => format!("mcp:{}", capability.as_ref()),
            Self::Subworkflow { dependency_id } => {
                format!("subworkflow:{}", dependency_id.as_ref())
            }
        }
    }

    fn parse_key(value: &str) -> Result<Self, WorkflowContractError> {
        let (kind, rest) =
            value
                .split_once(':')
                .ok_or_else(|| WorkflowContractError::InvalidIdentifier {
                    kind: "dependency key",
                    value: value.to_owned(),
                    reason: "must be prefixed with `skill:`, `plugin:`, `mcp:`, or `subworkflow:`",
                })?;
        match kind {
            "skill" => {
                let (name, plugin) = rest
                    .split_once('@')
                    .map(|(name, plugin)| (name.to_owned(), Some(plugin.to_owned())))
                    .unwrap_or((rest.to_owned(), None));
                let mut skill = SkillName::parse(name)?;
                if let Some(plugin) = plugin {
                    skill = skill.provided_by(plugin);
                }
                Ok(Self::Skill { skill })
            }
            "plugin" => PluginName::parse(rest).map(|plugin| Self::Plugin { plugin }),
            "mcp" => McpCapabilityId::parse(rest).map(|capability| Self::Mcp { capability }),
            "subworkflow" => SubworkflowDependencyId::parse(rest)
                .map(|dependency_id| Self::Subworkflow { dependency_id }),
            _ => Err(WorkflowContractError::InvalidIdentifier {
                kind: "dependency key",
                value: value.to_owned(),
                reason: "must be prefixed with `skill:`, `plugin:`, `mcp:`, or `subworkflow:`",
            }),
        }
    }
}

impl Serialize for DependencyKey {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.as_key())
    }
}

impl<'de> Deserialize<'de> for DependencyKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::parse_key(&value).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
#[path = "lock_tests.rs"]
mod tests;
