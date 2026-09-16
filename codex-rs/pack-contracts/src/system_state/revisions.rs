//! Candidate and promoted pack revision records.
//!
//! A [`CandidatePackState`] is a proposed system state: pack identity,
//! system-state content, provenance, and a computed revision identity. A
//! [`PromotedPackState`] is the immutable revision created by promoting a
//! candidate. The two are structurally distinct types, so lifecycle position
//! cannot be misreported, and promotion never mutates the historical
//! candidate record.
//!
//! Revision identities are derived from the full pack revision identity
//! tuple ([`crate::PackRevisionIdentity`]: pack lineage, semantic version,
//! system-state digest, mission digest, policy digest, dependency-lock
//! digest, parent revision) plus a record kind discriminator, mirroring how
//! published workflow versions derive identity: identical governed content
//! always yields the identical revision identity, regardless of who
//! proposed it. Descriptive metadata (roles, labels, constraint text) and
//! provenance producers are excluded. The record kind and — for promoted
//! revisions — the source candidate identity participate in the tuple, so a
//! promoted revision identity is always distinct from the candidate
//! identity it was promoted from, and a dependency or mission change can
//! never hide inside a revision that claims to be unchanged.
//!
//! Promotion here is a contract operation (record transformation plus
//! integrity rules), not a controller: no storage, no gates, no automation.
//! The promotion decision authority lives in control planes.

use codex_workflow_contracts::ContentDigest;
use codex_workflow_contracts::SemanticVersion;
use serde::Deserialize;
use serde::Serialize;

use super::digest_of;
use super::state::PackSystemState;
use super::validate_descriptive_text;
use crate::Mission;
use crate::PackContractError;
use crate::PackDependencyLock;
use crate::PackId;
use crate::PackPolicySet;
use crate::PackProvenance;
use crate::PackRevisionId;
use crate::PackRevisionIdentity;
use crate::ResolvedPackDependencyIdentity;

/// A lineage edge from a revision to the promoted revision it builds on.
///
/// The parent [`PackRevisionId`] preserves the exact parent identity. The
/// optional label is descriptive text about the lineage edge (for example
/// `"weekly baseline"`); it never participates in revision identity.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ParentRevision {
    /// The parent revision identity, preserved exactly.
    pub revision: PackRevisionId,
    /// Descriptive label for the lineage edge.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

impl ParentRevision {
    /// Records an unlabeled parent lineage edge.
    pub fn new(revision: PackRevisionId) -> Self {
        Self {
            revision,
            label: None,
        }
    }

    /// Records a parent lineage edge with a descriptive label.
    pub fn labeled(
        revision: PackRevisionId,
        label: impl Into<String>,
    ) -> Result<Self, PackContractError> {
        Ok(Self {
            revision,
            label: Some(validate_descriptive_text(
                "parent revision label",
                label.into(),
            )?),
        })
    }
}

/// The record kind included in every revision identity tuple.
///
/// Candidate and promoted records over identical content must never share an
/// identity, so the kind is part of the digested tuple.
#[derive(Clone, Copy, Serialize)]
enum RevisionRecordKind {
    /// A proposed system state.
    #[serde(rename = "candidate")]
    Candidate,
    /// An immutable promoted revision.
    #[serde(rename = "promoted")]
    Promoted,
}

/// The identity-covered projection of a candidate record: the full pack
/// revision identity tuple plus the candidate record-kind discriminator.
/// Descriptive metadata (labels, roles, constraints) and the provenance
/// producer are excluded so identical proposed content always yields the
/// identical candidate identity.
#[derive(Serialize)]
struct AnchoredCandidateRecord<'a> {
    record_kind: RevisionRecordKind,
    #[serde(flatten)]
    identity: &'a PackRevisionIdentity,
}

/// The identity-covered projection of a promoted record: the full pack
/// revision identity tuple, the promoted record-kind discriminator, and the
/// identity of the candidate record that was promoted.
#[derive(Serialize)]
struct AnchoredPromotedRecord<'a> {
    record_kind: RevisionRecordKind,
    #[serde(flatten)]
    identity: &'a PackRevisionIdentity,
    promoted_from: &'a PackRevisionId,
}

/// Computes the candidate record identity for a full revision tuple.
fn candidate_revision_identity(
    identity: &PackRevisionIdentity,
) -> Result<PackRevisionId, PackContractError> {
    let anchored = AnchoredCandidateRecord {
        record_kind: RevisionRecordKind::Candidate,
        identity,
    };
    Ok(PackRevisionId::from_digest(digest_of(&anchored)?))
}

/// Computes the promoted record identity for a full revision tuple.
fn promoted_revision_identity(
    identity: &PackRevisionIdentity,
    promoted_from: &PackRevisionId,
) -> Result<PackRevisionId, PackContractError> {
    let anchored = AnchoredPromotedRecord {
        record_kind: RevisionRecordKind::Promoted,
        identity,
        promoted_from,
    };
    Ok(PackRevisionId::from_digest(digest_of(&anchored)?))
}

/// The governed content of one pack revision: everything the frozen
/// architecture places under pack governance, assembled into one sealable
/// bundle.
///
/// This is the integration point between the contract domains: the mission
/// and governing policy records (PACK-001), the resolved dependency lock
/// pinning immutable [`codex_workflow_contracts::WorkflowVersionId`] and
/// capability identities (PACK-001), and the system-state snapshot
/// referencing them (PACK-002). [`PackRevisionContent::validate`] enforces
/// the integration invariants: every workflow version and capability the
/// system state references must be pinned by the dependency lock, so a
/// revision can never reference an unlocked dependency.
///
/// The bundle deliberately carries the full records (not only digests) so
/// record integrity verification can recompute every digest from content.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PackRevisionContent {
    /// The pack lineage this revision belongs to.
    pub pack_id: PackId,
    /// Semantic version of this revision.
    pub semantic_version: SemanticVersion,
    /// The mission this revision serves; user/organization authority.
    pub mission: Mission,
    /// The governing policy set: constitution plus governing policies.
    pub policy_set: PackPolicySet,
    /// The resolved dependency lock pinning immutable identities.
    pub dependency_lock: PackDependencyLock,
    /// The proposed system-state content.
    pub system_state: PackSystemState,
}

impl PackRevisionContent {
    /// Validates the content and the integration invariants.
    ///
    /// Validation covers the policy set, the system state's canonical form,
    /// and the lock-coverage cross-check: every workflow version and
    /// capability referenced by the system state must be resolved by the
    /// dependency lock to the exact same identity.
    pub fn validate(&self) -> Result<(), PackContractError> {
        self.policy_set.validate()?;
        self.system_state.validate()?;
        ensure_workflow_references_are_locked(&self.system_state, &self.dependency_lock)?;
        ensure_capability_references_are_locked(&self.system_state, &self.dependency_lock)?;
        Ok(())
    }

    /// Derives the provenance-stripped revision identity tuple for this
    /// content under the given parent revision.
    ///
    /// All component digests are computed from the carried records, so the
    /// tuple always reflects the content it was sealed against.
    pub fn identity_tuple(
        &self,
        parent_revision: Option<&PackRevisionId>,
    ) -> Result<PackRevisionIdentity, PackContractError> {
        Ok(PackRevisionIdentity {
            pack: self.pack_id.clone(),
            semantic_version: self.semantic_version.clone(),
            system_state_digest: self.system_state.content_digest()?,
            mission_digest: self.mission.digest()?,
            policy_digest: self.policy_set.digest()?,
            dependency_lock_digest: self.dependency_lock.digest()?,
            parent_revision: parent_revision.cloned(),
        })
    }
}

/// Ensures every workflow version referenced by the system state is pinned
/// by the dependency lock to the exact same identity.
fn ensure_workflow_references_are_locked(
    system_state: &PackSystemState,
    dependency_lock: &PackDependencyLock,
) -> Result<(), PackContractError> {
    for reference in &system_state.workflow_version_refs {
        let pinned = dependency_lock
            .entries
            .values()
            .any(|entry| match &entry.resolved {
                ResolvedPackDependencyIdentity::WorkflowVersion(version) => {
                    version == &reference.workflow_version_id
                }
                ResolvedPackDependencyIdentity::Capability(_) => false,
                ResolvedPackDependencyIdentity::PackRevision(_) => false,
            });
        if !pinned {
            return Err(PackContractError::IncompleteDependencyLock {
                reason: format!(
                    "workflow version reference `{}` is not pinned by the dependency lock",
                    reference.workflow_version_id
                ),
            });
        }
    }
    Ok(())
}

/// Ensures every capability referenced by the system state is pinned by the
/// dependency lock to the exact same identity.
fn ensure_capability_references_are_locked(
    system_state: &PackSystemState,
    dependency_lock: &PackDependencyLock,
) -> Result<(), PackContractError> {
    for reference in &system_state.capability_refs {
        let pinned = dependency_lock
            .entries
            .values()
            .any(|entry| match &entry.resolved {
                ResolvedPackDependencyIdentity::Capability(capability) => {
                    capability == &reference.capability
                }
                ResolvedPackDependencyIdentity::WorkflowVersion(_) => false,
                ResolvedPackDependencyIdentity::PackRevision(_) => false,
            });
        if !pinned {
            return Err(PackContractError::IncompleteDependencyLock {
                reason: format!(
                    "capability reference `{}` is not pinned by the dependency lock",
                    reference.capability.as_ref()
                ),
            });
        }
    }
    Ok(())
}

/// Checks that the parent revision record and the provenance parent agree.
///
/// Both fields describe the same lineage edge, so they must both be absent
/// (a root revision) or both name the identical parent revision. A record
/// where they disagree is ambiguous about its lineage and is rejected.
fn ensure_lineage_consistency(
    parent: Option<&ParentRevision>,
    provenance: &PackProvenance,
) -> Result<(), PackContractError> {
    let consistent = match (parent, provenance.parent_revision.as_ref()) {
        (Some(record), Some(provenance_parent)) => record.revision == *provenance_parent,
        (None, None) => true,
        (Some(_), None) | (None, Some(_)) => false,
    };
    if consistent {
        Ok(())
    } else {
        Err(PackContractError::RevisionIntegrity {
            reason: "parent revision record and provenance parent revision disagree".to_owned(),
        })
    }
}

/// A proposed pack system state awaiting a promotion decision.
///
/// A candidate is what LLMs, workers, and users produce: a hypothesis about
/// the next system state. It carries the pack identity, the system-state
/// content, the verbatim provenance, and a computed revision identity. It is
/// a plain immutable record — no setters, no lifecycle methods — and
/// promotion reads it by reference without altering it.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CandidatePackState {
    /// The pack lineage this revision belongs to.
    pub pack_id: PackId,
    /// Semantic version of this candidate revision.
    pub semantic_version: SemanticVersion,
    /// The promoted revision this candidate builds on, or `None` for a root
    /// candidate.
    #[serde(
        rename = "parentRevision",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub parent: Option<ParentRevision>,
    /// The mission this candidate serves; user/organization authority.
    pub mission: Mission,
    /// Digest of the mission content, pinned so integrity verification can
    /// detect any mission drift.
    pub mission_digest: ContentDigest,
    /// The governing policy set: constitution plus governing policies.
    pub policy_set: PackPolicySet,
    /// Digest of the governing policy content.
    pub policy_digest: ContentDigest,
    /// The resolved dependency lock pinning immutable identities.
    pub dependency_lock: PackDependencyLock,
    /// Digest of the dependency lock.
    pub dependency_lock_digest: ContentDigest,
    /// The proposed system-state content.
    pub system_state: PackSystemState,
    /// Digest of the system-state content, pinned so a revision identity can
    /// be verified against the exact content it covers.
    pub system_state_digest: ContentDigest,
    /// Provenance carried verbatim: parent lineage and producing principal.
    /// Provenance is descriptive and never grants authority.
    pub provenance: PackProvenance,
    /// The computed candidate revision identity.
    pub revision_id: PackRevisionId,
}

impl CandidatePackState {
    /// Proposes a root candidate: a first governed revision with no parent.
    ///
    /// The provenance must describe a root revision (no parent).
    pub fn propose_root(
        content: PackRevisionContent,
        provenance: PackProvenance,
    ) -> Result<Self, PackContractError> {
        Self::construct(content, None, provenance)
    }

    /// Proposes a candidate building on a parent promoted revision.
    ///
    /// The provenance must name the identical parent revision, so lineage is
    /// unambiguous.
    pub fn propose(
        content: PackRevisionContent,
        parent: ParentRevision,
        provenance: PackProvenance,
    ) -> Result<Self, PackContractError> {
        Self::construct(content, Some(parent), provenance)
    }

    /// Assembles a candidate record, deriving its digests and identity.
    fn construct(
        content: PackRevisionContent,
        parent: Option<ParentRevision>,
        provenance: PackProvenance,
    ) -> Result<Self, PackContractError> {
        content.validate()?;
        ensure_lineage_consistency(parent.as_ref(), &provenance)?;
        let mission_digest = content.mission.digest()?;
        let policy_digest = content.policy_set.digest()?;
        let dependency_lock_digest = content.dependency_lock.digest()?;
        let system_state_digest = content.system_state.content_digest()?;
        let identity = PackRevisionIdentity {
            pack: content.pack_id.clone(),
            semantic_version: content.semantic_version.clone(),
            system_state_digest: system_state_digest.clone(),
            mission_digest: mission_digest.clone(),
            policy_digest: policy_digest.clone(),
            dependency_lock_digest: dependency_lock_digest.clone(),
            parent_revision: parent.as_ref().map(|parent| parent.revision.clone()),
        };
        let revision_id = candidate_revision_identity(&identity)?;
        Ok(Self {
            pack_id: content.pack_id,
            semantic_version: content.semantic_version,
            parent,
            mission: content.mission,
            mission_digest,
            policy_set: content.policy_set,
            policy_digest,
            dependency_lock: content.dependency_lock,
            dependency_lock_digest,
            system_state: content.system_state,
            system_state_digest,
            provenance,
            revision_id,
        })
    }

    /// Verifies the record's integrity end to end.
    ///
    /// A candidate passes only when:
    ///
    /// - the system state is in canonical form (sorted, duplicate-free
    ///   references);
    /// - the parent revision record and the provenance parent agree;
    /// - the embedded system state digests to the recorded state digest;
    /// - the recorded revision id equals the digest of the anchored identity
    ///   tuple.
    ///
    /// This is the check that makes a candidate record effectively immutable:
    /// a tampered record fails recomputation.
    pub fn verify_integrity(&self) -> Result<(), PackContractError> {
        self.system_state.validate()?;
        self.policy_set.validate()?;
        ensure_workflow_references_are_locked(&self.system_state, &self.dependency_lock)?;
        ensure_capability_references_are_locked(&self.system_state, &self.dependency_lock)?;
        ensure_lineage_consistency(self.parent.as_ref(), &self.provenance)?;
        let recomputed_mission_digest = self.mission.digest()?;
        if recomputed_mission_digest != self.mission_digest {
            return Err(PackContractError::RevisionIntegrity {
                reason: "mission content does not match the recorded mission digest".to_owned(),
            });
        }
        let recomputed_policy_digest = self.policy_set.digest()?;
        if recomputed_policy_digest != self.policy_digest {
            return Err(PackContractError::RevisionIntegrity {
                reason: "policy content does not match the recorded policy digest".to_owned(),
            });
        }
        let recomputed_lock_digest = self.dependency_lock.digest()?;
        if recomputed_lock_digest != self.dependency_lock_digest {
            return Err(PackContractError::RevisionIntegrity {
                reason: "dependency lock does not match the recorded lock digest".to_owned(),
            });
        }
        let recomputed_state_digest = self.system_state.content_digest()?;
        if recomputed_state_digest != self.system_state_digest {
            return Err(PackContractError::RevisionIntegrity {
                reason: "system state content does not match the recorded state digest".to_owned(),
            });
        }
        let identity = PackRevisionIdentity {
            pack: self.pack_id.clone(),
            semantic_version: self.semantic_version.clone(),
            system_state_digest: recomputed_state_digest,
            mission_digest: recomputed_mission_digest,
            policy_digest: recomputed_policy_digest,
            dependency_lock_digest: recomputed_lock_digest,
            parent_revision: self.parent.as_ref().map(|parent| parent.revision.clone()),
        };
        let recomputed_revision_id = candidate_revision_identity(&identity)?;
        if recomputed_revision_id != self.revision_id {
            return Err(PackContractError::RevisionIntegrity {
                reason: "revision id does not match the candidate identity tuple".to_owned(),
            });
        }
        Ok(())
    }
}

/// An immutable promoted pack revision.
///
/// Promoted revisions are created exclusively by
/// [`PromotedPackState::promote`] from a verified candidate. The record
/// carries the same content as its source candidate plus the identity of
/// that candidate, and derives a distinct revision identity of its own.
/// There is no mutation API: altering any identity-covered field changes the
/// recomputed digest, so `verify_integrity` rejects the tampered record.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PromotedPackState {
    /// The pack lineage this revision belongs to.
    pub pack_id: PackId,
    /// Semantic version of this promoted revision.
    pub semantic_version: SemanticVersion,
    /// The promoted revision this revision builds on, or `None` for a root
    /// revision.
    #[serde(
        rename = "parentRevision",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub parent: Option<ParentRevision>,
    /// The mission this promoted revision serves.
    pub mission: Mission,
    /// Digest of the mission content.
    pub mission_digest: ContentDigest,
    /// The governing policy set.
    pub policy_set: PackPolicySet,
    /// Digest of the governing policy content.
    pub policy_digest: ContentDigest,
    /// The resolved dependency lock.
    pub dependency_lock: PackDependencyLock,
    /// Digest of the dependency lock.
    pub dependency_lock_digest: ContentDigest,
    /// The promoted system-state content.
    pub system_state: PackSystemState,
    /// Digest of the system-state content.
    pub system_state_digest: ContentDigest,
    /// Provenance carried verbatim from the source candidate.
    pub provenance: PackProvenance,
    /// The candidate revision this promoted revision was created from.
    pub promoted_from: PackRevisionId,
    /// The computed promoted revision identity.
    pub revision_id: PackRevisionId,
}

impl PromotedPackState {
    /// Promotes a verified candidate into a new immutable promoted revision.
    ///
    /// Promotion is a record transformation with integrity rules, not a
    /// decision: this function performs no gating, stores nothing, and
    /// grants no authority. It verifies the candidate's integrity (a
    /// tampered candidate is rejected with
    /// [`crate::PackContractError::RevisionIntegrity`]), then derives the
    /// promoted record from the candidate without altering it. The promoted
    /// revision identity is always distinct from the candidate identity.
    pub fn promote(candidate: &CandidatePackState) -> Result<Self, PackContractError> {
        candidate.verify_integrity()?;
        let identity = candidate_content_identity(candidate)?;
        let revision_id = promoted_revision_identity(&identity, &candidate.revision_id)?;
        Ok(Self {
            pack_id: candidate.pack_id.clone(),
            semantic_version: candidate.semantic_version.clone(),
            parent: candidate.parent.clone(),
            mission: candidate.mission.clone(),
            mission_digest: candidate.mission_digest.clone(),
            policy_set: candidate.policy_set.clone(),
            policy_digest: candidate.policy_digest.clone(),
            dependency_lock: candidate.dependency_lock.clone(),
            dependency_lock_digest: candidate.dependency_lock_digest.clone(),
            system_state: candidate.system_state.clone(),
            system_state_digest: candidate.system_state_digest.clone(),
            provenance: candidate.provenance.clone(),
            promoted_from: candidate.revision_id.clone(),
            revision_id,
        })
    }

    /// Verifies the record's integrity end to end.
    ///
    /// A promoted revision passes only when:
    ///
    /// - the system state is in canonical form;
    /// - the parent revision record and the provenance parent agree;
    /// - the embedded system state digests to the recorded state digest;
    /// - the recorded `promoted_from` equals the candidate identity
    ///   derivable from this record's own content;
    /// - the recorded revision id equals the digest of the anchored promoted
    ///   identity tuple.
    pub fn verify_integrity(&self) -> Result<(), PackContractError> {
        self.system_state.validate()?;
        self.policy_set.validate()?;
        ensure_workflow_references_are_locked(&self.system_state, &self.dependency_lock)?;
        ensure_capability_references_are_locked(&self.system_state, &self.dependency_lock)?;
        ensure_lineage_consistency(self.parent.as_ref(), &self.provenance)?;
        let recomputed_mission_digest = self.mission.digest()?;
        if recomputed_mission_digest != self.mission_digest {
            return Err(PackContractError::RevisionIntegrity {
                reason: "mission content does not match the recorded mission digest".to_owned(),
            });
        }
        let recomputed_policy_digest = self.policy_set.digest()?;
        if recomputed_policy_digest != self.policy_digest {
            return Err(PackContractError::RevisionIntegrity {
                reason: "policy content does not match the recorded policy digest".to_owned(),
            });
        }
        let recomputed_lock_digest = self.dependency_lock.digest()?;
        if recomputed_lock_digest != self.dependency_lock_digest {
            return Err(PackContractError::RevisionIntegrity {
                reason: "dependency lock does not match the recorded lock digest".to_owned(),
            });
        }
        let recomputed_state_digest = self.system_state.content_digest()?;
        if recomputed_state_digest != self.system_state_digest {
            return Err(PackContractError::RevisionIntegrity {
                reason: "system state content does not match the recorded state digest".to_owned(),
            });
        }
        let identity = PackRevisionIdentity {
            pack: self.pack_id.clone(),
            semantic_version: self.semantic_version.clone(),
            system_state_digest: recomputed_state_digest,
            mission_digest: recomputed_mission_digest,
            policy_digest: recomputed_policy_digest,
            dependency_lock_digest: recomputed_lock_digest,
            parent_revision: self.parent.as_ref().map(|parent| parent.revision.clone()),
        };
        let recomputed_source_candidate = candidate_revision_identity(&identity)?;
        if recomputed_source_candidate != self.promoted_from {
            return Err(PackContractError::RevisionIntegrity {
                reason:
                    "promoted-from does not match the candidate identity derivable from this record"
                        .to_owned(),
            });
        }
        let recomputed_revision_id = promoted_revision_identity(&identity, &self.promoted_from)?;
        if recomputed_revision_id != self.revision_id {
            return Err(PackContractError::RevisionIntegrity {
                reason: "revision id does not match the promoted identity tuple".to_owned(),
            });
        }
        Ok(())
    }
}

/// Derives the full revision identity tuple carried by a candidate record,
/// recomputing every component digest from the record's own content.
fn candidate_content_identity(
    candidate: &CandidatePackState,
) -> Result<PackRevisionIdentity, PackContractError> {
    Ok(PackRevisionIdentity {
        pack: candidate.pack_id.clone(),
        semantic_version: candidate.semantic_version.clone(),
        system_state_digest: candidate.system_state_digest.clone(),
        mission_digest: candidate.mission_digest.clone(),
        policy_digest: candidate.policy_digest.clone(),
        dependency_lock_digest: candidate.dependency_lock_digest.clone(),
        parent_revision: candidate
            .parent
            .as_ref()
            .map(|parent| parent.revision.clone()),
    })
}
