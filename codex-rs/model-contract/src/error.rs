use std::time::Duration;

use crate::capabilities::UnsupportedModelCapability;

/// Normalized provider failure taxonomy for the universal model plane.
///
/// Adapters map provider-specific errors (HTTP statuses, transport faults,
/// provider error codes) into this taxonomy. The contract intentionally does
/// not model every provider error code; unrecognized failures fall back to
/// [`ModelError::Provider`] with the raw status and message, which never
/// contain credentials.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ModelError {
    /// Transport-level failure while issuing or streaming a request.
    #[error("model transport error: {message}")]
    Transport {
        message: String,
        /// Whether retrying the request can plausibly succeed.
        retryable: bool,
        /// Delay the provider suggested before retrying, if any.
        retry_after: Option<Duration>,
        /// HTTP status code, when the failure carried one.
        status: Option<u16>,
    },
    /// Authentication failed, is missing, or expired.
    #[error("model authentication error: {message}")]
    Auth { message: String },
    /// The provider rejected the request as invalid.
    #[error("invalid model request: {message}")]
    InvalidRequest { message: String },
    /// The request exceeds the model's context window.
    #[error("context window exceeded: {message}")]
    ContextWindowExceeded { message: String },
    /// Account usage, quota, or plan limits were reached.
    #[error("usage limit exceeded: {message}")]
    UsageLimitExceeded { message: String },
    /// The provider is rate limiting requests.
    #[error("rate limited: {message}")]
    RateLimited {
        message: String,
        /// Delay the provider suggested before retrying, if any.
        retry_after: Option<Duration>,
    },
    /// The provider is temporarily overloaded.
    #[error("model provider overloaded: {message}")]
    Overloaded { message: String },
    /// A required model capability is unsupported; the request must not be
    /// silently degraded.
    #[error("{0}")]
    UnsupportedCapability(#[from] UnsupportedModelCapability),
    /// The provider stream ended before reporting completion.
    #[error("model stream ended before completion: {message}")]
    StreamIncomplete { message: String },
    /// The request was canceled before completion.
    #[error("model request canceled: {reason}")]
    Canceled { reason: String },
    /// A provider failure that is not otherwise classified.
    #[error("model provider error: {message}")]
    Provider {
        /// HTTP status code, when the failure carried one.
        status: Option<u16>,
        /// Provider error code, when one was reported.
        code: Option<String>,
        message: String,
    },
}

impl ModelError {
    /// Returns whether retrying the failed request can plausibly succeed.
    ///
    /// Auth failures recover only through provider-owned authentication
    /// recovery; capability failures never recover by retrying.
    pub fn is_retryable(&self) -> bool {
        match self {
            Self::Transport { retryable, .. } => *retryable,
            Self::RateLimited { .. } | Self::Overloaded { .. } => true,
            Self::Auth { .. }
            | Self::InvalidRequest { .. }
            | Self::ContextWindowExceeded { .. }
            | Self::UsageLimitExceeded { .. }
            | Self::UnsupportedCapability(_)
            | Self::StreamIncomplete { .. }
            | Self::Canceled { .. }
            | Self::Provider { .. } => false,
        }
    }

    /// Returns the HTTP status carried by the failure, when present.
    pub fn status(&self) -> Option<u16> {
        match self {
            Self::Transport { status, .. } => *status,
            Self::Provider { status, .. } => *status,
            _ => None,
        }
    }
}

#[cfg(test)]
#[path = "error_tests.rs"]
mod tests;
