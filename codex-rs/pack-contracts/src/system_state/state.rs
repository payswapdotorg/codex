//! The pack system-state content snapshot.
//!
//! [`PackSystemState`] is the durable representation of the proposed or
//! promoted software system at the content level: which immutable workflow
//! versions it is built from, which capabilities it requires, which policies
//! govern it, which evaluations assessed it, and — for non-root revisions —
//! a [`RollbackCheckpoint`] representation naming the prior promoted state a
//! rollback would restore.
//!
//! The state is content-addressed: [`PackSystemState::content_digest`]
//! digests the canonical JSON serialization, so a
//! [`crate::PackRevisionId`] can pin an exact state and any covered change
//! produces a different digest. Constructors canonicalize reference vectors
//! (sorted by identity, duplicates rejected), so two states with the same
//! references always produce the same digest regardless of insertion order.
//!
//! Phase 1 scope: architecture graphs, experiments, resource bindings, UX
//! model references, and evolution history are deliberately absent and
//! belong to later work orders.

use codex_workflow_contracts::ContentDigest;
use serde::Deserialize;
use serde::Serialize;

use super::digest_of;
use super::refs::CapabilityRef;
use super::refs::EvaluationRef;
use super::refs::PolicyRef;
use super::refs::WorkflowVersionRef;
use super::validate_descriptive_text;
use crate::PackContractError;
use crate::PackRevisionId;

/// A recorded rollback point for a pack revision.
///
/// A checkpoint is a *representation*: the prior promoted revision identity,
/// the content digest of that revision's system state, and a human-readable
/// reason. It is descriptive data inside a system state — not a rollback
/// engine. Executing a rollback is a governed control-plane decision owned
/// by later work orders.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RollbackCheckpoint {
    /// The prior promoted revision a rollback would restore.
    pub target_revision: PackRevisionId,
    /// Content digest of the target revision's system state, pinning the
    /// exact state a rollback would restore.
    pub state_digest: ContentDigest,
    /// Human-readable reason the rollback point was recorded.
    pub reason: String,
}

impl RollbackCheckpoint {
    /// Records a rollback point targeting a prior promoted revision.
    ///
    /// The reason is validated descriptive text; the target revision and
    /// state digest are recorded verbatim so the exact restore point is
    /// preserved.
    pub fn new(
        target_revision: PackRevisionId,
        state_digest: ContentDigest,
        reason: impl Into<String>,
    ) -> Result<Self, PackContractError> {
        Ok(Self {
            target_revision,
            state_digest,
            reason: validate_descriptive_text("rollback checkpoint reason", reason.into())?,
        })
    }
}

/// The durable representation of the proposed or promoted software system.
///
/// This is a model of the pack, not a replacement for Codex runtime state:
/// it references immutable workflow versions and policies by identity and
/// never redefines their semantics. The record is a plain immutable value —
/// no setters — and its content digest is what revision identities pin.
///
/// Duplicate references are rejected at construction: each workflow version,
/// capability, policy, and evaluation identity may appear at most once. A
/// state with two references to the same identity (even with different
/// descriptive roles or constraints) is ambiguous about what the system
/// includes, so it is rejected rather than silently deduplicated.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PackSystemState {
    /// Immutable workflow versions the system is built from, pinned by exact
    /// [`codex_workflow_contracts::WorkflowVersionId`].
    pub workflow_version_refs: Vec<WorkflowVersionRef>,
    /// Semantic capabilities the system requires, by Universal capability
    /// identity.
    pub capability_refs: Vec<CapabilityRef>,
    /// Governing and assurance policies, pinned by policy identity plus the
    /// content digest the state was validated against.
    pub policy_refs: Vec<PolicyRef>,
    /// Evaluation records the state was assessed by, as opaque
    /// content-addressed references.
    pub evaluation_refs: Vec<EvaluationRef>,
    /// Recorded rollback point naming the prior promoted state, present on
    /// revisions that supersede a promoted state.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rollback_checkpoint: Option<RollbackCheckpoint>,
}

impl PackSystemState {
    /// Assembles a system state from its references, canonicalizing them.
    ///
    /// Each reference vector is sorted by identity so that insertion order
    /// never affects the content digest, and duplicate identities are
    /// rejected with [`crate::PackContractError::InvalidIdentifier`].
    pub fn new(
        mut workflow_version_refs: Vec<WorkflowVersionRef>,
        mut capability_refs: Vec<CapabilityRef>,
        mut policy_refs: Vec<PolicyRef>,
        mut evaluation_refs: Vec<EvaluationRef>,
        rollback_checkpoint: Option<RollbackCheckpoint>,
    ) -> Result<Self, PackContractError> {
        workflow_version_refs.sort();
        capability_refs.sort();
        policy_refs.sort();
        evaluation_refs.sort();
        let state = Self {
            workflow_version_refs,
            capability_refs,
            policy_refs,
            evaluation_refs,
            rollback_checkpoint,
        };
        state.validate()?;
        Ok(state)
    }

    /// Computes the content digest of this state.
    ///
    /// The digest covers the canonical JSON serialization of the whole
    /// record, so equal content yields an equal digest and any change — a
    /// reference added, removed, or altered, a constraint edited, a
    /// checkpoint changed — yields a different digest.
    pub fn content_digest(&self) -> Result<ContentDigest, PackContractError> {
        digest_of(self)
    }

    /// Validates that the record is in the canonical form constructors
    /// produce.
    ///
    /// Reference vectors must be sorted by identity and free of duplicate
    /// identities. Records assembled by [`PackSystemState::new`] always
    /// satisfy this; a deserialized or otherwise crafted record that does
    /// not is rejected, which is what lets revision integrity verification
    /// catch forged duplicate references.
    pub fn validate(&self) -> Result<(), PackContractError> {
        ensure_canonical(
            &self.workflow_version_refs,
            "workflow version reference",
            |reference| reference.workflow_version_id.to_string(),
        )?;
        ensure_canonical(&self.capability_refs, "capability reference", |reference| {
            reference.capability.as_ref().to_owned()
        })?;
        ensure_canonical(&self.policy_refs, "policy reference", |reference| {
            reference.policy_id.to_string()
        })?;
        ensure_canonical(
            &self.evaluation_refs,
            "evaluation reference",
            ToString::to_string,
        )?;
        Ok(())
    }
}

/// Checks that a reference slice is sorted by full ordering and contains no
/// duplicate identities.
///
/// Duplicate detection compares identity keys (not full records) so two
/// references to the same identity with different descriptive text are still
/// rejected. Because every reference type orders by its identity field
/// first, duplicate identities are always adjacent in a sorted slice.
fn ensure_canonical<T>(
    references: &[T],
    kind: &'static str,
    identity_key: impl Fn(&T) -> String,
) -> Result<(), PackContractError>
where
    T: Ord,
{
    for index in 1..references.len() {
        let first = &references[index - 1];
        let second = &references[index];
        if first > second {
            return Err(PackContractError::RevisionIntegrity {
                reason: "references are not in canonical identity order".to_owned(),
            });
        }
        if identity_key(first) == identity_key(second) {
            return Err(PackContractError::InvalidIdentifier {
                kind,
                value: identity_key(first),
                reason: "duplicate reference; each referenced identity may appear at most once",
            });
        }
    }
    Ok(())
}
