use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use codex_protocol::openai_models::ModelInfo;

use crate::capabilities::ModelCapabilities;
use crate::descriptor::ModelDescriptor;
use crate::error::ModelError;
use crate::session::ModelAuthStatus;
use crate::session::ModelSession;
use crate::session::ModelTransportPolicy;

/// Boxed future returned by [`ModelProvider::auth_status`].
///
/// Boxed futures keep the trait dyn-compatible so providers can be shared as
/// `Arc<dyn ModelProvider>`, mirroring the runtime model provider's future
/// type.
pub type ModelAuthStatusFuture<'a> = Pin<Box<dyn Future<Output = ModelAuthStatus> + Send + 'a>>;

/// Universal provider boundary for the model plane.
///
/// Implementations own provider-specific behavior: identity, transport
/// policy, authentication lifecycle, capability resolution, and session
/// creation. No provider protocol type appears at this boundary; adapters
/// translate between their protocol and the contract types.
///
/// Model selection stays a runtime binding decision: switching the provider
/// that serves a thread re-creates a provider and session through this trait
/// without rewriting durable thread state.
pub trait ModelProvider: std::fmt::Debug + Send + Sync {
    /// Stable provider registry key used by [`ModelDescriptor`].
    fn provider_id(&self) -> &str;

    /// Effective transport policy: retry budgets, timeouts, websocket
    /// support.
    fn transport_policy(&self) -> ModelTransportPolicy;

    /// Current normalized authentication status; never carries credentials.
    fn auth_status(&self) -> ModelAuthStatusFuture<'_>;

    /// Effective capabilities for a catalog model.
    fn model_capabilities(&self, model: &ModelInfo) -> ModelCapabilities;

    /// Descriptor for a model id owned by this provider.
    fn descriptor(&self, model_id: &str) -> ModelDescriptor;

    /// Opens a session for executing requests against this provider.
    ///
    /// Providers that do not yet provide session execution must fail
    /// explicitly rather than returning a non-functional session.
    fn create_session(&self) -> Result<Arc<dyn ModelSession>, ModelError>;
}

#[cfg(test)]
#[path = "provider_tests.rs"]
mod tests;
