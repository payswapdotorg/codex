//! Workflow instances.
//!
//! A `WorkflowInstance` is the contract-level record of one execution of a
//! pinned, immutable workflow version. This Work Order defines the shape
//! only: durable lifecycle, legal transitions, idempotent triggers, and
//! recovery are owned by the workflow control plane (roadmap M4), which
//! remains the sole authority for instance state. Agents, models, and
//! execution adapters observe and propose; they cannot mutate instance
//! semantics.

use serde::Deserialize;
use serde::Serialize;

use crate::EvidenceReference;
use crate::TriggerSource;
use crate::WorkflowDefinitionId;
use crate::WorkflowInstanceId;
use crate::WorkflowVersionId;

/// Lifecycle status of a workflow instance.
///
/// The enum lists the contract-level states only. Transition rules between
/// states are control-plane authority and are intentionally not encoded
/// here.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum WorkflowInstanceStatus {
    /// Allocated but not yet started.
    Pending,
    /// Executing.
    Running,
    /// Paused, for example at a human gate.
    Paused,
    /// Completed successfully.
    Succeeded,
    /// Completed with failure.
    Failed,
    /// Cancelled before completion.
    Cancelled,
}

/// One execution instance of a published workflow version.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkflowInstance {
    /// The instance's identity, allocated by the control plane.
    pub instance_id: WorkflowInstanceId,
    /// The workflow being executed.
    pub workflow: WorkflowDefinitionId,
    /// The immutable version the instance is pinned to.
    ///
    /// Pinning is what makes instance semantics durable: the version cannot
    /// change under a running instance, and branch movement can never
    /// redirect it.
    pub version: WorkflowVersionId,
    /// Contract-level lifecycle status.
    pub status: WorkflowInstanceStatus,
    /// The accepted trigger that started the instance, when recorded.
    ///
    /// External trigger payloads are untrusted input; only the normalized
    /// control-plane trigger source is recorded.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trigger: Option<TriggerSource>,
    /// References to evidence produced by this instance.
    ///
    /// The evidence plane owns the payloads; these references are
    /// append-only pointers with integrity digests.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub evidence: Vec<EvidenceReference>,
}

impl WorkflowInstance {
    /// Starts shaping a new instance contract for `workflow` at `version`.
    pub fn new(
        instance_id: WorkflowInstanceId,
        workflow: WorkflowDefinitionId,
        version: WorkflowVersionId,
        status: WorkflowInstanceStatus,
    ) -> Self {
        Self {
            instance_id,
            workflow,
            version,
            status,
            trigger: None,
            evidence: Vec::new(),
        }
    }

    /// Appends an evidence reference.
    ///
    /// Evidence is append-only: records are added as execution observes,
    /// tests, approves, and recovers, and are never rewritten.
    pub fn record_evidence(&mut self, reference: EvidenceReference) {
        self.evidence.push(reference);
    }
}

#[cfg(test)]
#[path = "instance_tests.rs"]
mod tests;
