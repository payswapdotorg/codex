use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::task::Context;
use std::task::Poll;
use std::time::Duration;

use futures::Stream;
use tokio::sync::mpsc;

use crate::error::ModelError;
use crate::event::ModelEvent;

/// One item of a model event stream: an event or a normalized failure.
pub type ModelStreamItem = Result<ModelEvent, ModelError>;

/// Boxed future returned by [`ModelSession::stream`].
///
/// Boxed futures keep the trait dyn-compatible so sessions can be shared as
/// `Arc<dyn ModelSession>`, mirroring the runtime model provider's future
/// type. Implementations build them with `Box::pin(async move { ... })`.
pub type ModelSessionFuture<'a> =
    Pin<Box<dyn Future<Output = Result<ModelStream, ModelError>> + Send + 'a>>;

/// Normalized retry and transport policy for a provider.
///
/// Mirrors the effective values a provider configuration resolves (retry
/// budgets, stream idle timeout, websocket support). Providers own the
/// semantics; the runtime uses this to drive normalized stream retry and
/// fallback decisions without seeing provider transport details.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelTransportPolicy {
    /// Maximum attempts for a single unary request.
    pub request_max_retries: u64,
    /// Maximum stream reconnection attempts before failing a turn.
    pub stream_max_retries: u64,
    /// Idle time before a streaming connection is considered lost.
    pub stream_idle_timeout: Duration,
    /// Timeout for websocket connection attempts, when supported.
    pub websocket_connect_timeout: Option<Duration>,
    /// Whether the provider supports a streaming websocket transport.
    pub supports_websockets: bool,
}

impl Default for ModelTransportPolicy {
    fn default() -> Self {
        Self {
            request_max_retries: 4,
            stream_max_retries: 5,
            stream_idle_timeout: Duration::from_millis(300_000),
            websocket_connect_timeout: Some(Duration::from_millis(15_000)),
            supports_websockets: false,
        }
    }
}

/// Normalized authentication status for a provider.
///
/// Carries only status and an account label; credentials never cross the
/// universal boundary.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ModelAuthStatus {
    /// The provider needs no authentication.
    #[default]
    NotRequired,
    /// Authentication is present; `account` is a display label when known.
    Authenticated { account: Option<String> },
    /// Authentication is required but missing or expired.
    RequiresAuthentication {
        /// Provider guidance for obtaining credentials, when available.
        instructions: Option<String>,
    },
}

/// Provider-neutral event stream for one model request.
///
/// Cancellation is normalized through ownership: dropping the stream signals
/// cancellation, and adapters must observe the dropped receiver (or an
/// equivalent signal) and stop provider work promptly. The stream never
/// carries provider protocol types.
pub struct ModelStream {
    rx_event: mpsc::Receiver<ModelStreamItem>,
    /// Provider-assigned request correlation id, when one is known.
    pub request_id: Option<String>,
}

impl ModelStream {
    /// Creates a stream channel the adapter feeds events into.
    ///
    /// The returned sender is closed by dropping; when the [`ModelStream`]
    /// side is dropped, `send` fails and the adapter must treat the request
    /// as canceled.
    pub fn channel(
        capacity: usize,
        request_id: Option<String>,
    ) -> (mpsc::Sender<ModelStreamItem>, Self) {
        let (tx_event, rx_event) = mpsc::channel(capacity);
        (
            tx_event,
            Self {
                rx_event,
                request_id,
            },
        )
    }
}

impl fmt::Debug for ModelStream {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ModelStream")
            .field("request_id", &self.request_id)
            .finish_non_exhaustive()
    }
}

impl Stream for ModelStream {
    type Item = ModelStreamItem;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.rx_event.poll_recv(cx)
    }
}

/// Per-turn execution boundary behind the universal model contract.
///
/// A session executes [`crate::ModelRequest`]s against one provider and
/// normalizes streaming and cancellation:
///
/// - `stream` resolves to a [`ModelStream`] of
///   [`ModelEvent`]s or a [`ModelError`];
/// - dropping the returned stream cancels the in-flight provider request;
/// - adapter-owned transport retries and provider-owned authentication
///   recovery happen inside the session before failures surface here.
///
/// The OpenAI Responses runtime implements this contract through the existing
/// Codex model client; native provider adapters implement it directly
/// (work order WO-004 owns adapter execution).
pub trait ModelSession: fmt::Debug + Send + Sync {
    /// Streams one model invocation.
    fn stream(&self, request: crate::request::ModelRequest) -> ModelSessionFuture<'_>;
}

#[cfg(test)]
#[path = "session_tests.rs"]
mod tests;
