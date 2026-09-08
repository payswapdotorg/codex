use super::*;

#[test]
fn serde_json_errors_convert_to_serialization_failures() {
    let error = ExecutionContractError::from(
        serde_json::from_str::<serde_json::Value>("{").expect_err("malformed json must fail"),
    );
    assert!(matches!(
        error,
        ExecutionContractError::Serialization { .. }
    ));
}

#[test]
fn workflow_contract_errors_convert_for_digest_integration() {
    let workflow_error = codex_workflow_contracts::WorkflowContractError::InvalidIdentifier {
        kind: "capability id",
        value: "Bad Capability".to_owned(),
        reason: "must be non-empty lowercase snake_case",
    };
    let error = ExecutionContractError::from(workflow_error);
    assert!(matches!(
        error,
        ExecutionContractError::WorkflowContract { .. }
    ));
}
