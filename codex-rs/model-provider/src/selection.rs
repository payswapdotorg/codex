//! Capability-aware provider selection for the universal model plane.
//!
//! The universal contract treats provider routing as a runtime binding
//! decision (architecture v0.2.0, §4). This module gives the runtime a
//! small selection helper that:
//!
//! 1. resolves a registered [`ModelProvider`] by descriptor
//!    ([`select_provider_for_descriptor`]); and
//! 2. negotiates a request's required features against a provider's
//!    effective capabilities, surfacing explicit
//!    [`UnsupportedModelCapability`] diagnostics when a feature is not
//!    supported — never silently degrading
//!    ([`negotiate_model_capabilities`]).
//!
//! The selector never touches workflow or thread state; it only reads the
//! provider registry and the model catalog. Callers (the runtime, future
//! scheduling logic) own any durable binding decisions.

use std::sync::Arc;

use codex_model_contract::ModelCapabilities;
use codex_model_contract::ModelDescriptor;
use codex_model_contract::ModelError;
use codex_model_contract::ModelFeature;
use codex_model_contract::ModelProvider as ContractModelProvider;
use codex_model_contract::UnsupportedModelCapability;
use codex_protocol::openai_models::ModelInfo;

/// Error returned when a provider cannot be selected for a descriptor.
///
/// Provider routing failures must be explicit so callers can surface them
/// to the user instead of falling back to an unrelated provider.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ProviderSelectionError {
    /// No registered provider matches the descriptor's `provider_id`.
    #[error("no model provider registered for provider_id `{0}`")]
    UnknownProvider(String),
    /// The selected provider exists but the requested model is not in its
    /// catalog, so capabilities cannot be resolved.
    #[error("provider `{provider_id}` has no model catalog entry for `{model_id}`")]
    UnknownModel {
        provider_id: String,
        model_id: String,
    },
}

impl From<ProviderSelectionError> for ModelError {
    fn from(error: ProviderSelectionError) -> Self {
        match error {
            ProviderSelectionError::UnknownProvider(_) => ModelError::InvalidRequest {
                message: error.to_string(),
            },
            ProviderSelectionError::UnknownModel {
                provider_id,
                model_id,
            } => ModelError::InvalidRequest {
                message: format!(
                    "provider `{provider_id}` has no model catalog entry for `{model_id}`"
                ),
            },
        }
    }
}

/// Selects the registered provider that owns a descriptor's `provider_id`.
///
/// The returned provider can then be asked for its
/// [`ModelCapabilities`](ModelCapabilities) for a specific model and
/// negotiated against a request's required features via
/// [`negotiate_model_capabilities`].
///
/// Returns [`ProviderSelectionError::UnknownProvider`] when no registered
/// provider matches; the caller must surface this explicitly rather than
/// silently substituting another provider.
pub fn select_provider_for_descriptor<'a>(
    providers: &'a [Arc<dyn ContractModelProvider>],
    descriptor: &ModelDescriptor,
) -> Result<&'a Arc<dyn ContractModelProvider>, ProviderSelectionError> {
    providers
        .iter()
        .find(|provider| provider.provider_id() == descriptor.provider_id)
        .ok_or_else(|| ProviderSelectionError::UnknownProvider(descriptor.provider_id.clone()))
}

/// Resolves and negotiates a model's effective capabilities against the
/// features a request requires.
///
/// Capability negotiation runs through [`ModelCapabilities::negotiate`],
/// which fails on the first unsupported feature with explicit
/// [`UnsupportedModelCapability`] diagnostics — including supported
/// alternatives for enumerable features like reasoning efforts and service
/// tiers. The selector never silently degrades an unsupported feature.
///
/// Returns the resolved [`ModelCapabilities`] when negotiation succeeds,
/// so callers can attach them to evidence or use them for transport
/// decisions.
pub fn negotiate_model_capabilities(
    provider: &dyn ContractModelProvider,
    model: &ModelInfo,
    required: Vec<ModelFeature>,
) -> Result<ModelCapabilities, UnsupportedModelCapability> {
    let capabilities = provider.model_capabilities(model);
    capabilities.negotiate(required)?;
    Ok(capabilities)
}

#[cfg(test)]
#[path = "selection_tests.rs"]
mod tests;
