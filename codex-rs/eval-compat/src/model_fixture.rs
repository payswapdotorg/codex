//! Deterministic model-plane doubles for evaluation fixtures (WO-013).
//!
//! This module provides the scripted provider and session that stand in for
//! real model providers inside [`crate`]'s evaluation harnesses. The doubles
//! implement the frozen WO-002 universal model contract
//! ([`ModelProvider`], [`ModelSession`]) directly, so every evaluation run
//! exercises the same provider boundary the runtime uses — selection
//! (through `codex-model-provider`'s selection helpers), capability
//! negotiation, session creation, and normalized event streaming — without
//! touching a network, a credential, or a wall clock.
//!
//! # Determinism rules
//!
//! - Scripts are consumed strictly in order; a request when the script is
//!   exhausted fails loudly with `ModelError::Provider` (`script-exhausted`)
//!   instead of silently succeeding.
//! - Events are emitted through the real [`ModelStream`] channel contract
//!   with a channel capacity sized to the turn, so emission cannot reorder,
//!   drop, or block.
//! - Requests are recorded (in order) for evaluation evidence.
//! - Providers report a fixed transport policy and `NotRequired` auth so
//!   runs never depend on environment state.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::MutexGuard;
use std::sync::PoisonError;

use codex_model_contract::ModelAuthStatus;
use codex_model_contract::ModelAuthStatusFuture;
use codex_model_contract::ModelCapabilities;
use codex_model_contract::ModelDescriptor;
use codex_model_contract::ModelError;
use codex_model_contract::ModelEvent;
use codex_model_contract::ModelProvider;
use codex_model_contract::ModelRequest;
use codex_model_contract::ModelResponse;
use codex_model_contract::ModelSession;
use codex_model_contract::ModelSessionFuture;
use codex_model_contract::ModelStream;
use codex_model_contract::ModelStreamItem;
use codex_model_contract::ModelTransportPolicy;
use codex_protocol::models::ContentItem;
use codex_protocol::models::ResponseItem;
use codex_protocol::openai_models::InputModality;
use codex_protocol::openai_models::ModelInfo;
use codex_protocol::openai_models::ReasoningEffort;

/// The model slug every scripted fixture uses.
pub const EVAL_MODEL_SLUG: &str = "eval-scripted-model";

/// One scripted model invocation outcome.
///
/// A turn either completes with output items (aggregated by
/// [`ModelResponse::from_events`]) or fails with a normalized
/// [`ModelError`]. There is no third path: an exhausted script is a loud
/// provider failure, never a silent success.
#[derive(Clone, Debug, PartialEq)]
pub enum ScriptedTurn {
    /// The invocation completes with the given output items.
    Respond {
        /// Server-assigned response identity (excluded from semantic
        /// equivalence classes).
        response_id: String,
        /// Completed output items.
        output: Vec<ResponseItem>,
        /// Whether the model affirmatively ended its turn.
        end_turn: Option<bool>,
    },
    /// The invocation fails with a normalized model error.
    Fail {
        /// The normalized provider failure.
        error: ModelError,
    },
}

impl ScriptedTurn {
    /// A turn that answers with one assistant message `text`.
    pub fn message(text: impl Into<String>) -> Self {
        Self::Respond {
            response_id: "eval-scripted-response".to_string(),
            output: vec![assistant_message(text)],
            end_turn: Some(true),
        }
    }

    /// A turn that answers with several assistant messages.
    pub fn messages(texts: Vec<String>) -> Self {
        Self::Respond {
            response_id: "eval-scripted-response".to_string(),
            output: texts.into_iter().map(assistant_message).collect(),
            end_turn: Some(true),
        }
    }

    /// A turn that fails with a normalized model error.
    pub fn fail(error: ModelError) -> Self {
        Self::Fail { error }
    }
}

/// Builds one assistant message output item.
fn assistant_message(text: impl Into<String>) -> ResponseItem {
    ResponseItem::Message {
        id: None,
        role: "assistant".to_string(),
        content: vec![ContentItem::OutputText { text: text.into() }],
        phase: None,
        internal_chat_message_metadata_passthrough: None,
    }
}

/// A scripted provider double implementing the universal model contract.
///
/// The provider serves [`ScriptedTurn`]s in order, records every request,
/// and resolves capabilities from an explicit per-slug catalog. Unknown
/// slugs resolve to empty capabilities so capability negotiation fails
/// explicitly instead of silently degrading.
#[derive(Debug)]
pub struct ScriptedModelProvider {
    provider_id: String,
    catalog: BTreeMap<String, ModelCapabilities>,
    script: Arc<Mutex<Vec<ScriptedTurn>>>,
    requests: Arc<Mutex<Vec<ModelRequest>>>,
    provider_label: Arc<str>,
}

impl ScriptedModelProvider {
    /// Creates a provider with `provider_id` serving `script` in order.
    pub fn new(provider_id: impl Into<String>, script: Vec<ScriptedTurn>) -> Self {
        let provider_id = provider_id.into();
        let provider_label: Arc<str> = Arc::from(provider_id.as_str());
        Self {
            provider_id,
            catalog: BTreeMap::new(),
            script: Arc::new(Mutex::new(script)),
            requests: Arc::new(Mutex::new(Vec::new())),
            provider_label,
        }
    }

    /// Registers the capabilities served for one model slug.
    pub fn with_capability(mut self, model_slug: &str, capabilities: ModelCapabilities) -> Self {
        self.catalog.insert(model_slug.to_string(), capabilities);
        self
    }

    /// The provider registry key.
    pub fn id(&self) -> &str {
        &self.provider_id
    }

    /// Number of unscripted turns remaining.
    pub fn remaining(&self) -> usize {
        self.lock_script().len()
    }

    /// Snapshot of the requests served so far, in order.
    pub fn requests(&self) -> Vec<ModelRequest> {
        self.lock_requests().clone()
    }

    /// Number of requests served so far.
    pub fn request_count(&self) -> usize {
        self.lock_requests().len()
    }

    /// Synchronously drives one scripted invocation through the real
    /// session/stream contract path.
    ///
    /// This is the sync bridge the workflow action-proposal seam needs
    /// ([`crate::action_source`]): it creates a session, streams the request,
    /// collects every stream item, and aggregates the response through
    /// [`ModelResponse::from_events`]. The scripted future is immediate (no
    /// timers, no IO), so driving it with the futures executor is
    /// deterministic and cannot deadlock.
    pub fn respond(&self, request: ModelRequest) -> Result<ModelResponse, ModelError> {
        let session = self.create_session()?;
        let stream = futures::executor::block_on(session.stream(request))?;
        let items = futures::executor::block_on(collect_stream(stream));
        let mut events = Vec::with_capacity(items.len());
        for item in items {
            match item {
                ModelStreamItem::Ok(event) => events.push(event),
                ModelStreamItem::Err(error) => return Err(error),
            }
        }
        Ok(ModelResponse::from_events(events))
    }

    fn lock_script(&self) -> MutexGuard<'_, Vec<ScriptedTurn>> {
        self.script.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn lock_requests(&self) -> MutexGuard<'_, Vec<ModelRequest>> {
        self.requests.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl ModelProvider for ScriptedModelProvider {
    fn provider_id(&self) -> &str {
        &self.provider_id
    }

    fn transport_policy(&self) -> ModelTransportPolicy {
        // Fully deterministic, deliberately distinct from the default policy
        // so evaluation records surface transport as provider-owned data.
        ModelTransportPolicy {
            request_max_retries: 0,
            stream_max_retries: 0,
            stream_idle_timeout: std::time::Duration::from_millis(60_000),
            websocket_connect_timeout: None,
            supports_websockets: false,
        }
    }

    fn auth_status(&self) -> ModelAuthStatusFuture<'_> {
        Box::pin(async { ModelAuthStatus::NotRequired })
    }

    fn model_capabilities(&self, model: &ModelInfo) -> ModelCapabilities {
        match self.catalog.get(&model.slug) {
            Some(capabilities) => capabilities.clone(),
            // Unknown slugs resolve to empty capabilities: negotiation then
            // fails explicitly with the requested feature, never silently
            // degrades.
            None => ModelCapabilities {
                provider_id: self.provider_id.clone(),
                model_id: model.slug.clone(),
                ..ModelCapabilities::default()
            },
        }
    }

    fn descriptor(&self, model_id: &str) -> ModelDescriptor {
        ModelDescriptor::new(&self.provider_id, model_id)
    }

    fn create_session(&self) -> Result<Arc<dyn ModelSession>, ModelError> {
        Ok(Arc::new(ScriptedModelSession {
            script: self.script.clone(),
            requests: self.requests.clone(),
            provider_label: self.provider_label.clone(),
        }))
    }
}

/// A scripted session sharing its provider's script and request log.
#[derive(Debug)]
struct ScriptedModelSession {
    script: Arc<Mutex<Vec<ScriptedTurn>>>,
    requests: Arc<Mutex<Vec<ModelRequest>>>,
    provider_label: Arc<str>,
}

impl ScriptedModelSession {
    fn pop_turn(&self) -> Option<ScriptedTurn> {
        let mut script = self.script.lock().unwrap_or_else(PoisonError::into_inner);
        if script.is_empty() {
            None
        } else {
            Some(script.remove(0))
        }
    }

    fn record_request(&self, request: &ModelRequest) {
        let mut requests = self.requests.lock().unwrap_or_else(PoisonError::into_inner);
        requests.push(request.clone());
    }
}

impl ModelSession for ScriptedModelSession {
    fn stream(&self, request: ModelRequest) -> ModelSessionFuture<'_> {
        Box::pin(async move {
            self.record_request(&request);
            let Some(turn) = self.pop_turn() else {
                return Err(ModelError::Provider {
                    status: None,
                    code: Some("script-exhausted".to_string()),
                    message: format!(
                        "scripted model provider `{provider_label}` has no turn left",
                        provider_label = self.provider_label
                    ),
                });
            };
            match turn {
                ScriptedTurn::Respond {
                    response_id,
                    output,
                    end_turn,
                } => {
                    let mut events: Vec<ModelStreamItem> =
                        Vec::with_capacity(output.len().saturating_add(2));
                    events.push(ModelStreamItem::Ok(ModelEvent::Created {
                        response_id: None,
                    }));
                    for item in output {
                        events.push(ModelStreamItem::Ok(ModelEvent::OutputItemDone(item)));
                    }
                    events.push(ModelStreamItem::Ok(ModelEvent::Completed {
                        response_id: response_id.clone(),
                        token_usage: None,
                        usage_metadata: None,
                        end_turn,
                    }));
                    // Channel capacity equals the event count, so emission
                    // cannot fail.
                    let (sender, stream) = ModelStream::channel(events.len(), Some(response_id));
                    for event in events {
                        if let Err(send_error) = sender.try_send(event) {
                            return Err(ModelError::Provider {
                                status: None,
                                code: Some("stream-send-failed".to_string()),
                                message: format!("scripted stream emission failed: {send_error:?}"),
                            });
                        }
                    }
                    // Dropping the sender closes the channel: the stream
                    // ends after the buffered events, mirroring a completed
                    // provider turn.
                    drop(sender);
                    Ok(stream)
                }
                ScriptedTurn::Fail { error } => {
                    let (sender, stream) = ModelStream::channel(1, None);
                    if let Err(send_error) = sender.try_send(ModelStreamItem::Err(error)) {
                        return Err(ModelError::Provider {
                            status: None,
                            code: Some("stream-send-failed".to_string()),
                            message: format!("scripted stream emission failed: {send_error:?}"),
                        });
                    }
                    drop(sender);
                    Ok(stream)
                }
            }
        })
    }
}

/// Collects every item of a completed (already fed) scripted stream.
async fn collect_stream(mut stream: ModelStream) -> Vec<ModelStreamItem> {
    use futures::StreamExt;
    let mut items = Vec::new();
    while let Some(item) = stream.next().await {
        items.push(item);
    }
    items
}

/// The deterministic model catalog entry every fixture uses.
///
/// The entry mirrors the shape of a real catalog model (reasoning efforts,
/// modalities, context window, service tiers) so capability negotiation
/// exercises the real negotiation logic with meaningful values. Returns a
/// `serde_json` error only if the fixture JSON itself is malformed (a
/// fixture bug, never a system-under-evaluation failure).
pub fn fixture_model_info() -> Result<ModelInfo, serde_json::Error> {
    serde_json::from_value(serde_json::json!({
        "slug": EVAL_MODEL_SLUG,
        "display_name": EVAL_MODEL_SLUG,
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
        "truncation_policy": { "mode": "bytes", "limit": 10000 },
        "supports_image_detail_original": false,
        "context_window": 272000,
        "max_context_window": 272000,
        "experimental_supported_tools": [],
        "input_modalities": ["text", "image"],
        "service_tiers": [
            { "id": "standard", "name": "Standard", "description": "" }
        ],
    }))
}

/// Capabilities equivalent across providers, modulo the provider identity.
///
/// Two scripted providers equipped with these capabilities negotiate the
/// same features for [`fixture_model_info`]; only `provider_id` differs,
/// which is exactly the provider-substitution equivalence class WO-013
/// evaluates.
pub fn equivalent_capabilities(provider_id: impl Into<String>) -> ModelCapabilities {
    ModelCapabilities {
        provider_id: provider_id.into(),
        model_id: EVAL_MODEL_SLUG.to_string(),
        supported_reasoning_efforts: vec![
            ReasoningEffort::Low,
            ReasoningEffort::Medium,
            ReasoningEffort::High,
        ],
        supports_reasoning_summary: true,
        supports_verbosity: true,
        input_modalities: vec![InputModality::Text, InputModality::Image],
        context_window: Some(272_000),
        service_tiers: vec!["standard".to_string()],
        supports_image_detail_original: false,
        namespaced_tools: false,
        image_generation: false,
        hosted_web_search: false,
        standalone_web_search: false,
        external_web_access: false,
        remote_compaction: codex_model_contract::RemoteCompactionSupport::Unsupported,
    }
}
