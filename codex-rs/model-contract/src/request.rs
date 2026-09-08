use std::sync::Arc;

use codex_protocol::config_types::ReasoningSummary;
use codex_protocol::config_types::Verbosity;
use codex_protocol::models::ResponseItem;
use codex_tools::ToolSpec;
use serde::Serialize;
use serde_json::Value;

use crate::descriptor::ModelDescriptor;

/// Tool selection policy for a model request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ModelToolChoice {
    /// Let the model decide when to call tools.
    #[default]
    Auto,
    /// Forbid tool calls.
    None,
    /// Require at least one tool call.
    Required,
}

/// Reasoning controls attached to a model request.
///
/// Values are requests, not guarantees: adapters negotiate them against
/// [`crate::ModelCapabilities`] and must fail with
/// [`crate::UnsupportedModelCapability`] when a control is unsupported
/// instead of silently dropping it.
#[derive(Debug, Clone, PartialEq, Serialize, Default)]
pub struct ModelReasoningControls {
    /// Requested reasoning effort.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effort: Option<codex_protocol::openai_models::ReasoningEffort>,
    /// Requested reasoning summary behavior.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<ReasoningSummary>,
}

/// Provider-neutral request for one model invocation.
///
/// A `ModelRequest` carries the full semantic content of an invocation:
/// identity ([`ModelDescriptor`]), instructions, conversation items, and tool
/// declarations. Providers translate it into their own wire format behind the
/// adapter boundary; provider protocol types, endpoints, credentials, and
/// transport configuration never appear here.
///
/// Two providers serving the same durable thread receive requests whose
/// semantic fields are identical; only the descriptor's provider identity
/// differs.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ModelRequest {
    /// Model the request targets.
    pub model: ModelDescriptor,
    /// Base instructions for the invocation.
    pub instructions: String,
    /// Conversation items forming the request context.
    pub input: Vec<ResponseItem>,
    /// Tool declarations available to the model.
    pub tools: Arc<[ToolSpec]>,
    /// Tool selection policy.
    pub tool_choice: ModelToolChoice,
    /// Whether parallel tool calls are permitted.
    pub parallel_tool_calls: bool,
    /// Requested reasoning controls, when any.
    pub reasoning: Option<ModelReasoningControls>,
    /// JSON schema the model output should follow, when requested.
    pub output_schema: Option<Value>,
    /// Whether the output schema should be strictly validated.
    pub output_schema_strict: bool,
    /// Requested output verbosity, when any.
    pub verbosity: Option<Verbosity>,
    /// Whether the provider should store the response server-side.
    pub store: bool,
    /// Whether the invocation streams events.
    pub stream: bool,
    /// Requested service tier, when any.
    pub service_tier: Option<String>,
    /// Provider-side prompt cache key, when the provider supports one.
    pub prompt_cache_key: Option<String>,
}

impl Default for ModelRequest {
    fn default() -> Self {
        Self {
            model: ModelDescriptor::new("", ""),
            instructions: String::new(),
            input: Vec::new(),
            tools: Arc::default(),
            tool_choice: ModelToolChoice::default(),
            parallel_tool_calls: false,
            reasoning: None,
            output_schema: None,
            output_schema_strict: true,
            verbosity: None,
            store: false,
            stream: true,
            service_tier: None,
            prompt_cache_key: None,
        }
    }
}

impl ModelRequest {
    /// Returns the request addressed to a different provider with identical
    /// semantic content.
    ///
    /// Only the descriptor's provider identity changes; instructions, input
    /// items, tools, and controls are shared. This is the provider-swap
    /// primitive that keeps durable thread state stable when the serving
    /// provider changes.
    pub fn for_provider(&self, provider_id: impl Into<String>) -> Self {
        Self {
            model: ModelDescriptor {
                provider_id: provider_id.into(),
                model_id: self.model.model_id.clone(),
                display_name: self.model.display_name.clone(),
            },
            ..self.clone()
        }
    }
}

#[cfg(test)]
#[path = "request_tests.rs"]
mod tests;
