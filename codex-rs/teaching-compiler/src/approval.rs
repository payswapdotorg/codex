//! Approval decisions gating publication.
//!
//! Approver identity is a neutral label: identity, authentication, and
//! audit are control-plane concerns. This crate records the decision and
//! binds it to exact candidate content via the content epoch, so an
//! approval can never silently cover different semantics than the ones
//! reviewed.

use serde::Deserialize;
use serde::Serialize;

use crate::error::TeachingCompilerError;

/// Maximum byte length of an approver label.
pub const MAX_APPROVER_BYTES: usize = 128;
/// Maximum byte length of an approval reference.
pub const MAX_REFERENCE_BYTES: usize = 256;

/// The decision conveyed by an approval record.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ApprovalDecisionKind {
    /// The reviewer approves publication of exactly this content.
    Approved,
    /// The reviewer rejects the candidate.
    Rejected,
}

/// An externally supplied approval decision.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ApprovalDecision {
    approver: String,
    reference: String,
    kind: ApprovalDecisionKind,
}

impl ApprovalDecision {
    /// Validates and constructs an approval decision.
    pub fn new(
        approver: impl Into<String>,
        reference: impl Into<String>,
        kind: ApprovalDecisionKind,
    ) -> Result<Self, TeachingCompilerError> {
        let approver = approver.into();
        let reference = reference.into();
        if approver.is_empty() || approver.len() > MAX_APPROVER_BYTES {
            return Err(TeachingCompilerError::InvalidApproval {
                reason: format!("approver label must be 1..={MAX_APPROVER_BYTES} bytes"),
            });
        }
        if reference.is_empty() || reference.len() > MAX_REFERENCE_BYTES {
            return Err(TeachingCompilerError::InvalidApproval {
                reason: format!("approval reference must be 1..={MAX_REFERENCE_BYTES} bytes"),
            });
        }
        Ok(Self {
            approver,
            reference,
            kind,
        })
    }

    /// The approver label.
    pub fn approver(&self) -> &str {
        &self.approver
    }

    /// The external review reference.
    pub fn reference(&self) -> &str {
        &self.reference
    }

    /// The decision.
    pub fn kind(&self) -> ApprovalDecisionKind {
        self.kind
    }

    pub(crate) fn into_record(self, epoch: u64) -> ApprovalRecord {
        ApprovalRecord {
            approver: self.approver,
            reference: self.reference,
            kind: self.kind,
            epoch,
        }
    }
}

/// The approval bound to exact candidate content (content epoch).
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ApprovalRecord {
    /// The approver label.
    pub approver: String,
    /// The external review reference.
    pub reference: String,
    /// The recorded decision.
    pub kind: ApprovalDecisionKind,
    /// The content epoch covered by this approval.
    pub epoch: u64,
}

#[cfg(test)]
mod tests {
    use super::ApprovalDecision;
    use super::ApprovalDecisionKind;
    use super::MAX_REFERENCE_BYTES;

    #[test]
    fn accepts_valid_decisions() {
        let decision =
            ApprovalDecision::new("tech-lead", "review-1", ApprovalDecisionKind::Approved)
                .expect("decision");
        assert_eq!(decision.approver(), "tech-lead");
        assert_eq!(decision.kind(), ApprovalDecisionKind::Approved);
    }

    #[test]
    fn rejects_bad_labels_and_references() {
        assert!(ApprovalDecision::new("", "review", ApprovalDecisionKind::Approved).is_err());
        assert!(
            ApprovalDecision::new(
                "tech-lead",
                "r".repeat(MAX_REFERENCE_BYTES + 1),
                ApprovalDecisionKind::Approved
            )
            .is_err()
        );
        assert!(
            ApprovalDecision::new("tech-lead", "review", ApprovalDecisionKind::Rejected).is_ok()
        );
    }
}
