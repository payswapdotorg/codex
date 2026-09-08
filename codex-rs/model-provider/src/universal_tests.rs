use std::sync::Arc;

use codex_api::ApiError;
use codex_api::ResponseEvent;
use codex_api::SafetyBuffering;
use codex_api::TransportError;
use codex_login::AuthManager;
use codex_login::CodexAuth;
use codex_model_contract::ModelAuthStatus;
use codex_model_contract::ModelCapabilities;
use codex_model_contract::ModelDescriptor;
use codex_model_contract::ModelError;
use codex_model_contract::ModelFeature;
use codex_model_contract::ModelReasoningControls;
use codex_model_contract::ModelRequest;
use codex_model_contract::ModelToolChoice;
use codex_model_contract::ModelTransportPolicy;
use codex_model_contract::UnsupportedModelCapability;
use codex_model_provider_info::ModelProviderInfo;
use codex_model_provider_info::WireApi;
use codex_model_provider_info::create_oss_provider_with_base_url;
use codex_protocol::config_types::ReasoningSummary;
use codex_protocol::config_types::Verbosity;
use codex_protocol::models::ContentItem;
use codex_protocol::models::ResponseItem;
use codex_protocol::openai_models::ReasoningEffort;
use codex_tools::JsonSchema;
use codex_tools::ResponsesApiTool;
use codex_tools::ToolSpec;
use pretty_assertions::assert_eq;
use serde_json::json;

use super::*;
use crate::RemoteCompactionSupport;
use crate::create_model_provider;

fn openai_adapter() -> ModelContractAdapter {
    ModelContractAdapter::new(
        "openai",
        create_model_provider(
            ModelProviderInfo::create_openai_provider(/*base_url*/ None),
            /*auth_manager*/ None,
        ),
    )
}

fn adapter_for(provider_id: &str, info: ModelProviderInfo) -> ModelContractAdapter {
    ModelContractAdapter::new(
        provider_id,
        create_model_provider(info, /*auth_manager*/ None),
    )
}

fn model_info(slug: &str) -> ModelInfo {
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

fn text_only_model_info(slug: &str) -> ModelInfo {
    serde_json::from_value(json!({
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

fn image_message() -> ResponseItem {
    ResponseItem::Message {
        id: None,
        role: "user".to_string(),
        content: vec![ContentItem::InputImage {
            image_url: "https://example.test/cat.png".to_string(),
            detail: None,
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

fn sample_request(model: &ModelInfo) -> ModelRequest {
    ModelRequest {
        model: ModelDescriptor::new("openai", model.slug.clone()),
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

#[test]
fn openai_capability_matrix() {
    let adapter = openai_adapter();
    let model = model_info("gpt-5.6-luna");

    assert_eq!(
        adapter.model_capabilities(&model),
        ModelCapabilities {
            provider_id: "openai".to_string(),
            model_id: "gpt-5.6-luna".to_string(),
            supported_reasoning_efforts: vec![
                ReasoningEffort::Low,
                ReasoningEffort::Medium,
                ReasoningEffort::High
            ],
            supports_reasoning_summary: true,
            supports_verbosity: true,
            input_modalities: model.input_modalities.clone(),
            context_window: Some(272_000),
            service_tiers: vec!["standard".to_string(), "priority".to_string()],
            supports_image_detail_original: false,
            namespaced_tools: true,
            image_generation: true,
            hosted_web_search: true,
            standalone_web_search: true,
            external_web_access: true,
            remote_compaction: RemoteCompactionSupport::V2,
        }
    );
}

#[test]
fn non_openai_capability_matrix() {
    let ollama = adapter_for(
        "ollama",
        create_oss_provider_with_base_url("http://localhost:11434/v1", WireApi::Responses),
    );
    let model = model_info("local-model");

    assert_eq!(
        ollama.model_capabilities(&model),
        ModelCapabilities {
            provider_id: "ollama".to_string(),
            model_id: "local-model".to_string(),
            supported_reasoning_efforts: vec![
                ReasoningEffort::Low,
                ReasoningEffort::Medium,
                ReasoningEffort::High
            ],
            supports_reasoning_summary: true,
            supports_verbosity: true,
            input_modalities: model.input_modalities.clone(),
            context_window: Some(272_000),
            service_tiers: vec!["standard".to_string(), "priority".to_string()],
            supports_image_detail_original: false,
            namespaced_tools: true,
            image_generation: true,
            hosted_web_search: true,
            standalone_web_search: false,
            external_web_access: true,
            remote_compaction: RemoteCompactionSupport::Unsupported,
        }
    );
}

#[test]
fn bedrock_capability_matrix() {
    let bedrock = adapter_for(
        "amazon-bedrock",
        ModelProviderInfo::create_amazon_bedrock_provider(/*aws*/ None),
    );
    let model = model_info("openai.gpt-5.6-luna");

    let capabilities = bedrock.model_capabilities(&model);
    assert!(!capabilities.image_generation);
    assert!(!capabilities.external_web_access);
    assert_eq!(capabilities.remote_compaction, RemoteCompactionSupport::V2);
    assert_eq!(capabilities.provider_id, "amazon-bedrock");
}

#[test]
fn descriptor_is_opaque_identity() {
    let adapter = openai_adapter();
    let model = model_info("gpt-5.6-luna");

    assert_eq!(
        adapter.descriptor(&model),
        ModelDescriptor::new("openai", "gpt-5.6-luna").with_display_name("gpt-5.6-luna")
    );
}

#[test]
fn transport_policy_mirrors_provider_info() {
    let openai = openai_adapter();
    assert_eq!(
        openai.transport_policy(),
        ModelTransportPolicy {
            request_max_retries: 4,
            stream_max_retries: 5,
            stream_idle_timeout: std::time::Duration::from_millis(300_000),
            websocket_connect_timeout: Some(std::time::Duration::from_millis(15_000)),
            supports_websockets: true,
        }
    );

    let mut custom = create_oss_provider_with_base_url("http://localhost:1/v1", WireApi::Responses);
    custom.supports_websockets = false;
    let oss = adapter_for("ollama", custom);
    assert_eq!(
        oss.transport_policy(),
        ModelTransportPolicy {
            request_max_retries: 4,
            stream_max_retries: 5,
            stream_idle_timeout: std::time::Duration::from_millis(300_000),
            websocket_connect_timeout: Some(std::time::Duration::from_millis(15_000)),
            supports_websockets: false,
        }
    );
}

#[tokio::test]
async fn auth_status_requires_authentication_when_unauthenticated() {
    let openai = openai_adapter();
    assert_eq!(
        openai.auth_status().await,
        ModelAuthStatus::RequiresAuthentication { instructions: None }
    );
}

#[tokio::test]
async fn auth_status_authenticated_for_api_key() {
    let adapter = ModelContractAdapter::new(
        "openai",
        create_model_provider(
            ModelProviderInfo::create_openai_provider(/*base_url*/ None),
            Some(AuthManager::from_auth_for_testing(CodexAuth::from_api_key(
                "openai-api-key",
            ))),
        ),
    );

    assert_eq!(
        adapter.auth_status().await,
        ModelAuthStatus::Authenticated { account: None }
    );
}

#[tokio::test]
async fn auth_status_not_required_for_local_provider() {
    let ollama = adapter_for(
        "ollama",
        create_oss_provider_with_base_url("http://localhost:11434/v1", WireApi::Responses),
    );

    assert_eq!(ollama.auth_status().await, ModelAuthStatus::NotRequired);
}

#[test]
fn builds_openai_responses_wire_request() {
    let adapter = openai_adapter();
    let model = model_info("gpt-5.6-luna");
    let request = sample_request(&model);

    let wire = adapter
        .build_responses_request(&request, &model)
        .expect("supported request should build");

    assert_eq!(wire.model, "gpt-5.6-luna");
    assert_eq!(wire.instructions, "You are a helpful assistant.");
    assert_eq!(wire.input, request.input);
    assert_eq!(wire.tool_choice, "auto");
    assert!(!wire.parallel_tool_calls);
    assert_eq!(
        wire.reasoning,
        Some(codex_api::Reasoning {
            effort: Some(ReasoningEffort::High),
            summary: Some(ReasoningSummary::Concise),
            context: None,
        })
    );
    assert!(!wire.store);
    assert!(wire.stream);
    assert_eq!(wire.stream_options, None);
    assert_eq!(
        wire.include,
        vec!["reasoning.encrypted_content".to_string()]
    );
    assert_eq!(wire.service_tier, Some("standard".to_string()));
    assert_eq!(wire.prompt_cache_key, Some("thread-1".to_string()));
    assert_eq!(
        wire.text,
        Some(codex_api::TextControls {
            verbosity: Some(codex_api::OpenAiVerbosity::Medium),
            format: None,
        })
    );
    assert_eq!(wire.client_metadata, None);
    assert_eq!(wire.access_programs, None);

    let tools = wire.tools.expect("tools should serialize");
    let tools_json: serde_json::Value =
        serde_json::to_value(&tools).expect("tools should be valid JSON");
    assert_eq!(
        tools_json,
        json!([{
            "type": "function",
            "name": "get_weather",
            "description": "Get the weather.",
            "strict": false,
            "parameters": { "type": "object" },
        }])
    );
}

#[test]
fn same_semantic_request_serves_two_providers_without_semantic_change() {
    // The acceptance shape for provider swap: the same durable thread's
    // request content is served by a second provider with only the
    // descriptor changing; wire marshalling differences (internal metadata
    // scrubbing) never touch semantic content.
    let openai = openai_adapter();
    let ollama = adapter_for(
        "ollama",
        create_oss_provider_with_base_url("http://localhost:11434/v1", WireApi::Responses),
    );
    let model = model_info("gpt-5.6-luna");
    let request = sample_request(&model);
    let swapped = request.for_provider("ollama");

    let openai_wire = openai
        .build_responses_request(&request, &model)
        .expect("openai request should build");
    let ollama_wire = ollama
        .build_responses_request(&swapped, &model)
        .expect("ollama request should build");

    // Semantic content is identical across providers.
    assert_eq!(openai_wire.instructions, ollama_wire.instructions);
    assert_eq!(openai_wire.input, ollama_wire.input);
    assert_eq!(
        openai_wire
            .tools
            .map(|tools| serde_json::to_string(&tools).expect("tools should serialize")),
        ollama_wire
            .tools
            .map(|tools| serde_json::to_string(&tools).expect("tools should serialize"))
    );
    assert_eq!(openai_wire.tool_choice, ollama_wire.tool_choice);
    assert_eq!(openai_wire.reasoning, ollama_wire.reasoning);
    assert_eq!(openai_wire.store, ollama_wire.store);
    assert_eq!(openai_wire.stream, ollama_wire.stream);

    // The provider-facing model identity is a plain string in both cases;
    // provider-specific configuration (endpoints, headers, auth) stays out
    // of the wire request body entirely.
    assert_eq!(openai_wire.model, ollama_wire.model);
    assert_ne!(request.model.provider_id, swapped.model.provider_id);
}

#[test]
fn unsupported_reasoning_effort_fails_with_diagnostics() {
    let adapter = openai_adapter();
    let model = model_info("gpt-5.6-luna");
    let mut request = sample_request(&model);
    request.reasoning = Some(ModelReasoningControls {
        effort: Some(ReasoningEffort::Max),
        summary: None,
    });

    let error = adapter
        .build_responses_request(&request, &model)
        .expect_err("unsupported effort should fail");

    assert_eq!(
        error,
        ModelError::UnsupportedCapability(UnsupportedModelCapability {
            feature: ModelFeature::ReasoningEffort(ReasoningEffort::Max),
            provider_id: "openai".to_string(),
            model_id: "gpt-5.6-luna".to_string(),
            message: "model openai/gpt-5.6-luna does not support reasoning effort max".to_string(),
            supported_alternatives: vec![
                "low".to_string(),
                "medium".to_string(),
                "high".to_string()
            ],
        })
    );
}

#[test]
fn unsupported_controls_and_modalities_fail_explicitly() {
    let adapter = openai_adapter();
    let model = text_only_model_info("text-only-model");

    // Reasoning summary is unsupported for this model in the catalog sense
    // only when the parameter is absent; text-only models exercise the
    // modality and control failures here.
    let mut image_request = sample_request(&model);
    image_request.input = vec![image_message()];
    image_request.reasoning = None;
    image_request.verbosity = None;
    image_request.service_tier = None;
    let error = adapter
        .build_responses_request(&image_request, &model)
        .expect_err("image input should fail");
    assert!(
        matches!(
            error,
            ModelError::UnsupportedCapability(ref unsupported)
                if unsupported.feature == ModelFeature::ImageInput
        ),
        "expected image input diagnostics, got {error}"
    );

    let mut verbosity_request = sample_request(&model);
    verbosity_request.input = vec![user_message("hi")];
    verbosity_request.verbosity = Some(Verbosity::High);
    verbosity_request.reasoning = None;
    let error = adapter
        .build_responses_request(&verbosity_request, &model)
        .expect_err("verbosity should fail");
    assert!(
        matches!(
            error,
            ModelError::UnsupportedCapability(ref unsupported)
                if unsupported.feature == ModelFeature::Verbosity
        ),
        "expected verbosity diagnostics, got {error}"
    );

    let mut tier_request = sample_request(&model);
    tier_request.input = vec![user_message("hi")];
    tier_request.reasoning = None;
    tier_request.verbosity = None;
    tier_request.service_tier = Some("flex".to_string());
    let error = adapter
        .build_responses_request(&tier_request, &model)
        .expect_err("service tier should fail");
    assert!(
        matches!(
            &error,
            ModelError::UnsupportedCapability(unsupported)
                if matches!(&unsupported.feature, ModelFeature::ServiceTier(_))
        ),
        "expected service tier diagnostics, got {error}"
    );
}

#[test]
fn exhaustive_response_event_mapping() {
    let moderation = codex_protocol::protocol::TurnModerationMetadataEvent {
        metadata: serde_json::json!({"policy": "test"}),
    };
    let events = vec![
        (
            ResponseEvent::Created {
                response_id: Some("resp-1".to_string()),
            },
            ModelEvent::Created {
                response_id: Some("resp-1".to_string()),
            },
        ),
        (
            ResponseEvent::SafetyBuffering(SafetyBuffering {
                use_cases: vec!["cyber".to_string()],
                reasons: vec!["risk".to_string()],
                show_buffering_ui: true,
                faster_model: Some("gpt-5.6-sol".to_string()),
            }),
            ModelEvent::SafetyBuffering(ModelSafetyBuffering {
                use_cases: vec!["cyber".to_string()],
                reasons: vec!["risk".to_string()],
                retry_model: Some("gpt-5.6-sol".to_string()),
            }),
        ),
        (
            ResponseEvent::OutputItemDone(user_message("done")),
            ModelEvent::OutputItemDone(user_message("done")),
        ),
        (
            ResponseEvent::OutputItemAdded(user_message("added")),
            ModelEvent::OutputItemAdded(user_message("added")),
        ),
        (
            ResponseEvent::ServerModel("gpt-5.6-sol".to_string()),
            ModelEvent::ServerModel("gpt-5.6-sol".to_string()),
        ),
        (
            ResponseEvent::ModelVerifications(vec![
                codex_protocol::protocol::ModelVerification::TrustedAccessForCyber,
            ]),
            ModelEvent::ModelVerifications(vec![
                codex_protocol::protocol::ModelVerification::TrustedAccessForCyber,
            ]),
        ),
        (
            ResponseEvent::TurnModerationMetadata(moderation.clone()),
            ModelEvent::TurnModerationMetadata(moderation),
        ),
        (
            ResponseEvent::ServerReasoningIncluded(true),
            ModelEvent::ServerReasoningIncluded(true),
        ),
        (
            ResponseEvent::Completed {
                response_id: "resp-1".to_string(),
                token_usage: None,
                usage_metadata: None,
                end_turn: Some(true),
            },
            ModelEvent::Completed {
                response_id: "resp-1".to_string(),
                token_usage: None,
                usage_metadata: None,
                end_turn: Some(true),
            },
        ),
        (
            ResponseEvent::OutputTextDelta("delta".to_string()),
            ModelEvent::OutputTextDelta("delta".to_string()),
        ),
        (
            ResponseEvent::ToolCallInputDelta {
                item_id: "item-1".to_string(),
                call_id: Some("call-1".to_string()),
                delta: "{}".to_string(),
            },
            ModelEvent::ToolCallInputDelta {
                item_id: "item-1".to_string(),
                call_id: Some("call-1".to_string()),
                delta: "{}".to_string(),
            },
        ),
        (
            ResponseEvent::ReasoningSummaryDelta {
                delta: "summary".to_string(),
                summary_index: 0,
            },
            ModelEvent::ReasoningSummaryDelta {
                delta: "summary".to_string(),
                summary_index: 0,
            },
        ),
        (
            ResponseEvent::ReasoningSummaryDone {
                item_id: "item-2".to_string(),
                text: "done".to_string(),
                summary_index: 0,
            },
            ModelEvent::ReasoningSummaryDone {
                item_id: "item-2".to_string(),
                text: "done".to_string(),
                summary_index: 0,
            },
        ),
        (
            ResponseEvent::ReasoningContentDelta {
                delta: "content".to_string(),
                content_index: 1,
            },
            ModelEvent::ReasoningContentDelta {
                delta: "content".to_string(),
                content_index: 1,
            },
        ),
        (
            ResponseEvent::ReasoningSummaryPartAdded { summary_index: 2 },
            ModelEvent::ReasoningSummaryPartAdded { summary_index: 2 },
        ),
        (
            ResponseEvent::RateLimits(rate_limit_snapshot()),
            ModelEvent::RateLimits(rate_limit_snapshot()),
        ),
        (
            ResponseEvent::ModelsEtag("etag-1".to_string()),
            ModelEvent::ModelsEtag("etag-1".to_string()),
        ),
    ];

    for (response_event, expected) in events {
        assert_eq!(
            ModelContractAdapter::to_model_event(response_event),
            expected
        );
    }
}

fn rate_limit_snapshot() -> codex_protocol::protocol::RateLimitSnapshot {
    codex_protocol::protocol::RateLimitSnapshot {
        limit_id: None,
        limit_name: None,
        normal_model_slug: None,
        primary: None,
        secondary: None,
        credits: None,
        individual_limit: None,
        spend_control_reached: None,
        plan_type: None,
        rate_limit_reached_type: None,
    }
}

#[test]
fn api_error_mapping_covers_the_taxonomy() {
    let cases = vec![
        (
            ApiError::ContextWindowExceeded,
            ModelError::ContextWindowExceeded {
                message: "context window exceeded".to_string(),
            },
        ),
        (
            ApiError::QuotaExceeded,
            ModelError::UsageLimitExceeded {
                message: "quota exceeded".to_string(),
            },
        ),
        (
            ApiError::UsageNotIncluded,
            ModelError::UsageLimitExceeded {
                message: "usage not included".to_string(),
            },
        ),
        (
            ApiError::Retryable {
                message: "try again".to_string(),
                delay: Some(std::time::Duration::from_secs(1)),
            },
            ModelError::Transport {
                message: "try again".to_string(),
                retryable: true,
                retry_after: Some(std::time::Duration::from_secs(1)),
                status: None,
            },
        ),
        (
            ApiError::RateLimitExceeded {
                message: "slow down".to_string(),
                delay: None,
            },
            ModelError::RateLimited {
                message: "slow down".to_string(),
                retry_after: None,
            },
        ),
        (
            ApiError::InvalidRequest {
                message: "bad body".to_string(),
            },
            ModelError::InvalidRequest {
                message: "bad body".to_string(),
            },
        ),
        (
            ApiError::ServerOverloaded,
            ModelError::Overloaded {
                message: "server overloaded".to_string(),
            },
        ),
        (
            ApiError::Stream("stream dropped".to_string()),
            ModelError::StreamIncomplete {
                message: "stream dropped".to_string(),
            },
        ),
    ];
    for (api_error, expected) in cases {
        assert_eq!(ModelContractAdapter::to_model_error(api_error), expected);
    }
}

#[test]
fn transport_error_mapping_classifies_auth_and_retryability() {
    let unauthorized = TransportError::Http {
        status: StatusCode::UNAUTHORIZED,
        url: None,
        headers: None,
        body: Some("token expired".to_string()),
    };
    assert_eq!(
        ModelContractAdapter::to_model_error(ApiError::Transport(unauthorized)),
        ModelError::Auth {
            message: "http 401 Unauthorized: Some(\"token expired\")".to_string(),
        }
    );

    let server_error = TransportError::Http {
        status: StatusCode::BAD_GATEWAY,
        url: None,
        headers: None,
        body: None,
    };
    let mapped = ModelContractAdapter::to_model_error(ApiError::Transport(server_error));
    assert!(mapped.is_retryable());
    assert_eq!(mapped.status(), Some(502));

    let timeout = TransportError::Timeout;
    assert!(ModelContractAdapter::to_model_error(ApiError::Transport(timeout)).is_retryable());

    let retry_limit = TransportError::RetryLimit;
    assert!(!ModelContractAdapter::to_model_error(ApiError::Transport(retry_limit)).is_retryable());
}

#[test]
fn response_events_aggregate_through_contract_response() {
    let events = vec![
        ModelContractAdapter::to_model_event(ResponseEvent::Created {
            response_id: Some("resp-9".to_string()),
        }),
        ModelContractAdapter::to_model_event(ResponseEvent::OutputItemDone(user_message("final"))),
        ModelContractAdapter::to_model_event(ResponseEvent::Completed {
            response_id: "resp-9".to_string(),
            token_usage: None,
            usage_metadata: None,
            end_turn: Some(true),
        }),
    ];

    let response = codex_model_contract::ModelResponse::from_events(events);
    assert_eq!(response.response_id, Some("resp-9".to_string()));
    assert_eq!(response.output, vec![user_message("final")]);
    assert_eq!(response.end_turn, Some(true));
}
