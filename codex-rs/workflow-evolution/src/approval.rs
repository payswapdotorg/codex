//! The governed-approval seam.
//!
//! Nothing in this crate promotes a candidate without an explicit
//! approval decision recorded through an [`ApprovalPort`]. The port is the
//! single authorization path, and its shape enforces the Work Order's
//! invariants structurally:
//!
//! - decisions arrive only as explicit [`ApprovalDecision`] values carried
//!   by a host-supplied principal (a human or a policy engine identity);
//! - there is **no API path** through which model output, a suggestion, a
//!   run record, or the generator itself can mint an approval;
//! - approvals bind to one candidate and one validation digest
//!   ([`ApprovalRequest`]), so an approval never transfers to a different
//!   validation outcome;
//! - rejection is recorded too: audit covers both directions.
//!
//! The in-memory [`InMemoryApprovalPort`] is the reference
//! implementation for tests and hosts without a durable approval plane;
//! the Codex application replaces it with the native approval machinery
//! (Codex approvals stay the authority).

use std::collections::BTreeMap;

use codex_workflow_contracts::ContentDigest;
use codex_workflow_contracts::WorkflowDefinitionId;
use codex_workflow_contracts::WorkflowVersionId;
use serde::Deserialize;
use serde::Serialize;

use crate::CandidateId;
use crate::WorkflowEvolutionError;
use crate::scrub;

/// What is being approved: one candidate, its incumbent, and the exact
/// validation digest the approval covers.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ApprovalRequest {
    /// The candidate proposed for promotion.
    pub candidate: CandidateId,
    /// The workflow the candidate evolves.
    pub workflow: WorkflowDefinitionId,
    /// The incumbent version the candidate was validated against.
    pub incumbent: WorkflowVersionId,
    /// The digest of the validation report the approval covers.
    pub validation_digest: ContentDigest,
}

/// An explicit decision on an approval request.
///
/// The approver identity is supplied by the host's approval plane; this
/// crate never derives it from model output.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub enum ApprovalDecision {
    /// Approve the candidate under this principal.
    Approve {
        /// The approving principal (human or policy engine identity).
        approver: String,
        /// Optional review note (scrubbed before recording).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        note: Option<String>,
    },
    /// Reject the candidate under this principal.
    Reject {
        /// The rejecting principal.
        approver: String,
        /// Why the candidate was rejected.
        reason: String,
    },
}

impl ApprovalDecision {
    /// The principal that made the decision.
    pub fn approver(&self) -> &str {
        match self {
            Self::Approve { approver, .. } | Self::Reject { approver, .. } => approver,
        }
    }

    /// Whether the decision approves.
    pub fn approves(&self) -> bool {
        matches!(self, Self::Approve { .. })
    }
}

/// The recorded outcome of one approval decision: immutable history.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ApprovalRecord {
    /// The request that was decided.
    pub request: ApprovalRequest,
    /// The decision that was made.
    pub decision: ApprovalDecision,
}

impl ApprovalRecord {
    /// Whether this record approves the request.
    pub fn approves(&self) -> bool {
        self.decision.approves()
    }
}

/// The approval seam: the only path through which promotion is
/// authorized.
///
/// Implementations record decisions durably (the host's approval plane);
/// this crate reads them back through [`ApprovalPort::record_of`] when
/// publishing.
pub trait ApprovalPort: Send + Sync {
    /// Records one explicit decision on `request`.
    ///
    /// Note text is scrubbed before recording: credentials never enter
    /// approvals either.
    fn decide(
        &mut self,
        request: &ApprovalRequest,
        decision: ApprovalDecision,
    ) -> Result<ApprovalRecord, WorkflowEvolutionError>;

    /// The latest recorded decision for one candidate, when any.
    fn record_of(&self, candidate: &CandidateId) -> Option<ApprovalRecord>;
}

/// In-memory approval plane: records every decision per candidate.
///
/// The latest decision per candidate wins (a host may re-decide before
/// publication); every decision is kept only as the latest record, which
/// is the audit shape publication consumes.
#[derive(Clone, Debug, Default)]
pub struct InMemoryApprovalPort {
    records: BTreeMap<CandidateId, ApprovalRecord>,
}

impl InMemoryApprovalPort {
    /// Creates an empty approval port.
    pub fn new() -> Self {
        Self::default()
    }

    /// Number of recorded decisions.
    pub fn len(&self) -> usize {
        self.records.len()
    }

    /// Whether no decision was recorded.
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }
}

impl ApprovalPort for InMemoryApprovalPort {
    fn decide(
        &mut self,
        request: &ApprovalRequest,
        decision: ApprovalDecision,
    ) -> Result<ApprovalRecord, WorkflowEvolutionError> {
        let scrubbed = match &decision {
            ApprovalDecision::Approve { approver, note } => {
                scrub::require_clean_text(approver, "approver identity")?;
                if let Some(note) = note {
                    scrub::require_clean_text(note, "approval note")?;
                }
                ApprovalDecision::Approve {
                    approver: approver.clone(),
                    note: note.clone(),
                }
            }
            ApprovalDecision::Reject { approver, reason } => {
                scrub::require_clean_text(approver, "approver identity")?;
                scrub::require_clean_text(reason, "rejection reason")?;
                ApprovalDecision::Reject {
                    approver: approver.clone(),
                    reason: reason.clone(),
                }
            }
        };
        let record = ApprovalRecord {
            request: request.clone(),
            decision: scrubbed,
        };
        self.records
            .insert(request.candidate.clone(), record.clone());
        Ok(record)
    }

    fn record_of(&self, candidate: &CandidateId) -> Option<ApprovalRecord> {
        self.records.get(candidate).cloned()
    }
}
