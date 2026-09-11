//! Unit tests for the provider error mapping onto the contract failure
//! family.

use codex_execution_contracts::ExecutionContractError;
use codex_execution_contracts::FAILURE_MESSAGE_MAX_BYTES;
use codex_execution_contracts::FailureKind;
use pretty_assertions::assert_eq;

use crate::ResourceProviderError;
use crate::model::ProviderId;
use crate::model::ResourceId;
use crate::provider::LifecycleOp;

fn provider_id() -> ProviderId {
    ProviderId::parse("fixture-errors").expect("provider id")
}

#[test]
fn failure_kind_mapping_follows_the_recovery_model() {
    let provider = provider_id();
    let resource = ResourceId::parse("sbx-0000").expect("resource id");
    let lost = ResourceProviderError::ResourceLost {
        provider: provider.clone(),
        resource: resource.clone(),
    };
    let exhausted = ResourceProviderError::CapacityExhausted {
        provider: provider.clone(),
        detail: "no live capacity".to_owned(),
    };
    let unreachable = ResourceProviderError::Unreachable {
        provider: provider.clone(),
        detail: "outage".to_owned(),
    };
    assert_eq!(lost.failure_kind(), FailureKind::Unavailable);
    assert_eq!(exhausted.failure_kind(), FailureKind::Unavailable);
    assert_eq!(unreachable.failure_kind(), FailureKind::Unavailable);

    let unsupported = ResourceProviderError::Unsupported {
        provider: provider.clone(),
        operation: LifecycleOp::Fork,
    };
    let invalid = ResourceProviderError::InvalidSpec {
        reason: "labels out of bounds".to_owned(),
    };
    let not_bindable = ResourceProviderError::ResourceNotBindable {
        provider,
        resource,
        status: crate::model::ResourceStatus::Provisioning,
    };
    assert_eq!(unsupported.failure_kind(), FailureKind::Permanent);
    assert_eq!(invalid.failure_kind(), FailureKind::Permanent);
    assert_eq!(not_bindable.failure_kind(), FailureKind::Permanent);

    let contract = ResourceProviderError::from(ExecutionContractError::InvalidRecord {
        reason: "bad record".to_owned(),
    });
    assert_eq!(contract.failure_kind(), FailureKind::Permanent);
}

#[test]
fn rebind_advisable_marks_the_unavailable_family() {
    let provider = provider_id();
    let resource = ResourceId::parse("sbx-0000").expect("resource id");
    let lost = ResourceProviderError::ResourceLost { provider, resource };
    assert!(lost.rebind_advisable());

    let unsupported = ResourceProviderError::Unsupported {
        provider: provider_id(),
        operation: LifecycleOp::Resize(crate::provider::ResizeRequest::default()),
    };
    assert!(!unsupported.rebind_advisable());
}

#[test]
fn normalized_failure_bounds_messages() {
    let invalid = ResourceProviderError::InvalidSpec {
        reason: "labels out of bounds".to_owned(),
    };
    let failure = invalid.normalized_failure();
    assert_eq!(failure.kind, FailureKind::Permanent);
    assert_eq!(
        failure.message,
        "invalid resource record: labels out of bounds"
    );

    let verbose = ResourceProviderError::InvalidSpec {
        reason: "x".repeat(FAILURE_MESSAGE_MAX_BYTES + 128),
    };
    let clamped = verbose.normalized_failure();
    assert!(clamped.message.len() <= FAILURE_MESSAGE_MAX_BYTES);
    assert!(clamped.message.ends_with("...[truncated]"));
    assert!(!clamped.message.is_empty());
}

#[test]
fn unsupported_error_names_provider_and_operation() {
    let provider = provider_id();
    let unsupported = ResourceProviderError::Unsupported {
        provider: provider.clone(),
        operation: LifecycleOp::Fork,
    };
    let rendered = unsupported.to_string();
    assert!(rendered.contains("fixture-errors"));
    assert!(rendered.contains("fork"));

    let lost = ResourceProviderError::ResourceLost {
        provider,
        resource: ResourceId::parse("sbx-0001").expect("resource id"),
    };
    let rendered = lost.to_string();
    assert!(rendered.contains("fixture-errors"));
    assert!(rendered.contains("sbx-0001"));
}
