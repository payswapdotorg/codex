use pretty_assertions::assert_eq;
use std::sync::Arc;
use std::time::Duration;

use super::*;
use crate::capabilities::ModelCapabilities;
use crate::capabilities::ModelFeature;
use crate::error::ModelError;
use crate::event::ModelEvent;
use crate::request::ModelReasoningControls;
use crate::request::ModelRequest;
use crate::response::ModelResponse;
use crate::session::ModelSession;
use crate::session::ModelSessionFuture;
use crate::session::ModelStream;
use codex_protocol::openai_models::ReasoningEffort;
use futures::StreamExt;

/// Test-only provider proving the universal trait shape is implementable and
/// drives the full request lifecycle: negotiation, streaming, completion,
/// and cancellation.
#[derive(Debug)]
struct MockModelProvider {
    capabilities: ModelCapabilities,
}

impl MockModelProvider {
    fn new(capabilities: ModelCapabilities) -> Self {
        Self { capabilities }
    }
}

impl ModelProvider for MockModelProvider {
    fn provider_id(&self) -> &str {
        &self.capabilities.provider_id
    }

    fn transport_policy(&self) -> ModelTransportPolicy {
        ModelTransportPolicy {
            stream_idle_timeout: Duration::from_secs(1),
            ..ModelTransportPolicy::default()
        }
    }

    fn auth_status(&self) -> ModelAuthStatusFuture<'_> {
        Box::pin(async { ModelAuthStatus::NotRequired })
    }

    fn model_capabilities(&self, _model: &ModelInfo) -> ModelCapabilities {
        self.capabilities.clone()
    }

    fn descriptor(&self, model_id: &str) -> ModelDescriptor {
        ModelDescriptor::new(self.provider_id(), model_id)
    }

    fn create_session(&self) -> Result<Arc<dyn ModelSession>, ModelError> {
        Ok(Arc::new(MockModelSession {
            capabilities: self.capabilities.clone(),
        }))
    }
}

#[derive(Debug)]
struct MockModelSession {
    capabilities: ModelCapabilities,
}

impl ModelSession for MockModelSession {
    fn stream(&self, request: ModelRequest) -> ModelSessionFuture<'_> {
        let capabilities = self.capabilities.clone();
        Box::pin(async move {
            // Sessions negotiate requested controls before issuing work, so
            // unsupported capabilities fail explicitly.
            if let Some(reasoning) = request.reasoning.as_ref()
                && let Some(effort) = reasoning.effort.as_ref()
            {
                capabilities.require(ModelFeature::ReasoningEffort(effort.clone()))?;
            }

            let (tx, stream) = ModelStream::channel(8, Some("mock-req".to_string()));
            let response_id = format!("resp-{}", request.model.model_id);
            tokio::spawn(async move {
                let _ = tx
                    .send(Ok(ModelEvent::Created {
                        response_id: Some(response_id.clone()),
                    }))
                    .await;
                let _ = tx
                    .send(Ok(ModelEvent::OutputTextDelta("hello".to_string())))
                    .await;
                let _ = tx
                    .send(Ok(ModelEvent::Completed {
                        response_id,
                        token_usage: None,
                        usage_metadata: None,
                        end_turn: Some(true),
                    }))
                    .await;
            });
            Ok(stream)
        })
    }
}

fn capable_provider() -> MockModelProvider {
    let mut capabilities = ModelCapabilities {
        provider_id: "mock".to_string(),
        model_id: "mock-1".to_string(),
        ..ModelCapabilities::default()
    };
    capabilities.supported_reasoning_efforts = vec![ReasoningEffort::Medium];
    capabilities.supports_reasoning_summary = true;
    MockModelProvider::new(capabilities)
}

#[tokio::test]
async fn provider_serves_a_full_request_lifecycle() {
    let provider = capable_provider();
    let session = provider.create_session().expect("session should open");

    let request = ModelRequest {
        model: provider.descriptor("mock-1"),
        reasoning: Some(ModelReasoningControls {
            effort: Some(ReasoningEffort::Medium),
            summary: None,
        }),
        ..ModelRequest::default()
    };
    let mut stream = session.stream(request).await.expect("stream should start");
    let events: Vec<crate::session::ModelStreamItem> = stream.by_ref().collect().await;

    let response = ModelResponse::from_events(
        events
            .into_iter()
            .map(|item| item.expect("stream should succeed")),
    );
    assert_eq!(response.response_id, Some("resp-mock-1".to_string()));
    assert_eq!(response.end_turn, Some(true));
}

#[tokio::test]
async fn unsupported_capability_fails_explicitly_at_stream_time() {
    let provider = capable_provider();
    let session = provider.create_session().expect("session should open");

    let request = ModelRequest {
        model: provider.descriptor("mock-1"),
        reasoning: Some(ModelReasoningControls {
            effort: Some(ReasoningEffort::Max),
            summary: None,
        }),
        ..ModelRequest::default()
    };
    let error = session
        .stream(request)
        .await
        .expect_err("unsupported effort should fail");

    assert!(
        matches!(error, ModelError::UnsupportedCapability(_)),
        "expected capability diagnostics, got {error}"
    );
    assert!(!error.is_retryable());
}

#[tokio::test]
async fn provider_identity_is_opaque() {
    let provider = capable_provider();

    assert_eq!(provider.provider_id(), "mock");
    assert_eq!(
        provider.descriptor("some-model"),
        ModelDescriptor::new("mock", "some-model")
    );
}

#[tokio::test]
async fn provider_surfaces_transport_policy_and_auth() {
    let provider = capable_provider();

    assert_eq!(
        provider.transport_policy(),
        ModelTransportPolicy {
            stream_idle_timeout: Duration::from_secs(1),
            ..ModelTransportPolicy::default()
        }
    );
    assert_eq!(provider.auth_status().await, ModelAuthStatus::NotRequired);
}
