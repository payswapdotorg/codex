use pretty_assertions::assert_eq;

use super::*;
use codex_protocol::models::ContentItem;
use codex_protocol::models::ResponseItem;
use codex_protocol::protocol::ModelVerification;
use codex_protocol::protocol::TokenUsage;
use codex_protocol::protocol::TurnModerationMetadataEvent;

fn empty_rate_limits() -> codex_protocol::protocol::RateLimitSnapshot {
    codex_protocol::protocol::RateLimitSnapshot {
        limit_id: None,
        limit_name: None,
        normal_model_slug: None,
        primary: None,
        secondary: None,
        credits: None,
        individual_limit: None,
        spend_control_reached: None,
        plan_type: None,
        rate_limit_reached_type: None,
    }
}

fn assistant_message(text: &str) -> ResponseItem {
    ResponseItem::Message {
        id: None,
        role: "assistant".to_string(),
        content: vec![ContentItem::OutputText {
            text: text.to_string(),
        }],
        phase: None,
        internal_chat_message_metadata_passthrough: None,
    }
}

#[test]
fn completed_event_carries_usage_and_end_turn() {
    let usage = TokenUsage {
        input_tokens: 3,
        output_tokens: 2,
        ..TokenUsage::default()
    };
    let event = ModelEvent::Completed {
        response_id: "resp-9".to_string(),
        token_usage: Some(usage.clone()),
        usage_metadata: None,
        end_turn: Some(false),
    };

    let ModelEvent::Completed {
        response_id,
        token_usage,
        usage_metadata,
        end_turn,
    } = event
    else {
        panic!("event should be Completed");
    };

    assert_eq!(response_id, "resp-9");
    assert_eq!(token_usage, Some(usage));
    assert_eq!(usage_metadata, None);
    assert_eq!(end_turn, Some(false));
}

#[test]
fn lifecycle_events_aggregate_into_response() {
    // The documented lifecycle: Created -> deltas/items -> Completed.
    let events = vec![
        ModelEvent::Created {
            response_id: Some("resp-2".to_string()),
        },
        ModelEvent::OutputTextDelta("hi".to_string()),
        ModelEvent::OutputItemDone(assistant_message("hi")),
        ModelEvent::Completed {
            response_id: "resp-2".to_string(),
            token_usage: None,
            usage_metadata: None,
            end_turn: Some(true),
        },
    ];

    let response = crate::ModelResponse::from_events(events);

    assert_eq!(response.response_id, Some("resp-2".to_string()));
    assert_eq!(response.output, vec![assistant_message("hi")]);
    assert_eq!(response.end_turn, Some(true));
}

#[test]
fn diagnostic_events_carry_neutral_payloads() {
    let buffering = ModelSafetyBuffering {
        use_cases: vec!["cyber".to_string()],
        reasons: vec!["high risk".to_string()],
        retry_model: Some("gpt-5.6-sol".to_string()),
    };
    let verifications = vec![ModelVerification::TrustedAccessForCyber];

    let events = [
        ModelEvent::SafetyBuffering(buffering),
        ModelEvent::ModelVerifications(verifications),
        ModelEvent::TurnModerationMetadata(TurnModerationMetadataEvent {
            metadata: serde_json::Value::Null,
        }),
        ModelEvent::ServerReasoningIncluded(true),
        ModelEvent::RateLimits(empty_rate_limits()),
        ModelEvent::ModelsEtag("etag".to_string()),
        ModelEvent::ServerModel("other-model".to_string()),
        ModelEvent::ReasoningSummaryPartAdded { summary_index: 1 },
    ];

    // These events must remain constructible and comparable without any
    // provider protocol types; the compiler enforces the payloads.
    assert_eq!(events.len(), 8);
    assert_eq!(
        events[0],
        ModelEvent::SafetyBuffering(ModelSafetyBuffering {
            use_cases: vec!["cyber".to_string()],
            reasons: vec!["high risk".to_string()],
            retry_model: Some("gpt-5.6-sol".to_string()),
        })
    );
    assert_eq!(
        events[1],
        ModelEvent::ModelVerifications(vec![ModelVerification::TrustedAccessForCyber])
    );
}

#[test]
fn delta_events_carry_indices_and_payloads() {
    let events = vec![
        ModelEvent::ToolCallInputDelta {
            item_id: "item-1".to_string(),
            call_id: Some("call-1".to_string()),
            delta: "{\"a\"".to_string(),
        },
        ModelEvent::ReasoningSummaryDelta {
            delta: "thinking".to_string(),
            summary_index: 0,
        },
        ModelEvent::ReasoningSummaryDone {
            item_id: "item-2".to_string(),
            text: "done thinking".to_string(),
            summary_index: 0,
        },
        ModelEvent::ReasoningContentDelta {
            delta: "raw".to_string(),
            content_index: 3,
        },
    ];

    assert_eq!(
        events,
        vec![
            ModelEvent::ToolCallInputDelta {
                item_id: "item-1".to_string(),
                call_id: Some("call-1".to_string()),
                delta: "{\"a\"".to_string(),
            },
            ModelEvent::ReasoningSummaryDelta {
                delta: "thinking".to_string(),
                summary_index: 0,
            },
            ModelEvent::ReasoningSummaryDone {
                item_id: "item-2".to_string(),
                text: "done thinking".to_string(),
                summary_index: 0,
            },
            ModelEvent::ReasoningContentDelta {
                delta: "raw".to_string(),
                content_index: 3,
            },
        ]
    );
}
