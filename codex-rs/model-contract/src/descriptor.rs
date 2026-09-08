use serde::Deserialize;
use serde::Serialize;
use std::fmt;

/// Provider-neutral identity of a model invocation target.
///
/// This is the only model/provider identity shape that durable thread or
/// workflow state may reference. Both identifier fields are opaque strings:
/// the universal model plane never persists provider endpoints, wire
/// configuration, credentials, or protocol details alongside them.
///
/// `provider_id` is the provider registry key (for example `openai`), and
/// `model_id` is the provider-scoped model slug (for example `gpt-5.6-luna`).
/// Neither value is interpreted by the contract itself; providers own their
/// resolution.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ModelDescriptor {
    /// Registry key of the provider that owns the model.
    pub provider_id: String,
    /// Provider-scoped model identifier (model slug).
    pub model_id: String,
    /// Optional human-readable display name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
}

impl ModelDescriptor {
    /// Creates a descriptor from a provider registry key and model slug.
    pub fn new(provider_id: impl Into<String>, model_id: impl Into<String>) -> Self {
        Self {
            provider_id: provider_id.into(),
            model_id: model_id.into(),
            display_name: None,
        }
    }

    /// Attaches a display name for presentation surfaces.
    pub fn with_display_name(mut self, display_name: impl Into<String>) -> Self {
        self.display_name = Some(display_name.into());
        self
    }

    /// Returns the qualified `provider_id/model_id` identifier.
    pub fn qualified_id(&self) -> String {
        format!("{}/{}", self.provider_id, self.model_id)
    }
}

impl fmt::Display for ModelDescriptor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.qualified_id())
    }
}

#[cfg(test)]
#[path = "descriptor_tests.rs"]
mod tests;
