//! The local/container-shaped conformance fixture (WO-016).
//!
//! [`LocalContainersFixture`] is an in-process fake shaped like
//! **self-hosted local/container infrastructure**: it serves
//! [`ResourceClass::PersistentWorkspace`] only, supports create and
//! resize, has *no* fork, *no* snapshots, *no* pause/resume, and no
//! network-policy support (specs carrying one are rejected). It behaves
//! materially differently from the sandbox-cloud fixture: workspaces
//! persist across release, probe latency is local-scale (zero), there is
//! no provider cost metadata, there is no live-capacity ceiling, and its
//! failure modes are container-shaped (the backing container
//! disappearing underneath a workspace, and resize requests exceeding
//! the declared quota) rather than cloud-shaped (outages and capacity
//! exhaustion). No container runtime is linked: this fixture exists so
//! the provider-neutral contract and the shared conformance harness
//! prove the port is not shaped around any single vendor.
//!
//! The fixture is a **test double**, not a runtime: nothing is started,
//! and no credential material exists anywhere in it.

use std::collections::BTreeMap;
use std::sync::Mutex;
use std::sync::MutexGuard;
use std::sync::PoisonError;

use codex_workflow_contracts::EvidenceKind;

use crate::ExecutionResourceProvider;
use crate::ResourceProviderError;
use crate::descriptor::ProviderDescriptor;
use crate::descriptor::ProviderHealth;
use crate::descriptor::ProviderHealthStatus;
use crate::descriptor::ResourceCapability;
use crate::descriptor::ResourceLifecycleSupport;
use crate::descriptor::ResourceQuota;
use crate::evidence::ResourceEvidenceJournal;
use crate::evidence::ResourceEvidenceRecord;
use crate::model::ProviderId;
use crate::model::ResourceClass;
use crate::model::ResourceHandle;
use crate::model::ResourceId;
use crate::model::ResourceSpec;
use crate::model::ResourceStatus;
use crate::provider::LifecycleOp;
use crate::provider::LifecycleOutcome;

/// The fixture provider's identity.
pub const LOCAL_CONTAINERS_PROVIDER_ID: &str = "fixture-local-containers";

/// One persistent workspace in the fixture's resource table.
#[derive(Clone, Debug)]
struct LiveWorkspace {
    status: ResourceStatus,
    spec: ResourceSpec,
}

/// The fixture's mutable state.
#[derive(Debug)]
struct Inner {
    next_id: u64,
    workspaces: BTreeMap<ResourceId, LiveWorkspace>,
    journal: ResourceEvidenceJournal,
}

/// The local/container-shaped fixture provider.
pub struct LocalContainersFixture {
    descriptor: ProviderDescriptor,
    inner: Mutex<Inner>,
}

impl LocalContainersFixture {
    /// Creates the fixture.
    pub fn new() -> Result<Self, ResourceProviderError> {
        let id = ProviderId::parse(LOCAL_CONTAINERS_PROVIDER_ID)?;
        let descriptor = ProviderDescriptor {
            capabilities: vec![ResourceCapability {
                class: ResourceClass::PersistentWorkspace,
                lifecycle: ResourceLifecycleSupport::none().with_create().with_resize(),
                capacity: ResourceQuota {
                    max_cpu_millis: 4_000,
                    max_memory_mib: 8_192,
                    max_disk_mib: 65_536,
                },
            }],
            health: ProviderHealth {
                status: ProviderHealthStatus::Healthy,
                detail: Some("local container runtime fixture".to_owned()),
                latency_ms: Some(0),
            },
            regions: vec!["local".to_owned()],
            cost: None,
            provider: id,
        };
        descriptor.validate()?;
        Ok(Self {
            descriptor,
            inner: Mutex::new(Inner {
                next_id: 0,
                workspaces: BTreeMap::new(),
                journal: ResourceEvidenceJournal::new(LOCAL_CONTAINERS_PROVIDER_ID),
            }),
        })
    }

    /// Injects the disappearance of the container backing one workspace,
    /// the container-shaped failure mode used by recovery tests: the
    /// workspace record stays, its status becomes `Lost`.
    pub fn remove_container(&self, resource: &ResourceId) {
        let mut inner = self.lock_inner();
        if let Some(workspace) = inner.workspaces.get_mut(resource) {
            workspace.status = ResourceStatus::Lost;
            let payload = serde_json::json!({
                "provider": LOCAL_CONTAINERS_PROVIDER_ID,
                "fact": "container_removed",
                "resource": resource.as_ref(),
                "class": ResourceClass::PersistentWorkspace.as_str(),
            });
            let _ = inner.journal.record(EvidenceKind::Recovery, &payload);
        }
    }

    /// The provisioning spec recorded for one workspace, when it exists.
    pub fn workspace_spec(&self, resource: &ResourceId) -> Option<ResourceSpec> {
        self.lock_inner()
            .workspaces
            .get(resource)
            .map(|live| live.spec.clone())
    }

    fn lock_inner(&self) -> MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn provider_id(&self) -> &ProviderId {
        &self.descriptor.provider
    }

    fn mint_id(inner: &mut Inner) -> Result<ResourceId, ResourceProviderError> {
        let id = ResourceId::parse(format!("ws-{:04}", inner.next_id))?;
        inner.next_id += 1;
        Ok(id)
    }

    fn resource_lost(&self, resource: &ResourceId) -> ResourceProviderError {
        ResourceProviderError::ResourceLost {
            provider: self.provider_id().clone(),
            resource: resource.clone(),
        }
    }

    fn require_owned<'a>(
        &self,
        inner: &'a Inner,
        handle: &ResourceHandle,
    ) -> Result<&'a LiveWorkspace, ResourceProviderError> {
        if handle.provider != self.descriptor.provider {
            return Err(self.resource_lost(&handle.resource));
        }
        inner
            .workspaces
            .get(&handle.resource)
            .ok_or_else(|| self.resource_lost(&handle.resource))
    }

    fn journal_transition(
        inner: &mut Inner,
        resource: &ResourceId,
        op: &'static str,
    ) -> Result<(), ResourceProviderError> {
        let payload = serde_json::json!({
            "provider": LOCAL_CONTAINERS_PROVIDER_ID,
            "fact": "resource_transition",
            "resource": resource.as_ref(),
            "class": ResourceClass::PersistentWorkspace.as_str(),
            "op": op,
            "detail": "container-style persistent transition",
        });
        inner.journal.record(EvidenceKind::Artifact, &payload)?;
        Ok(())
    }
}

impl ExecutionResourceProvider for LocalContainersFixture {
    fn descriptor(&self) -> &ProviderDescriptor {
        &self.descriptor
    }

    fn probe(&self) -> Result<ProviderHealth, ResourceProviderError> {
        Ok(self.descriptor.health.clone())
    }

    fn create(&self, spec: ResourceSpec) -> Result<ResourceHandle, ResourceProviderError> {
        spec.validate()?;
        let capability = self.descriptor.capability_for(spec.class).ok_or_else(|| {
            ResourceProviderError::InvalidSpec {
                reason: format!(
                    "provider `{}` does not serve resource class `{}`",
                    self.provider_id(),
                    spec.class
                ),
            }
        })?;
        if !capability.lifecycle.network_policy && spec.network.is_some() {
            return Err(ResourceProviderError::InvalidSpec {
                reason: format!(
                    "provider `{}` does not accept network policy input for class `{}`",
                    self.provider_id(),
                    spec.class
                ),
            });
        }
        if !capability.lifecycle.gpu && spec.gpu.is_some() {
            return Err(ResourceProviderError::InvalidSpec {
                reason: format!(
                    "provider `{}` does not serve GPU specs for class `{}`",
                    self.provider_id(),
                    spec.class
                ),
            });
        }
        let capacity = capability.capacity;
        if let Some(cpu) = spec.cpu_millis
            && cpu > capacity.max_cpu_millis
        {
            let max = capacity.max_cpu_millis;
            return Err(ResourceProviderError::InvalidSpec {
                reason: format!("cpuMillis {cpu} exceeds the local quota {max}"),
            });
        }
        if let Some(memory) = spec.memory_mib
            && memory > capacity.max_memory_mib
        {
            let max = capacity.max_memory_mib;
            return Err(ResourceProviderError::InvalidSpec {
                reason: format!("memoryMiB {memory} exceeds the local quota {max}"),
            });
        }
        if let Some(disk) = spec.disk_mib
            && disk > capacity.max_disk_mib
        {
            let max = capacity.max_disk_mib;
            return Err(ResourceProviderError::InvalidSpec {
                reason: format!("diskMiB {disk} exceeds the local quota {max}"),
            });
        }
        let mut inner = self.lock_inner();
        let id = Self::mint_id(&mut inner)?;
        inner.workspaces.insert(
            id.clone(),
            LiveWorkspace {
                status: ResourceStatus::Active,
                spec,
            },
        );
        Self::journal_transition(&mut inner, &id, "created")?;
        Ok(ResourceHandle {
            resource: id,
            class: ResourceClass::PersistentWorkspace,
            provider: self.provider_id().clone(),
            status: ResourceStatus::Active,
        })
    }

    fn status(&self, handle: &ResourceHandle) -> Result<ResourceStatus, ResourceProviderError> {
        let inner = self.lock_inner();
        Ok(self.require_owned(&inner, handle)?.status)
    }

    fn release(&self, handle: &ResourceHandle) -> Result<ResourceHandle, ResourceProviderError> {
        let mut inner = self.lock_inner();
        let mut workspace = self.require_owned(&inner, handle)?.clone();
        workspace.status = ResourceStatus::Released;
        inner.workspaces.insert(handle.resource.clone(), workspace);
        Self::journal_transition(&mut inner, &handle.resource, "released")?;
        Ok(ResourceHandle {
            resource: handle.resource.clone(),
            class: ResourceClass::PersistentWorkspace,
            provider: self.provider_id().clone(),
            status: ResourceStatus::Released,
        })
    }

    fn lifecycle(
        &self,
        handle: &ResourceHandle,
        op: LifecycleOp,
    ) -> Result<LifecycleOutcome, ResourceProviderError> {
        if let LifecycleOp::Resize(request) = &op {
            request.validate()?;
        }
        let mut inner = self.lock_inner();
        let workspace = self.require_owned(&inner, handle)?.clone();
        let capability = self
            .descriptor
            .capability_for(handle.class)
            .ok_or_else(|| self.resource_lost(&handle.resource))?;
        if !capability.lifecycle.supports(&op) {
            return Err(ResourceProviderError::Unsupported {
                provider: self.provider_id().clone(),
                operation: op,
            });
        }
        // Non-create lifecycle ops require a live workspace; `Create`
        // (re-provision from the handle's shape) stays usable on any
        // known workspace, which is exactly the recovery path.
        if !matches!(op, LifecycleOp::Create) && !workspace.status.is_bindable() {
            return Err(self.resource_lost(&handle.resource));
        }
        match op {
            // The fixture's only declared lifecycle op beyond create:
            // apply the resize request to the workspace's spec, within
            // the declared quota.
            LifecycleOp::Resize(request) => {
                let capacity = capability.capacity;
                if let Some(cpu) = request.cpu_millis
                    && cpu > capacity.max_cpu_millis
                {
                    let max = capacity.max_cpu_millis;
                    return Err(ResourceProviderError::InvalidSpec {
                        reason: format!("resize cpuMillis {cpu} exceeds the local quota {max}"),
                    });
                }
                if let Some(memory) = request.memory_mib
                    && memory > capacity.max_memory_mib
                {
                    let max = capacity.max_memory_mib;
                    return Err(ResourceProviderError::InvalidSpec {
                        reason: format!("resize memoryMiB {memory} exceeds the local quota {max}"),
                    });
                }
                if let Some(disk) = request.disk_mib
                    && disk > capacity.max_disk_mib
                {
                    let max = capacity.max_disk_mib;
                    return Err(ResourceProviderError::InvalidSpec {
                        reason: format!("resize diskMiB {disk} exceeds the local quota {max}"),
                    });
                }
                let mut spec = workspace.spec;
                if let Some(cpu) = request.cpu_millis {
                    spec.cpu_millis = Some(cpu);
                }
                if let Some(memory) = request.memory_mib {
                    spec.memory_mib = Some(memory);
                }
                if let Some(disk) = request.disk_mib {
                    spec.disk_mib = Some(disk);
                }
                inner.workspaces.insert(
                    handle.resource.clone(),
                    LiveWorkspace {
                        status: ResourceStatus::Active,
                        spec,
                    },
                );
                Self::journal_transition(&mut inner, &handle.resource, "resized")?;
                Ok(LifecycleOutcome::Transitioned(ResourceHandle {
                    resource: handle.resource.clone(),
                    class: ResourceClass::PersistentWorkspace,
                    provider: self.provider_id().clone(),
                    status: ResourceStatus::Active,
                }))
            }
            // Every other op is undeclared for local containers; the
            // support check above already rejected it, and the arms stay
            // explicit and panic-free.
            // Re-provision from the handle's shape: a fresh persistent
            // workspace with a new id, mirroring the sandbox fixture's
            // recovery-friendly lifecycle create (the descriptor
            // declares it, so it must be honored, never rejected).
            LifecycleOp::Create => {
                let id = Self::mint_id(&mut inner)?;
                let spec = workspace.spec;
                inner.workspaces.insert(
                    id.clone(),
                    LiveWorkspace {
                        status: ResourceStatus::Active,
                        spec,
                    },
                );
                Self::journal_transition(&mut inner, &id, "recreated")?;
                Ok(LifecycleOutcome::Transitioned(ResourceHandle {
                    resource: id,
                    class: ResourceClass::PersistentWorkspace,
                    provider: self.provider_id().clone(),
                    status: ResourceStatus::Active,
                }))
            }
            LifecycleOp::Pause => Err(ResourceProviderError::Unsupported {
                provider: self.provider_id().clone(),
                operation: LifecycleOp::Pause,
            }),
            LifecycleOp::Resume => Err(ResourceProviderError::Unsupported {
                provider: self.provider_id().clone(),
                operation: LifecycleOp::Resume,
            }),
            LifecycleOp::Snapshot => Err(ResourceProviderError::Unsupported {
                provider: self.provider_id().clone(),
                operation: LifecycleOp::Snapshot,
            }),
            LifecycleOp::Fork => Err(ResourceProviderError::Unsupported {
                provider: self.provider_id().clone(),
                operation: LifecycleOp::Fork,
            }),
        }
    }

    fn evidence_records(&self) -> Vec<ResourceEvidenceRecord> {
        self.lock_inner().journal.records().to_vec()
    }
}

#[cfg(test)]
#[path = "local_containers_tests.rs"]
mod tests;
