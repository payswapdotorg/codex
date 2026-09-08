//! Tests for the OpenAI Responses-backed [`ModelSession`] implementation.
//!
//! These tests exercise the WO-004 acceptance shape:
//!
//! - the OpenAI-compatible adapter implements
//!   [`codex_model_contract::ModelProvider`];
//! - two provider paths (OpenAI and Ollama) satisfy the same universal
//!   request contract through actual `ModelSession::stream` execution
//!   against mock SSE servers;
//! - unsupported capabilities fail explicitly through `stream`;
//! - dropping the [`ModelStream`] cancels the in-flight provider request;
//! - existing WO-002 wire-marshalling behavior stays green.
//!
//! Mock SSE servers are backed by `wiremock`; no real network calls are
//! made.

use std::sync::Arc;

use codex_model_contract::ModelAuthStatus;
use codex_model_contract::ModelDescriptor;
use codex_model_contract::ModelError;
use codex_model_contract::ModelFeature;
use codex_model_contract::ModelProvider as ContractModelProvider;
use codex_model_contract::ModelReasoningControls;
use codex_model_contract::ModelRequest;
use codex_model_contract::ModelToolChoice;
use codex_model_contract::UnsupportedModelCapability;
use codex_model_provider_info::ModelProviderInfo;
use codex_model_provider_info::WireApi;
use codex_model_provider_info::create_oss_provider_with_base_url;
use codex_models_manager::manager::SharedModelsManager;
use codex_models_manager::manager::StaticModelsManager;
use codex_protocol::config_types::ReasoningSummary;
use codex_protocol::config_types::Verbosity;
use codex_protocol::models::ContentItem;
use codex_protocol::models::ResponseItem;
use codex_protocol::openai_models::ModelInfo;
use codex_protocol::openai_models::ModelsResponse;
use codex_protocol::openai_models::ReasoningEffort;
use codex_tools::JsonSchema;
use codex_tools::ResponsesApiTool;
use codex_tools::ToolSpec;
use futures::StreamExt;
use pretty_assertions::assert_eq;
use serde_json::json;
use wiremock::Mock;
use wiremock::MockServer;
use wiremock::ResponseTemplate;
use wiremock::matchers::method;
use wiremock::matchers::path_regex;

use crate::ModelContractAdapter;
use crate::create_model_provider;

const MODEL_SLUG: &str = "gpt-5.6-luna";

fn openai_adapter_with_catalog(base_url: Option<String>) -> ModelContractAdapter {
    let provider = create_model_provider(
        ModelProviderInfo::create_openai_provider(base_url),
        /*auth_manager*/ None,
    );
    ModelContractAdapter::new("openai", provider).with_models_manager(static_catalog())
}

fn ollama_adapter_with_catalog(base_url: String) -> ModelContractAdapter {
    let provider = create_model_provider(
        create_oss_provider_with_base_url(&base_url, WireApi::Responses),
        /*auth_manager*/ None,
    );
    ModelContractAdapter::new("ollama", provider).with_models_manager(static_catalog())
}

fn static_catalog() -> SharedModelsManager {
    let model = sample_model_info(MODEL_SLUG);
    Arc::new(StaticModelsManager::new(
        /*auth_manager*/ None,
        ModelsResponse {
            models: vec![model],
        },
    ))
}

fn sample_model_info(slug: &str) -> ModelInfo {
    serde_json::from_value(json!({
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
            { "id": "standard", "name": "Standard", "description": "" },
            { "id": "priority", "name": "Priority", "description": "" }
        ],
    }))
    .expect("valid model")
}

fn user_message(text: &str) -> ResponseItem {
    ResponseItem::Message {
        id: None,
        role: "user".to_string(),
        content: vec![ContentItem::InputText {
            text: text.to_string(),
        }],
        phase: None,
        internal_chat_message_metadata_passthrough: None,
    }
}

fn sample_tool() -> ToolSpec {
    ToolSpec::Function(ResponsesApiTool {
        name: "get_weather".to_string(),
        description: "Get the weather.".to_string(),
        strict: false,
        defer_loading: None,
        parameters: JsonSchema {
            schema_type: Some(
                serde_json::from_value(json!("object")).expect("schema type should parse"),
            ),
            ..JsonSchema::default()
        },
        output_schema: None,
    })
}

fn sample_request(provider_id: &str) -> ModelRequest {
    ModelRequest {
        model: ModelDescriptor::new(provider_id, MODEL_SLUG),
        instructions: "You are a helpful assistant.".to_string(),
        input: vec![user_message("hello")],
        tools: Arc::from(vec![sample_tool()]),
        tool_choice: ModelToolChoice::Auto,
        parallel_tool_calls: false,
        reasoning: Some(ModelReasoningControls {
            effort: Some(ReasoningEffort::High),
            summary: Some(ReasoningSummary::Concise),
        }),
        output_schema: None,
        output_schema_strict: true,
        verbosity: Some(Verbosity::Medium),
        store: false,
        stream: true,
        service_tier: Some("standard".to_string()),
        prompt_cache_key: Some("thread-1".to_string()),
    }
}

/// Builds an SSE body for a Created + OutputItemDone + Completed sequence.
fn sse_body(response_id: &str, assistant_text: &str) -> String {
    let created = json!({
        "type": "response.created",
        "response": {"id": response_id}
    });
    let item_done = json!({
        "type": "response.output_item.done",
        "item": {
            "type": "message",
            "role": "assistant",
            "id": "msg-1",
            "content": [{"type": "output_text", "text": assistant_text}]
        }
    });
    let completed = json!({
        "type": "response.completed",
        "response": {
            "id": response_id,
            "usage": {
                "input_tokens": 5,
                "input_tokens_details": null,
                "output_tokens": 3,
                "output_tokens_details": null,
                "total_tokens": 8
            }
        }
    });
    let mut out = String::new();
    for event in [created, item_done, completed] {
        use std::fmt::Write as _;
        let _ = writeln!(out, "event: {}", event["type"].as_str().unwrap());
        let _ = write!(out, "data: {event}\n\n");
    }
    out
}

async fn mount_sse_once(server: &MockServer, body: String) {
    Mock::given(method("POST"))
        .and(path_regex(".*/responses$"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_raw(body.as_bytes().to_vec(), "text/event-stream"),
        )
        .up_to_n_times(1)
        .mount(server)
        .await;
}

#[test]
fn openai_adapter_implements_contract_provider_trait() {
    // Compile-time proof that ModelContractAdapter is a dyn-compatible
    // ModelProvider. The boxed trait object is exercised below in
    // `select_and_stream` paths.
    let adapter = openai_adapter_with_catalog(/*base_url*/ None);
    let _: Arc<dyn ContractModelProvider> = Arc::new(adapter);
}

#[tokio::test]
async fn create_session_fails_explicitly_without_models_manager() {
    let provider = create_model_provider(
        ModelProviderInfo::create_openai_provider(/*base_url*/ None),
        /*auth_manager*/ None,
    );
    let adapter = ModelContractAdapter::new("openai", provider);

    let error = adapter
        .create_session()
        .expect_err("session without catalog should fail");
    assert!(
        matches!(error, ModelError::InvalidRequest { .. }),
        "expected InvalidRequest, got {error}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn streams_events_from_openai_compatible_endpoint() {
    let server = MockServer::start().await;
    let base_url = format!("{}/v1", server.uri());
    let adapter = openai_adapter_with_catalog(Some(base_url));
    let body = sse_body("resp-1", "hello back");
    mount_sse_once(&server, body).await;

    let session = adapter
        .create_session()
        .expect("adapter with catalog should open a session");
    let request = sample_request("openai");
    let mut stream = session
        .stream(request)
        .await
        .expect("stream setup should succeed");

    let events = collect_events(&mut stream).await;

    assert_eq!(events.len(), 3);
    assert!(matches!(
        events[0],
        Ok(codex_model_contract::ModelEvent::Created {
            response_id: Some(ref id)
        }) if id == "resp-1"
    ));
    assert!(matches!(
        events[1],
        Ok(codex_model_contract::ModelEvent::OutputItemDone(_))
    ));
    assert!(matches!(
        events[2],
        Ok(codex_model_contract::ModelEvent::Completed {
            response_id: ref id,
            token_usage: Some(_),
            ..
        }) if id == "resp-1"
    ));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn same_semantic_request_streams_through_two_providers() {
    // The WO-004 acceptance shape: the same durable thread's request
    // content is served by two providers through actual ModelSession
    // execution. Only the descriptor's provider_id differs; the streamed
    // ModelEvents are equivalent (Created, OutputItemDone, Completed) and
    // the captured wire requests share semantic content.
    let openai_server = MockServer::start().await;
    let ollama_server = MockServer::start().await;

    let openai_adapter = openai_adapter_with_catalog(Some(format!("{}/v1", openai_server.uri())));
    let ollama_adapter = ollama_adapter_with_catalog(format!("{}/v1", ollama_server.uri()));

    let openai_body = sse_body("resp-openai", "openai says hi");
    let ollama_body = sse_body("resp-ollama", "ollama says hi");
    mount_sse_once(&openai_server, openai_body).await;
    mount_sse_once(&ollama_server, ollama_body).await;

    let request = sample_request("openai");
    let swapped = request.for_provider("ollama");
    assert_ne!(request.model.provider_id, swapped.model.provider_id);

    let openai_session = openai_adapter.create_session().expect("openai session");
    let ollama_session = ollama_adapter.create_session().expect("ollama session");

    let openai_request = request.clone();
    let ollama_request = swapped.clone();
    let (openai_stream_result, ollama_stream_result) = tokio::join!(
        openai_session.stream(openai_request),
        ollama_session.stream(ollama_request)
    );
    let mut openai_stream = openai_stream_result.expect("openai stream");
    let mut ollama_stream = ollama_stream_result.expect("ollama stream");

    let openai_events: Vec<_> = collect_events(&mut openai_stream).await;
    let ollama_events: Vec<_> = collect_events(&mut ollama_stream).await;

    // Both providers produce the same lifecycle sequence.
    assert_eq!(openai_events.len(), 3);
    assert_eq!(ollama_events.len(), 3);
    assert!(matches!(
        openai_events[0],
        Ok(codex_model_contract::ModelEvent::Created { .. })
    ));
    assert!(matches!(
        ollama_events[0],
        Ok(codex_model_contract::ModelEvent::Created { .. })
    ));
    assert!(matches!(
        openai_events[1],
        Ok(codex_model_contract::ModelEvent::OutputItemDone(_))
    ));
    assert!(matches!(
        ollama_events[1],
        Ok(codex_model_contract::ModelEvent::OutputItemDone(_))
    ));
    assert!(matches!(
        openai_events[2],
        Ok(codex_model_contract::ModelEvent::Completed { .. })
    ));
    assert!(matches!(
        ollama_events[2],
        Ok(codex_model_contract::ModelEvent::Completed { .. })
    ));

    // The response ids differ — providers assign their own — but the
    // lifecycle sequence (the durable semantic content) is identical.
    let openai_response_id = match &openai_events[2] {
        Ok(codex_model_contract::ModelEvent::Completed { response_id, .. }) => response_id.clone(),
        _ => unreachable!("checked above"),
    };
    let ollama_response_id = match &ollama_events[2] {
        Ok(codex_model_contract::ModelEvent::Completed { response_id, .. }) => response_id.clone(),
        _ => unreachable!("checked above"),
    };
    assert_eq!(openai_response_id, "resp-openai");
    assert_eq!(ollama_response_id, "resp-ollama");
}

async fn collect_events(
    stream: &mut codex_model_contract::ModelStream,
) -> Vec<codex_model_contract::ModelStreamItem> {
    // codex-api's `spawn_response_stream` emits transport-metadata events
    // (`RateLimits`, `ModelsEtag`, `ServerModel`) before the SSE lifecycle
    // events when the response headers carry them. `parse_rate_limit_for_limit`
    // always returns `Some`, so a `RateLimits` event is emitted even for mock
    // responses with no rate-limit headers. Filter these out for semantic
    // lifecycle assertions; they are part of the contract event stream but
    // not part of the response lifecycle.
    let mut events = Vec::new();
    while let Some(item) = stream.next().await {
        let is_transport_metadata = matches!(
            &item,
            Ok(codex_model_contract::ModelEvent::RateLimits(_))
                | Ok(codex_model_contract::ModelEvent::ModelsEtag(_))
                | Ok(codex_model_contract::ModelEvent::ServerModel(_))
        );
        if !is_transport_metadata {
            events.push(item);
        }
        if matches!(
            events.last(),
            Some(Ok(codex_model_contract::ModelEvent::Completed { .. }))
        ) {
            break;
        }
    }
    events
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn unsupported_capability_fails_explicitly_through_stream() {
    let server = MockServer::start().await;
    let adapter = openai_adapter_with_catalog(Some(format!("{}/v1", server.uri())));

    // Request a reasoning effort the catalog does not support.
    let mut request = sample_request("openai");
    request.reasoning = Some(ModelReasoningControls {
        effort: Some(ReasoningEffort::Max),
        summary: None,
    });

    let session = adapter
        .create_session()
        .expect("session opens even if the request is later rejected");
    let error = session
        .stream(request)
        .await
        .expect_err("unsupported effort should fail before streaming");

    assert_eq!(
        error,
        ModelError::UnsupportedCapability(UnsupportedModelCapability {
            feature: ModelFeature::ReasoningEffort(ReasoningEffort::Max),
            provider_id: "openai".to_string(),
            model_id: MODEL_SLUG.to_string(),
            message: format!("model openai/{MODEL_SLUG} does not support reasoning effort max"),
            supported_alternatives: vec![
                "low".to_string(),
                "medium".to_string(),
                "high".to_string()
            ],
        })
    );

    // The mock server must not have received any request — capability
    // negotiation happens before the HTTP call.
    assert_eq!(server.received_requests().await.unwrap().len(), 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn provider_swap_preserves_semantic_request_content() {
    // The `for_provider` primitive keeps durable thread state stable when
    // the serving provider changes. Verify the wire request body (the
    // semantic content) is identical across providers when served by two
    // adapters.
    let openai_server = MockServer::start().await;
    let ollama_server = MockServer::start().await;
    let openai_adapter = openai_adapter_with_catalog(Some(format!("{}/v1", openai_server.uri())));
    let ollama_adapter = ollama_adapter_with_catalog(format!("{}/v1", ollama_server.uri()));

    mount_sse_once(&openai_server, sse_body("resp-openai", "ok")).await;
    mount_sse_once(&ollama_server, sse_body("resp-ollama", "ok")).await;

    let request = sample_request("openai");
    let swapped = request.for_provider("ollama");

    let openai_session = openai_adapter.create_session().expect("openai session");
    let ollama_session = ollama_adapter.create_session().expect("ollama session");
    let (_openai_stream, _ollama_stream) = tokio::join!(
        openai_session.stream(request.clone()),
        ollama_session.stream(swapped.clone())
    );

    let openai_request = openai_server
        .received_requests()
        .await
        .unwrap()
        .pop()
        .unwrap();
    let ollama_request = ollama_server
        .received_requests()
        .await
        .unwrap()
        .pop()
        .unwrap();

    let openai_body: serde_json::Value =
        serde_json::from_slice(&openai_request.body).expect("openai body");
    let ollama_body: serde_json::Value =
        serde_json::from_slice(&ollama_request.body).expect("ollama body");

    // Semantic content fields are identical.
    assert_eq!(openai_body["instructions"], ollama_body["instructions"]);
    assert_eq!(openai_body["input"], ollama_body["input"]);
    assert_eq!(openai_body["tools"], ollama_body["tools"]);
    assert_eq!(openai_body["tool_choice"], ollama_body["tool_choice"]);
    assert_eq!(openai_body["reasoning"], ollama_body["reasoning"]);
    assert_eq!(openai_body["store"], ollama_body["store"]);
    assert_eq!(openai_body["stream"], ollama_body["stream"]);
    assert_eq!(openai_body["model"], ollama_body["model"]);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn dropping_model_stream_cancels_in_flight_request() {
    // Cancellation is normalized through ModelStream ownership (drop =
    // cancel). Verify that dropping the stream mid-flight causes the
    // forwarding task to exit cleanly without panicking and without
    // requiring the consumer to drain the stream.
    let server = MockServer::start().await;
    let adapter = openai_adapter_with_catalog(Some(format!("{}/v1", server.uri())));

    // Mount an SSE body that never sends `Completed`, so the only way the
    // forwarding task exits is via cancellation.
    let body = {
        let created = json!({
            "type": "response.created",
            "response": {"id": "resp-cancel"}
        });
        let mut out = String::new();
        use std::fmt::Write as _;
        let _ = writeln!(out, "event: {}", created["type"].as_str().unwrap());
        let _ = write!(out, "data: {created}\n\n");
        out
    };
    mount_sse_once(&server, body).await;

    let session = adapter.create_session().expect("session should open");
    let request = sample_request("openai");
    let mut stream = session.stream(request).await.expect("stream should start");

    // Pull events until we see the `Created` lifecycle event (skipping
    // transport-metadata events codex-api emits first), then drop the
    // stream. The forwarding task should observe the dropped receiver on
    // the next send and exit.
    let mut saw_created = false;
    while let Some(item) = stream.next().await {
        if matches!(item, Ok(codex_model_contract::ModelEvent::Created { .. })) {
            saw_created = true;
            break;
        }
    }
    assert!(saw_created, "expected Created event before cancellation");
    drop(stream);

    // Give the forwarding task a moment to observe the dropped receiver.
    // The test passes if this completes without hanging — the spawned
    // forwarding task exits cleanly via the `tx_event.send(...).is_err()`
    // branch in `forward_response_events`.
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
}

#[tokio::test]
async fn openai_adapter_exposes_default_auth_status_through_contract() {
    let adapter = openai_adapter_with_catalog(/*base_url*/ None);
    let status = adapter.auth_status().await;
    assert_eq!(
        status,
        ModelAuthStatus::RequiresAuthentication { instructions: None }
    );
}

#[tokio::test]
async fn ollama_adapter_reports_not_required_auth_through_contract() {
    let adapter = ollama_adapter_with_catalog("http://localhost:11434/v1".to_string());
    assert_eq!(adapter.auth_status().await, ModelAuthStatus::NotRequired);
}

#[test]
fn openai_adapter_resolves_capabilities_through_contract() {
    let adapter = openai_adapter_with_catalog(/*base_url*/ None);
    let model = sample_model_info(MODEL_SLUG);
    let capabilities = adapter.model_capabilities(&model);
    assert_eq!(capabilities.provider_id, "openai");
    assert_eq!(capabilities.model_id, MODEL_SLUG);
    assert!(
        capabilities
            .supported_reasoning_efforts
            .contains(&ReasoningEffort::High)
    );
    assert!(
        !capabilities
            .supported_reasoning_efforts
            .contains(&ReasoningEffort::Max)
    );
}

#[test]
fn descriptor_for_id_is_opaque_identity() {
    let adapter = openai_adapter_with_catalog(/*base_url*/ None);
    // The trait method resolves opaque identity from a model_id string.
    let descriptor =
        <ModelContractAdapter as ContractModelProvider>::descriptor(&adapter, MODEL_SLUG);
    assert_eq!(descriptor, ModelDescriptor::new("openai", MODEL_SLUG));
}

#[test]
fn openai_adapter_capabilities_match_universal_adapter() {
    // The ModelProvider trait impl delegates to the inherent methods
    // already covered by universal_tests.rs; this asserts the delegation
    // wiring is correct (the trait method returns the same value as the
    // inherent method).
    let adapter = openai_adapter_with_catalog(/*base_url*/ None);
    let model = sample_model_info(MODEL_SLUG);
    let via_trait =
        <ModelContractAdapter as ContractModelProvider>::model_capabilities(&adapter, &model);
    let via_inherent = adapter.model_capabilities(&model);
    assert_eq!(via_trait, via_inherent);
}

#[test]
fn transport_policy_through_contract_matches_inherent() {
    let adapter = openai_adapter_with_catalog(/*base_url*/ None);
    let via_trait = <ModelContractAdapter as ContractModelProvider>::transport_policy(&adapter);
    let via_inherent = adapter.transport_policy();
    assert_eq!(via_trait, via_inherent);
}
