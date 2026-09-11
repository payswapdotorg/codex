//! Workflow lifecycle events.
//!
//! Events are plain data records emitted through the [`EventSink`] port as
//! the lifecycle advances. They make the application surface observable
//! (the frozen architecture's observability requirement) without granting
//! any listener authority over lifecycle state: events are append-only
//! records, never control flow.

use codex_execution_contracts::CapabilityBindingId;
use codex_execution_contracts::ExecutionEnvironment;
use codex_execution_contracts::FailureKind;
use codex_workflow_contracts::IrNodeId;
use codex_workflow_contracts::ResourceTypeId;
use codex_workflow_contracts::WorkflowDefinitionId;
use codex_workflow_contracts::WorkflowInstanceId;
use codex_workflow_contracts::WorkflowInstanceStatus;
use codex_workflow_contracts::WorkflowVersionId;
use serde::Deserialize;
use serde::Serialize;

/// One observable workflow lifecycle event.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub enum WorkflowEvent {
    /// An immutable workflow version was selected and verified.
    VersionSelected {
        /// The selected version identity.
        version: WorkflowVersionId,
    },
    /// A workflow instance record was created (initially `Pending`).
    InstanceCreated {
        /// The new instance.
        instance: WorkflowInstanceId,
        /// The workflow being executed.
        workflow: WorkflowDefinitionId,
        /// The immutable version the instance pins.
        version: WorkflowVersionId,
        /// The instance's initial status.
        status: WorkflowInstanceStatus,
    },
    /// A binding passed the approval gate (`READY -> AUTHORIZED`).
    BindingAuthorized {
        /// The instance the authorization belongs to.
        instance: WorkflowInstanceId,
        /// The binding that was authorized.
        binding: CapabilityBindingId,
        /// The environment class the binding routes to.
        environment: ExecutionEnvironment,
    },
    /// A binding attached its required resources (`AUTHORIZED -> BOUND`).
    ResourcesBound {
        /// The instance the binding belongs to.
        instance: WorkflowInstanceId,
        /// The binding that became bound.
        binding: CapabilityBindingId,
        /// The resource types that were attached.
        resources: Vec<ResourceTypeId>,
    },
    /// The instance began executing (`Pending -> Running`).
    InstanceStarted {
        /// The instance that started.
        instance: WorkflowInstanceId,
    },
    /// A step node began executing.
    StepStarted {
        /// The instance executing the step.
        instance: WorkflowInstanceId,
        /// The step node being executed.
        node: IrNodeId,
    },
    /// A step node completed successfully.
    StepCompleted {
        /// The instance that executed the step.
        instance: WorkflowInstanceId,
        /// The step node that completed.
        node: IrNodeId,
    },
    /// An action failed during a step; recovery follows.
    ActionFailed {
        /// The instance the action belongs to.
        instance: WorkflowInstanceId,
        /// The step node the action was dispatched for.
        node: IrNodeId,
        /// The binding that failed.
        binding: CapabilityBindingId,
        /// The normalized failure classification.
        kind: FailureKind,
        /// Bounded failure detail (untrusted-derived).
        message: String,
    },
    /// A recovery attempt verified and execution continued.
    Recovered {
        /// The instance that recovered.
        instance: WorkflowInstanceId,
        /// The step node that recovered.
        node: IrNodeId,
        /// The binding that was recovered or rebound.
        binding: CapabilityBindingId,
        /// The strategy that was applied (`retry`, `rebind`, ...).
        strategy: String,
    },
    /// A failure was escalated to the control plane.
    Escalated {
        /// The instance that escalated.
        instance: WorkflowInstanceId,
        /// The step node that escalated.
        node: IrNodeId,
        /// The binding that failed.
        binding: CapabilityBindingId,
        /// Why the escalation happened.
        reason: String,
    },
    /// The run settled successfully.
    RunCompleted {
        /// The instance that completed.
        instance: WorkflowInstanceId,
    },
    /// The run paused at a wait or human-gate node.
    RunPaused {
        /// The instance that paused.
        instance: WorkflowInstanceId,
        /// The node the run paused at.
        node: IrNodeId,
        /// Why the run paused.
        reason: String,
    },
    /// The run (or instantiation) settled with failure.
    RunFailed {
        /// The instance that failed.
        instance: WorkflowInstanceId,
        /// Why the run failed.
        reason: String,
    },
    /// The instance was cancelled through the control plane
    /// (`Pending`, `Running`, or `Paused` -> `Cancelled`).
    InstanceCancelled {
        /// The instance that was cancelled.
        instance: WorkflowInstanceId,
        /// The operator-visible cancellation reason.
        reason: String,
    },
}
