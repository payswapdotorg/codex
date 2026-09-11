//! Unit tests for the resource binding surface.

use codex_execution_contracts::ExecutionEnvironment;
use pretty_assertions::assert_eq;

use crate::ResourceProviderError;
use crate::bind::bind;
use crate::bind::bind_as;
use crate::bind::canonical_resource_type;
use crate::bind::holder_environment;
use crate::model::ProviderId;
use crate::model::ResourceClass;
use crate::model::ResourceHandle;
use crate::model::ResourceId;
use crate::model::ResourceStatus;
use codex_workflow_contracts::ResourceTypeId;

fn handle(class: ResourceClass, status: ResourceStatus) -> ResourceHandle {
    ResourceHandle {
        resource: ResourceId::parse("sbx-0007").expect("resource id"),
        class,
        provider: ProviderId::parse("fixture-binding").expect("provider id"),
        status,
    }
}

#[test]
fn canonical_binding_surfaces_per_class() {
    let expected = [
        (
            ResourceClass::Sandbox,
            "sandbox_compute",
            ExecutionEnvironment::Terminal,
        ),
        (
            ResourceClass::RemoteDesktop,
            "remote_desktop_resource",
            ExecutionEnvironment::Computer,
        ),
        (
            ResourceClass::PersistentWorkspace,
            "persistent_workspace",
            ExecutionEnvironment::Terminal,
        ),
        (
            ResourceClass::GpuCompute,
            "gpu_compute",
            ExecutionEnvironment::Terminal,
        ),
    ];
    for (class, logical_type, holder) in expected {
        let live = handle(class, ResourceStatus::Active);
        let binding = bind(&live).expect("canonical bind");
        assert_eq!(
            binding.resource_type,
            ResourceTypeId::parse(logical_type).expect("logical type")
        );
        assert_eq!(binding.resource, live.resource);
        assert_eq!(binding.holder, holder_environment(class));
        assert_eq!(binding.holder, holder);
        binding.validate().expect("binding validates");
        assert_eq!(
            canonical_resource_type(class).expect("canonical type"),
            binding.resource_type
        );
    }
}

#[test]
fn bind_as_honors_logical_type_with_class_holder() {
    // The WO-015 remote-desktop adapter requires the
    // `remote_desktop_session` logical type; a provider-minted
    // remote-desktop resource binds under it while the holder stays the
    // class-derived Computer environment.
    let live = handle(ResourceClass::RemoteDesktop, ResourceStatus::Active);
    let binding = bind_as(
        &live,
        ResourceTypeId::parse("remote_desktop_session").expect("logical type"),
    )
    .expect("bind as");
    assert_eq!(binding.resource_type.as_ref(), "remote_desktop_session");
    assert_eq!(binding.holder, ExecutionEnvironment::Computer);
    assert_eq!(binding.resource, live.resource);
    binding.validate().expect("binding validates");

    // Malformed logical types are rejected by the WO-003 identifier
    // contract before they can reach the bind surface.
    assert!(ResourceTypeId::parse("Remote-Desktop").is_err());
    assert!(ResourceTypeId::parse("remoteDesktop").is_err());
}

#[test]
fn bind_rejects_non_live_handles() {
    for status in [
        ResourceStatus::Provisioning,
        ResourceStatus::Released,
        ResourceStatus::Lost,
    ] {
        let stale = handle(ResourceClass::Sandbox, status);
        let error = bind(&stale).expect_err("non-live handle cannot bind");
        assert!(
            matches!(error, ResourceProviderError::ResourceNotBindable { .. }),
            "status {status} must be rejected, got {error:?}"
        );
    }
    // Paused resources remain bindable.
    let paused = handle(ResourceClass::Sandbox, ResourceStatus::Paused);
    bind(&paused).expect("paused resources bind");
}
