//! Unit tests for the local/container-shaped fixture.

use pretty_assertions::assert_eq;

use crate::ExecutionResourceProvider;
use crate::LocalContainersFixture;
use crate::ResourceProviderError;
use crate::conformance::assert_provider_conformance;
use crate::local_containers::LOCAL_CONTAINERS_PROVIDER_ID;
use crate::model::ResourceClass;
use crate::model::ResourceSpec;
use crate::model::ResourceStatus;
use crate::provider::LifecycleOp;
use crate::provider::ResizeRequest;

fn workspace_spec() -> ResourceSpec {
    ResourceSpec {
        class: ResourceClass::PersistentWorkspace,
        cpu_millis: Some(500),
        memory_mib: Some(1_024),
        disk_mib: Some(4_096),
        gpu: None,
        os: Some("linux".to_owned()),
        region: None,
        labels: std::collections::BTreeMap::new(),
        network: None,
    }
}

#[test]
fn local_containers_fixture_passes_the_shared_conformance_suite() {
    let fixture = LocalContainersFixture::new().expect("fixture");
    assert_provider_conformance(&fixture);
}

#[test]
fn local_failure_modes_and_persistence_are_container_shaped() {
    let fixture = LocalContainersFixture::new().expect("fixture");

    // Material differences from the sandbox cloud: local-scale latency,
    // one region, no cost metadata, persistent class only.
    let descriptor = fixture.descriptor();
    assert_eq!(descriptor.provider.as_ref(), LOCAL_CONTAINERS_PROVIDER_ID);
    assert_eq!(descriptor.health.latency_ms, Some(0));
    assert_eq!(descriptor.regions, vec!["local".to_owned()]);
    assert!(descriptor.cost.is_none());
    assert_eq!(descriptor.capabilities.len(), 1);
    assert!(!descriptor.capabilities[0].lifecycle.fork);
    assert!(!descriptor.capabilities[0].lifecycle.pause_resume);

    // Persistence: the workspace survives release with its spec
    // queryable, and undeclared lifecycle ops stay explicitly
    // unsupported.
    let workspace = fixture.create(workspace_spec()).expect("create");
    let released = fixture.release(&workspace).expect("release");
    assert_eq!(released.status, ResourceStatus::Released);
    let spec = fixture
        .workspace_spec(&workspace.resource)
        .expect("spec kept");
    assert_eq!(spec.cpu_millis, Some(500));
    let unsupported = fixture
        .lifecycle(&workspace, LifecycleOp::Fork)
        .expect_err("fork is undeclared");
    assert!(matches!(
        unsupported,
        ResourceProviderError::Unsupported { .. }
    ));
    assert_eq!(
        unsupported.failure_kind(),
        codex_execution_contracts::FailureKind::Permanent
    );
    assert!(!unsupported.rebind_advisable());

    // Container-shaped loss: the container disappears underneath the
    // workspace; the record stays with status Lost.
    let live = fixture.create(workspace_spec()).expect("second workspace");
    fixture.remove_container(&live.resource);
    assert_eq!(fixture.status(&live).expect("status"), ResourceStatus::Lost);
    let lost = fixture
        .lifecycle(
            &live,
            LifecycleOp::Resize(ResizeRequest {
                cpu_millis: Some(1_000),
                memory_mib: None,
                disk_mib: None,
            }),
        )
        .expect_err("resize on a removed container");
    assert!(matches!(lost, ResourceProviderError::ResourceLost { .. }));

    // Resize applies within the declared quota and is observable in the
    // recorded spec.
    let resized_workspace = fixture.create(workspace_spec()).expect("third workspace");
    let outcome = fixture
        .lifecycle(
            &resized_workspace,
            LifecycleOp::Resize(ResizeRequest {
                cpu_millis: Some(2_000),
                memory_mib: Some(2_048),
                disk_mib: None,
            }),
        )
        .expect("resize");
    let handle = outcome.handle().expect("transition");
    assert_eq!(handle.status, ResourceStatus::Active);
    assert_eq!(handle.resource, resized_workspace.resource);
    let spec = fixture
        .workspace_spec(&resized_workspace.resource)
        .expect("spec kept");
    assert_eq!(spec.cpu_millis, Some(2_000));
    assert_eq!(spec.memory_mib, Some(2_048));

    // Resize over the declared quota is an explicit invalid request.
    let over = fixture
        .lifecycle(
            &resized_workspace,
            LifecycleOp::Resize(ResizeRequest {
                cpu_millis: Some(9_999),
                memory_mib: None,
                disk_mib: None,
            }),
        )
        .expect_err("over-quota resize");
    assert!(matches!(over, ResourceProviderError::InvalidSpec { .. }));

    // Snapshotted outcomes never occur for this fixture: snapshots are
    // undeclared, and every recorded transition is a plain handle.
    let records = fixture.evidence_records();
    assert!(!records.is_empty());
    assert!(
        records
            .iter()
            .all(|record| !record.locator.contains("snapshot"))
    );
}
