use super::*;

#[test]
fn identifier_errors_carry_kind_value_and_reason() {
    let error = WorkflowContractError::InvalidIdentifier {
        kind: "workflow definition id",
        value: "!!".to_string(),
        reason: "must be non-empty",
    };

    assert_eq!(
        error.to_string(),
        "invalid workflow definition id `!!`: must be non-empty"
    );
}
