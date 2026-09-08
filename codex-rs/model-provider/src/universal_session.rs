//! Universal model session execution over the OpenAI-compatible Responses wire.
//!
//! [`ResponsesModelSession`] implements the frozen universal contract's
//! `ModelSession` for the OpenAI-compatible runtime provider path. It is the
//! execution bridge WO-004 adds on top of the WO-002 marshalling adapter:
//! requests are translated by [`crate::ModelContractAdapter`] and executed by
//! the existing `codex-api` `ResponsesClient`, so the runtime's request shape,
//! transport retry policy, authentication behavior, SSE streaming, and
//! cancellation semantics are reused rather than reimplemented:
//!
//! - **Request**: the wire request is built with explicit capability
//!   negotiation; unsupported features fail with
//!   `ModelError::UnsupportedCapability` diagnostics.
//! - **Streaming**: provider SSE events are forwarded as normalized
//!   `ModelEvent`s through the contract stream channel.
//! - **Cancellation**: dropping the returned `ModelStream` closes the channel,
//!   which drops the runtime `ResponseStream`, which cancels the in-flight
//!   provider request.
//! - **Retry**: transport-level retries run inside `codex-api` using the
//!   provider's configured retry policy.
//! - **Authentication**: request-time auth resolution plus provider-owned
//!   recovery and the auth-manager unauthorized-recovery ladder run inside the
//!   session before failures surface, mirroring the runtime client loop.
//!
//! No provider protocol type, credential, or endpoint crosses the contract
//! boundary: the session maps everything into the universal types.

use codex_api::ApiError;
use codex_api::ResponseStream;
use codex_api::ResponsesApiRequest;
use codex_api::ResponsesClient;
use codex_api::ResponsesEndpoint;
use codex_api::ResponsesOptions;
use codex_login::RefreshTokenError;
use codex_model_contract::ModelError;
use codex_model_contract::ModelEvent;
use codex_model_contract::ModelRequest;
use codex_model_contract::ModelSession;
use codex_model_contract::ModelSessionFuture;
use codex_model_contract::ModelStream;
use codex_protocol::error::CodexErr;
use futures::StreamExt;
use tokio::sync::mpsc::Sender;

use crate::provider::ProviderUnauthorizedRecovery;
use crate::universal::ModelContractAdapter;

/// Event channel capacity for contract streams.
///
/// Matches the runtime SSE event channel so forwarding does not introduce
/// different backpressure behavior than the existing request path.
const MODEL_STREAM_CHANNEL_CAPACITY: usize = 1600;

/// Universal-contract session executing requests through the OpenAI-compatible
/// runtime provider.
///
/// Created through `ModelContractAdapter::create_session` (the contract's
/// `ModelProvider::create_session`); not constructed directly. Holds the
/// adapter so each request resolves fresh model metadata, provider auth, and
/// transport state, exactly like the runtime client loop.
pub(crate) struct ResponsesModelSession {
    adapter: ModelContractAdapter,
}

impl ResponsesModelSession {
    pub(crate) fn new(adapter: ModelContractAdapter) -> Self {
        Self { adapter }
    }
}

impl std::fmt::Debug for ResponsesModelSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ResponsesModelSession")
            .field("provider_id", &self.adapter.provider_id())
            .finish_non_exhaustive()
    }
}

impl ModelSession for ResponsesModelSession {
    fn stream(&self, request: ModelRequest) -> ModelSessionFuture<'_> {
        Box::pin(async move { self.stream_request(request).await })
    }
}

impl ResponsesModelSession {
    #[tracing::instrument(
        name = "universal_model_session.stream",
        level = "info",
        skip_all,
        fields(
            provider = %request.model.provider_id,
            model = %request.model.model_id
        )
    )]
    async fn stream_request(&self, request: ModelRequest) -> Result<ModelStream, ModelError> {
        // Guard: a session serves exactly the provider it was created for, so
        // mis-addressed requests fail explicitly instead of silently hitting
        // the wrong backend.
        if request.model.provider_id != self.adapter.provider_id() {
            return Err(ModelError::InvalidRequest {
                message: format!(
                    "request addressed to provider {} but this session serves provider {}",
                    request.model.provider_id,
                    self.adapter.provider_id(),
                ),
            });
        }

        let catalog = self.adapter.model_catalog().ok_or_else(|| {
            ModelError::InvalidRequest {
                message: format!(
                    "provider {} has no model catalog configured; attach one before creating sessions",
                    self.adapter.provider_id(),
                ),
            }
        })?;
        let model_info = catalog.resolve(&request.model.model_id).await;

        // Capability negotiation and wire formatting stay owned by the WO-002
        // adapter so the runtime request shape is preserved exactly.
        let wire_request = self
            .adapter
            .build_responses_request(&request, &model_info)?;

        let response_stream = self.execute_with_auth_recovery(wire_request).await?;
        Ok(forward_response_stream(response_stream))
    }

    /// Executes the wire request, recovering from authentication failures the
    /// same way the runtime client loop does: provider-owned recovery first
    /// (once), then the auth-manager unauthorized-recovery ladder.
    ///
    /// Provider and auth state are re-resolved on every attempt so recovered
    /// credentials are used immediately.
    #[tracing::instrument(
        name = "universal_model_session.execute",
        level = "info",
        skip_all,
        fields(api.path = ResponsesEndpoint::Responses.path())
    )]
    async fn execute_with_auth_recovery(
        &self,
        wire_request: ResponsesApiRequest,
    ) -> Result<ResponseStream, ModelError> {
        let provider = self.adapter.shared_provider();
        let mut provider_recovery_attempted = false;
        let mut auth_recovery = provider
            .auth_manager()
            .map(|manager| manager.unauthorized_recovery());
        loop {
            let api_provider = provider
                .api_provider()
                .await
                .map_err(codex_err_to_model_error)?;
            let api_auth = provider
                .api_auth()
                .await
                .map_err(codex_err_to_model_error)?;
            let transport = self.adapter.build_transport(&api_provider)?;
            let client = ResponsesClient::new(transport, api_provider, api_auth);
            let stream_result = client
                .stream_request(wire_request.clone(), ResponsesOptions::default())
                .await;

            match stream_result {
                Ok(stream) => return Ok(stream),
                Err(ApiError::Transport(transport_error))
                    if provider.is_recoverable_auth_error(&transport_error) =>
                {
                    if !provider_recovery_attempted {
                        provider_recovery_attempted = true;
                        match provider.recover_from_unauthorized().await {
                            Ok(ProviderUnauthorizedRecovery::Recovered) => continue,
                            Ok(ProviderUnauthorizedRecovery::NotConfigured) => {}
                            Err(recovery_error) => {
                                // Mirror the runtime: a retryable recovery failure
                                // surfaces the original auth error; a permanent one
                                // surfaces the recovery failure.
                                if recovery_error.is_retryable() {
                                    return Err(ModelContractAdapter::to_model_error(
                                        ApiError::Transport(transport_error),
                                    ));
                                }
                                return Err(codex_err_to_model_error(recovery_error));
                            }
                        }
                    }
                    if let Some(recovery) = auth_recovery.as_mut()
                        && recovery.has_next()
                    {
                        match recovery.next().await {
                            Ok(_) => continue,
                            Err(RefreshTokenError::Permanent(failed)) => {
                                return Err(ModelError::Auth {
                                    message: failed.to_string(),
                                });
                            }
                            Err(RefreshTokenError::Transient(other)) => {
                                return Err(ModelError::Transport {
                                    message: other.to_string(),
                                    retryable: true,
                                    retry_after: None,
                                    status: None,
                                });
                            }
                        }
                    }
                    return Err(ModelContractAdapter::to_model_error(ApiError::Transport(
                        transport_error,
                    )));
                }
                Err(error) => return Err(ModelContractAdapter::to_model_error(error)),
            }
        }
    }
}

/// Forwards a runtime `ResponseStream` into a contract `ModelStream`.
///
/// Events and errors are mapped with the WO-002 adapter conversions. When the
/// contract stream is dropped, `send` fails, the forwarder exits, and the
/// runtime stream is dropped, canceling the in-flight provider request.
fn forward_response_stream(stream: ResponseStream) -> ModelStream {
    let request_id = stream.upstream_request_id.clone();
    let (tx, model_stream) = ModelStream::channel(MODEL_STREAM_CHANNEL_CAPACITY, request_id);
    tokio::spawn(forward_events(stream, tx));
    model_stream
}

async fn forward_events(mut stream: ResponseStream, tx: Sender<Result<ModelEvent, ModelError>>) {
    while let Some(item) = stream.next().await {
        let mapped = item
            .map(ModelContractAdapter::to_model_event)
            .map_err(ModelContractAdapter::to_model_error);
        if tx.send(mapped).await.is_err() {
            // The ModelStream was dropped: cancel the provider request.
            break;
        }
    }
}

/// Maps a runtime protocol error surfaced by provider/auth resolution into
/// the universal error taxonomy. Messages never contain credentials.
pub(crate) fn codex_err_to_model_error(error: CodexErr) -> ModelError {
    ModelError::Provider {
        status: None,
        code: None,
        message: error.to_string(),
    }
}

#[cfg(test)]
#[path = "universal_session_tests.rs"]
mod tests;
