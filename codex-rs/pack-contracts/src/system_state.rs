//! Pack system-state contracts.
//!
//! Owned by the PACK-002 work order (Pack System State).
//!
//! [`PackSystemState`] is the durable representation of the currently
//! proposed or promoted software system. Candidate state and promoted state
//! are structurally distinct types — [`CandidatePackState`] versus
//! [`PromotedPackState`] — so lifecycle position is a type-level fact, not a
//! boolean on a shared record. Promotion creates a new immutable promoted
//! revision and never mutates the historical candidate.
//!
//! ## Authority model
//!
//! A pack references immutable
//! [`codex_workflow_contracts::WorkflowVersionId`] identities — never moving
//! branches or mutable workflow state — and reuses the Universal
//! [`codex_workflow_contracts::CapabilityId`] and
//! [`codex_workflow_contracts::ContentDigest`] machinery instead of
//! duplicating identity semantics. Workflow transitions remain owned by the
//! Workflow Control Plane: nothing in this module performs, models, or
//! bypasses workflow transitions, and there is no promotion controller, no
//! rollback engine, and no pack composition here — those belong to later
//! Pack work orders. Promotion as modeled here is a contract operation
//! (record transformation plus integrity rules); the promotion *decision*
//! remains a control-plane authority.
//!
//! ## Revision identity scope
//!
//! The revision identities computed here are system-state-scoped: they cover
//! the pack lineage identity, the parent revision, the system-state content
//! digest, and (for promoted revisions) the source candidate identity. The
//! full [`crate::PackRevisionId`] composition — including mission, policy,
//! and dependency components — is owned by the PACK-001 work order and will
//! compose with the system-state digest pinned by these records.
//!
//! ## Module map
//!
//! ```text
//! refs      ── workflow version / capability / policy / evaluation references
//! state     ── PackSystemState content snapshot + rollback checkpoint record
//! revisions ── ParentRevision, CandidatePackState, PromotedPackState
//! ```

mod refs;
mod revisions;
mod state;

pub use refs::CapabilityRef;
pub use refs::EvaluationRef;
pub use refs::PolicyRef;
pub use refs::WorkflowVersionRef;
pub use revisions::CandidatePackState;
pub use revisions::ParentRevision;
pub use revisions::PromotedPackState;
pub use state::PackSystemState;
pub use state::RollbackCheckpoint;

use codex_workflow_contracts::ContentDigest;
use codex_workflow_contracts::WorkflowContractError;
use serde::Serialize;

use crate::PackContractError;

/// Maximum length accepted for descriptive text fields (roles, constraints,
/// labels, reasons).
///
/// The bound keeps descriptive metadata small and reviewable; the rejection
/// message in [`validate_descriptive_text`] states the same limit.
const DESCRIPTIVE_TEXT_MAX_LENGTH: usize = 512;

/// Digests the canonical JSON serialization of `value`.
///
/// This delegates to [`ContentDigest::of`] so pack records share the
/// Universal canonicalization and digest semantics instead of defining a
/// second addressing scheme.
fn digest_of<T>(value: &T) -> Result<ContentDigest, PackContractError>
where
    T: Serialize,
{
    ContentDigest::of(value).map_err(|error| match error {
        WorkflowContractError::Serialization { source } => {
            PackContractError::Serialization { source }
        }
        WorkflowContractError::InvalidIdentifier { .. }
        | WorkflowContractError::InvalidDigest { .. }
        | WorkflowContractError::VersionIntegrity { .. }
        | WorkflowContractError::IncompleteDependencyLock { .. }
        | WorkflowContractError::MalformedIr { .. }
        | WorkflowContractError::InvalidRepositoryPath { .. }
        | WorkflowContractError::ForgeLookup { .. } => PackContractError::InvalidDigest {
            reason: error.to_string(),
        },
    })
}

/// Validates a non-empty, edge-whitespace-free, length-bounded descriptive
/// text field.
///
/// Descriptive text never participates in identity: it documents a record for
/// human reviewers. It is still validated so a pack record cannot carry blank
/// or unboundedly large annotations.
fn validate_descriptive_text(
    kind: &'static str,
    value: String,
) -> Result<String, PackContractError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        Err(PackContractError::InvalidIdentifier {
            kind,
            value,
            reason: "must be non-empty",
        })
    } else if trimmed.len() != value.len() {
        Err(PackContractError::InvalidIdentifier {
            kind,
            value,
            reason: "must not start or end with whitespace",
        })
    } else if value.len() > DESCRIPTIVE_TEXT_MAX_LENGTH {
        Err(PackContractError::InvalidIdentifier {
            kind,
            value,
            reason: "must not exceed 512 characters",
        })
    } else {
        Ok(value)
    }
}

#[cfg(test)]
#[path = "system_state_tests.rs"]
mod tests;
