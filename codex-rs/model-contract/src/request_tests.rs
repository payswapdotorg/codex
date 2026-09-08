use pretty_assertions::assert_eq;
use serde_json::json;

use super::*;
use codex_protocol::models::ContentItem;
use codex_protocol::models::ResponseItem;
use codex_protocol::openai_models::ReasoningEffort;
use codex_tools::JsonSchema;
use codex_tools::ResponsesApiTool;
use codex_tools::ToolSpec;

fn sample_request() -> ModelRequest {
    ModelRequest {
        model: ModelDescriptor::new("openai", "gpt-5.6-luna"),
        instructions: "You are a helpful assistant.".to_string(),
        input: vec![user_message("hello")],
        tools: Arc::from(vec![sample_tool()]),
        tool_choice: ModelToolChoice::Auto,
        parallel_tool_calls: false,
        reasoning: Some(ModelReasoningControls {
            effort: Some(ReasoningEffort::Medium),
            summary: None,
        }),
        output_schema: None,
        output_schema_strict: true,
        verbosity: None,
        store: false,
        stream: true,
        service_tier: None,
        prompt_cache_key: Some("thread-1".to_string()),
    }
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

#[test]
fn request_serializes_without_provider_protocol_fields() {
    let value = serde_json::to_value(sample_request()).expect("request should serialize");

    // The serialized request may only carry neutral semantic fields. The
    // presence of provider configuration (base URLs, wire APIs, headers,
    // auth) would mean provider types crossed the universal boundary.
    let keys: std::collections::BTreeSet<&str> = value
        .as_object()
        .expect("request should serialize to an object")
        .keys()
        .map(String::as_str)
        .collect();
    let expected_keys: std::collections::BTreeSet<&str> = [
        "model",
        "instructions",
        "input",
        "tools",
        "tool_choice",
        "parallel_tool_calls",
        "reasoning",
        "output_schema",
        "output_schema_strict",
        "verbosity",
        "store",
        "stream",
        "service_tier",
        "prompt_cache_key",
    ]
    .into_iter()
    .collect();
    assert_eq!(keys, expected_keys);

    for forbidden in [
        "base_url",
        "wire_api",
        "headers",
        "env_key",
        "auth",
        "api_key",
        "token",
        "query_params",
    ] {
        assert!(
            !value.as_object().expect("object").contains_key(forbidden),
            "serialized request must not contain provider field {forbidden}"
        );
    }
}

#[test]
fn for_provider_changes_only_provider_identity() {
    let openai = sample_request();
    let bedrock = openai.for_provider("amazon-bedrock");

    assert_eq!(bedrock.model.provider_id, "amazon-bedrock");
    assert_eq!(bedrock.model.model_id, openai.model.model_id);
    assert_eq!(bedrock.instructions, openai.instructions);
    assert_eq!(bedrock.input, openai.input);
    assert_eq!(bedrock.tools.as_ref(), openai.tools.as_ref());
    assert_eq!(bedrock.tool_choice, openai.tool_choice);
    assert_eq!(bedrock.reasoning, openai.reasoning);
    assert_eq!(bedrock.prompt_cache_key, openai.prompt_cache_key);

    // Semantics beyond identity must serialize identically.
    let openai_value = serde_json::to_value(&openai).expect("should serialize");
    let bedrock_value = serde_json::to_value(&bedrock).expect("should serialize");
    for key in [
        "instructions",
        "input",
        "tools",
        "tool_choice",
        "parallel_tool_calls",
        "reasoning",
        "store",
        "stream",
        "prompt_cache_key",
    ] {
        assert_eq!(
            openai_value.get(key),
            bedrock_value.get(key),
            "field {key} must not change across providers"
        );
    }
    assert_ne!(
        openai_value.get("model").and_then(|m| m.get("provider_id")),
        bedrock_value
            .get("model")
            .and_then(|m| m.get("provider_id"))
    );
}

#[test]
fn request_default_is_empty_streaming_request() {
    let request = ModelRequest::default();

    assert_eq!(request.model, ModelDescriptor::new("", ""));
    assert!(request.input.is_empty());
    assert!(request.tools.is_empty());
    assert_eq!(request.tool_choice, ModelToolChoice::Auto);
    assert!(request.stream);
    assert!(!request.store);
    assert!(request.output_schema.is_none());
    assert!(request.reasoning.is_none());
}
