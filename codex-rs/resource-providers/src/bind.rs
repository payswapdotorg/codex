//! The resource binding surface (WO-016).
//!
//! This module is the *only* place provider-minted resources become
//! workflow-visible: it mints plain WO-005
//! `codex_execution_contracts::ResourceBinding` records — a logical
//! snake_case [`ResourceTypeId`] from `codex-workflow-contracts`, the
//! opaque [`ResourceId`] the provider minted, and the peer
//! [`ExecutionEnvironment`] holder the resource class maps onto — so a
//! workflow's declared resource requirement binds at install/instantiate
//! time through the **existing** registry path
//! (`CapabilityRegistry::bind_resource`, fed from `workflow-triggers`
//! `InstallRequest::resources` / `workflow-app`
//! `InstantiateRequest::resources`).
//!
//! Class-to-surface mapping follows the frozen environment peers:
//! sandbox/compute-class resources bind under the `Terminal` peer
//! environment, and remote-desktop-class resources bind under the
//! `Computer` peer environment — exactly like the WO-015
//! `remote_desktop_session` resource — so the Computer Use semantic
//! contract stays the reach for desktop-class resources. Provider names
//! never enter the binding, the resource type, or any workflow contract:
//! the binding carries only the opaque instance id.
//!
//! Rebinding is minting again: [`bind`] on a replacement handle produces
//! a **new** binding record with a new opaque resource id under the same
//! logical type, while the workflow's semantic identity (version, source
//! revision, dependency pins) stays untouched.

use codex_execution_contracts::ExecutionEnvironment;
use codex_execution_contracts::ResourceBinding;
use codex_workflow_contracts::ResourceTypeId;

use crate::ResourceProviderError;
use crate::model::ResourceClass;
use crate::model::ResourceHandle;

/// The canonical logical resource type for sandbox-class compute
/// resources.
pub const SANDBOX_COMPUTE_RESOURCE_TYPE: &str = "sandbox_compute";
/// The canonical logical resource type for remote-desktop-class
/// resources.
pub const REMOTE_DESKTOP_RESOURCE_TYPE: &str = "remote_desktop_resource";
/// The canonical logical resource type for persistent-workspace
/// resources.
pub const PERSISTENT_WORKSPACE_RESOURCE_TYPE: &str = "persistent_workspace";
/// The canonical logical resource type for GPU compute resources.
pub const GPU_COMPUTE_RESOURCE_TYPE: &str = "gpu_compute";

/// The canonical logical resource type for one resource class.
///
/// Logical types are lowercase snake_case workflow-facing names; they
/// never name a provider, region, or vendor.
pub fn canonical_resource_type(
    class: ResourceClass,
) -> Result<ResourceTypeId, ResourceProviderError> {
    Ok(ResourceTypeId::parse(class.logical_resource_type())?)
}

/// The peer execution environment a resource class binds under.
///
/// The mapping is provider-independent and class-only: providers never
/// choose the holder environment, and both holders are existing peer
/// environments, so the reserved `MOBILE`/`REMOTE_DESKTOP` variants stay
/// untouched.
pub const fn holder_environment(class: ResourceClass) -> ExecutionEnvironment {
    match class {
        ResourceClass::Sandbox | ResourceClass::PersistentWorkspace | ResourceClass::GpuCompute => {
            ExecutionEnvironment::Terminal
        }
        ResourceClass::RemoteDesktop => ExecutionEnvironment::Computer,
    }
}

/// Mints the canonical binding for a live provider handle.
///
/// The binding names the class's canonical logical resource type, the
/// opaque instance id the provider minted, and the class's peer holder
/// environment. Only live resources mint bindings: a provisioning,
/// released, or lost resource is an explicit error, never a silently
/// stale binding.
pub fn bind(handle: &ResourceHandle) -> Result<ResourceBinding, ResourceProviderError> {
    bind_as(handle, canonical_resource_type(handle.class)?)
}

/// Mints a binding for a live provider handle under an explicit logical
/// resource type.
///
/// Hosts use this when the workflow or serving adapter declared a
/// different logical type for the same class (for example binding a
/// provider-minted remote-desktop resource under the WO-015
/// `remote_desktop_session` type the remote-desktop adapter requires).
/// The holder environment is still derived from the class alone.
pub fn bind_as(
    handle: &ResourceHandle,
    resource_type: ResourceTypeId,
) -> Result<ResourceBinding, ResourceProviderError> {
    if !handle.status.is_bindable() {
        return Err(ResourceProviderError::ResourceNotBindable {
            provider: handle.provider.clone(),
            resource: handle.resource.clone(),
            status: handle.status,
        });
    }
    let binding = ResourceBinding {
        resource_type,
        resource: handle.resource.clone(),
        holder: holder_environment(handle.class),
    };
    binding.validate()?;
    Ok(binding)
}

#[cfg(test)]
#[path = "bind_tests.rs"]
mod tests;
