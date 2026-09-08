use codex_protocol::ResponseUsageMetadata;
use codex_protocol::models::ResponseItem;
use codex_protocol::protocol::TokenUsage;

use crate::event::ModelEvent;

/// Provider-neutral summary of a completed model invocation.
///
/// Aggregated from [`ModelEvent`]s by [`ModelResponse::from_events`] or
/// constructed directly by adapters. Carries only normalized output: response
/// identity, output items, usage, and turn-completion state.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ModelResponse {
    /// Server-assigned response identity, when the provider supplies one.
    pub response_id: Option<String>,
    /// Completed output items from the invocation.
    pub output: Vec<ResponseItem>,
    /// Token usage reported for the invocation.
    pub usage: Option<TokenUsage>,
    /// Usage metadata reported for the invocation.
    pub usage_metadata: Option<ResponseUsageMetadata>,
    /// Whether the model affirmatively ended its turn, when known.
    pub end_turn: Option<bool>,
    /// Model the provider actually served, when it differs from the request.
    pub server_model: Option<String>,
}

impl ModelResponse {
    /// Aggregates a completed event stream into a response summary.
    ///
    /// `output` collects every completed output item
    /// ([`ModelEvent::OutputItemDone`]); usage and completion state come from
    /// [`ModelEvent::Completed`]; a server-side model substitution is
    /// recorded from [`ModelEvent::ServerModel`]. Items added but never
    /// completed are not included, mirroring completed-item semantics of the
    /// runtime.
    pub fn from_events(events: impl IntoIterator<Item = ModelEvent>) -> Self {
        let mut response = Self::default();
        for event in events {
            match event {
                ModelEvent::Created { .. } => {}
                ModelEvent::SafetyBuffering(_) => {}
                ModelEvent::OutputItemAdded(_) => {}
                ModelEvent::OutputItemDone(item) => response.output.push(item),
                ModelEvent::ServerModel(model) => {
                    response.server_model = Some(model);
                }
                ModelEvent::ModelVerifications(_) => {}
                ModelEvent::TurnModerationMetadata(_) => {}
                ModelEvent::ServerReasoningIncluded(_) => {}
                ModelEvent::Completed {
                    response_id,
                    token_usage,
                    usage_metadata,
                    end_turn,
                } => {
                    response.response_id = Some(response_id);
                    response.usage = token_usage;
                    response.usage_metadata = usage_metadata;
                    response.end_turn = end_turn;
                }
                ModelEvent::OutputTextDelta(_) => {}
                ModelEvent::ToolCallInputDelta { .. } => {}
                ModelEvent::ReasoningSummaryDelta { .. } => {}
                ModelEvent::ReasoningSummaryDone { .. } => {}
                ModelEvent::ReasoningContentDelta { .. } => {}
                ModelEvent::ReasoningSummaryPartAdded { .. } => {}
                ModelEvent::RateLimits(_) => {}
                ModelEvent::ModelsEtag(_) => {}
            }
        }
        response
    }
}

#[cfg(test)]
#[path = "response_tests.rs"]
mod tests;
