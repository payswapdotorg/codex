use pretty_assertions::assert_eq;
use std::time::Duration;

use super::*;
use crate::capabilities::ModelCapabilities;
use crate::error::ModelError;
use crate::event::ModelEvent;
use crate::request::ModelRequest;
use futures::StreamExt;
use tokio::sync::mpsc;

fn completed_stream() -> (mpsc::Sender<ModelStreamItem>, ModelStream) {
    ModelStream::channel(8, Some("req-1".to_string()))
}

#[test]
fn transport_policy_defaults_match_provider_defaults() {
    let policy = ModelTransportPolicy::default();

    assert_eq!(policy.request_max_retries, 4);
    assert_eq!(policy.stream_max_retries, 5);
    assert_eq!(policy.stream_idle_timeout, Duration::from_millis(300_000));
    assert_eq!(
        policy.websocket_connect_timeout,
        Some(Duration::from_millis(15_000))
    );
    assert!(!policy.supports_websockets);
}

#[test]
fn auth_status_never_carries_credentials() {
    // The type system only admits status plus an account label; the
    // following constructions are the complete API surface.
    let statuses = [
        ModelAuthStatus::NotRequired,
        ModelAuthStatus::Authenticated {
            account: Some("user@example.com".to_string()),
        },
        ModelAuthStatus::Authenticated { account: None },
        ModelAuthStatus::RequiresAuthentication {
            instructions: Some("run codex login".to_string()),
        },
    ];

    assert_eq!(statuses[0], ModelAuthStatus::default());
    assert!(matches!(
        statuses[1],
        ModelAuthStatus::Authenticated { account: Some(_) }
    ));
}

#[tokio::test]
async fn stream_yields_events_in_order() {
    let (tx, mut stream) = completed_stream();
    let request_id = stream.request_id.clone();
    let events = vec![
        ModelEvent::Created {
            response_id: Some("resp-1".to_string()),
        },
        ModelEvent::OutputTextDelta("hi".to_string()),
        ModelEvent::Completed {
            response_id: "resp-1".to_string(),
            token_usage: None,
            usage_metadata: None,
            end_turn: Some(true),
        },
    ];
    for event in events {
        tx.send(Ok(event)).await.expect("receiver should be alive");
    }
    drop(tx);

    let received: Vec<ModelStreamItem> = stream.by_ref().collect().await;
    assert_eq!(received.len(), 3);
    assert_eq!(
        received[0],
        Ok(ModelEvent::Created {
            response_id: Some("resp-1".to_string())
        })
    );
    assert_eq!(request_id.as_deref(), Some("req-1"));
}

#[tokio::test]
async fn dropping_the_stream_cancels_the_producer() {
    // A producer that streams until the consumer goes away. Dropping the
    // ModelStream must be observable as a channel closure so adapters can
    // cancel provider work promptly.
    let (tx, stream) = ModelStream::channel(1, None);
    let producer = tokio::spawn(async move {
        let mut sent = 0u32;
        loop {
            let event = ModelEvent::OutputTextDelta("chunk".to_string());
            if tx.send(Ok(event)).await.is_err() {
                break;
            }
            sent += 1;
        }
        sent
    });

    let mut stream = stream;
    // Consume one event, then drop the stream mid-flight.
    let _ = stream.next().await.expect("first event should arrive");
    drop(stream);

    let sent = producer
        .await
        .expect("producer should observe cancellation");
    // The producer sent the consumed event and at most the buffered one
    // before observing the closed receiver.
    assert!(sent <= 2, "producer should stop promptly, sent {sent}");
}

#[tokio::test]
async fn stream_surfaces_normalized_errors() {
    let (tx, stream) = ModelStream::channel(8, None);
    tx.send(Err(ModelError::StreamIncomplete {
        message: "closed before completion".to_string(),
    }))
    .await
    .expect("receiver should be alive");
    drop(tx);

    let items: Vec<ModelStreamItem> = stream.collect().await;
    assert_eq!(
        items,
        vec![Err(ModelError::StreamIncomplete {
            message: "closed before completion".to_string()
        })]
    );
}

#[tokio::test]
async fn request_payload_is_visible_to_sessions() {
    // Sessions receive the full neutral request; nothing in the type forces
    // provider protocol knowledge.
    let request = ModelRequest {
        instructions: "be brief".to_string(),
        ..ModelRequest::default()
    };
    let observed_instructions = request.instructions.clone();
    drop(request);

    assert_eq!(observed_instructions, "be brief");
    let _capabilities = ModelCapabilities::default();
    let _error = ModelError::Canceled {
        reason: "test".to_string(),
    };
}
