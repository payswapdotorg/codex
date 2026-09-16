//! Pack revision identity: the identity tuple and compatibility contract.
//!
//! Owned by the PACK-001 work order (Contract Foundation).
//!
//! A pack revision is an immutable, content-addressed system-state hypothesis.
//! Its identity is the frozen tuple:
//!
//! ```text
//! pack identity
//! pack semantic version
//! system-state digest
//! mission digest
//! policy digest (constitution + governing policy)
//! dependency lock digest
//! parent revision
//! ```
//!
//! The tuple mirrors [`codex_workflow_contracts::ExecutionVersionIdentity`]:
//! [`PackRevisionIdentity::revision_id`] digests a provenance-stripped anchored
//! projection of the tuple into a [`crate::PackRevisionId`]. Publication
//! provenance (the producing principal, see [`crate::PackProvenance`]) is
//! deliberately excluded: identical governed content yields the same revision
//! identity regardless of who proposed it. The parent revision, by contrast,
//! IS part of identity — a child revision that builds on a different parent is
//! a different revision, mirroring how lineage anchors immutable history.
//!
//! The system-state digest is an opaque input here: the system-state content
//! it covers is owned by the `system_state` module (PACK-002). This module
//! never interprets it; it only pins it.
//!
//! Candidate-versus-promoted revision semantics are likewise owned by
//! PACK-002; this module owns only the identity tuple and the
//! [`PackCompatibility`] contract.

use codex_workflow_contracts::ContentDigest;
use codex_workflow_contracts::SemanticVersion;
use serde::Deserialize;
use serde::Serialize;
use std::collections::BTreeMap;
use std::fmt;

use crate::PackContractError;
use crate::PackId;
use crate::PackRevisionId;

/// The wire-format version of the pack revision identity and compatibility
/// contracts defined in this module.
///
/// Bumping this constant is a governed, breaking change: every recorded
/// [`PackCompatibility`] carries the version it was authored against, and
/// validation rejects records authored against any other version rather than
/// guessing at their meaning.
pub const PACK_REVISION_CONTRACT_VERSION: u32 = 1;

/// The identity tuple of a pack revision, exactly as frozen by the pack
/// architecture.
///
/// Every field is identity-covered: mutating any of them produces a different
/// [`PackRevisionId`]. The tuple pins immutable content by digest
/// (system state, mission, policy, dependency lock) rather than by reference,
/// so a revision cannot silently drift from the content it was sealed
/// against.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PackRevisionIdentity {
    /// The pack lineage this revision belongs to.
    pub pack: PackId,
    /// Semantic version of this revision, reusing the Universal
    /// [`SemanticVersion`] for parsing and ordering semantics.
    pub semantic_version: SemanticVersion,
    /// Digest of the pack system state (opaque here; content owned by the
    /// `system_state` module, PACK-002).
    pub system_state_digest: ContentDigest,
    /// Digest of the mission model ([`crate::Mission::digest`]).
    pub mission_digest: ContentDigest,
    /// Digest of the governing policy content: constitution plus governing
    /// policies ([`crate::PackPolicySet::digest`]).
    pub policy_digest: ContentDigest,
    /// Digest of the resolved dependency lock
    /// ([`crate::PackDependencyLock::digest`]).
    pub dependency_lock_digest: ContentDigest,
    /// The revision this revision builds on, or `None` for a root revision.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_revision: Option<PackRevisionId>,
}

impl PackRevisionIdentity {
    /// Computes the immutable revision identity digest for this tuple.
    ///
    /// The digest covers exactly the identity tuple fields above. Production
    /// provenance is excluded by construction: the tuple carries no producer
    /// field, so the same governed content proposed by a user, an agent, or a
    /// control plane always yields the same revision identity.
    pub fn revision_id(&self) -> Result<PackRevisionId, PackContractError> {
        let anchored = AnchoredPackRevision {
            pack: &self.pack,
            semantic_version: &self.semantic_version,
            system_state_digest: &self.system_state_digest,
            mission_digest: &self.mission_digest,
            policy_digest: &self.policy_digest,
            dependency_lock_digest: &self.dependency_lock_digest,
            parent_revision: self.parent_revision.as_ref(),
        };
        Ok(PackRevisionId::from_digest(ContentDigest::of(&anchored)?))
    }
}

/// The identity-covered projection of a pack revision: exactly the
/// [`PackRevisionIdentity`] tuple, serialized with an explicit (never
/// omitted) parent so root and child revisions can never collide.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AnchoredPackRevision<'a> {
    pack: &'a PackId,
    semantic_version: &'a SemanticVersion,
    system_state_digest: &'a ContentDigest,
    mission_digest: &'a ContentDigest,
    policy_digest: &'a ContentDigest,
    dependency_lock_digest: &'a ContentDigest,
    parent_revision: Option<&'a PackRevisionId>,
}

/// The compatibility and version semantics of a pack revision.
///
/// Compatibility is explicit and typed: a revision declares which platform
/// contract surfaces it requires (and at which minimum versions) and how it
/// relates to its parent revision. There is no implicit or guessed
/// compatibility: a record that omits or contradicts these declarations fails
/// validation instead of being interpreted.
///
/// This contract is revision metadata, not part of the frozen identity tuple:
/// a revision and its successor may declare the same compatibility posture
/// without that affecting their identities. Whether a compatibility record
/// should additionally be pinned by the identity tuple is a convergence
/// decision for the Pack program (flagged by PACK-001).
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PackCompatibility {
    /// The pack revision contract version this record was authored against.
    ///
    /// Must equal [`PACK_REVISION_CONTRACT_VERSION`]; any other value is
    /// rejected by [`PackCompatibility::validate`] instead of being
    /// interpreted.
    pub contract_version: u32,
    /// Platform contracts the pack requires, each with a minimum version.
    ///
    /// A pack must declare a requirement for every platform surface it
    /// depends on structurally; consumers resolve the requirement list before
    /// activating the revision. Each contract may appear at most once.
    pub required_contracts: Vec<ContractVersionRequirement>,
    /// How this revision relates to its parent, when it has one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_relation: Option<ParentRevisionRelation>,
}

impl PackCompatibility {
    /// Builds a compatibility record for the current contract version from
    /// the given contract requirements.
    ///
    /// Requirements are collected into canonical (contract-ordered) form, so
    /// two records built from the same requirements are always equal.
    pub fn new(
        required_contracts: impl IntoIterator<Item = ContractVersionRequirement>,
        parent_relation: Option<ParentRevisionRelation>,
    ) -> Self {
        let requirements: BTreeMap<PlatformContract, SemanticVersion> = required_contracts
            .into_iter()
            .map(|requirement| (requirement.contract, requirement.minimum_version))
            .collect();
        Self {
            contract_version: PACK_REVISION_CONTRACT_VERSION,
            required_contracts: requirements
                .into_iter()
                .map(|(contract, minimum_version)| ContractVersionRequirement {
                    contract,
                    minimum_version,
                })
                .collect(),
            parent_relation,
        }
    }

    /// Validates the compatibility record.
    ///
    /// A record is valid only when it was authored against the contract
    /// version this surface implements and declares at most one requirement
    /// per platform contract.
    pub fn validate(&self) -> Result<(), PackContractError> {
        if self.contract_version != PACK_REVISION_CONTRACT_VERSION {
            return Err(PackContractError::PolicyConflict {
                reason: format!(
                    "authored against pack revision contract version {}, but this surface \
                     implements version {PACK_REVISION_CONTRACT_VERSION}",
                    self.contract_version
                ),
            });
        }
        let mut seen = std::collections::BTreeSet::new();
        for requirement in &self.required_contracts {
            if !seen.insert(requirement.contract.clone()) {
                return Err(PackContractError::PolicyConflict {
                    reason: format!(
                        "duplicate requirement for platform contract `{}`",
                        requirement.contract
                    ),
                });
            }
        }
        Ok(())
    }
}

/// A required platform contract and the minimum version this pack needs from
/// it.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ContractVersionRequirement {
    /// The platform contract surface being required.
    pub contract: PlatformContract,
    /// Minimum contract version the pack requires.
    pub minimum_version: SemanticVersion,
}

/// A well-known platform contract surface a pack revision can declare
/// requirements against.
///
/// The list is closed and typed on purpose: packs must not guess at platform
/// surface names. New surfaces are added as variants, which is a wire-format
/// change governed by [`PACK_REVISION_CONTRACT_VERSION`].
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PlatformContract {
    /// The pack contract surface (this crate): identity, mission,
    /// constitution, policy, and dependency contracts.
    PackContracts,
    /// The workflow contract surface (`codex-workflow-contracts`): workflow
    /// identity, versions, and dependency locks referenced by packs.
    WorkflowContracts,
}

impl PlatformContract {
    /// The contract's stable label (the camelCase wire name).
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::PackContracts => "packContracts",
            Self::WorkflowContracts => "workflowContracts",
        }
    }
}

impl fmt::Display for PlatformContract {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// How a pack revision relates to its parent revision.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "relation")]
pub enum ParentRevisionRelation {
    /// A compatible successor: consumers may migrate from the parent under
    /// the declarations already recorded for it.
    Compatible,
    /// A breaking successor: consumers must re-validate before migrating.
    /// Carries the required, non-empty rationale explaining what broke.
    Breaking {
        /// Why this revision is breaking relative to its parent.
        rationale: CompatibilityRationale,
    },
}

/// A required, non-empty rationale explaining a breaking relation to a
/// parent revision.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct CompatibilityRationale(String);

impl CompatibilityRationale {
    /// Parses a breaking-change rationale.
    ///
    /// Rationales must be non-empty and must not start or end with
    /// whitespace: a breaking relation without an explanation is a guess, and
    /// guesses are not part of this contract.
    pub fn parse(value: impl Into<String>) -> Result<Self, PackContractError> {
        parse_rationale(value.into()).map(Self)
    }
}

impl TryFrom<String> for CompatibilityRationale {
    type Error = PackContractError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl From<CompatibilityRationale> for String {
    fn from(value: CompatibilityRationale) -> Self {
        value.0
    }
}

impl AsRef<str> for CompatibilityRationale {
    fn as_ref(&self) -> &str {
        self.0.as_str()
    }
}

/// Validates a breaking-change rationale: non-empty, no edge whitespace.
fn parse_rationale(value: String) -> Result<String, PackContractError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        Err(PackContractError::InvalidIdentifier {
            kind: "compatibility rationale",
            value,
            reason: "must be non-empty",
        })
    } else if trimmed.len() != value.len() {
        Err(PackContractError::InvalidIdentifier {
            kind: "compatibility rationale",
            value,
            reason: "must not start or end with whitespace",
        })
    } else {
        Ok(value)
    }
}

#[cfg(test)]
#[path = "revision_tests.rs"]
mod tests;
