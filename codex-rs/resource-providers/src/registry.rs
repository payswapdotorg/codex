//! The host wiring registry for execution resource providers (WO-016).
//!
//! [`ProviderRegistry`] is a small, resource-plane-only lookup of the
//! providers a host wired up, plus their credential *references*. It
//! exists so host code can ask "which provider serves this id" and probe
//! the fleet — nothing more:
//!
//! - it does **not** duplicate or bypass the WO-005 `CapabilityRegistry`:
//!   capability binding, readiness, authorization, and fallback remain
//!   that registry's authority under policy and approvals;
//! - it does **not** select providers for workflows: the host (or a
//!   future control-plane Work Order) reads descriptors and makes
//!   selection decisions;
//! - it stores **no credential values**: [`ProviderCredentials`] carries
//!   an environment-variable *name* at most, following the
//!   `codex-model-provider-info` `env_key` precedent. Resolution of the
//!   referenced material is host-owned; the secrets crate's
//!   `SecretsManager` is the approved store.

use std::collections::BTreeMap;
use std::sync::Arc;

use crate::ResourceProviderError;
use crate::descriptor::ProviderHealth;
use crate::model::ProviderId;
use crate::provider::ExecutionResourceProvider;

/// Maximum length of a credential environment-variable name.
pub const CREDENTIAL_ENV_KEY_MAX_BYTES: usize = 128;

/// A runtime configuration reference to a provider's credentials.
///
/// The reference is a *name*, never a value: `EnvKey` names the
/// environment variable the host's secret machinery resolves. No
/// credential material ever enters this type, the registry, provider
/// records, or evidence payloads.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ProviderCredentials {
    /// The name of the environment variable holding the provider's
    /// secret material (for example `SANDBOX_CLOUD_API_KEY`), resolved
    /// host-side through the approved secret store.
    EnvKey(String),
}

impl ProviderCredentials {
    /// Parses an environment-variable *name* reference.
    ///
    /// Names must be non-empty, at most
    /// [`CREDENTIAL_ENV_KEY_MAX_BYTES`] bytes, and free of characters
    /// that cannot appear in an environment-variable name.
    pub fn parse_env_key(name: impl Into<String>) -> Result<Self, ResourceProviderError> {
        let name = name.into();
        let valid = !name.is_empty()
            && name.len() <= CREDENTIAL_ENV_KEY_MAX_BYTES
            && name
                .chars()
                .all(|character| character.is_ascii_graphic() && character != '=');
        if valid {
            Ok(Self::EnvKey(name))
        } else {
            Err(ResourceProviderError::InvalidIdentifier {
                kind: "credential env key",
                value: name,
                reason: "must be a non-empty environment-variable name",
            })
        }
    }

    /// The referenced environment-variable name.
    pub fn env_key_name(&self) -> &str {
        match self {
            Self::EnvKey(name) => name,
        }
    }
}

/// The host-side provider wiring registry.
///
/// Registration validates each provider's descriptor so a malformed
/// declaration cannot silently mislead selection, and rejects duplicate
/// provider identities.
#[derive(Default)]
pub struct ProviderRegistry {
    providers: BTreeMap<ProviderId, Arc<dyn ExecutionResourceProvider>>,
    credentials: BTreeMap<ProviderId, ProviderCredentials>,
}

impl ProviderRegistry {
    /// Creates an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers one provider.
    ///
    /// Returns the provider's identity. Duplicate registrations are
    /// rejected explicitly.
    pub fn register(
        &mut self,
        provider: Arc<dyn ExecutionResourceProvider>,
    ) -> Result<ProviderId, ResourceProviderError> {
        let id = provider.descriptor().provider.clone();
        provider.descriptor().validate()?;
        if self.providers.contains_key(&id) {
            return Err(ResourceProviderError::InvalidSpec {
                reason: format!("provider `{id}` is already registered"),
            });
        }
        self.providers.insert(id.clone(), provider);
        Ok(id)
    }

    /// Registers one provider together with its credential reference.
    pub fn register_with_credentials(
        &mut self,
        provider: Arc<dyn ExecutionResourceProvider>,
        credentials: ProviderCredentials,
    ) -> Result<ProviderId, ResourceProviderError> {
        let id = self.register(provider)?;
        self.credentials.insert(id.clone(), credentials);
        Ok(id)
    }

    /// The provider registered under `id`, when wired.
    pub fn provider(&self, id: &ProviderId) -> Option<Arc<dyn ExecutionResourceProvider>> {
        self.providers.get(id).cloned()
    }

    /// Every registered provider identity, in id order.
    pub fn provider_ids(&self) -> impl Iterator<Item = &ProviderId> {
        self.providers.keys()
    }

    /// Number of registered providers.
    pub fn len(&self) -> usize {
        self.providers.len()
    }

    /// Whether no provider is registered (the ordinary-Codex default).
    pub fn is_empty(&self) -> bool {
        self.providers.is_empty()
    }

    /// The credential reference registered for `id`, when any.
    ///
    /// The reference names an environment variable at most; the value
    /// never exists here.
    pub fn credentials(&self, id: &ProviderId) -> Option<&ProviderCredentials> {
        self.credentials.get(id)
    }

    /// Probes every registered provider's live health, in id order.
    ///
    /// Probe failures are collected as errors, not short-circuits, so a
    /// fleet report is complete and every outage is diagnosable.
    pub fn probe_all(&self) -> Vec<(ProviderId, Result<ProviderHealth, ResourceProviderError>)> {
        self.providers
            .iter()
            .map(|(id, provider)| (id.clone(), provider.probe()))
            .collect()
    }
}

#[cfg(test)]
#[path = "registry_tests.rs"]
mod tests;
