//! The mount's approval-plane source over the durable evidence store.
//!
//! Bindings reach `AUTHORIZED` only through an
//! [`AuthorizationGrant`](codex_execution_contracts::AuthorizationGrant)
//! resting on recorded approval evidence. The engine's in-memory double
//! (`InMemoryApprovalSource`) records that evidence in an in-memory
//! store; this source records it in the shared durable evidence store so
//! approval evidence survives kill/restart with the instance it belongs
//! to. The verdict recorded is always `Approved` for the bindings the
//! mount authorizes, mirroring the engine double's approving behavior.

use codex_workflow_app::ApprovalEvidence;
use codex_workflow_app::ApprovalRequest;
use codex_workflow_app::ApprovalSource;
use codex_workflow_app::ApprovalVerdict;
use codex_workflow_app::WorkflowAppError;
use codex_workflow_contracts::EvidenceKind;
use codex_workflow_contracts::EvidenceReference;
use codex_workflow_durable::DurableEvidenceStore;

/// An approval source that records approval evidence durably.
#[derive(Clone, Debug)]
pub struct DurableBackedApprovalSource {
    evidence: DurableEvidenceStore,
    approver: String,
}

impl DurableBackedApprovalSource {
    /// Creates a source that approves every request as `approver`,
    /// recording the decision through the shared durable evidence store.
    pub fn new(evidence: DurableEvidenceStore, approver: impl Into<String>) -> Self {
        Self {
            evidence,
            approver: approver.into(),
        }
    }
}

impl ApprovalSource for DurableBackedApprovalSource {
    fn approve(
        &mut self,
        request: &ApprovalRequest,
    ) -> Result<EvidenceReference, WorkflowAppError> {
        let record = ApprovalEvidence {
            approver: self.approver.clone(),
            binding: request.binding.clone(),
            environment: request.environment,
            capability: request.capability.clone(),
            verdict: ApprovalVerdict::Approved,
            instance: request.instance,
        };
        let payload = serde_json::to_value(&record)?;
        self.evidence.store(EvidenceKind::Approval, &payload)
    }
}
