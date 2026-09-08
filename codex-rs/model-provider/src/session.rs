//! [`ModelSession`] implementation that streams OpenAI Responses events
//! through the existing `codex-api` client.
//!
//! [`OpenAiResponsesSession`] is the executable contract surface for the
//! OpenAI-compatible adapter ([`crate::ModelContractAdapter`]). It owns one
//! model invocation at a time:
//!
//! 1. resolve [`ModelInfo`] for the request's [`ModelDescriptor`] from the
//!    attached model catalog;
//! 2. build the OpenAI Responses wire request through
//!    [`ModelContractAdapter::build_responses_request`], which performs
//!    capability negotiation first (unsupported features fail with
//!    [`ModelError::UnsupportedCapability`] instead of being silently
//!    dropped);
//! 3. resolve the runtime provider's API provider config and auth state;
//! 4. construct a `ResponsesClient` over a fresh reqwest transport and
//!    stream the wire request;
//! 5. forward each `ResponseEvent` to a [`ModelStream`] channel as a
//!    normalized [`ModelEvent`];
//! 6. cancel the in-flight provider request when the [`ModelStream`] is
//!    dropped — the channel sender observes the dropped receiver and the
//!    forwarding task exits, which drops the underlying `ResponseStream`
//!    and tears down the SSE connection.
//!
//! The session is the only execution path introduced by WO-004. It reuses
//! `codex-api`'s `ResponsesClient`, `Provider`, `SharedAuthProvider`,
//! `ReqwestTransport`, and `ResponseStream` rather than building a parallel
//! runtime, satisfying the "no second agent runtime" and "no second workflow
//! engine" invariants. Provider-specific protocol types stay behind this
//! boundary; only [`ModelEvent`]s and [`ModelError`]s cross it.

use codex_api::ReqwestTransport;
use codex_api::ResponsesClient as ApiResponsesClient;
use codex_api::ResponsesOptions;
use codex_model_contract::ModelError;
use codex_model_contract::ModelRequest;
use codex_model_contract::ModelSession;
use codex_model_contract::ModelSessionFuture;
use codex_model_contract::ModelStream;
use codex_model_contract::ModelStreamItem;
use codex_models_manager::ModelsManagerConfig;
use codex_models_manager::manager::SharedModelsManager;
use futures::StreamExt;
use tracing::instrument;

use crate::ModelContractAdapter;
use crate::auth::resolve_provider_auth;

/// Capacity of the channel feeding the [`ModelStream`].
///
/// Matches the codex-api `ResponseStream` internal channel width so the
/// forwarding task does not block on a slow consumer under normal load.
const STREAM_CHANNEL_CAPACITY: usize = 1600;

/// OpenAI Responses-backed execution session behind the universal contract.
///
/// Constructed by [`ModelContractAdapter::create_session`] (via the
/// [`codex_model_contract::ModelProvider`] impl) and owned by the caller as
/// `Arc<dyn ModelSession>`. Each `stream` call resolves the model catalog,
/// builds the wire request, and streams events; the session itself is
/// stateless across calls.
#[derive(Debug, Clone)]
pub struct OpenAiResponsesSession {
    adapter: ModelContractAdapter,
    models_manager: SharedModelsManager,
}

impl OpenAiResponsesSession {
    /// Wraps the adapter and catalog resolved by
    /// [`ModelContractAdapter::create_session`].
    pub fn new(adapter: ModelContractAdapter, models_manager: SharedModelsManager) -> Self {
        Self {
            adapter,
            models_manager,
        }
    }

    /// Resolves [`ModelInfo`] for a request's model id from the attached
    /// catalog. The codex models manager always returns a `ModelInfo`
    /// (falling back to a slug-derived default); capability negotiation
    /// then surfaces explicit failures for any unsupported feature.
    async fn resolve_model_info(&self, model_id: &str) -> codex_protocol::openai_models::ModelInfo {
        let config = ModelsManagerConfig::default();
        self.models_manager.get_model_info(model_id, &config).await
    }

    /// Builds the OpenAI Responses wire request, performing capability
    /// negotiation first. Unsupported features fail with
    /// [`ModelError::UnsupportedCapability`] instead of being silently
    /// dropped.
    fn build_wire_request(
        &self,
        request: &ModelRequest,
        model_info: &codex_protocol::openai_models::ModelInfo,
    ) -> Result<codex_api::ResponsesApiRequest, ModelError> {
        self.adapter.build_responses_request(request, model_info)
    }

    /// Streams a wire request through a freshly constructed
    /// [`ApiResponsesClient`] and forwards events to a [`ModelStream`].
    ///
    /// The returned stream owns the channel receiver; dropping it cancels
    /// the in-flight provider request through the cancellation-via-drop
    /// chain documented at the module top.
    async fn stream_wire_request(
        &self,
        wire_request: codex_api::ResponsesApiRequest,
    ) -> Result<ModelStream, ModelError> {
        let provider = self.adapter.runtime_provider().clone();
        let api_provider = provider
            .api_provider()
            .await
            .map_err(|err| ModelError::Provider {
                status: None,
                code: None,
                message: err.to_string(),
            })?;
        let auth = provider.auth().await;
        let api_auth = resolve_provider_auth(auth.as_ref(), provider.info()).map_err(|err| {
            ModelError::Provider {
                status: None,
                code: None,
                message: err.to_string(),
            }
        })?;

        let transport = ReqwestTransport::new(reqwest::Client::new());
        let client = ApiResponsesClient::new(transport, api_provider, api_auth);
        let options = ResponsesOptions::default();
        let stream_result = client.stream_request(wire_request, options).await;
        let response_stream = stream_result.map_err(ModelContractAdapter::to_model_error)?;

        let (tx_event, model_stream) =
            ModelStream::channel(STREAM_CHANNEL_CAPACITY, /*request_id*/ None);
        tokio::spawn(forward_response_events(response_stream, tx_event));
        Ok(model_stream)
    }
}

impl ModelSession for OpenAiResponsesSession {
    #[instrument(
        name = "model_session.stream",
        level = "info",
        skip_all,
        fields(
            provider = %self.adapter.provider_id(),
            model = %request.model.model_id,
        )
    )]
    fn stream(&self, request: ModelRequest) -> ModelSessionFuture<'_> {
        let session = self.clone();
        Box::pin(async move {
            let model_info = session.resolve_model_info(&request.model.model_id).await;
            let wire_request = session.build_wire_request(&request, &model_info)?;
            session.stream_wire_request(wire_request).await
        })
    }
}

/// Forwards `ResponseEvent`s from a `codex-api` `ResponseStream` to a
/// [`ModelStream`] channel as normalized [`ModelEvent`]s.
///
/// Cancellation propagates through ownership: when the [`ModelStream`]
/// receiver is dropped, `tx_event.send` returns an error and the task
/// exits, dropping the underlying `ResponseStream` and tearing down the
/// SSE connection.
async fn forward_response_events(
    mut response_stream: codex_api::ResponseStream,
    tx_event: tokio::sync::mpsc::Sender<ModelStreamItem>,
) {
    while let Some(item) = response_stream.next().await {
        let forwarded = match item {
            Ok(event) => Ok(ModelContractAdapter::to_model_event(event)),
            Err(error) => Err(ModelContractAdapter::to_model_error(error)),
        };
        if tx_event.send(forwarded).await.is_err() {
            // Receiver dropped: cancellation. Stop forwarding and let the
            // underlying `ResponseStream` drop on exit, which tears down the
            // SSE connection through codex-api's own channel.
            break;
        }
    }
}

#[cfg(test)]
#[path = "session_tests.rs"]
mod tests;
