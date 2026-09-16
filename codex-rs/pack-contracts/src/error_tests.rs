//! Tests for the Pack contract error surface.

use pretty_assertions::assert_eq;

use crate::PackContractError;

#[test]
fn serialization_errors_wrap_the_underlying_serde_failure() {
    let serde_error = serde_json::from_str::<u32>("not-a-number").expect_err("must fail");
    let error = PackContractError::from(serde_error);
    assert!(matches!(error, PackContractError::Serialization { .. }));
    let message = error.to_string();
    assert!(
        message.contains("pack contract serialization failure"),
        "unexpected message: {message}"
    );
}

#[test]
fn identifier_errors_render_kind_value_and_reason() {
    let error = PackContractError::InvalidIdentifier {
        kind: "pack id",
        value: "Bad Slug".to_owned(),
        reason: "must be 1-128 chars of [a-z0-9-]",
    };
    assert_eq!(
        error.to_string(),
        "invalid pack id `Bad Slug`: must be 1-128 chars of [a-z0-9-]"
    );
}
