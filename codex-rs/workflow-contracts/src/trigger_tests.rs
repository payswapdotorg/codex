use super::*;
use pretty_assertions::assert_eq;

#[test]
fn trigger_classes_match_the_frozen_architecture_list() {
    let serialized: Vec<String> = TriggerClass::ALL
        .iter()
        .map(|class| serde_json::to_string(class).expect("trigger class serializes"))
        .collect();

    assert_eq!(
        serialized,
        vec![
            "\"user\"",
            "\"schedule\"",
            "\"webhook\"",
            "\"connectorEvent\"",
            "\"browserEvent\"",
            "\"computerEvent\"",
            "\"workflowEvent\"",
            "\"humanEvent\"",
        ]
    );
}

#[test]
fn trigger_declarations_round_trip_through_serde() {
    let trigger = WorkflowTrigger {
        trigger: TriggerClass::ConnectorEvent,
        description: Some("GitHub pull request merged".to_owned()),
    };

    let serialized = serde_json::to_string(&trigger).expect("trigger serializes");
    let parsed: WorkflowTrigger = serde_json::from_str(&serialized).expect("trigger parses");

    assert_eq!(trigger, parsed);
    assert_eq!(
        serde_json::to_value(&trigger).expect("trigger serializes"),
        serde_json::json!({
            "trigger": "connectorEvent",
            "description": "GitHub pull request merged",
        })
    );
}

#[test]
fn trigger_sources_record_only_normalized_event_ids() {
    let source = TriggerSource {
        trigger: TriggerClass::Webhook,
        event_id: Some("accepted-7f3c".to_owned()),
    };

    let serialized = serde_json::to_value(&source).expect("source serializes");
    assert_eq!(
        serialized,
        serde_json::json!({ "trigger": "webhook", "eventId": "accepted-7f3c" })
    );
}
