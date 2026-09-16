//! Candidate and promoted pack revision records.
//!
//! A [`CandidatePackState`] is a proposed system state: pack identity,
//! system-state content, provenance, and a computed revision identity. A
//! [`PromotedPackState`] is the immutable revision created by promoting a
//! candidate. The two are structurally distinct types, so lifecycle position
//! cannot be misreported, and promotion never mutates the historical
//! candidate record.
//!
//! Revision identities are derived from anchored identity tuples that
//! deliberately exclude descriptive metadata (roles, labels, constraint
//! text) and provenance producers, mirroring how published workflow versions
//! derive identity: identical content always yields the identical revision
//! identity, regardless of who proposed it. The record kind and — for
//! promoted revisions — the source candidate identity participate in the
//! tuple, so a promoted revision identity is always distinct from the
//! candidate identity it was promoted from.
//!
//! Promotion here is a contract operation (record transformation plus
//! integrity rules), not a controller: no storage, no gates, no automation.
//! The promotion decision authority lives in control planes.

use codex_workflow_contracts::ContentDigest;
use serde::Deserialize;
use serde::Serialize;

use super::digest_of;
use super::state::PackSystemState;
use super::validate_descriptive_text;
use crate::PackContractError;
use crate::PackId;
use crate::PackProvenance;
use crate::PackRevisionId;

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

/// The identity-covered projection of a candidate revision: pack lineage,
/// parent revision identity, and system-state content digest. Descriptive
/// metadata (labels, roles, constraints) and the provenance producer are
/// excluded so identical proposed content always yields the identical
/// candidate identity.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AnchoredCandidateIdentity<'a> {
    record_kind: RevisionRecordKind,
    pack_id: &'a PackId,
    parent_revision: Option<&'a PackRevisionId>,
    system_state_digest: &'a ContentDigest,
}

/// The identity-covered projection of a promoted revision: the candidate
/// tuple plus the identity of the candidate record that was promoted.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AnchoredPromotedIdentity<'a> {
    record_kind: RevisionRecordKind,
    pack_id: &'a PackId,
    parent_revision: Option<&'a PackRevisionId>,
    system_state_digest: &'a ContentDigest,
    promoted_from: &'a PackRevisionId,
}

/// Computes the candidate revision identity for an anchored tuple.
fn candidate_revision_identity(
    pack_id: &PackId,
    parent_revision: Option<&PackRevisionId>,
    system_state_digest: &ContentDigest,
) -> Result<PackRevisionId, PackContractError> {
    let anchored = AnchoredCandidateIdentity {
        record_kind: RevisionRecordKind::Candidate,
        pack_id,
        parent_revision,
        system_state_digest,
    };
    Ok(PackRevisionId::from_digest(digest_of(&anchored)?))
}

/// Computes the promoted revision identity for an anchored tuple.
fn promoted_revision_identity(
    pack_id: &PackId,
    parent_revision: Option<&PackRevisionId>,
    system_state_digest: &ContentDigest,
    promoted_from: &PackRevisionId,
) -> Result<PackRevisionId, PackContractError> {
    let anchored = AnchoredPromotedIdentity {
        record_kind: RevisionRecordKind::Promoted,
        pack_id,
        parent_revision,
        system_state_digest,
        promoted_from,
    };
    Ok(PackRevisionId::from_digest(digest_of(&anchored)?))
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
    /// The promoted revision this candidate builds on, or `None` for a root
    /// candidate.
    #[serde(
        rename = "parentRevision",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub parent: Option<ParentRevision>,
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
    /// Proposes a root candidate: a first system state with no parent.
    ///
    /// The provenance must describe a root revision (no parent).
    pub fn propose_root(
        pack_id: PackId,
        system_state: PackSystemState,
        provenance: PackProvenance,
    ) -> Result<Self, PackContractError> {
        Self::construct(pack_id, None, system_state, provenance)
    }

    /// Proposes a candidate building on a parent promoted revision.
    ///
    /// The provenance must name the identical parent revision, so lineage is
    /// unambiguous.
    pub fn propose(
        pack_id: PackId,
        parent: ParentRevision,
        system_state: PackSystemState,
        provenance: PackProvenance,
    ) -> Result<Self, PackContractError> {
        Self::construct(pack_id, Some(parent), system_state, provenance)
    }

    /// Assembles a candidate record, deriving its digests and identity.
    fn construct(
        pack_id: PackId,
        parent: Option<ParentRevision>,
        system_state: PackSystemState,
        provenance: PackProvenance,
    ) -> Result<Self, PackContractError> {
        ensure_lineage_consistency(parent.as_ref(), &provenance)?;
        let system_state_digest = system_state.content_digest()?;
        let revision_id = candidate_revision_identity(
            &pack_id,
            parent.as_ref().map(|parent| &parent.revision),
            &system_state_digest,
        )?;
        Ok(Self {
            pack_id,
            parent,
            system_state,
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
        ensure_lineage_consistency(self.parent.as_ref(), &self.provenance)?;
        let recomputed_state_digest = self.system_state.content_digest()?;
        if recomputed_state_digest != self.system_state_digest {
            return Err(PackContractError::RevisionIntegrity {
                reason: "system state content does not match the recorded state digest".to_owned(),
            });
        }
        let recomputed_revision_id = candidate_revision_identity(
            &self.pack_id,
            self.parent.as_ref().map(|parent| &parent.revision),
            &recomputed_state_digest,
        )?;
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
    /// The promoted revision this revision builds on, or `None` for a root
    /// revision.
    #[serde(
        rename = "parentRevision",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub parent: Option<ParentRevision>,
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
        let revision_id = promoted_revision_identity(
            &candidate.pack_id,
            candidate.parent.as_ref().map(|parent| &parent.revision),
            &candidate.system_state_digest,
            &candidate.revision_id,
        )?;
        Ok(Self {
            pack_id: candidate.pack_id.clone(),
            parent: candidate.parent.clone(),
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
        ensure_lineage_consistency(self.parent.as_ref(), &self.provenance)?;
        let recomputed_state_digest = self.system_state.content_digest()?;
        if recomputed_state_digest != self.system_state_digest {
            return Err(PackContractError::RevisionIntegrity {
                reason: "system state content does not match the recorded state digest".to_owned(),
            });
        }
        let parent_revision = self.parent.as_ref().map(|parent| &parent.revision);
        let recomputed_source_candidate =
            candidate_revision_identity(&self.pack_id, parent_revision, &recomputed_state_digest)?;
        if recomputed_source_candidate != self.promoted_from {
            return Err(PackContractError::RevisionIntegrity {
                reason:
                    "promoted-from does not match the candidate identity derivable from this record"
                        .to_owned(),
            });
        }
        let recomputed_revision_id = promoted_revision_identity(
            &self.pack_id,
            parent_revision,
            &recomputed_state_digest,
            &self.promoted_from,
        )?;
        if recomputed_revision_id != self.revision_id {
            return Err(PackContractError::RevisionIntegrity {
                reason: "revision id does not match the promoted identity tuple".to_owned(),
            });
        }
        Ok(())
    }
}
