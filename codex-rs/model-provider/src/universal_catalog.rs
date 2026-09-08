use codex_models_manager::ModelsManagerConfig;
use codex_models_manager::manager::ModelsManagerFuture;
use codex_models_manager::manager::SharedModelsManager;
use codex_protocol::openai_models::ModelInfo;

/// Provider-scoped model catalog backing universal adapter sessions.
///
/// Wraps the runtime models manager plus the manager configuration used to
/// resolve a [`ModelInfo`] for a model slug. Resolution follows the runtime
/// semantics: unknown slugs fall back to fallback model metadata (with config
/// overrides applied), so custom configured models keep working exactly as
/// they do on the existing runtime request path.
///
/// The catalog is shared by reference; cloning is cheap.
#[derive(Clone)]
pub struct ModelCatalog {
    manager: SharedModelsManager,
    config: ModelsManagerConfig,
}

impl ModelCatalog {
    /// Creates a catalog over a models manager and its configuration.
    pub fn new(manager: SharedModelsManager, config: ModelsManagerConfig) -> Self {
        Self { manager, config }
    }

    /// Returns the shared models manager backing the catalog.
    pub fn manager(&self) -> SharedModelsManager {
        std::sync::Arc::clone(&self.manager)
    }

    /// Resolves the effective model metadata for a model slug.
    ///
    /// Mirrors `ModelsManager::get_model_info`: longest-prefix catalog match
    /// (plus namespaced-slug retry), fallback metadata for unknown models,
    /// then configuration overrides.
    pub fn resolve<'a>(&'a self, model_id: &'a str) -> ModelsManagerFuture<'a, ModelInfo> {
        self.manager.get_model_info(model_id, &self.config)
    }
}

impl std::fmt::Debug for ModelCatalog {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ModelCatalog").finish_non_exhaustive()
    }
}
