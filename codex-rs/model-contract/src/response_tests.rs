use pretty_assertions::assert_eq;

use super::*;
use crate::event::ModelEvent;
use crate::event::ModelSafetyBuffering;
use codex_protocol::models::ContentItem;
use codex_protocol::models::ResponseItem;
use codex_protocol::protocol::TokenUsage;

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
fn from_events_aggregates_completed_response() {
    let usage = TokenUsage {
        input_tokens: 10,
        output_tokens: 5,
        ..TokenUsage::default()
    };
    let events = vec![
        ModelEvent::Created {
            response_id: Some("resp-1".to_string()),
        },
        ModelEvent::OutputItemAdded(assistant_message("partial")),
        ModelEvent::OutputItemDone(assistant_message("hello")),
        ModelEvent::OutputTextDelta("hello".to_string()),
        ModelEvent::ServerModel("gpt-5.6-sol".to_string()),
        ModelEvent::Completed {
            response_id: "resp-1".to_string(),
            token_usage: Some(usage.clone()),
            usage_metadata: None,
            end_turn: Some(true),
        },
    ];

    let response = ModelResponse::from_events(events);

    assert_eq!(
        response,
        ModelResponse {
            response_id: Some("resp-1".to_string()),
            output: vec![assistant_message("hello")],
            usage: Some(usage),
            usage_metadata: None,
            end_turn: Some(true),
            server_model: Some("gpt-5.6-sol".to_string()),
        }
    );
}

#[test]
fn from_events_ignores_partial_and_diagnostic_events() {
    let events = vec![
        ModelEvent::OutputItemAdded(assistant_message("in flight")),
        ModelEvent::SafetyBuffering(ModelSafetyBuffering {
            use_cases: vec!["cyber".to_string()],
            reasons: vec!["risk".to_string()],
            retry_model: Some("gpt-5.6-sol".to_string()),
        }),
        ModelEvent::RateLimits(empty_rate_limits()),
        ModelEvent::ModelsEtag("etag-1".to_string()),
    ];

    assert_eq!(ModelResponse::from_events(events), ModelResponse::default());
}

#[test]
fn from_events_defaults_when_no_completed_event() {
    let response = ModelResponse::from_events([ModelEvent::OutputTextDelta("orphan".to_string())]);

    assert_eq!(response, ModelResponse::default());
}
