use std::collections::HashMap;
use std::sync::Arc;

use codex_http_client::HttpClientFactory;
use codex_http_client::OutboundProxyPolicy;
use codex_model_contract::ModelDescriptor;
use codex_model_contract::ModelError;
use codex_model_contract::ModelFeature;
use codex_model_contract::ModelRequest;
use codex_model_provider_info::ModelProviderInfo;
use codex_model_provider_info::WireApi;
use codex_model_provider_info::create_oss_provider_with_base_url;
use codex_models_manager::ModelsManagerConfig;
use codex_models_manager::manager::StaticModelsManager;
use codex_protocol::openai_models::ModelsResponse;
use codex_protocol::openai_models::ReasoningEffort;
use pretty_assertions::assert_eq;
use serde_json::Value;
use serde_json::json;

use super::*;
use crate::create_model_provider;

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

fn openai_adapter(models: Vec<Value>) -> ModelContractAdapter {
    ModelContractAdapter::new(
        "openai",
        create_model_provider(
            ModelProviderInfo::create_openai_provider(Some("http://127.0.0.1:9/v1".to_string())),
            /*auth_manager*/ None,
        ),
    )
    .with_model_catalog(
        Arc::new(StaticModelsManager::new(
            /*auth_manager*/ None,
            ModelsResponse {
                models: models
                    .into_iter()
                    .map(|value| serde_json::from_value(value).expect("model should parse"))
                    .collect(),
            },
        )),
        ModelsManagerConfig::default(),
    )
}

fn ollama_adapter(models: Vec<Value>) -> ModelContractAdapter {
    ModelContractAdapter::new(
        "ollama",
        create_model_provider(
            create_oss_provider_with_base_url("http://127.0.0.1:8/v1", WireApi::Responses),
            /*auth_manager*/ None,
        ),
    )
    .with_model_catalog(
        Arc::new(StaticModelsManager::new(
            /*auth_manager*/ None,
            ModelsResponse {
                models: models
                    .into_iter()
                    .map(|value| serde_json::from_value(value).expect("model should parse"))
                    .collect(),
            },
        )),
        ModelsManagerConfig::default(),
    )
}

fn two_provider_registry() -> ModelProviderRegistry {
    let mut registry = ModelProviderRegistry::new();
    registry.register_adapter(openai_adapter(vec![
        reasoning_model_json("gpt-5.6-luna"),
        text_only_model_json("gpt-5.6-mini"),
    ]));
    registry.register_adapter(ollama_adapter(vec![reasoning_model_json("local-model")]));
    registry
}

#[tokio::test]
async fn registry_selects_a_capable_model_and_creates_a_session() {
    let registry = two_provider_registry();

    let selected = registry
        .select(
            &ModelDescriptor::new("openai", "gpt-5.6-luna"),
            &[ModelFeature::ReasoningEffort(ReasoningEffort::High)],
        )
        .await
        .expect("capable model should select");

    assert_eq!(
        selected.descriptor,
        ModelDescriptor::new("openai", "gpt-5.6-luna")
    );
    assert_eq!(selected.capabilities.provider_id, "openai");
    assert_eq!(selected.capabilities.model_id, "gpt-5.6-luna");
    assert!(
        selected
            .capabilities
            .supports(&ModelFeature::ReasoningEffort(ReasoningEffort::High))
    );
    assert!(selected.fallback_from.is_none());
    // The session is ready to execute requests against the selected provider.
    let provider_id = selected.provider.provider_id().to_string();
    assert_eq!(provider_id, "openai");
    drop(selected.session);
}

#[tokio::test]
async fn registry_lists_registered_providers() {
    let registry = two_provider_registry();

    assert_eq!(
        registry.provider_ids(),
        vec!["ollama".to_string(), "openai".to_string()]
    );
    assert!(registry.provider("openai").is_some());
    assert!(registry.provider("missing").is_none());
}

#[tokio::test]
async fn unknown_provider_fails_with_registered_provider_diagnostics() {
    let registry = two_provider_registry();

    let error = registry
        .select(&ModelDescriptor::new("anthropic", "claude-x"), &[])
        .await
        .expect_err("unknown provider should fail");
    let ModelError::InvalidRequest { message } = &error else {
        panic!("expected an invalid-request diagnostic, got {error}");
    };
    assert!(message.contains("anthropic"), "message: {message}");
    assert!(message.contains("openai"), "message: {message}");
    assert!(message.contains("ollama"), "message: {message}");
}

#[tokio::test]
async fn unsupported_capability_fails_explicitly_at_selection_time() {
    let registry = two_provider_registry();

    let error = registry
        .select(
            &ModelDescriptor::new("openai", "gpt-5.6-mini"),
            &[ModelFeature::ReasoningEffort(ReasoningEffort::High)],
        )
        .await
        .expect_err("unsupported effort should fail");
    let ModelError::UnsupportedCapability(unsupported) = &error else {
        panic!("expected capability diagnostics, got {error}");
    };
    assert_eq!(unsupported.provider_id, "openai");
    assert_eq!(unsupported.model_id, "gpt-5.6-mini");
    assert_eq!(
        unsupported.supported_alternatives,
        vec!["medium".to_string()]
    );
    assert!(!error.is_retryable());
}

#[tokio::test]
async fn selection_falls_back_to_a_capable_candidate_and_records_the_preference() {
    let registry = two_provider_registry();

    let selected = registry
        .select_with_fallbacks(
            &[
                ModelDescriptor::new("openai", "gpt-5.6-mini"),
                ModelDescriptor::new("openai", "gpt-5.6-luna"),
                ModelDescriptor::new("ollama", "local-model"),
            ],
            &[ModelFeature::ReasoningEffort(ReasoningEffort::High)],
        )
        .await
        .expect("fallback candidate should select");

    assert_eq!(
        selected.descriptor,
        ModelDescriptor::new("openai", "gpt-5.6-luna")
    );
    assert_eq!(
        selected.fallback_from,
        Some(ModelDescriptor::new("openai", "gpt-5.6-mini"))
    );
}

#[tokio::test]
async fn all_candidates_failing_returns_the_preferred_diagnostics() {
    let registry = two_provider_registry();

    let error = registry
        .select_with_fallbacks(
            &[
                ModelDescriptor::new("openai", "gpt-5.6-mini"),
                ModelDescriptor::new("ollama", "unheard-of-model"),
            ],
            &[ModelFeature::Verbosity],
        )
        .await
        .expect_err("all candidates failing should error");
    // The preferred candidate's diagnostics describe the caller's intent.
    let ModelError::UnsupportedCapability(unsupported) = &error else {
        panic!("expected capability diagnostics, got {error}");
    };
    assert_eq!(unsupported.model_id, "gpt-5.6-mini");
    assert_eq!(unsupported.feature, ModelFeature::Verbosity);
}

#[tokio::test]
async fn select_for_request_derives_required_features_from_the_request() {
    let registry = two_provider_registry();

    let request = ModelRequest {
        model: ModelDescriptor::new("openai", "gpt-5.6-luna"),
        reasoning: Some(codex_model_contract::ModelReasoningControls {
            effort: Some(ReasoningEffort::High),
            summary: None,
        }),
        ..ModelRequest::default()
    };
    let selected = registry
        .select_for_request(&request)
        .await
        .expect("capable request should select");
    assert_eq!(selected.descriptor, request.model);

    // The same derivation makes selection-time and stream-time checks agree.
    let unsupported_request = ModelRequest {
        model: ModelDescriptor::new("openai", "gpt-5.6-mini"),
        ..request
    };
    let error = registry
        .select_for_request(&unsupported_request)
        .await
        .expect_err("unsupported request should fail");
    assert!(matches!(
        error,
        ModelError::UnsupportedCapability(ref unsupported)
            if unsupported.model_id == "gpt-5.6-mini"
    ));
}

#[tokio::test]
async fn selection_without_a_catalog_fails_explicitly() {
    let mut registry = ModelProviderRegistry::new();
    let uncataloged = ModelContractAdapter::new(
        "uncataloged",
        create_model_provider(
            create_oss_provider_with_base_url("http://127.0.0.1:9/v1", WireApi::Responses),
            /*auth_manager*/ None,
        ),
    );
    registry.register_adapter(uncataloged);

    let error = registry
        .select(&ModelDescriptor::new("uncataloged", "some-model"), &[])
        .await
        .expect_err("selection without a catalog should fail");
    let ModelError::InvalidRequest { message } = &error else {
        panic!("expected an invalid-request diagnostic, got {error}");
    };
    assert!(message.contains("model catalog"), "message: {message}");
}

/// The provider configuration integration: a registry built from configured
/// provider metadata serves the universal contract for every entry, with the
/// config-supplied catalog winning over remote fetching (mirroring the
/// runtime's models-manager construction).
#[tokio::test]
async fn build_universal_registry_wraps_configured_providers() {
    let mut providers = HashMap::new();
    providers.insert(
        "openai".to_string(),
        ModelProviderInfo::create_openai_provider(Some("http://127.0.0.1:9/v1".to_string())),
    );
    providers.insert(
        "ollama".to_string(),
        create_oss_provider_with_base_url("http://127.0.0.1:8/v1", WireApi::Responses),
    );

    let config_catalog = ModelsResponse {
        models: vec![
            serde_json::from_value(reasoning_model_json("gpt-5.6-luna"))
                .expect("model should parse"),
        ],
    };
    let tempdir = tempfile_for_registry_tests();

    let registry = build_universal_registry(
        &providers,
        /*auth_manager*/ None,
        &tempdir,
        Some(config_catalog),
        &ModelsManagerConfig::default(),
        HttpClientFactory::new(OutboundProxyPolicy::ReqwestDefault),
    );

    assert_eq!(
        registry.provider_ids(),
        vec!["ollama".to_string(), "openai".to_string()]
    );

    // Both provider paths satisfy the same universal selection contract.
    for provider_id in ["openai", "ollama"] {
        let selected = registry
            .select(
                &ModelDescriptor::new(provider_id, "gpt-5.6-luna"),
                &[ModelFeature::ReasoningEffort(ReasoningEffort::High)],
            )
            .await
            .unwrap_or_else(|error| panic!("{provider_id} should select: {error}"));
        assert_eq!(selected.capabilities.provider_id, provider_id);
        assert_eq!(
            selected.capabilities.supported_reasoning_efforts,
            vec![
                ReasoningEffort::Low,
                ReasoningEffort::Medium,
                ReasoningEffort::High
            ]
        );
    }
}

fn tempfile_for_registry_tests() -> std::path::PathBuf {
    std::env::temp_dir().join(format!("codex-wo004-registry-test-{}", std::process::id()))
}
