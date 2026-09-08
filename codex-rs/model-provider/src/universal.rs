//! OpenAI Responses adapter for the frozen universal model contract.
//!
//! [`ModelContractAdapter`] exposes the existing OpenAI-compatible runtime
//! provider ([`crate::ModelProvider`]) behind the provider-neutral contract
//! types from `codex-model-contract`:
//!
//! - provider identity and descriptors ([`ModelDescriptor`]);
//! - effective capabilities with explicit negotiation
//!   ([`ModelCapabilities`]);
//! - normalized transport policy ([`ModelTransportPolicy`]) and
//!   authentication status ([`ModelAuthStatus`]);
//! - normalized streaming events ([`ModelEvent`]) and errors
//!   ([`ModelError`]) mapped from the OpenAI protocol;
//! - the OpenAI Responses wire request built from a [`ModelRequest`], with
//!   unsupported capabilities failing explicitly instead of silently
//!   degrading.
//!
//! This is the marshalling and policy boundary only. Request execution
//! through `codex_model_contract::ModelSession` remains owned by the runtime
//! model client and follow-on provider adapters (work order WO-004), so no
//! second execution path is introduced here.
//!
//! The adapter preserves the default OpenAI behavior of the existing runtime
//! request path; the legacy runtime path is untouched and keeps its
//! compatibility clamps, while this adapter surfaces negotiation failures
//! explicitly.

use codex_api::ApiError;
use codex_api::Reasoning;
use codex_api::ResponseEvent;
use codex_api::ResponsesApiRequest;
use codex_api::ResponsesApiTools;
use codex_api::SafetyBuffering;
use codex_api::TransportError;
use codex_api::create_text_param_for_request;
use codex_model_contract::ModelAuthStatus;
use codex_model_contract::ModelAuthStatusFuture;
use codex_model_contract::ModelCapabilities;
use codex_model_contract::ModelDescriptor;
use codex_model_contract::ModelError;
use codex_model_contract::ModelEvent;
use codex_model_contract::ModelFeature;
use codex_model_contract::ModelProvider as ContractModelProvider;
use codex_model_contract::ModelRequest;
use codex_model_contract::ModelSafetyBuffering;
use codex_model_contract::ModelSession;
use codex_model_contract::ModelToolChoice;
use codex_model_contract::ModelTransportPolicy;
use codex_models_manager::manager::SharedModelsManager;
use codex_protocol::models::ContentItem;
use codex_protocol::models::FunctionCallOutputContentItem;
use codex_protocol::models::ResponseItem;
use codex_protocol::openai_models::ModelInfo;
use codex_tools::create_tools_raw_json_for_responses_api;
use http::StatusCode;
use std::sync::Arc;

use crate::provider::SharedModelProvider;
use crate::session::OpenAiResponsesSession;

/// Value the Responses runtime includes to receive encrypted reasoning
/// content, matching the existing runtime request path.
const REASONING_ENCRYPTED_CONTENT_INCLUDE: &str = "reasoning.encrypted_content";

/// Universal-contract adapter over an OpenAI-compatible runtime provider.
///
/// Construct it with the provider registry key (for example `openai`) and
/// the runtime provider handle; the adapter never exposes provider protocol
/// types through its contract surface.
///
/// The adapter is the marshalling and policy boundary. To open an executable
/// [`ModelSession`] against the contract, attach a model catalog with
/// [`ModelContractAdapter::with_models_manager`] and then call
/// [`ModelContractAdapter::create_session`] (provided by the
/// [`codex_model_contract::ModelProvider`] impl).
#[derive(Debug, Clone)]
pub struct ModelContractAdapter {
    provider_id: String,
    provider: SharedModelProvider,
    models_manager: Option<SharedModelsManager>,
}

impl ModelContractAdapter {
    /// Wraps a runtime provider with its registry key.
    pub fn new(provider_id: impl Into<String>, provider: SharedModelProvider) -> Self {
        Self {
            provider_id: provider_id.into(),
            provider,
            models_manager: None,
        }
    }

    /// Attaches a model catalog so the adapter can resolve
    /// [`ModelInfo`] for a request's [`ModelDescriptor`] and open
    /// [`ModelSession`]s.
    ///
    /// Without a catalog, [`codex_model_contract::ModelProvider::create_session`]
    /// fails explicitly with [`ModelError::InvalidRequest`]; capability and
    /// wire-request marshalling remain available regardless.
    pub fn with_models_manager(mut self, models_manager: SharedModelsManager) -> Self {
        self.models_manager = Some(models_manager);
        self
    }

    /// Provider registry key this adapter was constructed with.
    pub fn provider_id(&self) -> &str {
        &self.provider_id
    }

    /// Returns the descriptor for a catalog model.
    pub fn descriptor(&self, model_info: &ModelInfo) -> ModelDescriptor {
        ModelDescriptor::new(self.provider_id.clone(), model_info.slug.clone())
            .with_display_name(model_info.display_name.clone())
    }

    /// Returns the descriptor for a model id without requiring catalog
    /// resolution. The descriptor is opaque identity only; capabilities are
    /// resolved separately through [`ModelContractAdapter::model_capabilities`].
    pub fn descriptor_for_id(&self, model_id: &str) -> ModelDescriptor {
        ModelDescriptor::new(self.provider_id.clone(), model_id)
    }

    /// Returns the underlying runtime provider handle, for adapters and
    /// sessions that need to resolve auth, transport, or catalog state.
    pub fn runtime_provider(&self) -> &SharedModelProvider {
        &self.provider
    }

    /// Returns the attached model catalog, when one was provided through
    /// [`ModelContractAdapter::with_models_manager`].
    pub fn models_manager(&self) -> Option<&SharedModelsManager> {
        self.models_manager.as_ref()
    }

    /// Returns the effective contract capabilities for a catalog model.
    ///
    /// Model-level support (reasoning, modalities, tiers) comes from the
    /// catalog entry; provider-owned upper bounds come from the runtime
    /// provider capabilities and provider info.
    pub fn model_capabilities(&self, model_info: &ModelInfo) -> ModelCapabilities {
        let provider_capabilities = self.provider.capabilities();
        let info = self.provider.info();
        ModelCapabilities {
            provider_id: self.provider_id.clone(),
            model_id: model_info.slug.clone(),
            supported_reasoning_efforts: model_info
                .supported_reasoning_levels
                .iter()
                .map(|preset| preset.effort.clone())
                .collect(),
            supports_reasoning_summary: model_info.supports_reasoning_summary_parameter,
            supports_verbosity: model_info.support_verbosity,
            input_modalities: model_info.input_modalities.clone(),
            context_window: model_info.resolved_context_window(),
            service_tiers: model_info
                .service_tiers
                .iter()
                .map(|tier| tier.id.clone())
                .collect(),
            supports_image_detail_original: model_info.supports_image_detail_original,
            namespaced_tools: provider_capabilities.namespace_tools,
            image_generation: provider_capabilities.image_generation,
            hosted_web_search: provider_capabilities.web_search,
            standalone_web_search: info.supports_standalone_web_search,
            external_web_access: provider_capabilities.external_web_access,
            remote_compaction: provider_capabilities.remote_compaction,
        }
    }

    /// Returns the normalized transport policy for the provider.
    pub fn transport_policy(&self) -> ModelTransportPolicy {
        let info = self.provider.info();
        ModelTransportPolicy {
            request_max_retries: info.request_max_retries(),
            stream_max_retries: info.stream_max_retries(),
            stream_idle_timeout: info.stream_idle_timeout(),
            websocket_connect_timeout: Some(info.websocket_connect_timeout()),
            supports_websockets: info.supports_websockets,
        }
    }

    /// Resolves the normalized authentication status; never exposes
    /// credentials, only status and an account label.
    pub async fn auth_status(&self) -> ModelAuthStatus {
        let info = self.provider.info();
        let auth = self.provider.auth().await;
        match auth {
            Some(auth) => ModelAuthStatus::Authenticated {
                account: auth.get_account_email(),
            },
            None => {
                let requires_auth = info.requires_openai_auth
                    || info.env_key.is_some()
                    || info.auth.is_some()
                    || info.aws.is_some()
                    || info.experimental_bearer_token.is_some();
                if requires_auth {
                    ModelAuthStatus::RequiresAuthentication {
                        instructions: info.env_key_instructions.clone(),
                    }
                } else {
                    ModelAuthStatus::NotRequired
                }
            }
        }
    }

    /// Builds the OpenAI Responses wire request from a universal request.
    ///
    /// Requested controls are negotiated against the model's effective
    /// capabilities first: unsupported reasoning efforts, reasoning
    /// summaries, verbosity, service tiers, or image inputs fail with
    /// [`ModelError::UnsupportedCapability`] diagnostics instead of being
    /// silently clamped or dropped.
    pub fn build_responses_request(
        &self,
        request: &ModelRequest,
        model_info: &ModelInfo,
    ) -> Result<ResponsesApiRequest, ModelError> {
        let capabilities = self.model_capabilities(model_info);
        capabilities.negotiate(required_features(request))?;

        let info = self.provider.info();
        let mut input = format_request_input(request, model_info);
        if !info.is_openai() {
            scrub_internal_input_metadata(&mut input);
        }
        let tools = Some(
            create_tools_raw_json_for_responses_api(&request.tools)
                .map(ResponsesApiTools::from)
                .map_err(|error| ModelError::InvalidRequest {
                    message: error.to_string(),
                })?,
        );

        Ok(ResponsesApiRequest {
            model: request.model.model_id.clone(),
            instructions: request.instructions.clone(),
            input,
            tools,
            tool_choice: tool_choice_wire_value(request.tool_choice).to_string(),
            parallel_tool_calls: request.parallel_tool_calls,
            reasoning: request.reasoning.as_ref().map(|controls| Reasoning {
                effort: controls.effort.clone(),
                summary: controls.summary,
                context: None,
            }),
            store: request.store,
            stream: request.stream,
            stream_options: None,
            include: vec![REASONING_ENCRYPTED_CONTENT_INCLUDE.to_string()],
            service_tier: request.service_tier.clone(),
            prompt_cache_key: request.prompt_cache_key.clone(),
            text: create_text_param_for_request(
                request.verbosity,
                &request.output_schema,
                request.output_schema_strict,
            ),
            client_metadata: None,
            access_programs: None,
        })
    }

    /// Maps an OpenAI Responses stream event to the normalized contract
    /// event. The mapping is exhaustive, so provider event shapes cannot be
    /// lost when the runtime moves behind the contract.
    pub fn to_model_event(event: ResponseEvent) -> ModelEvent {
        match event {
            ResponseEvent::Created { response_id } => ModelEvent::Created { response_id },
            ResponseEvent::SafetyBuffering(buffering) => {
                ModelEvent::SafetyBuffering(to_model_safety_buffering(buffering))
            }
            ResponseEvent::OutputItemDone(item) => ModelEvent::OutputItemDone(item),
            ResponseEvent::OutputItemAdded(item) => ModelEvent::OutputItemAdded(item),
            ResponseEvent::ServerModel(model) => ModelEvent::ServerModel(model),
            ResponseEvent::ModelVerifications(verifications) => {
                ModelEvent::ModelVerifications(verifications)
            }
            ResponseEvent::TurnModerationMetadata(moderation) => {
                ModelEvent::TurnModerationMetadata(moderation)
            }
            ResponseEvent::ServerReasoningIncluded(included) => {
                ModelEvent::ServerReasoningIncluded(included)
            }
            ResponseEvent::Completed {
                response_id,
                token_usage,
                usage_metadata,
                end_turn,
            } => ModelEvent::Completed {
                response_id,
                token_usage,
                usage_metadata,
                end_turn,
            },
            ResponseEvent::OutputTextDelta(delta) => ModelEvent::OutputTextDelta(delta),
            ResponseEvent::ToolCallInputDelta {
                item_id,
                call_id,
                delta,
            } => ModelEvent::ToolCallInputDelta {
                item_id,
                call_id,
                delta,
            },
            ResponseEvent::ReasoningSummaryDelta {
                delta,
                summary_index,
            } => ModelEvent::ReasoningSummaryDelta {
                delta,
                summary_index,
            },
            ResponseEvent::ReasoningSummaryDone {
                item_id,
                text,
                summary_index,
            } => ModelEvent::ReasoningSummaryDone {
                item_id,
                text,
                summary_index,
            },
            ResponseEvent::ReasoningContentDelta {
                delta,
                content_index,
            } => ModelEvent::ReasoningContentDelta {
                delta,
                content_index,
            },
            ResponseEvent::ReasoningSummaryPartAdded { summary_index } => {
                ModelEvent::ReasoningSummaryPartAdded { summary_index }
            }
            ResponseEvent::RateLimits(rate_limits) => ModelEvent::RateLimits(rate_limits),
            ResponseEvent::ModelsEtag(etag) => ModelEvent::ModelsEtag(etag),
        }
    }

    /// Maps an OpenAI API client error into the normalized error taxonomy.
    ///
    /// The provider's raw message is preserved for diagnostics but never
    /// contains credentials.
    pub fn to_model_error(error: ApiError) -> ModelError {
        match error {
            ApiError::Transport(transport) => to_model_error_from_transport(transport),
            ApiError::Api { status, message } => ModelError::Provider {
                status: Some(status.as_u16()),
                code: None,
                message,
            },
            ApiError::Stream(message) => ModelError::StreamIncomplete { message },
            ApiError::ContextWindowExceeded => ModelError::ContextWindowExceeded {
                message: "context window exceeded".to_string(),
            },
            ApiError::QuotaExceeded => ModelError::UsageLimitExceeded {
                message: "quota exceeded".to_string(),
            },
            ApiError::UsageNotIncluded => ModelError::UsageLimitExceeded {
                message: "usage not included".to_string(),
            },
            ApiError::Retryable { message, delay } => ModelError::Transport {
                message,
                retryable: true,
                retry_after: delay,
                status: None,
            },
            ApiError::RateLimitExceeded { message, delay } => ModelError::RateLimited {
                message,
                retry_after: delay,
            },
            ApiError::RateLimit(message) => ModelError::RateLimited {
                message,
                retry_after: None,
            },
            ApiError::InvalidRequest { message } => ModelError::InvalidRequest { message },
            ApiError::CyberPolicy { message } => ModelError::Provider {
                status: None,
                code: Some("cyber_policy".to_string()),
                message,
            },
            ApiError::MisalignmentPolicyViolation { message, .. } => ModelError::Provider {
                status: None,
                code: Some("misalignment_policy_violation".to_string()),
                message,
            },
            ApiError::ServerOverloaded => ModelError::Overloaded {
                message: "server overloaded".to_string(),
            },
        }
    }
}

/// Provider-neutral contract implementation for the OpenAI Responses runtime.
///
/// The adapter is one concrete [`codex_model_contract::ModelProvider`]: the
/// OpenAI-compatible path used by OpenAI, Azure Responses, Amazon Bedrock
/// (Mantle), and OpenAI-compatible OSS runtimes (Ollama, LM Studio). Each
/// provider is a separate adapter instance constructed with its own
/// registry key and runtime provider handle; provider swap is done by
/// constructing a second adapter for the same durable thread, not by
/// mutating this one.
///
/// Execution through [`ModelSession`] requires an attached model catalog
/// (see [`ModelContractAdapter::with_models_manager`]); without one,
/// [`ModelContractAdapter::create_session`] fails explicitly so callers
/// never silently construct a non-functional session.
impl ContractModelProvider for ModelContractAdapter {
    fn provider_id(&self) -> &str {
        ModelContractAdapter::provider_id(self)
    }

    fn transport_policy(&self) -> ModelTransportPolicy {
        ModelContractAdapter::transport_policy(self)
    }

    fn auth_status(&self) -> ModelAuthStatusFuture<'_> {
        let this = self.clone();
        Box::pin(async move { this.auth_status().await })
    }

    fn model_capabilities(&self, model: &ModelInfo) -> ModelCapabilities {
        ModelContractAdapter::model_capabilities(self, model)
    }

    fn descriptor(&self, model_id: &str) -> ModelDescriptor {
        ModelContractAdapter::descriptor_for_id(self, model_id)
    }

    fn create_session(&self) -> Result<Arc<dyn ModelSession>, ModelError> {
        let models_manager = self.models_manager.clone().ok_or_else(|| {
            ModelError::InvalidRequest {
                message: format!(
                    "model provider {} cannot open a session without an attached model catalog; \
                     construct the adapter with `with_models_manager`",
                    self.provider_id
                ),
            }
        })?;
        let session = OpenAiResponsesSession::new(self.clone(), models_manager);
        Ok(Arc::new(session))
    }
}

/// Features a request requires, derived from its requested controls.
fn required_features(request: &ModelRequest) -> Vec<ModelFeature> {
    let mut required = Vec::new();
    if let Some(reasoning) = request.reasoning.as_ref() {
        if let Some(effort) = reasoning.effort.as_ref() {
            required.push(ModelFeature::ReasoningEffort(effort.clone()));
        }
        if reasoning.summary.is_some() {
            required.push(ModelFeature::ReasoningSummary);
        }
    }
    if request.verbosity.is_some() {
        required.push(ModelFeature::Verbosity);
    }
    if let Some(service_tier) = request.service_tier.as_ref() {
        required.push(ModelFeature::ServiceTier(service_tier.clone()));
    }
    if request_contains_image_input(&request.input) {
        required.push(ModelFeature::ImageInput);
    }
    required
}

/// Returns whether the request input carries image content.
fn request_contains_image_input(input: &[ResponseItem]) -> bool {
    input.iter().any(|item| match item {
        ResponseItem::Message { content, .. } => content
            .iter()
            .any(|content_item| matches!(content_item, ContentItem::InputImage { .. })),
        ResponseItem::FunctionCallOutput { output, .. }
        | ResponseItem::CustomToolCallOutput { output, .. } => {
            output.content_items().is_some_and(|items| {
                items
                    .iter()
                    .any(|item| matches!(item, FunctionCallOutputContentItem::InputImage { .. }))
            })
        }
        ResponseItem::AdditionalTools { .. }
        | ResponseItem::Reasoning { .. }
        | ResponseItem::AgentMessage { .. }
        | ResponseItem::LocalShellCall { .. }
        | ResponseItem::FunctionCall { .. }
        | ResponseItem::ToolSearchCall { .. }
        | ResponseItem::CustomToolCall { .. }
        | ResponseItem::ToolSearchOutput { .. }
        | ResponseItem::WebSearchCall { .. }
        | ResponseItem::ImageGenerationCall { .. }
        | ResponseItem::Compaction { .. }
        | ResponseItem::ConfigurationUpdate { .. }
        | ResponseItem::CompactionTrigger { .. }
        | ResponseItem::ContextCompaction { .. }
        | ResponseItem::Other => false,
    })
}

/// Formats the request input for the wire, mirroring the runtime's
/// responses-lite image-detail stripping.
fn format_request_input(request: &ModelRequest, model_info: &ModelInfo) -> Vec<ResponseItem> {
    let mut input = request.input.clone();
    if model_info.use_responses_lite {
        strip_image_details(&mut input);
    }
    input
}

/// Strips image detail parameters, matching the runtime's responses-lite
/// input formatting.
fn strip_image_details(items: &mut [ResponseItem]) {
    for item in items {
        match item {
            ResponseItem::Message { content, .. } => {
                for content_item in content {
                    if let ContentItem::InputImage { detail, .. } = content_item {
                        *detail = None;
                    }
                }
            }
            ResponseItem::FunctionCallOutput { output, .. }
            | ResponseItem::CustomToolCallOutput { output, .. } => {
                if let Some(content) = output.content_items_mut() {
                    for content_item in content {
                        if let FunctionCallOutputContentItem::InputImage { detail, .. } =
                            content_item
                        {
                            *detail = None;
                        }
                    }
                }
            }
            ResponseItem::AdditionalTools { .. }
            | ResponseItem::Reasoning { .. }
            | ResponseItem::AgentMessage { .. }
            | ResponseItem::LocalShellCall { .. }
            | ResponseItem::FunctionCall { .. }
            | ResponseItem::ToolSearchCall { .. }
            | ResponseItem::CustomToolCall { .. }
            | ResponseItem::ToolSearchOutput { .. }
            | ResponseItem::WebSearchCall { .. }
            | ResponseItem::ImageGenerationCall { .. }
            | ResponseItem::Compaction { .. }
            | ResponseItem::ConfigurationUpdate { .. }
            | ResponseItem::CompactionTrigger { .. }
            | ResponseItem::ContextCompaction { .. }
            | ResponseItem::Other => {}
        }
    }
}

/// Removes OpenAI-internal passthrough metadata and encrypted function
/// arguments from request input, matching the runtime's non-OpenAI request
/// path.
fn scrub_internal_input_metadata(input: &mut [ResponseItem]) {
    for item in input {
        item.clear_internal_chat_message_metadata_passthrough();
        if let ResponseItem::FunctionCall {
            encrypted_function_args,
            ..
        } = item
        {
            *encrypted_function_args = None;
        }
    }
}

fn tool_choice_wire_value(tool_choice: ModelToolChoice) -> &'static str {
    match tool_choice {
        ModelToolChoice::Auto => "auto",
        ModelToolChoice::None => "none",
        ModelToolChoice::Required => "required",
    }
}

fn to_model_safety_buffering(buffering: SafetyBuffering) -> ModelSafetyBuffering {
    ModelSafetyBuffering {
        use_cases: buffering.use_cases,
        reasons: buffering.reasons,
        retry_model: buffering.faster_model,
    }
}

fn to_model_error_from_transport(transport: TransportError) -> ModelError {
    match transport {
        TransportError::Http { status, .. } => {
            if status == StatusCode::UNAUTHORIZED || status == StatusCode::FORBIDDEN {
                ModelError::Auth {
                    message: transport.to_string(),
                }
            } else {
                ModelError::Transport {
                    message: transport.to_string(),
                    retryable: status.is_server_error() || status == StatusCode::TOO_MANY_REQUESTS,
                    retry_after: None,
                    status: Some(status.as_u16()),
                }
            }
        }
        TransportError::RetryLimit => ModelError::Transport {
            message: transport.to_string(),
            retryable: false,
            retry_after: None,
            status: None,
        },
        TransportError::Timeout | TransportError::Connection(_) | TransportError::Network(_) => {
            ModelError::Transport {
                message: transport.to_string(),
                retryable: true,
                retry_after: None,
                status: None,
            }
        }
        TransportError::Build(message) => ModelError::InvalidRequest { message },
    }
}

#[cfg(test)]
#[path = "universal_tests.rs"]
mod tests;
