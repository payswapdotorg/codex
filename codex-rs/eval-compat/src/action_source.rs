//! The model-backed action-proposal seam for evaluation runs (WO-013).
//!
//! In the real application, the Codex agent/model loop proposes concrete
//! actions through the WO-010 [`StepActionSource`] port. In evaluation,
//! [`ModelBackedActionSource`] plays that role: it turns one action request
//! into a provider-neutral [`ModelRequest`], drives the scripted provider
//! through the real session/stream contract (via
//! [`ScriptedModelProvider::respond`]), and translates the model's
//! completed output into a validated execution [`Action`].
//!
//! The translation protocol is deliberately minimal and versioned with the
//! fixtures: the model answers with one assistant message whose text is a
//! JSON object `{"operation": ..., "target": ..., "inputs": ...}`.
//! Malformed or non-conforming output fails loudly as
//! `ActionUnavailable` — which flows into the standard escalation pipeline
//! — instead of being silently dropped.

use std::collections::BTreeMap;
use std::sync::Arc;

use codex_execution_contracts::Action;
use codex_execution_contracts::ActionInputs;
use codex_execution_contracts::ActionTarget;
use codex_execution_contracts::OperationId;
use codex_model_contract::ModelProvider;
use codex_model_contract::ModelRequest;
use codex_protocol::models::ContentItem;
use codex_protocol::models::ResponseItem;
use codex_workflow_app::ActionRequest;
use codex_workflow_app::StepActionSource;
use codex_workflow_app::WorkflowAppError;

use crate::model_fixture::ScriptedModelProvider;

/// The instructions every action-proposal request carries.
const ACTION_PROPOSAL_INSTRUCTIONS: &str = "You propose one workflow action. Answer with a single JSON object with an `operation` (lowercase snake_case), an optional `target`, and optional `inputs`.";

/// An action-proposal source served by a scripted model provider.
#[derive(Debug)]
pub struct ModelBackedActionSource {
    provider: Arc<ScriptedModelProvider>,
    model_slug: String,
}

impl ModelBackedActionSource {
    /// Creates the source proposing actions through `provider` for
    /// `model_slug`.
    pub fn new(provider: Arc<ScriptedModelProvider>, model_slug: impl Into<String>) -> Self {
        Self {
            provider,
            model_slug: model_slug.into(),
        }
    }

    /// Builds the deterministic action-proposal request for one step
    /// capability.
    ///
    /// The request carries the step context (node, capability, purpose,
    /// selected binding) as a single user message; only the model identity
    /// is provider-specific.
    fn request_for(&self, request: &ActionRequest) -> ModelRequest {
        let context = serde_json::json!({
            "node": request.node.to_string(),
            "capability": request.requirement.capability.as_ref().to_string(),
            "purpose": request.requirement.purpose,
            "binding": request.selected.binding.to_string(),
        });
        ModelRequest {
            model: self.provider.descriptor(&self.model_slug),
            instructions: ACTION_PROPOSAL_INSTRUCTIONS.to_string(),
            input: vec![user_message(context.to_string())],
            ..ModelRequest::default()
        }
    }
}

impl StepActionSource for ModelBackedActionSource {
    fn action_for(&mut self, request: &ActionRequest) -> Result<Action, WorkflowAppError> {
        let unavailable = |reason: String| WorkflowAppError::ActionUnavailable {
            node: request.node.to_string(),
            capability: request.requirement.capability.as_ref().to_string(),
            reason,
        };
        let invocation = self.provider.respond(self.request_for(request));
        let response =
            invocation.map_err(|error| unavailable(format!("model invocation failed: {error}")))?;
        let output = response
            .output
            .first()
            .ok_or_else(|| unavailable("model produced no completed output items".to_string()))?;
        let text = output_text(output).ok_or_else(|| {
            unavailable("model output is not a single assistant text message".to_string())
        })?;
        parse_action(text)
            .map_err(|error| unavailable(format!("model output is not a valid action: {error}")))
    }
}

/// Parses the fixture action protocol into a validated [`Action`].
fn parse_action(text: &str) -> Result<Action, WorkflowAppError> {
    let spec: ActionSpec = serde_json::from_str(text).map_err(|error| {
        WorkflowAppError::Execution(
            codex_execution_contracts::ExecutionContractError::InvalidRecord {
                reason: format!("malformed action proposal: {error}"),
            },
        )
    })?;
    let operation = OperationId::parse(spec.operation).map_err(|error| {
        WorkflowAppError::Execution(
            codex_execution_contracts::ExecutionContractError::InvalidRecord {
                reason: format!("malformed action proposal: {error}"),
            },
        )
    })?;
    let target = match spec.target {
        Some(target) => Some(ActionTarget::parse(target).map_err(|error| {
            WorkflowAppError::Execution(
                codex_execution_contracts::ExecutionContractError::InvalidRecord {
                    reason: format!("malformed action proposal: {error}"),
                },
            )
        })?),
        None => None,
    };
    let mut inputs = ActionInputs::default();
    for (key, value) in spec.inputs {
        inputs.insert(key, value);
    }
    Ok(Action {
        operation,
        target,
        inputs,
    })
}

/// The wire shape of one proposed action.
#[derive(serde::Deserialize)]
struct ActionSpec {
    operation: String,
    target: Option<String>,
    #[serde(default)]
    inputs: BTreeMap<String, serde_json::Value>,
}

/// Builds the user message carrying `text`.
fn user_message(text: String) -> ResponseItem {
    ResponseItem::Message {
        id: None,
        role: "user".to_string(),
        content: vec![ContentItem::InputText { text }],
        phase: None,
        internal_chat_message_metadata_passthrough: None,
    }
}

/// Returns the text of a message item that is exactly one output text.
fn output_text(item: &ResponseItem) -> Option<&str> {
    let ResponseItem::Message { content, .. } = item else {
        return None;
    };
    match content.as_slice() {
        [ContentItem::OutputText { text }] => Some(text),
        _ => None,
    }
}
