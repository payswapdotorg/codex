use super::*;
use crate::ExecutionContractError;
use pretty_assertions::assert_eq;

#[test]
fn operations_use_snake_case_tokens() {
    for value in [
        "navigate",
        "click",
        "run_command",
        "invoke_tool",
        "read_screen",
    ] {
        assert!(OperationId::parse(value).is_ok(), "`{value}` must parse");
    }
    for value in ["", "Navigate", "click-element", "_lead", "trail_", "sp ace"] {
        assert!(
            OperationId::parse(value).is_err(),
            "`{value}` must be rejected"
        );
    }
}

#[test]
fn targets_are_bounded_opaque_selectors() {
    assert!(ActionTarget::parse("https://example.com").is_ok());
    assert!(ActionTarget::parse("#submit-button").is_ok());
    assert!(ActionTarget::parse("").is_err());
    assert!(ActionTarget::parse("line\nbreak").is_err());
    let oversized = "x".repeat(ACTION_TARGET_MAX_BYTES + 1);
    assert!(ActionTarget::parse(oversized).is_err());
}

#[test]
fn actions_validate_operation_target_and_inputs() {
    let mut action = Action::new(OperationId::parse("navigate").expect("valid operation"));
    action.target = Some(ActionTarget::parse("https://example.com").expect("valid target"));
    action
        .inputs
        .insert("url".to_owned(), serde_json::json!("https://example.com"));
    action.validate().expect("bounded action is valid");
}

#[test]
fn actions_reject_oversized_inputs() {
    let mut action = Action::new(OperationId::parse("type_text").expect("valid operation"));
    action.inputs.insert(
        "text".to_owned(),
        serde_json::Value::String("x".repeat(70_000)),
    );
    let error = action
        .validate()
        .expect_err("oversized inputs must be rejected");
    assert!(matches!(
        error,
        ExecutionContractError::PayloadTooLarge {
            field: "action inputs",
            ..
        }
    ));
}

#[test]
fn actions_reject_deeply_nested_inputs() {
    let mut nested = serde_json::Value::Null;
    for _ in 0..(ACTION_INPUTS_MAX_DEPTH + 1) {
        nested = serde_json::json!({ "child": nested });
    }
    let mut action = Action::new(OperationId::parse("configure").expect("valid operation"));
    action.inputs.insert("tree".to_owned(), nested);
    let error = action
        .validate()
        .expect_err("deeply nested inputs must be rejected");
    assert!(matches!(
        error,
        ExecutionContractError::PayloadTooDeep { .. }
    ));
}

#[test]
fn action_inputs_serialize_transparently_in_sorted_order() {
    let mut action = Action::new(OperationId::parse("invoke").expect("valid operation"));
    action
        .inputs
        .insert("zebra".to_owned(), serde_json::json!({ "last": true }));
    action
        .inputs
        .insert("alpha".to_owned(), serde_json::json!(1));
    let serialized = serde_json::to_string(&action).expect("serializable");
    let alpha = serialized.find("\"alpha\"").expect("alpha key present");
    let zebra = serialized.find("\"zebra\"").expect("zebra key present");
    assert!(
        alpha < zebra,
        "inputs must serialize in canonical key order"
    );
    let round_tripped: Action = serde_json::from_str(&serialized).expect("deserializable");
    assert_eq!(round_tripped, action);
}

#[test]
fn empty_actions_have_no_target_and_no_inputs() {
    let action = Action::new(OperationId::parse("observe").expect("valid operation"));
    assert!(action.target.is_none());
    assert!(action.inputs.is_empty());
    assert_eq!(action.inputs.len(), 0);
    action.validate().expect("minimal action is valid");
}
