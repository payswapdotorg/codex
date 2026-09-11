//! Workflow instances.
//!
//! A `WorkflowInstance` is the contract-level record of one execution of a
//! pinned, immutable workflow version. This crate defines the record shape
//! and the frozen legal-transition table the workflow control plane
//! declares for instance statuses; durable lifecycle, idempotent triggers,
//! and recovery remain control-plane authority (roadmap M4), which stays
//! the sole authority for instance state. Agents, models, and execution
//! adapters observe and propose; they cannot mutate instance semantics.

use serde::Deserialize;
use serde::Serialize;

use crate::EvidenceReference;
use crate::TriggerSource;
use crate::WorkflowDefinitionId;
use crate::WorkflowInstanceId;
use crate::WorkflowVersionId;

/// Lifecycle status of a workflow instance.
///
/// The enum lists the contract-level states, and
/// [`WorkflowInstanceStatus::legal_transitions`] declares the frozen
/// legal-transition table between them. The table is the control plane's
/// declared authority (the M4 remediation program, mirroring the per-cause
/// transition tables of the execution contracts' readiness lifecycle):
/// only the control plane transitions an instance, and only along declared
/// edges; every other pair is an illegal transition. Agents, models, and
/// execution adapters never mutate instance status.
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

impl WorkflowInstanceStatus {
    /// Every lifecycle status defined by the contract.
    pub const ALL: [Self; 6] = [
        Self::Pending,
        Self::Running,
        Self::Paused,
        Self::Succeeded,
        Self::Failed,
        Self::Cancelled,
    ];

    /// The statuses an instance in this status may legally transition to.
    ///
    /// This is the frozen legal-transition table declared by the workflow
    /// control plane:
    ///
    /// ```text
    /// Pending              -> Running | Failed | Cancelled
    /// Running              -> Paused | Succeeded | Failed | Cancelled
    /// Paused               -> Running | Failed | Cancelled
    /// Succeeded, Failed, Cancelled   terminal: no transitions
    /// ```
    ///
    /// `Pending -> Failed` is the documented instantiation-failure
    /// settlement: a validate/approve/bind gate failure settles the record
    /// `Failed` before any execution starts. Terminal statuses admit no
    /// transitions, so settled records are immutable.
    pub fn legal_transitions(self) -> &'static [Self] {
        match self {
            Self::Pending => &[Self::Running, Self::Failed, Self::Cancelled],
            Self::Running => &[Self::Paused, Self::Succeeded, Self::Failed, Self::Cancelled],
            Self::Paused => &[Self::Running, Self::Failed, Self::Cancelled],
            Self::Succeeded | Self::Failed | Self::Cancelled => &[],
        }
    }

    /// Whether the transition `self -> target` is legal per the declared
    /// table.
    pub fn can_transition_to(self, target: Self) -> bool {
        self.legal_transitions().contains(&target)
    }

    /// Whether the status is terminal: an instance in this status can
    /// never transition again.
    pub const fn is_terminal(self) -> bool {
        matches!(self, Self::Succeeded | Self::Failed | Self::Cancelled)
    }
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
