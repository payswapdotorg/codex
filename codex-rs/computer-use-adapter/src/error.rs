//! Adapter error type.

use crate::capability::CapabilityLifecycleState;
use thiserror::Error;

/// Errors raised by the computer use workflow adapter.
#[derive(Clone, Debug, Error)]
pub enum ComputerUseAdapterError {
    /// The capability lifecycle received an illegal transition request.
    #[error("invalid capability lifecycle transition from {from:?} to {to:?}")]
    InvalidLifecycleTransition {
        /// State the lifecycle was in.
        from: CapabilityLifecycleState,
        /// State the caller asked to move to.
        to: CapabilityLifecycleState,
    },
    /// Native computer use bridge/runtime provisioning failed readiness.
    #[error("computer use capability failed readiness: {diagnostic}")]
    Readiness {
        /// Human-readable, remediation-oriented diagnostic.
        diagnostic: String,
    },
    /// Native policy denies access to the requested application.
    #[error("computer use policy denies access to application {subject}")]
    PolicyDenied {
        /// Display identity of the denied application.
        subject: String,
    },
    /// Access is unspecified by policy and no host approval was supplied.
    #[error("application {subject} requires explicit host authorization")]
    AuthorizationRequired {
        /// Display identity of the application requiring approval.
        subject: String,
    },
    /// A step targeted an application that was never authorized.
    #[error("step target {subject} is not authorized for this session")]
    TargetNotAuthorized {
        /// Display identity of the unauthorized target.
        subject: String,
    },
    /// The caller is not the recorded owner of the session.
    #[error("owner mismatch: session {session} belongs to another owner")]
    OwnerMismatch {
        /// Session identifier involved.
        session: String,
    },
    /// A graceful takeover raced a session that is not idle.
    #[error("session {session} is busy and cannot be taken over gracefully")]
    SessionBusy {
        /// Session identifier involved.
        session: String,
    },
    /// Operation requires a lifecycle state the capability is not in.
    #[error("operation requires lifecycle state {expected:?} but capability is {actual:?}")]
    WrongLifecycleState {
        /// Required state.
        expected: CapabilityLifecycleState,
        /// Actual state.
        actual: CapabilityLifecycleState,
    },
    /// The bridge failed while executing an action or observation.
    #[error("computer use bridge call failed: {0}")]
    BridgeCall(String),
    /// A fallback was rejected because it cannot honor the required semantic
    /// or policy requirements.
    #[error("desktop adapter fallback rejected: {0}")]
    FallbackRejected(String),
    /// A forced takeover was requested without a recorded reason.
    #[error("forced takeover requires a recorded reason")]
    TakeoverReasonRequired,
    /// Policy does not permit forced takeover by a new owner.
    #[error("policy does not allow session takeover")]
    TakeoverPolicyDenied,
}
