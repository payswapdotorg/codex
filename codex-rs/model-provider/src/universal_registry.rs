//! Capability-aware registry and selection over universal model providers.
//!
//! The registry is the provider configuration/routing integration point of
//! the universal model plane: configured providers (built-in or
//! user-configured `ModelProviderInfo` entries) are wrapped in
//! [`ModelContractAdapter`]s with their provider-scoped [`ModelCatalog`]s and
//! addressed by provider registry key.
//!
//! Selection is capability-aware and explicit:
//!
//! - an unknown provider fails with a diagnostic listing the registered
//!   providers;
//! - a required feature the model does not support fails with
//!   [`ModelError::UnsupportedCapability`] diagnostics (including supported
//!   alternatives for enumerable features), never silent degradation;
//! - ordered fallback selection is opt-in through
//!   [`ModelProviderRegistry::select_with_fallbacks`]; the selected model
//!   records the preferred descriptor it fell back from.
//!
//! Model routing policy itself stays owned by callers; this module only
//! resolves and verifies bindings between requests and providers.

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use codex_http_client::HttpClientFactory;
use codex_login::AuthManager;
use codex_model_contract::ModelCapabilities;
use codex_model_contract::ModelDescriptor;
use codex_model_contract::ModelError;
use codex_model_contract::ModelFeature;
use codex_model_contract::ModelProvider as ContractModelProvider;
use codex_model_contract::ModelRequest;
use codex_model_contract::ModelSession;
use codex_model_provider_info::ModelProviderInfo;
use codex_models_manager::ModelsManagerConfig;
use codex_protocol::openai_models::ModelsResponse;

use crate::create_model_provider;
use crate::universal::ModelContractAdapter;
use crate::universal::required_features;
use crate::universal_catalog::ModelCatalog;

/// Registry of universal model providers addressable by provider id.
///
/// Entries pair a contract provider with the provider-scoped model catalog
/// used to resolve model metadata for capability-aware selection.
#[derive(Debug, Default)]
pub struct ModelProviderRegistry {
    entries: HashMap<String, RegistryEntry>,
}

#[derive(Debug)]
struct RegistryEntry {
    provider: Arc<dyn ContractModelProvider>,
    catalog: Option<ModelCatalog>,
}

/// A provider/model binding that satisfied the required capabilities, ready
/// to execute requests.
#[derive(Debug)]
pub struct SelectedModel {
    /// The provider that will serve the model.
    pub provider: Arc<dyn ContractModelProvider>,
    /// Session executing requests against the provider.
    pub session: Arc<dyn ModelSession>,
    /// The descriptor the selection bound.
    pub descriptor: ModelDescriptor,
    /// Effective capabilities the selection negotiated against.
    pub capabilities: ModelCapabilities,
    /// The preferred descriptor that was rejected when a fallback candidate
    /// satisfied the requirements, when ordered fallbacks were requested.
    pub fallback_from: Option<ModelDescriptor>,
}

impl ModelProviderRegistry {
    /// Creates an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a contract provider under its provider id, replacing any
    /// existing entry.
    ///
    /// `catalog` attaches the provider-scoped model catalog used for
    /// capability-aware selection; providers without one can still be
    /// addressed directly but cannot participate in capability evaluation.
    pub fn register_provider(
        &mut self,
        provider: Arc<dyn ContractModelProvider>,
        catalog: Option<ModelCatalog>,
    ) {
        self.entries.insert(
            provider.provider_id().to_string(),
            RegistryEntry { provider, catalog },
        );
    }

    /// Registers a [`ModelContractAdapter`] with its attached model catalog.
    pub fn register_adapter(&mut self, adapter: ModelContractAdapter) {
        let catalog = adapter.model_catalog();
        self.register_provider(Arc::new(adapter), catalog);
    }

    /// Registered provider ids, sorted for deterministic diagnostics.
    pub fn provider_ids(&self) -> Vec<String> {
        let mut ids: Vec<String> = self.entries.keys().cloned().collect();
        ids.sort();
        ids
    }

    /// Returns the registered provider for a provider id, when present.
    pub fn provider(&self, provider_id: &str) -> Option<Arc<dyn ContractModelProvider>> {
        self.entries
            .get(provider_id)
            .map(|entry| entry.provider.clone())
    }

    /// Selects the preferred model, verifying it satisfies the required
    /// features before creating the session.
    pub async fn select(
        &self,
        preferred: &ModelDescriptor,
        required: &[ModelFeature],
    ) -> Result<SelectedModel, ModelError> {
        let entry =
            self.entries
                .get(&preferred.provider_id)
                .ok_or_else(|| ModelError::InvalidRequest {
                    message: format!(
                        "provider {} is not registered; registered providers: {}",
                        preferred.provider_id,
                        self.provider_ids().join(", "),
                    ),
                })?;
        let catalog = entry.catalog.as_ref().ok_or_else(|| {
            ModelError::InvalidRequest {
                message: format!(
                    "provider {} has no model catalog configured; cannot evaluate model capabilities",
                    preferred.provider_id,
                ),
            }
        })?;
        let model_info = catalog.resolve(&preferred.model_id).await;
        let capabilities = entry.provider.model_capabilities(&model_info);
        capabilities.negotiate(required.iter().cloned())?;
        let session = entry.provider.create_session()?;
        Ok(SelectedModel {
            provider: entry.provider.clone(),
            session,
            descriptor: preferred.clone(),
            capabilities,
            fallback_from: None,
        })
    }

    /// Selects the first candidate that satisfies the required features.
    ///
    /// Candidates are tried in order. The returned selection records the
    /// preferred (first) descriptor in [`SelectedModel::fallback_from`] when a
    /// later candidate satisfied the requirements. When no candidate
    /// qualifies, the preferred candidate's diagnostics are returned so the
    /// failure describes the caller's intent, not an arbitrary alternative.
    pub async fn select_with_fallbacks(
        &self,
        candidates: &[ModelDescriptor],
        required: &[ModelFeature],
    ) -> Result<SelectedModel, ModelError> {
        let mut preferred_error = None;
        for (index, candidate) in candidates.iter().enumerate() {
            match self.select(candidate, required).await {
                Ok(mut selected) => {
                    if index > 0 {
                        selected.fallback_from = Some(candidates[0].clone());
                    }
                    return Ok(selected);
                }
                Err(error) => {
                    if index == 0 {
                        preferred_error = Some(error);
                    }
                }
            }
        }
        Err(
            preferred_error.unwrap_or_else(|| ModelError::InvalidRequest {
                message: "no model selection candidates provided".to_string(),
            }),
        )
    }

    /// Selects the model addressed by a request, deriving the required
    /// features from the request's requested controls.
    ///
    /// The derivation is shared with the adapter's wire-request negotiation,
    /// so selection-time and stream-time capability checks cannot drift.
    pub async fn select_for_request(
        &self,
        request: &ModelRequest,
    ) -> Result<SelectedModel, ModelError> {
        let required = required_features(request);
        self.select(&request.model, &required).await
    }
}

/// Builds a registry of universal providers from configured provider
/// metadata.
///
/// This is the provider configuration/routing integration: every configured
/// provider entry (built-in plus user-configured) is wrapped in a
/// [`ModelContractAdapter`] with the models manager and auth wiring the
/// runtime would create for it, so registry-served requests reuse the exact
/// runtime provider behavior. Mirrors the runtime's models-manager
/// construction: a config-supplied catalog wins over remote fetching.
pub fn build_universal_registry(
    providers: &HashMap<String, ModelProviderInfo>,
    auth_manager: Option<Arc<AuthManager>>,
    codex_home: &Path,
    config_model_catalog: Option<ModelsResponse>,
    models_config: &ModelsManagerConfig,
    http_client_factory: HttpClientFactory,
) -> ModelProviderRegistry {
    let mut registry = ModelProviderRegistry::new();
    for (provider_id, provider_info) in providers {
        let provider = create_model_provider(provider_info.clone(), auth_manager.clone());
        let models_manager =
            provider.models_manager(codex_home.to_path_buf(), config_model_catalog.clone());
        let adapter = ModelContractAdapter::new(provider_id.clone(), provider)
            .with_model_catalog(models_manager, models_config.clone())
            .with_http_client_factory(http_client_factory.clone());
        registry.register_adapter(adapter);
    }
    registry
}

#[cfg(test)]
#[path = "universal_registry_tests.rs"]
mod tests;
