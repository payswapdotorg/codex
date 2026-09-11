//! Unit tests for the provider lifecycle vocabulary.

use pretty_assertions::assert_eq;

use crate::model::ProviderId;
use crate::model::ResourceClass;
use crate::model::ResourceId;
use crate::model::ResourceSnapshot;
use crate::model::ResourceHandle;
use crate::model::ResourceStatus;
use crate::provider::LifecycleOp;
use crate::provider::LifecycleOutcome;
use crate::provider::ResizeRequest;
use crate::ResourceProviderError;

#[test]
fn lifecycle_op_names_and_resize_validation() {
    assert_eq!(LifecycleOp::Create.name(), "create");
    assert_eq!(LifecycleOp::Pause.name(), "pause");
    assert_eq!(LifecycleOp::Resume.name(), "resume");
    assert_eq!(LifecycleOp::Snapshot.name(), "snapshot");
    assert_eq!(LifecycleOp::Fork.name(), "fork");
    assert_eq!(
        LifecycleOp::Resize(ResizeRequest::default()).name(),
        "resize"
    );

    let empty = ResizeRequest::default();
    let error = empty.validate().expect_err("empty resize");
    assert!(matches!(error, ResourceProviderError::InvalidSpec { .. }));

    let bounded = ResizeRequest {
        cpu_millis: None,
        memory_mib: Some(512),
        disk_mib: None,
    };
    bounded.validate().expect("valid resize");

    let over = ResizeRequest {
        cpu_millis: Some(0),
        memory_mib: None,
        disk_mib: None,
    };
    assert!(over.validate().is_err());

    let handle = ResourceHandle {
        resource: ResourceId::parse("sbx-0001").expect("resource id"),
        class: ResourceClass::Sandbox,
        provider: ProviderId::parse("fixture-op").expect("provider id"),
        status: ResourceStatus::Active,
    };
    let transitioned = LifecycleOutcome::Transitioned(handle);
    assert!(transitioned.handle().is_some());
    let snapshotted = LifecycleOutcome::Snapshotted(ResourceSnapshot {
        resource: ResourceId::parse("sbx-0001").expect("resource id"),
        artifact: "snap-0001".to_owned(),
    });
    assert!(snapshotted.handle().is_none());
}
