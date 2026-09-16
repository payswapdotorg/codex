//! Pack dependency contracts.
//!
//! Owned by the PACK-001 work order (Contract Foundation).
//!
//! Pack dependencies reference immutable identities: workflow version
//! identities ([`codex_workflow_contracts::WorkflowVersionId`]), semantic
//! capability identities ([`codex_workflow_contracts::CapabilityId`]), and
//! other pack revisions ([`crate::PackRevisionId`]). Declarations pin
//! immutable identities exactly — packs never pin moving references such as
//! branches or version ranges — so a dependency update is always a new
//! resolution and produces a new lock digest, and therefore a new pack
//! revision identity (through `dependency_lock_digest` in
//! [`crate::PackRevisionIdentity`]). Candidate-versus-promoted semantics for
//! that update are owned by the `system_state` module (PACK-002).
//!
//! The [`PackDependencyLock`] is the resolution artifact: it maps every
//! declared dependency to the immutable identity it resolved to plus the
//! content digest of the resolved artifact as recorded by the resolver:
//!
//! - workflow version dependencies record the pinned version's definition
//!   content digest (obtained from the Workflow Control Plane, which stays
//!   the sole authority for workflow content);
//! - capability dependencies record the digest of the resolved capability
//!   implementation or binding;
//! - pack dependencies record the pinned revision's system-state digest.
//!
//! Verifying those digests against the live artifacts is the control plane's
//! job at seal time; the lock records them so pack revisions pin an exact
//! dependency state. The lock never verifies workflow semantics and never
//! gains the right to perform workflow transitions.
//!
//! The boundary against PACK-002: the `system_state` module owns how system
//! state *references* dependencies; this module owns the declared-set and
//! lock contract. State references resolve into [`PackDependencies`] values
//! before a revision locks them.

use codex_workflow_contracts::CapabilityId;
use codex_workflow_contracts::ContentDigest;
use codex_workflow_contracts::DependencyProvenance;
use codex_workflow_contracts::WorkflowVersionId;
use serde::Deserialize;
use serde::Serialize;
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::fmt;

use crate::PackContractError;
use crate::PackRevisionId;

/// A local reference name for a declared pack dependency.
///
/// Dependency ids are stable, snake_case names used to refer to one declared
/// dependency inside a pack (for example `billing` or `browser`). They must
/// be unique across a pack's entire declared dependency set, including
/// across kinds, so a lock entry is never ambiguous.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct PackDependencyId(String);

/// A declared dependency on an immutable workflow version.
///
/// The declaration pins the exact [`WorkflowVersionId`]: no ranges, no
/// branches, no moving references. Updating the pin is a new resolution and
/// a new candidate pack revision; it can never silently change a locked one.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkflowVersionDependency {
    /// The local reference name of the dependency.
    pub dependency_id: PackDependencyId,
    /// The pinned immutable workflow version identity.
    pub version: WorkflowVersionId,
}

/// A declared dependency on a semantic capability.
///
/// The declaration names the semantic capability; the lock records which
/// implementation content the capability resolved to.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CapabilityDependency {
    /// The local reference name of the dependency.
    pub dependency_id: PackDependencyId,
    /// The required semantic capability.
    pub capability: CapabilityId,
}

/// A declared dependency on another pack revision.
///
/// Pack-to-pack dependencies pin an exact [`PackRevisionId`]: composition
/// (PACK-005) resolves compatibility; the dependency here only pins the
/// immutable identity.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PackRevisionDependency {
    /// The local reference name of the dependency.
    pub dependency_id: PackDependencyId,
    /// The pinned immutable pack revision identity.
    pub pack: PackRevisionId,
}

/// The complete declared dependency set of a pack revision.
///
/// Dependencies of different kinds are stored in separate fields so
/// consumers can reason about them independently, mirroring
/// [`codex_workflow_contracts::WorkflowDependencies`]. Local dependency ids
/// must be unique across all three kinds.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields, default)]
pub struct PackDependencies {
    /// Immutable workflow versions the pack composes.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub workflows: Vec<WorkflowVersionDependency>,
    /// Semantic capabilities the pack requires.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub capabilities: Vec<CapabilityDependency>,
    /// Other pack revisions the pack builds on.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub packs: Vec<PackRevisionDependency>,
}

/// The key of one resolved lock entry.
///
/// Serializes as a compact string key (see [`PackDependencyKey::as_key`])
/// so lock entries can be stored in a JSON map with deterministic ordering.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PackDependencyKey {
    /// A resolved workflow version dependency.
    WorkflowVersion {
        /// The declared dependency's local reference name.
        dependency_id: PackDependencyId,
    },
    /// A resolved capability dependency.
    Capability {
        /// The declared dependency's local reference name.
        dependency_id: PackDependencyId,
    },
    /// A resolved pack revision dependency.
    PackRevision {
        /// The declared dependency's local reference name.
        dependency_id: PackDependencyId,
    },
}

/// The immutable identity a pack dependency resolved to.
///
/// Each variant carries exactly one identity payload, so the variants are
/// newtype-shaped: the external tag names the dependency kind and the
/// payload is the identity itself. The sibling `key` field of a
/// [`ResolvedPackDependency`] independently names the dependency kind, so
/// records stay self-describing on the wire without redundant nesting.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ResolvedPackDependencyIdentity {
    /// A resolved workflow version dependency, pinned to the immutable
    /// published version identity.
    WorkflowVersion(WorkflowVersionId),
    /// A resolved capability dependency, carrying the semantic capability
    /// identity.
    Capability(CapabilityId),
    /// A resolved pack dependency, pinned to the immutable pack revision
    /// identity.
    PackRevision(PackRevisionId),
}

/// One resolved dependency entry inside a pack dependency lock.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResolvedPackDependency {
    /// The declared dependency this entry resolves.
    pub key: PackDependencyKey,
    /// The immutable identity the dependency resolved to.
    pub resolved: ResolvedPackDependencyIdentity,
    /// Content digest of the resolved artifact as recorded by the resolver
    /// (see the module documentation for the per-kind semantics).
    pub content_digest: ContentDigest,
    /// Where and under which terms the dependency was resolved; provenance
    /// only, never credentials.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provenance: Option<DependencyProvenance>,
}

/// The resolved, immutable dependency identities of a pack revision.
///
/// The lock is the resolution artifact: every declared dependency resolves to
/// exactly one immutable identity plus the content digest of the resolved
/// artifact. The lock is content-addressed
/// ([`PackDependencyLock::digest`]) and its digest participates in the pack
/// revision identity, so any dependency change — a re-pinned version, a
/// different capability implementation, an updated dependency pack —
/// produces a different revision identity.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields, default)]
pub struct PackDependencyLock {
    /// Resolved entries, keyed by declared dependency.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub entries: BTreeMap<PackDependencyKey, ResolvedPackDependency>,
}

impl PackDependencyLock {
    /// Inserts a resolved entry, replacing any previous resolution of the
    /// same key.
    pub fn insert(&mut self, entry: ResolvedPackDependency) {
        self.entries.insert(entry.key.clone(), entry);
    }

    /// Verifies the lock covers every dependency declared by `dependencies`
    /// and that each resolution matches its declaration exactly.
    ///
    /// Completeness and pin fidelity are hard invariants of a lockable pack
    /// revision: a lock that does not resolve all declared dependencies, or
    /// that resolved one to a different identity than the declaration pins,
    /// is rejected. A pack revision therefore cannot silently alter a
    /// referenced [`WorkflowVersionId`].
    pub fn verify_covers(&self, dependencies: &PackDependencies) -> Result<(), PackContractError> {
        // A wire-tampered lock cannot hide an entry behind another
        // dependency's key: each entry's recorded key must match the map key
        // it is stored under.
        for (map_key, entry) in &self.entries {
            if map_key != &entry.key {
                return Err(PackContractError::IncompleteDependencyLock {
                    reason: format!("lock entry `{}` records a mismatched key", map_key.as_key()),
                });
            }
        }
        let mut seen_ids = BTreeSet::new();
        for dependency in &dependencies.workflows {
            reject_duplicate_id(&mut seen_ids, &dependency.dependency_id)?;
            let key = PackDependencyKey::WorkflowVersion {
                dependency_id: dependency.dependency_id.clone(),
            };
            let Some(entry) = self.entries.get(&key) else {
                return Err(PackContractError::IncompleteDependencyLock {
                    reason: format!(
                        "declared workflow version dependency `{}` is not locked",
                        dependency.dependency_id
                    ),
                });
            };
            let ResolvedPackDependencyIdentity::WorkflowVersion(version) = &entry.resolved else {
                return Err(PackContractError::IncompleteDependencyLock {
                    reason: format!(
                        "workflow version dependency `{}` resolved to a non-workflow identity",
                        dependency.dependency_id
                    ),
                });
            };
            if version != &dependency.version {
                return Err(PackContractError::IncompleteDependencyLock {
                    reason: format!(
                        "workflow version dependency `{}` resolved to a different version than \
                         declared",
                        dependency.dependency_id
                    ),
                });
            }
        }
        for dependency in &dependencies.capabilities {
            reject_duplicate_id(&mut seen_ids, &dependency.dependency_id)?;
            let key = PackDependencyKey::Capability {
                dependency_id: dependency.dependency_id.clone(),
            };
            let Some(entry) = self.entries.get(&key) else {
                return Err(PackContractError::IncompleteDependencyLock {
                    reason: format!(
                        "declared capability dependency `{}` is not locked",
                        dependency.dependency_id
                    ),
                });
            };
            let ResolvedPackDependencyIdentity::Capability(capability) = &entry.resolved else {
                return Err(PackContractError::IncompleteDependencyLock {
                    reason: format!(
                        "capability dependency `{}` resolved to a non-capability identity",
                        dependency.dependency_id
                    ),
                });
            };
            if capability != &dependency.capability {
                return Err(PackContractError::IncompleteDependencyLock {
                    reason: format!(
                        "capability dependency `{}` resolved to a different capability than \
                         declared",
                        dependency.dependency_id
                    ),
                });
            }
        }
        for dependency in &dependencies.packs {
            reject_duplicate_id(&mut seen_ids, &dependency.dependency_id)?;
            let key = PackDependencyKey::PackRevision {
                dependency_id: dependency.dependency_id.clone(),
            };
            let Some(entry) = self.entries.get(&key) else {
                return Err(PackContractError::IncompleteDependencyLock {
                    reason: format!(
                        "declared pack dependency `{}` is not locked",
                        dependency.dependency_id
                    ),
                });
            };
            let ResolvedPackDependencyIdentity::PackRevision(pack) = &entry.resolved else {
                return Err(PackContractError::IncompleteDependencyLock {
                    reason: format!(
                        "pack dependency `{}` resolved to a non-pack identity",
                        dependency.dependency_id
                    ),
                });
            };
            if pack != &dependency.pack {
                return Err(PackContractError::IncompleteDependencyLock {
                    reason: format!(
                        "pack dependency `{}` resolved to a different revision than declared",
                        dependency.dependency_id
                    ),
                });
            }
        }
        Ok(())
    }

    /// Canonical digest of the lock, used inside the pack revision identity.
    pub fn digest(&self) -> Result<ContentDigest, PackContractError> {
        Ok(ContentDigest::of(&self.entries)?)
    }
}

impl PackDependencyKey {
    /// Compact string form used as the lock's serialization key, for example
    /// `workflow:billing`, `capability:browser`, or `pack:atlas`.
    ///
    /// The three kinds use disjoint prefixes and the identifier is a
    /// validated snake_case token, so the string form round-trips
    /// losslessly.
    pub fn as_key(&self) -> String {
        match self {
            Self::WorkflowVersion { dependency_id } => {
                format!("workflow:{}", dependency_id.as_ref())
            }
            Self::Capability { dependency_id } => {
                format!("capability:{}", dependency_id.as_ref())
            }
            Self::PackRevision { dependency_id } => format!("pack:{}", dependency_id.as_ref()),
        }
    }

    fn parse_key(value: &str) -> Result<Self, PackContractError> {
        let Some((kind, rest)) = value.split_once(':') else {
            return Err(PackContractError::InvalidIdentifier {
                kind: "pack dependency key",
                value: value.to_owned(),
                reason: "must be prefixed with `workflow:`, `capability:`, or `pack:`",
            });
        };
        let dependency_id = PackDependencyId::parse(rest)?;
        match kind {
            "workflow" => Ok(Self::WorkflowVersion { dependency_id }),
            "capability" => Ok(Self::Capability { dependency_id }),
            "pack" => Ok(Self::PackRevision { dependency_id }),
            _ => Err(PackContractError::InvalidIdentifier {
                kind: "pack dependency key",
                value: value.to_owned(),
                reason: "must be prefixed with `workflow:`, `capability:`, or `pack:`",
            }),
        }
    }
}

impl Serialize for PackDependencyKey {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.as_key())
    }
}

impl<'de> Deserialize<'de> for PackDependencyKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::parse_key(&value).map_err(serde::de::Error::custom)
    }
}

impl PackDependencyId {
    /// Parses a dependency reference name.
    ///
    /// Dependency ids use non-empty lowercase snake_case (ASCII letters,
    /// digits, `_`), must not start or end with `_`, and must not contain
    /// whitespace, so they stay stable across serializations and language
    /// bindings.
    pub fn parse(value: impl Into<String>) -> Result<Self, PackContractError> {
        let value = value.into();
        let valid = !value.is_empty()
            && value
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
            && !value.starts_with('_')
            && !value.ends_with('_');
        if valid {
            Ok(Self(value))
        } else {
            Err(PackContractError::InvalidIdentifier {
                kind: "pack dependency id",
                value,
                reason: "must be non-empty lowercase snake_case",
            })
        }
    }

    /// The dependency reference name.
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl TryFrom<String> for PackDependencyId {
    type Error = PackContractError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl TryFrom<&str> for PackDependencyId {
    type Error = PackContractError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::parse(value.to_owned())
    }
}

impl From<PackDependencyId> for String {
    fn from(value: PackDependencyId) -> Self {
        value.0
    }
}

impl AsRef<str> for PackDependencyId {
    fn as_ref(&self) -> &str {
        self.0.as_str()
    }
}

impl fmt::Display for PackDependencyId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0.as_str())
    }
}

impl fmt::Debug for PackDependencyId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PackDependencyId({})", self.0)
    }
}

/// Rejects dependency ids that were already declared under another kind.
fn reject_duplicate_id(
    seen: &mut BTreeSet<PackDependencyId>,
    dependency_id: &PackDependencyId,
) -> Result<(), PackContractError> {
    if !seen.insert(dependency_id.clone()) {
        return Err(PackContractError::InvalidIdentifier {
            kind: "pack dependency id",
            value: dependency_id.to_string(),
            reason: "dependency ids must be unique across the declared set",
        });
    }
    Ok(())
}

#[cfg(test)]
#[path = "dependency_tests.rs"]
mod tests;
