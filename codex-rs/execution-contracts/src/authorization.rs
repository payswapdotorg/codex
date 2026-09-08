//! Authorization grants.
//!
//! The frozen invariant *capability availability does not imply permission
//! to execute it* is enforced structurally: a binding cannot reach the
//! `AUTHORIZED` readiness state without an [`AuthorizationGrant`], and a
//! grant is only valid when it references a recorded approval decision.
//!
//! Codex's existing approval machinery stays the authority: the
//! [`EvidenceReference`] carried by a grant is produced by the existing
//! approval flow (human review, execpolicy decisions, managed
//! requirements) and is referenced here by identity and digest, never
//! re-modeled. The execution plane records and checks the reference; it
//! does not decide approvals.

use serde::Deserialize;
use serde::Serialize;

use crate::CapabilityBindingId;
use crate::ExecutionContractError;
use codex_workflow_contracts::EvidenceKind;
use codex_workflow_contracts::EvidenceReference;

/// A recorded authorization for one capability binding.
///
/// The grant is the bridge between Codex approvals and binding readiness:
/// it moves a `READY` binding to `AUTHORIZED` and is retained in the
/// binding's transition history for audit.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AuthorizationGrant {
    /// The binding the grant authorizes.
    pub binding: CapabilityBindingId,
    /// The recorded approval decision the grant rests on.
    ///
    /// Must reference evidence of kind [`EvidenceKind::Approval`].
    pub approval: EvidenceReference,
}

impl AuthorizationGrant {
    /// Creates a grant for `binding` resting on `approval`.
    ///
    /// Fails with [`crate::ExecutionContractError::ExpectedApprovalEvidence`]
    /// when the evidence reference is not an approval, so a grant can
    /// never silently rest on unrelated evidence.
    pub fn new(
        binding: CapabilityBindingId,
        approval: EvidenceReference,
    ) -> Result<Self, ExecutionContractError> {
        if approval.kind != EvidenceKind::Approval {
            return Err(ExecutionContractError::ExpectedApprovalEvidence {
                actual: approval.kind,
            });
        }
        Ok(Self { binding, approval })
    }
}

#[cfg(test)]
#[path = "authorization_tests.rs"]
mod tests;
