use pretty_assertions::assert_eq;

use super::*;
use codex_protocol::openai_models::ReasoningEffort;

fn capabilities() -> ModelCapabilities {
    ModelCapabilities {
        provider_id: "openai".to_string(),
        model_id: "gpt-5.6-luna".to_string(),
        supported_reasoning_efforts: Vec::new(),
        supports_reasoning_summary: true,
        supports_verbosity: true,
        input_modalities: Vec::new(),
        context_window: Some(272_000),
        service_tiers: Vec::new(),
        supports_image_detail_original: true,
        namespaced_tools: true,
        image_generation: true,
        hosted_web_search: true,
        standalone_web_search: false,
        external_web_access: true,
        remote_compaction: RemoteCompactionSupport::V2,
    }
}

#[test]
fn negotiates_supported_features() {
    let mut caps = capabilities();
    caps.supported_reasoning_efforts = vec![
        ReasoningEffort::Medium,
        ReasoningEffort::High,
        ReasoningEffort::Max,
    ];
    caps.input_modalities = vec![InputModality::Text, InputModality::Image];
    caps.service_tiers = vec!["standard".to_string(), "priority".to_string()];

    caps.negotiate([
        ModelFeature::ReasoningEffort(ReasoningEffort::High),
        ModelFeature::ImageInput,
        ModelFeature::ServiceTier("priority".to_string()),
        ModelFeature::ReasoningSummary,
        ModelFeature::Verbosity,
        ModelFeature::HostedWebSearch,
        ModelFeature::ImageGeneration,
        ModelFeature::NamespacedTools,
        ModelFeature::ExternalWebAccess,
        ModelFeature::RemoteCompaction,
    ])
    .expect("supported features should negotiate");
}

#[test]
fn unsupported_reasoning_effort_fails_with_alternatives() {
    let mut caps = capabilities();
    caps.supported_reasoning_efforts = vec![ReasoningEffort::Medium, ReasoningEffort::High];

    let failure = caps
        .require(ModelFeature::ReasoningEffort(ReasoningEffort::Ultra))
        .expect_err("unsupported effort should fail");

    assert_eq!(failure.provider_id, "openai");
    assert_eq!(failure.model_id, "gpt-5.6-luna");
    assert_eq!(
        failure.supported_alternatives,
        vec!["medium".to_string(), "high".to_string()]
    );
    assert!(
        failure.message.contains("does not support"),
        "diagnostic should explain the failure: {failure}"
    );
    assert!(
        failure.to_string().contains("supported: medium, high"),
        "display should list alternatives: {failure}"
    );
}

#[test]
fn empty_effort_list_fails_negotiation() {
    // Unknown support must fail explicitly rather than silently clamping.
    let caps = capabilities();

    let failure = caps
        .require(ModelFeature::ReasoningEffort(ReasoningEffort::Medium))
        .expect_err("unknown effort support should fail");

    assert!(failure.supported_alternatives.is_empty());
}

#[test]
fn unsupported_modalities_and_tiers_fail() {
    let mut caps = capabilities();
    caps.input_modalities = vec![InputModality::Text];
    caps.service_tiers = vec!["standard".to_string()];
    caps.remote_compaction = RemoteCompactionSupport::Unsupported;
    caps.supports_image_detail_original = false;

    for feature in [
        ModelFeature::ImageInput,
        ModelFeature::AudioInput,
        ModelFeature::ServiceTier("flex".to_string()),
        ModelFeature::RemoteCompaction,
        ModelFeature::ImageDetailOriginal,
        ModelFeature::StandaloneWebSearch,
    ] {
        let unsupported = caps
            .require(feature.clone())
            .err()
            .unwrap_or_else(|| panic!("{feature} should be unsupported"));
        assert_eq!(unsupported.feature, feature);
    }
}

#[test]
fn negotiate_stops_at_first_failure() {
    let mut caps = capabilities();
    caps.supports_verbosity = false;
    caps.supports_reasoning_summary = false;

    let failure = caps
        .negotiate([ModelFeature::ReasoningSummary, ModelFeature::Verbosity])
        .expect_err("negotiation should fail");

    assert_eq!(failure.feature, ModelFeature::ReasoningSummary);
}

#[test]
fn capability_equality_and_defaults() {
    assert_eq!(ModelCapabilities::default(), ModelCapabilities::default());
    assert_eq!(
        ModelCapabilities::default().remote_compaction,
        RemoteCompactionSupport::Unsupported
    );
    assert_eq!(
        capabilities(),
        capabilities(),
        "capabilities should compare by value"
    );
}
