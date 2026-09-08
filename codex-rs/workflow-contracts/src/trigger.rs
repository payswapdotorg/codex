//! Workflow triggers.
//!
//! Triggers are the normalized classes through which workflow execution may
//! begin or advance, per the frozen architecture: `USER`, `SCHEDULE`,
//! `WEBHOOK`, `CONNECTOR_EVENT`, `BROWSER_EVENT`, `COMPUTER_EVENT`,
//! `WORKFLOW_EVENT`, and `HUMAN_EVENT`.
//!
//! Triggers are declarations of *what may start work*, not authorization:
//! no external event mutates workflow state directly. External trigger
//! payloads are untrusted input that the control plane (a later Work Order)
//! authenticates, deduplicates, and evaluates against policy before any
//! durable transition.

use serde::Deserialize;
use serde::Serialize;

/// Normalized trigger class.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TriggerClass {
    /// Direct user action.
    User,
    /// Time-based schedule.
    Schedule,
    /// Inbound webhook.
    Webhook,
    /// Event raised by a connector.
    ConnectorEvent,
    /// Event raised in a browser session.
    BrowserEvent,
    /// Event raised on a computer/desktop session.
    ComputerEvent,
    /// Event raised by another workflow.
    WorkflowEvent,
    /// Event raised by a human participant.
    HumanEvent,
}

/// A trigger declaration on a workflow definition.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkflowTrigger {
    /// The class of event that may start or advance the workflow.
    pub trigger: TriggerClass,
    /// Human-facing description of what the trigger responds to.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// The trigger that started a workflow instance, when recorded.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TriggerSource {
    /// The normalized trigger class that fired.
    pub trigger: TriggerClass,
    /// Opaque identifier of the triggering event, allocated by the
    /// control plane when the event was accepted.
    ///
    /// Raw external event payloads are untrusted input and never enter
    /// workflow semantics; only this control-plane identifier is recorded.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event_id: Option<String>,
}

impl TriggerClass {
    /// All trigger classes defined by the frozen architecture.
    pub const ALL: [TriggerClass; 8] = [
        TriggerClass::User,
        TriggerClass::Schedule,
        TriggerClass::Webhook,
        TriggerClass::ConnectorEvent,
        TriggerClass::BrowserEvent,
        TriggerClass::ComputerEvent,
        TriggerClass::WorkflowEvent,
        TriggerClass::HumanEvent,
    ];
}

#[cfg(test)]
#[path = "trigger_tests.rs"]
mod tests;
