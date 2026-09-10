//! Trigger event key validation.

use codex_workflow_contracts::TriggerClass;
use pretty_assertions::assert_eq;

use crate::IncomingTrigger;
use crate::TriggerEventKey;
use crate::WorkflowTriggerError;

fn definition_id(value: &str) -> codex_workflow_contracts::WorkflowDefinitionId {
    codex_workflow_contracts::WorkflowDefinitionId::parse(value).expect("definition id")
}

#[test]
fn keys_accept_route_safe_tokens_and_reject_everything_else() {
    assert!(TriggerEventKey::parse("user-1").is_ok());
    assert!(TriggerEventKey::parse("webhook:ops-hook:42").is_ok());
    assert!(TriggerEventKey::parse("schedule:triage-report:sched-1:1000").is_ok());
    assert!(TriggerEventKey::parse("a.b_c-d").is_ok());
    for invalid in [
        "",
        " spaced",
        "with space",
        "with/slash",
        "with#hash",
        &"x".repeat(129),
    ] {
        assert!(
            matches!(
                TriggerEventKey::parse(invalid),
                Err(WorkflowTriggerError::InvalidEventKey { .. })
            ),
            "`{invalid}` must be rejected"
        );
    }
    // Round-trips through its string form.
    let key = TriggerEventKey::parse("user-1").expect("valid key");
    assert_eq!(String::from(key.clone()), "user-1");
    assert_eq!(key.as_ref(), "user-1");
}

#[test]
fn external_events_carry_routing_without_a_target() {
    let event = IncomingTrigger::external(
        TriggerClass::Webhook,
        "webhook-42",
        1_000,
        "webhook:ops-hook",
        Some("ops-hook".to_string()),
        Some("ab".repeat(32)),
    )
    .expect("external event");
    assert_eq!(event.trigger, TriggerClass::Webhook);
    assert_eq!(event.key.as_ref(), "webhook-42");
    assert_eq!(event.occurred_at_unix_ms, 1_000);
    assert_eq!(event.source, "webhook:ops-hook");
    assert_eq!(event.routing.as_deref(), Some("ops-hook"));
    assert_eq!(event.target, None);
    assert_eq!(
        event.payload_digest.as_deref(),
        Some("ab".repeat(32).as_str())
    );
}

#[test]
fn schedule_envelopes_use_deterministic_keys() {
    let event = IncomingTrigger::scheduled(&definition_id("triage-report"), "sched-1", 5_000)
        .expect("scheduled event");
    assert_eq!(event.trigger, TriggerClass::Schedule);
    assert_eq!(event.key.as_ref(), "schedule:sched-1:5000");
    assert_eq!(event.source, "schedule:sched-1");
    assert_eq!(event.occurred_at_unix_ms, 5_000);
    assert_eq!(event.target, Some(definition_id("triage-report")));
    // The same occurrence derives the same key: double-polling dedupes.
    let again = IncomingTrigger::scheduled(&definition_id("triage-report"), "sched-1", 5_000)
        .expect("scheduled event");
    assert_eq!(event.key, again.key);
}
