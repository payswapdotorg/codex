use codex_protocol::ResponseUsageMetadata;
use codex_protocol::models::ResponseItem;
use codex_protocol::protocol::ModelVerification;
use codex_protocol::protocol::RateLimitSnapshot;
use codex_protocol::protocol::TokenUsage;
use codex_protocol::protocol::TurnModerationMetadataEvent;

/// Provider-neutral safety-buffering notice.
///
/// Mapped from provider-specific payloads; carries no provider protocol
/// types. `retry_model` names the model the provider suggests retrying with,
/// when it communicates one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelSafetyBuffering {
    /// Use cases the buffering decision applies to.
    pub use_cases: Vec<String>,
    /// Reasons the provider reported for buffering.
    pub reasons: Vec<String>,
    /// Model the provider recommends retrying with, if any.
    pub retry_model: Option<String>,
}

/// Normalized streaming events for one model invocation.
///
/// Events carry only provider-neutral payloads: conversation items
/// ([`ResponseItem`]), usage ([`TokenUsage`], [`ResponseUsageMetadata`]),
/// rate limits, and diagnostics. The lifecycle is
/// [`ModelEvent::Created`] -> zero or more item/delta events ->
/// [`ModelEvent::Completed`]; failures surface through the stream's
/// `Err` item ([`crate::ModelError`]), not as events.
///
/// Each event type is the normalized equivalent of one provider stream
/// event; adapters must map exhaustively so no provider behavior is lost.
#[derive(Debug, Clone, PartialEq)]
#[allow(clippy::large_enum_variant)]
pub enum ModelEvent {
    /// The provider accepted the request and started the response.
    Created {
        /// Existing server response identity, when supplied.
        response_id: Option<String>,
    },
    /// The provider is buffering the turn for safety reasons.
    SafetyBuffering(ModelSafetyBuffering),
    /// An output item completed.
    OutputItemDone(ResponseItem),
    /// An output item was added to the response.
    OutputItemAdded(ResponseItem),
    /// The provider served the request with a different model than requested.
    ServerModel(String),
    /// The provider recommends additional account verification.
    ModelVerifications(Vec<ModelVerification>),
    /// Moderation metadata attached to the turn by the provider.
    TurnModerationMetadata(TurnModerationMetadataEvent),
    /// The provider already accounted for past reasoning tokens.
    ServerReasoningIncluded(bool),
    /// The invocation completed.
    Completed {
        response_id: String,
        token_usage: Option<TokenUsage>,
        usage_metadata: Option<ResponseUsageMetadata>,
        /// Whether the model affirmatively ended its turn; `None` when the
        /// provider does not report it.
        end_turn: Option<bool>,
    },
    /// Incremental assistant text.
    OutputTextDelta(String),
    /// Incremental tool call arguments.
    ToolCallInputDelta {
        item_id: String,
        call_id: Option<String>,
        delta: String,
    },
    /// Incremental reasoning summary text.
    ReasoningSummaryDelta { delta: String, summary_index: i64 },
    /// A reasoning summary part completed.
    ReasoningSummaryDone {
        item_id: String,
        text: String,
        summary_index: i64,
    },
    /// Incremental reasoning content text.
    ReasoningContentDelta { delta: String, content_index: i64 },
    /// A reasoning summary part was added.
    ReasoningSummaryPartAdded { summary_index: i64 },
    /// Rate limit snapshot observed during the invocation.
    RateLimits(RateLimitSnapshot),
    /// Model catalog version observed during the invocation.
    ModelsEtag(String),
}

#[cfg(test)]
#[path = "event_tests.rs"]
mod tests;
