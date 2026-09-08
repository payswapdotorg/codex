use std::fmt;

use codex_protocol::openai_models::InputModality;
use codex_protocol::openai_models::ReasoningEffort;

/// Remote context-compaction protocols supported by a model provider.
///
/// Owned by the universal contract so capability negotiation stays
/// provider-neutral. `codex-model-provider` re-exports this type for runtime
/// consumers.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum RemoteCompactionSupport {
    /// The provider does not support remote compaction.
    #[default]
    Unsupported,
    /// The provider supports `compaction_trigger` items over the Responses endpoint.
    V2,
}

/// A model feature that can be negotiated against [`ModelCapabilities`].
///
/// Features describe what a *model* supports (reasoning controls, input
/// modalities, provider-hosted tools). They are distinct from runtime
/// execution capabilities such as browser, desktop, shell, MCP, or skills,
/// which stay available to any model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModelFeature {
    /// A specific reasoning effort level.
    ReasoningEffort(ReasoningEffort),
    /// Reasoning summary controls.
    ReasoningSummary,
    /// Output verbosity controls.
    Verbosity,
    /// Image inputs.
    ImageInput,
    /// Audio inputs.
    AudioInput,
    /// A specific service tier.
    ServiceTier(String),
    /// Original-resolution image detail.
    ImageDetailOriginal,
    /// Provider-hosted web search tool.
    HostedWebSearch,
    /// Standalone (separate endpoint) web search.
    StandaloneWebSearch,
    /// Provider-side image generation.
    ImageGeneration,
    /// Namespaced tool declarations.
    NamespacedTools,
    /// Live external web access.
    ExternalWebAccess,
    /// Remote context compaction.
    RemoteCompaction,
}

impl fmt::Display for ModelFeature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ReasoningEffort(effort) => write!(f, "reasoning effort {effort}"),
            Self::ReasoningSummary => f.write_str("reasoning summary"),
            Self::Verbosity => f.write_str("verbosity"),
            Self::ImageInput => f.write_str("image input"),
            Self::AudioInput => f.write_str("audio input"),
            Self::ServiceTier(tier) => write!(f, "service tier {tier}"),
            Self::ImageDetailOriginal => f.write_str("original image detail"),
            Self::HostedWebSearch => f.write_str("hosted web search"),
            Self::StandaloneWebSearch => f.write_str("standalone web search"),
            Self::ImageGeneration => f.write_str("image generation"),
            Self::NamespacedTools => f.write_str("namespaced tools"),
            Self::ExternalWebAccess => f.write_str("external web access"),
            Self::RemoteCompaction => f.write_str("remote compaction"),
        }
    }
}

/// Effective capabilities of a model behind the universal contract.
///
/// Combines model-level support (reasoning efforts, input modalities,
/// context window, service tiers) with the provider-owned upper bound for
/// provider-hosted features. Providers and the runtime construct this from
/// their catalog and provider metadata; the contract only defines the
/// negotiation semantics.
///
/// An empty `supported_reasoning_efforts` list means the model's supported
/// levels are unknown, and effort negotiation fails explicitly rather than
/// silently clamping.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ModelCapabilities {
    /// Provider registry key the capabilities were resolved for.
    pub provider_id: String,
    /// Model slug the capabilities were resolved for.
    pub model_id: String,
    /// Reasoning effort levels the model supports, in catalog order.
    pub supported_reasoning_efforts: Vec<ReasoningEffort>,
    /// Whether the model accepts reasoning summary controls.
    pub supports_reasoning_summary: bool,
    /// Whether the model accepts verbosity controls.
    pub supports_verbosity: bool,
    /// Input modalities the model accepts.
    pub input_modalities: Vec<InputModality>,
    /// Effective context window in tokens, when known.
    pub context_window: Option<i64>,
    /// Service tier identifiers the model supports.
    pub service_tiers: Vec<String>,
    /// Whether original-resolution image detail can be requested.
    pub supports_image_detail_original: bool,
    /// Whether the provider accepts namespaced tool declarations.
    pub namespaced_tools: bool,
    /// Whether provider-side image generation is available.
    pub image_generation: bool,
    /// Whether the provider can host the web search tool.
    pub hosted_web_search: bool,
    /// Whether the provider exposes a standalone web search endpoint.
    pub standalone_web_search: bool,
    /// Whether live external web access is permitted by the provider.
    pub external_web_access: bool,
    /// Remote context compaction support.
    pub remote_compaction: RemoteCompactionSupport,
}

impl ModelCapabilities {
    /// Returns whether a feature is supported.
    pub fn supports(&self, feature: &ModelFeature) -> bool {
        match feature {
            ModelFeature::ReasoningEffort(effort) => self.supports_reasoning_effort(effort),
            ModelFeature::ReasoningSummary => self.supports_reasoning_summary,
            ModelFeature::Verbosity => self.supports_verbosity,
            ModelFeature::ImageInput => self.supports_modality(InputModality::Image),
            ModelFeature::AudioInput => self.supports_modality(InputModality::Audio),
            ModelFeature::ServiceTier(tier) => {
                self.service_tiers.iter().any(|supported| supported == tier)
            }
            ModelFeature::ImageDetailOriginal => self.supports_image_detail_original,
            ModelFeature::HostedWebSearch => self.hosted_web_search,
            ModelFeature::StandaloneWebSearch => self.standalone_web_search,
            ModelFeature::ImageGeneration => self.image_generation,
            ModelFeature::NamespacedTools => self.namespaced_tools,
            ModelFeature::ExternalWebAccess => self.external_web_access,
            ModelFeature::RemoteCompaction => {
                self.remote_compaction != RemoteCompactionSupport::Unsupported
            }
        }
    }

    /// Requires a feature, returning explicit diagnostics when unsupported.
    ///
    /// Unsupported capabilities must fail through this path instead of being
    /// silently dropped or substituted.
    pub fn require(&self, feature: ModelFeature) -> Result<(), UnsupportedModelCapability> {
        if self.supports(&feature) {
            return Ok(());
        }
        Err(self.unsupported(feature))
    }

    /// Requires every feature in `required`, failing on the first unsupported
    /// one with diagnostics.
    pub fn negotiate(
        &self,
        required: impl IntoIterator<Item = ModelFeature>,
    ) -> Result<(), UnsupportedModelCapability> {
        for feature in required {
            self.require(feature)?;
        }
        Ok(())
    }

    /// Returns whether a reasoning effort is supported by the model.
    pub fn supports_reasoning_effort(&self, effort: &ReasoningEffort) -> bool {
        self.supported_reasoning_efforts.contains(effort)
    }

    fn supports_modality(&self, modality: InputModality) -> bool {
        self.input_modalities.contains(&modality)
    }

    fn unsupported(&self, feature: ModelFeature) -> UnsupportedModelCapability {
        let message = format!(
            "model {}/{} does not support {feature}",
            self.provider_id, self.model_id
        );
        let supported_alternatives = match &feature {
            ModelFeature::ReasoningEffort(_) => self
                .supported_reasoning_efforts
                .iter()
                .map(ToString::to_string)
                .collect(),
            ModelFeature::ServiceTier(_) => self.service_tiers.clone(),
            _ => Vec::new(),
        };
        UnsupportedModelCapability {
            feature,
            provider_id: self.provider_id.clone(),
            model_id: self.model_id.clone(),
            message,
            supported_alternatives,
        }
    }
}

/// Diagnostics for a required model feature that is unsupported.
///
/// `supported_alternatives` lists the supported values for enumerable
/// features (for example reasoning efforts or service tiers) so callers and
/// users can recover explicitly instead of guessing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnsupportedModelCapability {
    /// The unsupported feature that was requested.
    pub feature: ModelFeature,
    /// Provider the negotiation ran against.
    pub provider_id: String,
    /// Model the negotiation ran against.
    pub model_id: String,
    /// Human-readable diagnostic message.
    pub message: String,
    /// Supported values for enumerable features, for recovery.
    pub supported_alternatives: Vec<String>,
}

impl fmt::Display for UnsupportedModelCapability {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unsupported model capability: {}", self.message)?;
        if !self.supported_alternatives.is_empty() {
            write!(f, "; supported: {}", self.supported_alternatives.join(", "))?;
        }
        Ok(())
    }
}

impl std::error::Error for UnsupportedModelCapability {}

#[cfg(test)]
#[path = "capabilities_tests.rs"]
mod tests;
