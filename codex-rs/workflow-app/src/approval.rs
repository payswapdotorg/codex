//! Approval records crossing the approval plane seam.
//!
//! Codex's native approval machinery stays the authority: this record is
//! only the decision *as data*, digested by the evidence plane and
//! referenced from [`codex_execution_contracts::AuthorizationGrant`]. The
//! approver label is attribution only; credentials never appear here.

use codex_execution_contracts::CapabilityBindingId;
use codex_execution_contracts::ExecutionEnvironment;
use codex_workflow_contracts::CapabilityId;
use codex_workflow_contracts::WorkflowInstanceId;
use serde::Deserialize;
use serde::Serialize;

/// The recorded verdict of one approval decision.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ApprovalVerdict {
    /// The approval plane authorized the binding.
    Approved,
    /// The approval plane refused the binding.
    Denied,
}

/// A request to authorize one capability binding for one instance.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ApprovalRequest {
    /// The instance the authorization belongs to.
    pub instance: WorkflowInstanceId,
    /// The binding to authorize.
    pub binding: CapabilityBindingId,
    /// The environment class the binding routes to.
    pub environment: ExecutionEnvironment,
    /// The capability the binding provides.
    pub capability: CapabilityId,
}

/// The recorded approval decision for one binding.
///
/// The evidence plane digests this record and returns an
/// [`codex_workflow_contracts::EvidenceReference`] of kind `Approval`,
/// which is the only form an [`AuthorizationGrant`] accepts.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ApprovalEvidence {
    /// Attribution label of the approver (never a credential).
    pub approver: String,
    /// The binding that was approved.
    pub binding: CapabilityBindingId,
    /// The environment class the binding routes to.
    pub environment: ExecutionEnvironment,
    /// The capability the binding provides.
    pub capability: CapabilityId,
    /// The recorded verdict.
    pub verdict: ApprovalVerdict,
    /// The workflow instance the approval was recorded for.
    pub instance: WorkflowInstanceId,
}

impl ApprovalEvidence {
    /// Builds the evidence record for an approved request.
    pub fn approved(request: &ApprovalRequest, approver: impl Into<String>) -> Self {
        Self {
            approver: approver.into(),
            binding: request.binding.clone(),
            environment: request.environment,
            capability: request.capability.clone(),
            verdict: ApprovalVerdict::Approved,
            instance: request.instance,
        }
    }
}
