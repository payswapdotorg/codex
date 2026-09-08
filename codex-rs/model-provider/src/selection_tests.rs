//! Tests for the capability-aware provider selection helper.
//!
//! Verifies that:
//!
//! - [`select_provider_for_descriptor`] resolves a registered provider by
//!   `provider_id` and fails explicitly when no provider matches;
//! - [`negotiate_model_capabilities`] surfaces [`UnsupportedModelCapability`]
//!   diagnostics for unsupported features, including supported alternatives
//!   for enumerable features;
//! - the two-step select+negotiate composition surfaces
//!   [`ModelError::InvalidRequest`] for unknown providers and
//!   [`ModelError::UnsupportedCapability`] for unsupported features.

use std::sync::Arc;

use codex_model_contract::ModelDescriptor;
use codex_model_contract::ModelError;
use codex_model_contract::ModelFeature;
use codex_model_contract::ModelProvider as ContractModelProvider;
use codex_model_contract::UnsupportedModelCapability;
use codex_model_provider_info::ModelProviderInfo;
use codex_model_provider_info::WireApi;
use codex_model_provider_info::create_oss_provider_with_base_url;
use codex_protocol::openai_models::ReasoningEffort;
use pretty_assertions::assert_eq;
use serde_json::json;

use super::*;
use crate::ModelContractAdapter;
use crate::create_model_provider;

const MODEL_SLUG: &str = "gpt-5.6-luna";

fn sample_model_info() -> codex_protocol::openai_models::ModelInfo {
    serde_json::from_value(json!({
        "slug": MODEL_SLUG,
        "display_name": MODEL_SLUG,
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

fn openai_adapter() -> Arc<dyn ContractModelProvider> {
    Arc::new(ModelContractAdapter::new(
        "openai",
        create_model_provider(
            ModelProviderInfo::create_openai_provider(/*base_url*/ None),
            /*auth_manager*/ None,
        ),
    ))
}

fn ollama_adapter() -> Arc<dyn ContractModelProvider> {
    Arc::new(ModelContractAdapter::new(
        "ollama",
        create_model_provider(
            create_oss_provider_with_base_url("http://localhost:11434/v1", WireApi::Responses),
            /*auth_manager*/ None,
        ),
    ))
}

fn registry() -> Vec<Arc<dyn ContractModelProvider>> {
    vec![openai_adapter(), ollama_adapter()]
}

#[test]
fn select_provider_for_descriptor_resolves_registered_provider() {
    let providers = registry();
    let descriptor = ModelDescriptor::new("ollama", MODEL_SLUG);
    let selected = select_provider_for_descriptor(&providers, &descriptor).expect("known provider");
    assert_eq!(selected.provider_id(), "ollama");
}

#[test]
fn select_provider_for_descriptor_fails_for_unknown_provider() {
    let providers = registry();
    let descriptor = ModelDescriptor::new("anthropic", MODEL_SLUG);
    let error =
        select_provider_for_descriptor(&providers, &descriptor).expect_err("unknown provider");
    assert_eq!(
        error,
        ProviderSelectionError::UnknownProvider("anthropic".to_string())
    );
}

#[test]
fn negotiate_capabilities_returns_supported_features() {
    let provider = openai_adapter();
    let model = sample_model_info();
    let required = vec![
        ModelFeature::ReasoningEffort(ReasoningEffort::High),
        ModelFeature::ImageInput,
    ];
    let capabilities =
        negotiate_model_capabilities(provider.as_ref(), &model, required).expect("supported");
    assert_eq!(capabilities.provider_id, "openai");
    assert_eq!(capabilities.model_id, MODEL_SLUG);
}

#[test]
fn negotiate_capabilities_surfaces_unsupported_feature_with_alternatives() {
    let provider = openai_adapter();
    let model = sample_model_info();
    let required = vec![ModelFeature::ReasoningEffort(ReasoningEffort::Max)];
    let error =
        negotiate_model_capabilities(provider.as_ref(), &model, required).expect_err("unsupported");
    assert_eq!(
        error,
        UnsupportedModelCapability {
            feature: ModelFeature::ReasoningEffort(ReasoningEffort::Max),
            provider_id: "openai".to_string(),
            model_id: MODEL_SLUG.to_string(),
            message: format!("model openai/{MODEL_SLUG} does not support reasoning effort max"),
            supported_alternatives: vec![
                "low".to_string(),
                "medium".to_string(),
                "high".to_string()
            ],
        }
    );
}

#[test]
fn select_then_negotiate_returns_provider_and_capabilities() {
    // The runtime shape: select a provider for a descriptor, then negotiate
    // the model's capabilities. The two-step API keeps the provider handle
    // available for `create_session`.
    let providers = registry();
    let descriptor = ModelDescriptor::new("openai", MODEL_SLUG);
    let model = sample_model_info();
    let provider = select_provider_for_descriptor(&providers, &descriptor).expect("known provider");
    let capabilities = negotiate_model_capabilities(
        provider.as_ref(),
        &model,
        vec![ModelFeature::ReasoningEffort(ReasoningEffort::Medium)],
    )
    .expect("supported");
    assert_eq!(provider.provider_id(), "openai");
    assert_eq!(capabilities.model_id, MODEL_SLUG);
}

#[test]
fn select_then_negotiate_surfaces_unsupported_capability_error() {
    let providers = registry();
    let descriptor = ModelDescriptor::new("openai", MODEL_SLUG);
    let model = sample_model_info();
    let provider = select_provider_for_descriptor(&providers, &descriptor).expect("known provider");
    let error = negotiate_model_capabilities(
        provider.as_ref(),
        &model,
        vec![ModelFeature::ReasoningEffort(ReasoningEffort::Max)],
    )
    .expect_err("unsupported");
    assert_eq!(
        error,
        UnsupportedModelCapability {
            feature: ModelFeature::ReasoningEffort(ReasoningEffort::Max),
            provider_id: "openai".to_string(),
            model_id: MODEL_SLUG.to_string(),
            message: format!("model openai/{MODEL_SLUG} does not support reasoning effort max"),
            supported_alternatives: vec![
                "low".to_string(),
                "medium".to_string(),
                "high".to_string()
            ],
        }
    );
}

#[test]
fn select_then_negotiate_surfaces_unknown_provider_as_invalid_request() {
    // The selection failure converts to ModelError::InvalidRequest so
    // runtime callers can surface it through the universal error taxonomy.
    let providers = registry();
    let descriptor = ModelDescriptor::new("anthropic", MODEL_SLUG);
    let selection_error =
        select_provider_for_descriptor(&providers, &descriptor).expect_err("unknown provider");
    let error: ModelError = selection_error.into();
    assert!(matches!(error, ModelError::InvalidRequest { .. }));
}

#[test]
fn provider_selection_error_converts_to_model_error() {
    let error: ModelError = ProviderSelectionError::UnknownProvider("anthropic".to_string()).into();
    assert!(matches!(error, ModelError::InvalidRequest { .. }));

    let error: ModelError = ProviderSelectionError::UnknownModel {
        provider_id: "openai".to_string(),
        model_id: "gpt-x".to_string(),
    }
    .into();
    assert!(matches!(error, ModelError::InvalidRequest { .. }));
}

#[test]
fn two_provider_paths_satisfy_same_descriptor_family() {
    // The WO-004 acceptance shape at the selection layer: the same model
    // slug (semantic identity) can be served by either registered provider
    // when capabilities permit. Switching providers is a registry lookup,
    // not a thread/workflow state mutation.
    let providers = registry();
    let model = sample_model_info();

    let openai_descriptor = ModelDescriptor::new("openai", MODEL_SLUG);
    let ollama_descriptor = ModelDescriptor::new("ollama", MODEL_SLUG);

    let openai_provider =
        select_provider_for_descriptor(&providers, &openai_descriptor).expect("openai");
    let ollama_provider =
        select_provider_for_descriptor(&providers, &ollama_descriptor).expect("ollama");

    let openai_caps = negotiate_model_capabilities(
        openai_provider.as_ref(),
        &model,
        vec![ModelFeature::ReasoningEffort(ReasoningEffort::High)],
    )
    .expect("openai supports high");
    let ollama_caps = negotiate_model_capabilities(
        ollama_provider.as_ref(),
        &model,
        vec![ModelFeature::ReasoningEffort(ReasoningEffort::High)],
    )
    .expect("ollama supports high");

    // Both providers expose equivalent model-level capability for the same
    // catalog model. Provider-owned upper bounds (standalone_web_search,
    // remote_compaction) differ and are surfaced distinctly.
    assert_eq!(
        openai_caps.supported_reasoning_efforts,
        ollama_caps.supported_reasoning_efforts
    );
    assert_eq!(openai_caps.input_modalities, ollama_caps.input_modalities);
    assert_ne!(openai_caps.provider_id, ollama_caps.provider_id);
}
