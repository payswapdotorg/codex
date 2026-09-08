//! Diagnostics for binding readiness and resolution failures.
//!
//! The frozen architecture requires capability and environment availability
//! to be *diagnosable*: an unavailable required capability must fail with a
//! structured explanation instead of a silent substitution. Diagnostics are
//! the carriers of those explanations. They are records, not control flow:
//! they explain why a binding is not selectable, why a transition moved a
//! binding into [`crate::ReadinessState::Failed`] or
//! [`crate::ReadinessState::Unavailable`], or why resolution rejected a
//! candidate.
//!
//! Diagnostic messages are derived from untrusted external output (adapter
//! probes, environment errors) by default. They are bounded: messages over
//! [`BindingDiagnostic::MAX_MESSAGE_BYTES`] bytes are truncated at a UTF-8
//! character boundary so diagnostics stay safe to log, store, and digest.

use serde::Deserialize;
use serde::Serialize;

use crate::ExecutionEnvironment;

/// Structured classification of a binding diagnostic.
///
/// Codes are the diagnosable taxonomy: every failed or unavailable binding
/// carries one, so tooling can aggregate and explain failures without
/// parsing free text.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DiagnosticCode {
    /// No adapter provides the capability at all.
    NotRegistered,
    /// The capability's implementation is not installed or discoverable.
    NotInstalled,
    /// The adapter probe failed.
    ProbeFailed,
    /// Binding policy rejected the candidate environment or class.
    PolicyDenied,
    /// Authorization (approval) was refused or is missing.
    ApprovalDenied,
    /// A required resource type has no bound resource.
    ResourceMissing,
    /// An action failed during execution.
    ExecutionFailure,
    /// The environment or session was lost mid-flight.
    EnvironmentLost,
    /// The environment is reserved for a future Work Order.
    ReservedEnvironment,
}

/// A structured explanation of a binding failure or unavailability.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BindingDiagnostic {
    /// Structured classification of the diagnostic.
    pub code: DiagnosticCode,
    /// Human-facing explanation, bounded and untrusted-derived.
    pub message: String,
    /// The environment the diagnostic is about, when known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub environment: Option<ExecutionEnvironment>,
}

impl BindingDiagnostic {
    /// Maximum message length in bytes before truncation.
    pub const MAX_MESSAGE_BYTES: usize = 4096;

    /// Creates a diagnostic, truncating an over-long message at a UTF-8
    /// character boundary.
    ///
    /// Truncation keeps diagnostics bounded even when the underlying
    /// adapter or environment produces very long untrusted output.
    pub fn new(code: DiagnosticCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: clamp_message(message.into()),
            environment: None,
        }
    }

    /// Attaches the environment the diagnostic is about.
    pub fn in_environment(mut self, environment: ExecutionEnvironment) -> Self {
        self.environment = Some(environment);
        self
    }
}

/// Bounds a diagnostic message, truncating at a UTF-8 character boundary
/// and marking the truncation.
fn clamp_message(message: String) -> String {
    if message.len() <= BindingDiagnostic::MAX_MESSAGE_BYTES {
        return message;
    }
    let mut end = BindingDiagnostic::MAX_MESSAGE_BYTES;
    while !message.is_char_boundary(end) {
        end -= 1;
    }
    let mut truncated = message;
    truncated.truncate(end);
    truncated.push_str("...[truncated]");
    truncated
}

#[cfg(test)]
#[path = "diagnostic_tests.rs"]
mod tests;
