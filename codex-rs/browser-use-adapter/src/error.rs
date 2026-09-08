//! Adapter error type.
//!
//! Errors carry actionable, credential-free context where applicable so
//! callers (and operators) can resolve provisioning or policy gaps without
//! inspecting adapter internals.

use crate::diagnostics::{AdapterDiagnostic, DiagnosticCode};
use crate::lifecycle::CapabilityLifecycleState;
use thiserror::Error;

/// Errors raised by the browser use workflow adapter.
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum AdapterError {
    /// The adapter has not resolved readiness; `prepare` must succeed first.
    #[error("browser use adapter is not prepared: readiness has not been resolved")]
    NotPrepared,
    /// The adapter has no binding; `bind` must succeed first.
    #[error("browser use adapter has no binding: bind must run before this operation")]
    NotBound,
    /// The native capability is unavailable (bridge missing or runtime not
    /// provisioned) and no compatible fallback was provided.
    #[error("browser use capability unavailable ({code:?})")]
    Unavailable {
        /// Which native gap was detected.
        code: DiagnosticCode,
        /// Actionable, credential-free diagnostics.
        diagnostics: Vec<AdapterDiagnostic>,
    },
    /// Requirement-vs-policy authorization failed; the lifecycle stays put.
    #[error("browser use authorization failed with {count} violation(s)")]
    Unauthorized {
        /// How many violations were found.
        count: usize,
        /// One actionable diagnostic per violation.
        diagnostics: Vec<AdapterDiagnostic>,
    },
    /// A lifecycle operation was requested from an illegal state.
    #[error("illegal capability lifecycle transition from {from:?}: {attempted}")]
    IllegalTransition {
        /// State the lifecycle was in.
        from: CapabilityLifecycleState,
        /// Operation that was attempted.
        attempted: &'static str,
    },
}
