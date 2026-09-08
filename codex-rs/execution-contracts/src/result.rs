//! Normalized action results.
//!
//! An [`ActionResult`] is the outcome of one dispatched action: the
//! binding that executed it, whether it succeeded, the normalized failure
//! when it did not, the observations captured during execution, and any
//! bounded output payload.
//!
//! Results are the execution plane's normalized currency for *what
//! happened*, distinct from [`crate::Observation`] (what the environment
//! looked like). Result outputs and failure messages are untrusted input
//! by default: they are bounded, and downstream consumers (models,
//! workflow control) must treat them as unverified external output.
//!
//! Failure classification (via [`FailureKind`]) is the input to recovery:
//! the frozen architecture's failure pipeline is
//! classify -> recover/retry/takeover/rebind/replan -> verify -> continue
//! or escalate, and [`crate::Recovery`] consumes these classifications.

use serde::Deserialize;
use serde::Serialize;

use crate::CapabilityBindingId;
use crate::ExecutionContractError;
use crate::Observation;

/// Maximum serialized size of action outputs, in bytes.
pub const ACTION_OUTPUTS_MAX_BYTES: usize = 65_536;

/// Maximum serialized size of a failure message, in bytes.
pub const FAILURE_MESSAGE_MAX_BYTES: usize = 4096;

/// Whether an action succeeded or failed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ActionOutcome {
    /// The action completed successfully.
    Succeeded,
    /// The action failed; a failure classification is present.
    Failed,
}

/// Normalized classification of an execution failure.
///
/// The classification drives recovery strategy selection: transient
/// failures retry, timeouts retry with bounds, policy denials escalate,
/// unavailable bindings rebind, and permanent failures rebind, escalate,
/// or take over.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FailureKind {
    /// A transient failure; retrying the same action may succeed.
    Transient,
    /// A permanent failure; the same action on the same binding will not
    /// succeed.
    Permanent,
    /// The action exceeded its deadline.
    Timeout,
    /// Policy or approval refused the action.
    PolicyDenied,
    /// The binding lost its environment mid-flight.
    Unavailable,
}

/// A normalized execution failure with a bounded, untrusted-derived
/// message.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExecutionFailure {
    /// The normalized classification.
    pub kind: FailureKind,
    /// Bounded human-facing failure detail, derived from untrusted output.
    pub message: String,
}

impl ExecutionFailure {
    /// Creates a failure, validating that the message is non-empty and
    /// bounded.
    pub fn new(
        kind: FailureKind,
        message: impl Into<String>,
    ) -> Result<Self, ExecutionContractError> {
        let message = message.into();
        if message.is_empty() {
            return Err(ExecutionContractError::InvalidRecord {
                reason: "failure message must be non-empty".to_owned(),
            });
        }
        if message.len() > FAILURE_MESSAGE_MAX_BYTES {
            return Err(ExecutionContractError::PayloadTooLarge {
                field: "failure message",
                limit: FAILURE_MESSAGE_MAX_BYTES,
            });
        }
        Ok(Self { kind, message })
    }
}

/// The outcome of one dispatched action.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ActionResult {
    /// The binding that executed the action.
    pub binding: CapabilityBindingId,
    /// Whether the action succeeded.
    pub outcome: ActionOutcome,
    /// The normalized failure, present exactly when the action failed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failure: Option<ExecutionFailure>,
    /// Observations captured during execution.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub observations: Vec<Observation>,
    /// Bounded normalized output payload, untrusted input by default.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outputs: Option<serde_json::Value>,
    /// Wall-clock duration of the action in milliseconds, when measured.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
}

impl ActionResult {
    /// Creates a successful result with no observations.
    pub fn succeeded(binding: CapabilityBindingId) -> Self {
        Self {
            binding,
            outcome: ActionOutcome::Succeeded,
            failure: None,
            observations: Vec::new(),
            outputs: None,
            duration_ms: None,
        }
    }

    /// Creates a failed result from a normalized failure.
    pub fn failed(binding: CapabilityBindingId, failure: ExecutionFailure) -> Self {
        Self {
            binding,
            outcome: ActionOutcome::Failed,
            failure: Some(failure),
            observations: Vec::new(),
            outputs: None,
            duration_ms: None,
        }
    }

    /// Attaches observations captured during execution.
    pub fn with_observations(mut self, observations: Vec<Observation>) -> Self {
        self.observations = observations;
        self
    }

    /// Attaches a bounded output payload.
    pub fn with_outputs(mut self, outputs: serde_json::Value) -> Self {
        self.outputs = Some(outputs);
        self
    }

    /// Records the measured wall-clock duration in milliseconds.
    pub fn with_duration_ms(mut self, duration_ms: u64) -> Self {
        self.duration_ms = Some(duration_ms);
        self
    }

    /// Validates the result's consistency and bounds.
    ///
    /// - a failed result carries exactly one failure;
    /// - a successful result carries none;
    /// - the output payload is bounded;
    /// - every observation is a valid, bounded observation.
    pub fn validate(&self) -> Result<(), ExecutionContractError> {
        match (self.outcome, &self.failure) {
            (ActionOutcome::Failed, None) => {
                return Err(ExecutionContractError::InvalidRecord {
                    reason: format!("failed result `{}` has no failure", self.binding),
                });
            }
            (ActionOutcome::Succeeded, Some(_)) => {
                return Err(ExecutionContractError::InvalidRecord {
                    reason: format!("succeeded result `{}` carries a failure", self.binding),
                });
            }
            _ => {}
        }
        if let Some(outputs) = &self.outputs {
            let serialized = serde_json::to_vec(outputs)?;
            if serialized.len() > ACTION_OUTPUTS_MAX_BYTES {
                return Err(ExecutionContractError::PayloadTooLarge {
                    field: "action outputs",
                    limit: ACTION_OUTPUTS_MAX_BYTES,
                });
            }
        }
        for observation in &self.observations {
            observation.validate()?;
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "result_tests.rs"]
mod tests;
