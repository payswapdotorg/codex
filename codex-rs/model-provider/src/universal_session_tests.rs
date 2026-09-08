use std::sync::Arc;
use std::time::Duration;

use codex_api::ResponseEvent;
use codex_login::AuthManager;
use codex_login::CodexAuth;
use codex_model_contract::ModelDescriptor;
use codex_model_contract::ModelReasoningControls;
use codex_model_contract::ModelResponse;
use codex_model_provider_info::ModelProviderInfo;
use codex_model_provider_info::WireApi;
use codex_model_provider_info::create_oss_provider_with_base_url;
use codex_models_manager::ModelsManagerConfig;
use codex_models_manager::manager::StaticModelsManager;
use codex_protocol::openai_models::ModelInfo;
use codex_protocol::openai_models::ModelsResponse;
use codex_protocol::openai_models::ReasoningEffort;
use pretty_assertions::assert_eq;
use serde_json::Value;
use serde_json::json;
use tokio::sync::mpsc;
use wiremock::Mock;
use wiremock::MockServer;
use wiremock::ResponseTemplate;
use wiremock::matchers::header;
use wiremock::matchers::method;
use wiremock::matchers::path;

use super::*;
use crate::create_model_provider;
use crate::universal_catalog::ModelCatalog;

// Bring the contract provider trait into scope so `create_session` resolves
// on the concrete adapter.
use codex_model_contract::ModelProvider as _;

/// Bound for tests that await streams or background tasks.
const TEST_TIMEOUT: Duration = Duration::from_secs(30);

fn reasoning_model_json(slug: &str) -> Value {
    json!({
        "slug": slug,
        "display_name": slug,
        "description": null,
        "default_reasoning_level": "medium",
        "supported_reasoning_levels": [
            { "effort": "low", "description": "" },
            { "effort": "medium", "description": "" },
            { "effort": "high", "description": "" }
        ],
        "shell_type": "shell_command",
        "visibility": "list",
        "supported_in_api": true,
        "priority": 0,
        "upgrade": null,
        "availability_nux": null,
        "support_verbosity": true,
        "default_verbosity": null,
        "apply_patch_tool_type": null,
        "truncation_policy": {"mode": "bytes", "limit": 10_000},
        "supports_image_detail_original": false,
        "context_window": 272_000,
        "max_context_window": 272_000,
        "experimental_supported_tools": [],
        "input_modalities": ["text", "image"],
        "service_tiers": [
            { "id": "standard", "name": "Standard", "description": "" }
        ],
    })
}

fn text_only_model_json(slug: &str) -> Value {
    json!({
        "slug": slug,
        "display_name": slug,
        "description": null,
        "default_reasoning_level": "medium",
        "supported_reasoning_levels": [
            { "effort": "medium", "description": "" }
        ],
        "shell_type": "shell_command",
        "visibility": "list",
        "supported_in_api": true,
        "priority": 0,
        "upgrade": null,
        "availability_nux": null,
        "support_verbosity": false,
        "default_verbosity": null,
        "apply_patch_tool_type": null,
        "truncation_policy": {"mode": "bytes", "limit": 10_000},
        "supports_image_detail_original": false,
        "context_window": 272_000,
        "max_context_window": 272_000,
        "experimental_supported_tools": [],
        "input_modalities": ["text"],
        "service_tiers": [],
    })
}

fn static_catalog(models: Vec<Value>) -> ModelCatalog {
    let models: Vec<ModelInfo> = models
        .into_iter()
        .map(|value| serde_json::from_value(value).expect("model should parse"))
        .collect();
    ModelCatalog::new(
        Arc::new(StaticModelsManager::new(
            /*auth_manager*/ None,
            ModelsResponse { models },
        )),
        ModelsManagerConfig::default(),
    )
}
fn openai_adapter(server_uri: &str, models: Vec<Value>) -> ModelContractAdapter {
    ModelContractAdapter::new(
        "openai",
        create_model_provider(
            ModelProviderInfo::create_openai_provider(Some(server_uri.to_string())),
            Some(AuthManager::from_auth_for_testing(CodexAuth::from_api_key(
                "test-openai-key",
            ))),
        ),
    )
    .with_model_catalog(
        static_catalog(models).manager(),
        ModelsManagerConfig::default(),
    )
}

fn ollama_adapter(server_uri: &str, models: Vec<Value>) -> ModelContractAdapter {
    ModelContractAdapter::new(
        "ollama",
        create_model_provider(
            create_oss_provider_with_base_url(server_uri, WireApi::Responses),
            /*auth_manager*/ None,
        ),
    )
    .with_model_catalog(
        static_catalog(models).manager(),
        ModelsManagerConfig::default(),
    )
}

fn user_message(text: &str) -> codex_protocol::models::ResponseItem {
    use codex_protocol::models::ContentItem;
    codex_protocol::models::ResponseItem::Message {
        id: None,
        role: "user".to_string(),
        content: vec![ContentItem::InputText {
            text: text.to_string(),
        }],
        phase: None,
        internal_chat_message_metadata_passthrough: None,
    }
}

/// The durable semantic content of a thread turn: instructions, input, tools.
/// Provider identity lives only in the descriptor.
fn semantic_request(provider_id: &str, model_id: &str) -> ModelRequest {
    ModelRequest {
        model: ModelDescriptor::new(provider_id, model_id),
        instructions: "You are a helpful assistant.".to_string(),
        input: vec![user_message("hello from a durable thread")],
        ..ModelRequest::default()
    }
}

fn sse_body(events: &[Value]) -> String {
    let mut body = String::new();
    for event in events {
        body.push_str(&format!(
            "event: {}\n",
            event["type"].as_str().expect("typed event")
        ));
        body.push_str(&format!("data: {event}\n\n"));
    }
    body
}

fn sse_template(body: String) -> ResponseTemplate {
    ResponseTemplate::new(200)
        .insert_header("content-type", "text/event-stream")
        .set_body_raw(body, "text/event-stream")
}

fn successful_stream(response_id: &str, text: &str) -> String {
    sse_body(&[
        json!({
            "type": "response.created",
            "response": { "id": response_id }
        }),
        json!({
            "type": "response.output_text.delta",
            "delta": text
        }),
        json!({
            "type": "response.output_item.done",
            "item": {
                "type": "message",
                "role": "assistant",
                "id": "msg-1",
                "content": [{"type": "output_text", "text": text}]
            }
        }),
        json!({
            "type": "response.completed",
            "response": {
                "id": response_id,
                "end_turn": true,
                "usage": {
                    "input_tokens": 7,
                    "input_tokens_details": null,
                    "output_tokens": 3,
                    "output_tokens_details": null,
                    "total_tokens": 10
                }
            }
        }),
    ])
}

async fn mount_sse(server: &MockServer, body: String, expect: u64) {
    Mock::given(method("POST"))
        .and(path("/responses"))
        .respond_with(sse_template(body))
        .expect(expect)
        .mount(server)
        .await;
}

async fn collect_stream(
    stream: ModelStream,
) -> Vec<Result<codex_model_contract::ModelEvent, ModelError>> {
    let events = tokio::time::timeout(TEST_TIMEOUT, futures_collect(stream)).await;
    events.expect("stream should complete within the test timeout")
}

async fn futures_collect(
    mut stream: ModelStream,
) -> Vec<Result<codex_model_contract::ModelEvent, ModelError>> {
    use futures::StreamExt as _;
    let mut items = Vec::new();
    while let Some(item) = stream.next().await {
        items.push(item);
    }
    items
}

/// The full request lifecycle: created -> delta -> completed, with usage
/// normalized into the contract response.
#[tokio::test]
async fn session_streams_a_full_request_lifecycle() {
    let server = MockServer::start().await;
    mount_sse(&server, successful_stream("resp-openai-1", "hello"), 1).await;

    let adapter = openai_adapter(&server.uri(), vec![reasoning_model_json("gpt-5.6-luna")]);
    let session = adapter.create_session().expect("session should open");

    let request = semantic_request("openai", "gpt-5.6-luna");
    let stream = session.stream(request).await.expect("stream should start");
    let events = collect_stream(stream).await;

    // The runtime emits a default rate-limit snapshot at stream start, so a
    // successful lifecycle is: rate limits, created, delta, item done,
    // completed.
    assert_eq!(events.len(), 5);
    assert!(matches!(
        events[0].as_ref().expect("first event should be an event"),
        codex_model_contract::ModelEvent::RateLimits(_)
    ));
    assert_eq!(
        events[1].as_ref().expect("created should be an event"),
        &codex_model_contract::ModelEvent::Created {
            response_id: Some("resp-openai-1".to_string()),
        }
    );
    assert_eq!(
        events[2].as_ref().expect("delta should be an event"),
        &codex_model_contract::ModelEvent::OutputTextDelta("hello".to_string())
    );
    assert!(matches!(
        events[3].as_ref().expect("item done should be an event"),
        codex_model_contract::ModelEvent::OutputItemDone(
            codex_protocol::models::ResponseItem::Message { role, .. }
        ) if role == "assistant"
    ));
    assert!(matches!(
        events[4].as_ref().expect("completed should be an event"),
        codex_model_contract::ModelEvent::Completed {
            response_id,
            token_usage,
            end_turn: Some(true),
            ..
        } if response_id == "resp-openai-1" && token_usage.as_ref().is_some_and(|usage| usage.total_tokens == 10)
    ));

    server.verify().await;
}

/// The provider-swap demonstration: the same semantic request (identical
/// instructions, input, and tools) serves two provider paths through the
/// universal contract. The wire bodies recorded by both backends carry the
/// same semantic content, and the normalized responses are equivalent, while
/// only the descriptor's provider identity differs.
#[tokio::test]
async fn same_semantic_request_serves_two_providers() {
    let openai_server = MockServer::start().await;
    let ollama_server = MockServer::start().await;

    // The openai path authenticates with a bearer key; the ollama path is a
    // local runtime without auth.
    Mock::given(method("POST"))
        .and(path("/responses"))
        .and(header("authorization", "Bearer test-openai-key"))
        .respond_with(sse_template(successful_stream("resp-openai-1", "hello")))
        .expect(1)
        .mount(&openai_server)
        .await;
    Mock::given(method("POST"))
        .and(path("/responses"))
        .respond_with(sse_template(successful_stream("resp-ollama-1", "hello")))
        .expect(1)
        .mount(&ollama_server)
        .await;

    let openai = openai_adapter(
        &openai_server.uri(),
        vec![reasoning_model_json("gpt-5.6-luna")],
    );
    let ollama = ollama_adapter(
        &ollama_server.uri(),
        vec![reasoning_model_json("gpt-5.6-luna")],
    );

    // One durable request; the provider swap changes only the descriptor.
    let request = semantic_request("openai", "gpt-5.6-luna");
    let swapped = request.for_provider("ollama");
    assert_eq!(swapped.instructions, request.instructions);
    assert_eq!(swapped.input, request.input);
    assert_eq!(swapped.tools, request.tools);
    assert_eq!(swapped.model.provider_id, "ollama");

    let openai_events = collect_stream(
        openai
            .create_session()
            .expect("openai session should open")
            .stream(request)
            .await
            .expect("openai stream should start"),
    )
    .await;
    let ollama_events = collect_stream(
        ollama
            .create_session()
            .expect("ollama session should open")
            .stream(swapped)
            .await
            .expect("ollama stream should start"),
    )
    .await;

    // Durable runtime semantics stay provider-neutral: identical normalized
    // output text and turn completion across both providers.
    let openai_response = ModelResponse::from_events(
        openai_events
            .into_iter()
            .map(|item| item.expect("openai stream item")),
    );
    let ollama_response = ModelResponse::from_events(
        ollama_events
            .into_iter()
            .map(|item| item.expect("ollama stream item")),
    );
    assert_eq!(openai_response.end_turn, ollama_response.end_turn);
    assert_eq!(
        text_of(&openai_response.output),
        text_of(&ollama_response.output)
    );
    assert_eq!(text_of(&openai_response.output), vec!["hello".to_string()]);

    // The wire bodies recorded by each backend carry the same semantic
    // content; only the model identity and provider transport differ.
    let openai_body = openai_server
        .received_requests()
        .await
        .expect("openai requests should be recorded")[0]
        .body_json::<Value>()
        .expect("openai request body should parse");
    let ollama_body = ollama_server
        .received_requests()
        .await
        .expect("ollama requests should be recorded")[0]
        .body_json::<Value>()
        .expect("ollama request body should parse");
    assert_eq!(openai_body["instructions"], ollama_body["instructions"]);
    assert_eq!(openai_body["input"], ollama_body["input"]);
    assert_eq!(openai_body["model"], ollama_body["model"]);

    openai_server.verify().await;
    ollama_server.verify().await;
}

fn text_of(output: &[codex_protocol::models::ResponseItem]) -> Vec<String> {
    use codex_protocol::models::ContentItem;
    output
        .iter()
        .flat_map(|item| match item {
            codex_protocol::models::ResponseItem::Message { content, .. } => content
                .iter()
                .filter_map(|content_item| match content_item {
                    ContentItem::OutputText { text } => Some(text.clone()),
                    _ => None,
                })
                .collect::<Vec<_>>(),
            _ => Vec::new(),
        })
        .collect()
}

/// Unsupported capabilities fail with explicit diagnostics before any request
/// reaches the provider.
#[tokio::test]
async fn unsupported_reasoning_effort_fails_explicitly_before_the_request() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/responses"))
        .respond_with(sse_template(successful_stream("resp-1", "hello")))
        .expect(0)
        .mount(&server)
        .await;

    let adapter = openai_adapter(&server.uri(), vec![text_only_model_json("local-model")]);
    let session = adapter.create_session().expect("session should open");

    let mut request = semantic_request("openai", "local-model");
    request.reasoning = Some(ModelReasoningControls {
        effort: Some(ReasoningEffort::High),
        summary: None,
    });
    let error = session
        .stream(request)
        .await
        .expect_err("unsupported effort should fail");
    let ModelError::UnsupportedCapability(unsupported) = &error else {
        panic!("expected capability diagnostics, got {error}");
    };
    assert_eq!(unsupported.provider_id, "openai");
    assert_eq!(unsupported.model_id, "local-model");
    assert_eq!(
        unsupported.supported_alternatives,
        vec!["medium".to_string()],
        "diagnostics should list supported alternatives"
    );
    assert!(!error.is_retryable());

    server.verify().await;
}

/// A session serves exactly the provider it was created for; mis-addressed
/// requests fail explicitly instead of silently hitting the wrong backend.
#[tokio::test]
async fn session_rejects_requests_addressed_to_another_provider() {
    let server = MockServer::start().await;

    let adapter = openai_adapter(&server.uri(), vec![reasoning_model_json("gpt-5.6-luna")]);
    let session = adapter.create_session().expect("session should open");

    let error = session
        .stream(semantic_request("ollama", "gpt-5.6-luna"))
        .await
        .expect_err("cross-provider request should fail");
    let ModelError::InvalidRequest { message } = &error else {
        panic!("expected an invalid-request diagnostic, got {error}");
    };
    assert!(message.contains("ollama"), "message: {message}");
    assert!(message.contains("openai"), "message: {message}");
}

/// Unknown models resolve through fallback metadata, matching the runtime's
/// behavior for custom configured models.
#[tokio::test]
async fn unknown_model_uses_fallback_metadata() {
    let server = MockServer::start().await;
    mount_sse(&server, successful_stream("resp-fallback-1", "hello"), 1).await;

    let adapter = openai_adapter(&server.uri(), vec![reasoning_model_json("gpt-5.6-luna")]);
    let session = adapter.create_session().expect("session should open");

    // No reasoning controls requested, so fallback metadata negotiates fine.
    let stream = session
        .stream(semantic_request("openai", "custom-undocumented-model"))
        .await
        .expect("fallback model should stream");
    let events = collect_stream(stream).await;
    // Rate limits snapshot + created/delta/item done/completed.
    assert_eq!(events.len(), 5);

    server.verify().await;
}

/// Unauthorized responses surface as normalized auth errors after the
/// provider-owned recovery path declines to recover (single attempt).
#[tokio::test]
async fn unauthorized_failure_surfaces_as_auth_error() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/responses"))
        .respond_with(ResponseTemplate::new(401).set_body_string("unauthorized"))
        .expect(1)
        .mount(&server)
        .await;

    let adapter = openai_adapter(&server.uri(), vec![reasoning_model_json("gpt-5.6-luna")]);
    let session = adapter.create_session().expect("session should open");

    let error = session
        .stream(semantic_request("openai", "gpt-5.6-luna"))
        .await
        .expect_err("unauthorized should fail");
    assert!(
        matches!(error, ModelError::Auth { .. }),
        "expected auth error, got {error}"
    );
    assert!(!error.is_retryable());

    server.verify().await;
}

/// Transport-level retries (the provider's configured retry policy inside the
/// codex-api endpoint session) are reused: a transient 5xx is retried and the
/// turn succeeds.
#[tokio::test]
async fn transient_server_error_is_retried_by_the_transport() {
    let server = MockServer::start().await;
    // Mount order matters: later-mounted mocks match first, so the transient
    // 5xx is served once, then the SSE response takes over.
    Mock::given(method("POST"))
        .and(path("/responses"))
        .respond_with(sse_template(successful_stream("resp-openai-1", "hello")))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/responses"))
        .and(header("authorization", "Bearer test-openai-key"))
        .respond_with(ResponseTemplate::new(500).set_body_string("transient"))
        .up_to_n_times(1)
        .mount(&server)
        .await;

    let adapter = openai_adapter(&server.uri(), vec![reasoning_model_json("gpt-5.6-luna")]);
    let session = adapter.create_session().expect("session should open");

    let stream = session
        .stream(semantic_request("openai", "gpt-5.6-luna"))
        .await
        .expect("retry should succeed");
    let events = collect_stream(stream).await;
    // Rate limits snapshot + created/delta/item done/completed.
    assert_eq!(events.len(), 5);

    server.verify().await;
}

/// Cancellation is normalized through stream ownership: dropping the
/// `ModelStream` stops the forwarder, which drops the runtime stream (and
/// with it the provider request).
#[tokio::test]
async fn dropping_the_model_stream_cancels_forwarding() {
    let (event_tx, event_rx) = mpsc::channel::<Result<ResponseEvent, codex_api::ApiError>>(16);
    let response_stream = ResponseStream {
        rx_event: event_rx,
        upstream_request_id: Some("req-9".to_string()),
    };

    let mut model_stream = forward_response_stream(response_stream);
    assert_eq!(model_stream.request_id.as_deref(), Some("req-9"));

    event_tx
        .send(Ok(ResponseEvent::Created {
            response_id: Some("resp-1".to_string()),
        }))
        .await
        .expect("created event should send");
    let first = tokio::time::timeout(TEST_TIMEOUT, model_stream.next())
        .await
        .expect("first event should arrive")
        .expect("stream should not end yet");
    assert!(first.is_ok());

    // Cancel by ownership: dropping the contract stream must tear down the
    // forwarder and the runtime stream it owns.
    drop(model_stream);

    // The forwarder only observes the drop when it tries to forward the next
    // event; after that, the runtime event channel closes.
    event_tx
        .send(Ok(ResponseEvent::OutputTextDelta("late".to_string())))
        .await
        .expect("late event should be accepted into the channel");

    let deadline = std::time::Instant::now() + TEST_TIMEOUT;
    while !event_tx.is_closed() {
        assert!(
            std::time::Instant::now() < deadline,
            "forwarder should exit after the model stream is dropped"
        );
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

/// Sessions cannot be created without a model catalog: the failure is
/// explicit, not a non-functional session.
#[tokio::test]
async fn create_session_requires_a_model_catalog() {
    let server = MockServer::start().await;
    let adapter = ModelContractAdapter::new(
        "openai",
        create_model_provider(
            ModelProviderInfo::create_openai_provider(Some(server.uri())),
            /*auth_manager*/ None,
        ),
    );

    let error = adapter
        .create_session()
        .expect_err("session creation should fail without a catalog");
    let ModelError::InvalidRequest { message } = &error else {
        panic!("expected an invalid-request diagnostic, got {error}");
    };
    assert!(message.contains("model catalog"), "message: {message}");
}

/// Server-side failures inside the SSE stream surface through the normalized
/// error taxonomy.
#[tokio::test]
async fn in_stream_failures_surface_as_normalized_errors() {
    let server = MockServer::start().await;
    let failed = sse_body(&[
        json!({
            "type": "response.created",
            "response": { "id": "resp-1" }
        }),
        json!({
            "type": "response.failed",
            "response": {
                "id": "resp-1",
                "error": { "code": "rate_limit_exceeded", "message": "slow down" }
            }
        }),
    ]);
    mount_sse(&server, failed, 1).await;

    let adapter = openai_adapter(&server.uri(), vec![reasoning_model_json("gpt-5.6-luna")]);
    let session = adapter.create_session().expect("session should open");

    let stream = session
        .stream(semantic_request("openai", "gpt-5.6-luna"))
        .await
        .expect("stream should start");
    let events = collect_stream(stream).await;
    // Rate limits snapshot, created, then the in-stream failure.
    assert_eq!(events.len(), 3);
    let error = events[2]
        .as_ref()
        .expect_err("stream should surface the failure");
    assert!(
        matches!(error, ModelError::RateLimited { .. }),
        "expected rate limit, got {error}"
    );
    assert!(error.is_retryable());

    server.verify().await;
}
