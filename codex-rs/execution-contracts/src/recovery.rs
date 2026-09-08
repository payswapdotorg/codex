//! Recovery and takeover contracts.
//!
//! The frozen architecture's failure pipeline is:
//!
//! ```text
//! failure -> classify -> recover/retry/takeover/rebind/replan
//!         -> verify -> continue or escalate
//! ```
//!
//! This module defines the execution-plane half of that pipeline:
//!
//! - [`Recovery`] records one recovery attempt: the classified failure
//!   (from [`crate::ExecutionFailure`]), the strategy applied, and the
//!   verified outcome. Recovery records are evidence-bearing (kind
//!   [`EvidenceKind::Recovery`]).
//! - [`TakeoverRequest`]/[`TakeoverGrant`] model human takeover. The
//!   frozen invariant *human takeover cannot bypass authorization* is
//!   structural: a takeover grant requires an approval evidence
//!   reference, exactly like [`crate::AuthorizationGrant`].
//!
//! Replanning remains control-plane authority: an execution-plane
//! [`RecoveryStrategy::Escalate`] proposes escalation upward; it never
//! mutates workflow semantics.

use serde::Deserialize;
use serde::Serialize;

use crate::BindingDecision;
use crate::CapabilityBindingId;
use crate::ExecutionContractError;
use crate::ExecutionFailure;
use codex_workflow_contracts::ContentDigest;
use codex_workflow_contracts::EvidenceKind;
use codex_workflow_contracts::EvidenceReference;

/// The recovery strategy applied to a classified failure.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub enum RecoveryStrategy {
    /// Retry the same action on the same binding.
    Retry,
    /// Hand execution to a human under an approved takeover.
    Takeover(TakeoverGrant),
    /// Rebind the capability to a compatible alternate binding; the
    /// decision records the selected binding and the fallback reason.
    Rebind(BindingDecision),
    /// Escalate to the control plane (including replanning proposals).
    Escalate,
}

/// The verified outcome of one recovery attempt.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RecoveryOutcome {
    /// Execution continued successfully after recovery.
    Recovered,
    /// The attempt was escalated to the control plane.
    Escalated,
    /// The attempt did not recover execution.
    Failed,
}

/// One recorded recovery attempt for a binding.
///
/// Recovery records are append-only evidence: they state what failed, how
/// it was classified, which strategy was applied, and whether the
/// recovery was verified to work.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Recovery {
    /// The binding the failure occurred on.
    pub binding: CapabilityBindingId,
    /// The classified failure being recovered from.
    pub failure: ExecutionFailure,
    /// The strategy applied.
    pub strategy: RecoveryStrategy,
    /// The verified outcome of the attempt.
    pub outcome: RecoveryOutcome,
}

impl Recovery {
    /// Validates the recovery record's consistency.
    ///
    /// - the failure is well-formed;
    /// - a takeover strategy targets the same binding the recovery is
    ///   recorded for.
    pub fn validate(&self) -> Result<(), ExecutionContractError> {
        if self.failure.message.is_empty() || self.failure.message.len() > 4096 {
            return Err(ExecutionContractError::InvalidRecord {
                reason: format!(
                    "recovery for `{}` has an invalid failure message",
                    self.binding
                ),
            });
        }
        if let RecoveryStrategy::Takeover(grant) = &self.strategy
            && grant.request.binding != self.binding
        {
            return Err(ExecutionContractError::InvalidRecord {
                reason: format!(
                    "takeover grant targets `{}`, but recovery is recorded for `{}`",
                    grant.request.binding, self.binding
                ),
            });
        }
        if let RecoveryStrategy::Rebind(decision) = &self.strategy
            && decision.selected.binding == self.binding
        {
            return Err(ExecutionContractError::InvalidRecord {
                reason: format!(
                    "rebind strategy for `{}` selects the same binding it is recovering from",
                    self.binding
                ),
            });
        }
        Ok(())
    }

    /// Converts the recovery record into an evidence reference for the
    /// evidence plane.
    ///
    /// Recovery evidence carries the failure classification, the strategy,
    /// and the verified outcome, so failure handling stays auditable.
    pub fn to_evidence_reference(
        &self,
        locator: impl Into<String>,
    ) -> Result<EvidenceReference, ExecutionContractError> {
        let digest = ContentDigest::of(self)?;
        Ok(EvidenceReference {
            kind: EvidenceKind::Recovery,
            locator: crate::validate_locator(locator)?,
            digest,
        })
    }
}

/// Why a human takeover was requested.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TakeoverReason {
    /// Execution failed and the environment is best handled by a human.
    ExecutionFailure,
    /// A human gate requested human participation.
    HumanGate,
    /// An operator chose to intervene.
    OperatorIntervention,
}

/// A request for human takeover of a binding's execution.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TakeoverRequest {
    /// The binding whose execution the human would take over.
    pub binding: CapabilityBindingId,
    /// Why the takeover is requested.
    pub reason: TakeoverReason,
    /// The bounded instruction presented to the human.
    pub instruction: String,
}

impl TakeoverRequest {
    /// Maximum length of the takeover instruction, in bytes.
    pub const MAX_INSTRUCTION_BYTES: usize = 4096;

    /// Creates a takeover request, validating the instruction bounds.
    pub fn new(
        binding: CapabilityBindingId,
        reason: TakeoverReason,
        instruction: impl Into<String>,
    ) -> Result<Self, ExecutionContractError> {
        let instruction = instruction.into();
        if instruction.is_empty() || instruction.len() > Self::MAX_INSTRUCTION_BYTES {
            return Err(ExecutionContractError::PayloadTooLarge {
                field: "takeover instruction",
                limit: Self::MAX_INSTRUCTION_BYTES,
            });
        }
        Ok(Self {
            binding,
            reason,
            instruction,
        })
    }
}

/// An approved human takeover.
///
/// Human takeover cannot bypass authorization: the grant is only valid
/// when it rests on a recorded approval decision, referenced by evidence
/// identity and digest. Takeover grants control *participation* (a human
/// joins execution of the binding); they never grant authority beyond
/// what the approval already allowed.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TakeoverGrant {
    /// The takeover being granted.
    pub request: TakeoverRequest,
    /// The recorded approval decision authorizing the takeover.
    ///
    /// Must reference evidence of kind [`EvidenceKind::Approval`].
    pub approval: EvidenceReference,
}

impl TakeoverGrant {
    /// Creates a takeover grant resting on `approval`.
    ///
    /// Fails with [`crate::ExecutionContractError::ExpectedApprovalEvidence`]
    /// when the evidence reference is not an approval: takeover without
    /// authorization is a contract violation, not a runtime state.
    pub fn new(
        request: TakeoverRequest,
        approval: EvidenceReference,
    ) -> Result<Self, ExecutionContractError> {
        if approval.kind != EvidenceKind::Approval {
            return Err(ExecutionContractError::ExpectedApprovalEvidence {
                actual: approval.kind,
            });
        }
        Ok(Self { request, approval })
    }
}

#[cfg(test)]
#[path = "recovery_tests.rs"]
mod tests;
