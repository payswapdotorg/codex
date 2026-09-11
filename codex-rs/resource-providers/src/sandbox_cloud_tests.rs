//! Unit tests for the E2B-shaped sandbox-cloud fixture.

use pretty_assertions::assert_eq;

use crate::ExecutionResourceProvider;
use crate::ResourceProviderError;
use crate::SandboxCloudFixture;
use crate::conformance::assert_provider_conformance;
use crate::model::ResourceClass;
use crate::model::ResourceSpec;
use crate::model::ResourceStatus;
use crate::provider::LifecycleOp;

fn spec(class: ResourceClass) -> ResourceSpec {
    ResourceSpec {
        class,
        cpu_millis: None,
        memory_mib: None,
        disk_mib: None,
        gpu: None,
        os: None,
        region: None,
        labels: std::collections::BTreeMap::new(),
        network: None,
    }
}

#[test]
fn sandbox_cloud_fixture_passes_the_shared_conformance_suite() {
    let fixture = SandboxCloudFixture::new(4).expect("fixture");
    assert_provider_conformance(&fixture);
}

#[test]
fn sandbox_cloud_failure_modes_are_cloud_shaped() {
    let fixture = SandboxCloudFixture::new(1).expect("fixture");

    // Capacity exhaustion: the second live sandbox is rejected and maps
    // onto the contract Unavailable family (rebind-advisable).
    let first = fixture.create(spec(ResourceClass::Sandbox)).expect("first");
    let error = fixture
        .create(spec(ResourceClass::Sandbox))
        .expect_err("capacity");
    assert!(matches!(
        error,
        ResourceProviderError::CapacityExhausted { .. }
    ));
    assert_eq!(
        error.failure_kind(),
        codex_execution_contracts::FailureKind::Unavailable
    );
    assert!(error.rebind_advisable());

    // Release frees the slot: ephemeral sandboxes do not hold capacity.
    let released = fixture.release(&first).expect("release");
    assert_eq!(released.status, ResourceStatus::Released);
    let second = fixture
        .create(spec(ResourceClass::Sandbox))
        .expect("second");
    assert_eq!(fixture.live_count(), 1);

    // Outage: probes and creates fail with an explicit Unreachable
    // error, the cloud-shaped failure mode.
    fixture.set_outage(true);
    let probe = fixture.probe().expect_err("probe during outage");
    assert!(matches!(probe, ResourceProviderError::Unreachable { .. }));
    assert!(matches!(
        fixture.create(spec(ResourceClass::RemoteDesktop)),
        Err(ResourceProviderError::Unreachable { .. })
    ));
    fixture.set_outage(false);
    assert!(fixture.probe().is_ok());

    // Resource loss is injectable and surfaces as ResourceLost for
    // lifecycle ops, with the failure classified Unavailable.
    fixture.lose_resource(&second.resource);
    assert_eq!(
        fixture.status(&second).expect("status"),
        ResourceStatus::Lost
    );
    let error = fixture
        .lifecycle(&second, LifecycleOp::Pause)
        .expect_err("pause on a lost resource");
    assert!(matches!(error, ResourceProviderError::ResourceLost { .. }));
    assert_eq!(
        error.failure_kind(),
        codex_execution_contracts::FailureKind::Unavailable
    );

    // Snapshot->fork chain: the E2B-shaped capability pair works
    // end-to-end (snapshot artifact, fork mints a new id).
    let live = fixture.create(spec(ResourceClass::Sandbox)).expect("third");
    assert!(matches!(
        fixture
            .lifecycle(&live, LifecycleOp::Snapshot)
            .expect("snapshot"),
        crate::provider::LifecycleOutcome::Snapshotted(_)
    ));
    let forked = fixture.lifecycle(&live, LifecycleOp::Fork).expect("fork");
    let fork_handle = forked.handle().expect("transition").clone();
    assert_ne!(fork_handle.resource, live.resource);

    // Evidence carries the capacity and loss facts under provider-scoped
    // locators.
    let records = fixture.evidence_records();
    assert!(!records.is_empty());
    assert!(records.iter().any(|record| {
        record
            .locator
            .starts_with("codex-resource-providers/fixture-sandbox-cloud/")
    }));
}
