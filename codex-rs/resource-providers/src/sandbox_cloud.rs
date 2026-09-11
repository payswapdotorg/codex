//! The E2B-shaped sandbox-cloud conformance fixture (WO-016).
//!
//! [`SandboxCloudFixture`] is an in-process fake shaped like an ephemeral
//! sandbox **cloud** provider (the E2B-shaped candidate class): it serves
//! [`ResourceClass::Sandbox`] and [`ResourceClass::RemoteDesktop`],
//! supports create/pause_resume/snapshot/fork plus network-policy
//! inputs, has *no* persistence and *no* resize, enforces a hard live
//! capacity, and models a cloud-style failure mode (provider outage) and
//! cloud-style timings (non-zero probe latency). No real provider SDK is
//! linked: this fixture exists so the provider-neutral contract and the
//! shared conformance harness can be exercised without any vendor
//! runtime, and so recovery paths can inject resource loss
//! deterministically.
//!
//! The fixture is a **test double**, not a runtime: nothing is connected,
//! launched, or provisioned, and no credential material exists anywhere
//! in it.

use std::collections::BTreeMap;
use std::sync::Mutex;
use std::sync::MutexGuard;
use std::sync::PoisonError;

use codex_workflow_contracts::EvidenceKind;

use crate::ExecutionResourceProvider;
use crate::ResourceProviderError;
use crate::descriptor::CostMetadata;
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
use crate::model::ResourceSnapshot;
use crate::model::ResourceSpec;
use crate::model::ResourceStatus;
use crate::provider::LifecycleOp;
use crate::provider::LifecycleOutcome;

/// The fixture provider's identity.
pub const SANDBOX_CLOUD_PROVIDER_ID: &str = "fixture-sandbox-cloud";

/// One live sandbox in the fixture's resource table.
#[derive(Clone, Debug)]
struct LiveSandbox {
    class: ResourceClass,
    status: ResourceStatus,
    spec: ResourceSpec,
}

/// The fixture's mutable state.
#[derive(Debug)]
struct Inner {
    next_id: u64,
    next_snapshot: u64,
    resources: BTreeMap<ResourceId, LiveSandbox>,
    journal: ResourceEvidenceJournal,
}

/// The E2B-shaped sandbox-cloud fixture provider.
pub struct SandboxCloudFixture {
    descriptor: ProviderDescriptor,
    max_live: usize,
    outage: Mutex<bool>,
    inner: Mutex<Inner>,
}

impl SandboxCloudFixture {
    /// Creates the fixture allowing at most `max_live` live resources.
    pub fn new(max_live: usize) -> Result<Self, ResourceProviderError> {
        let id = ProviderId::parse(SANDBOX_CLOUD_PROVIDER_ID)?;
        let lifecycle = ResourceLifecycleSupport::none()
            .with_create()
            .with_pause_resume()
            .with_snapshot()
            .with_fork()
            .with_network_policy();
        let descriptor = ProviderDescriptor {
            capabilities: vec![
                ResourceCapability {
                    class: ResourceClass::Sandbox,
                    lifecycle,
                    capacity: ResourceQuota {
                        max_cpu_millis: 2_000,
                        max_memory_mib: 4_096,
                        max_disk_mib: 20_480,
                    },
                },
                ResourceCapability {
                    class: ResourceClass::RemoteDesktop,
                    lifecycle,
                    capacity: ResourceQuota {
                        max_cpu_millis: 1_000,
                        max_memory_mib: 2_048,
                        max_disk_mib: 10_240,
                    },
                },
            ],
            health: ProviderHealth {
                status: ProviderHealthStatus::Healthy,
                detail: Some("ephemeral sandbox cloud fixture".to_owned()),
                latency_ms: Some(42),
            },
            regions: vec!["us-east-1".to_owned(), "eu-west-1".to_owned()],
            cost: Some(CostMetadata {
                currency: "usd".to_owned(),
                per_hour_micros: 3_600_000,
            }),
            provider: id,
        };
        descriptor.validate()?;
        Ok(Self {
            descriptor,
            max_live,
            outage: Mutex::new(false),
            inner: Mutex::new(Inner {
                next_id: 0,
                next_snapshot: 0,
                resources: BTreeMap::new(),
                journal: ResourceEvidenceJournal::new(SANDBOX_CLOUD_PROVIDER_ID),
            }),
        })
    }

    /// Puts the fixture's cloud endpoint into (or out of) an outage.
    ///
    /// While in outage, probes and creates fail with an explicit
    /// `Unreachable` error — the cloud-style failure mode.
    pub fn set_outage(&self, outage: bool) {
        *self.lock_outage() = outage;
    }

    /// Injects the loss of one resource (eviction/expiry/crash), the
    /// deterministic fault used by recovery tests.
    pub fn lose_resource(&self, resource: &ResourceId) {
        let mut inner = self.lock_inner();
        if let Some(live) = inner.resources.get_mut(resource) {
            live.status = ResourceStatus::Lost;
            let payload = transition_payload(
                resource,
                live.class,
                "lost",
                "resource evicted by the fixture",
            );
            let _ = inner.journal.record(EvidenceKind::Recovery, &payload);
        }
    }

    /// Number of resources currently live (active or paused).
    pub fn live_count(&self) -> usize {
        self.lock_inner()
            .resources
            .values()
            .filter(|live| live.status.is_bindable())
            .count()
    }

    fn lock_inner(&self) -> MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn lock_outage(&self) -> MutexGuard<'_, bool> {
        self.outage.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn provider_id(&self) -> &ProviderId {
        &self.descriptor.provider
    }

    fn mint_id(inner: &mut Inner) -> Result<ResourceId, ResourceProviderError> {
        let id = ResourceId::parse(format!("sbx-{:04}", inner.next_id))?;
        inner.next_id += 1;
        Ok(id)
    }

    /// Rejects handles the fixture does not own, mirroring the port rule
    /// for foreign or unknown resources.
    fn require_owned<'a>(
        &self,
        inner: &'a Inner,
        handle: &ResourceHandle,
    ) -> Result<&'a LiveSandbox, ResourceProviderError> {
        if handle.provider != self.descriptor.provider {
            return Err(self.resource_lost(&handle.resource));
        }
        inner
            .resources
            .get(&handle.resource)
            .ok_or_else(|| self.resource_lost(&handle.resource))
    }

    fn resource_lost(&self, resource: &ResourceId) -> ResourceProviderError {
        ResourceProviderError::ResourceLost {
            provider: self.provider_id().clone(),
            resource: resource.clone(),
        }
    }

    fn journal_transition(
        inner: &mut Inner,
        resource: &ResourceId,
        class: ResourceClass,
        op: &'static str,
    ) -> Result<(), ResourceProviderError> {
        let payload = transition_payload(resource, class, op, "cloud-style ephemeral transition");
        inner.journal.record(EvidenceKind::Artifact, &payload)?;
        Ok(())
    }
}

impl ExecutionResourceProvider for SandboxCloudFixture {
    fn descriptor(&self) -> &ProviderDescriptor {
        &self.descriptor
    }

    fn probe(&self) -> Result<ProviderHealth, ResourceProviderError> {
        if *self.lock_outage() {
            return Err(ResourceProviderError::Unreachable {
                provider: self.provider_id().clone(),
                detail: "sandbox cloud fixture is in an injected outage".to_owned(),
            });
        }
        Ok(self.descriptor.health.clone())
    }

    fn create(&self, spec: ResourceSpec) -> Result<ResourceHandle, ResourceProviderError> {
        if *self.lock_outage() {
            return Err(ResourceProviderError::Unreachable {
                provider: self.provider_id().clone(),
                detail: "sandbox cloud fixture is in an injected outage".to_owned(),
            });
        }
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
        let mut inner = self.lock_inner();
        if inner
            .resources
            .values()
            .filter(|live| live.status.is_bindable())
            .count()
            >= self.max_live
        {
            let payload = serde_json::json!({
                "provider": SANDBOX_CLOUD_PROVIDER_ID,
                "fact": "capacity_exhausted",
                "maxLive": self.max_live,
            });
            inner.journal.record(EvidenceKind::Recovery, &payload)?;
            let max_live = self.max_live;
            return Err(ResourceProviderError::CapacityExhausted {
                provider: self.provider_id().clone(),
                detail: format!("at most {max_live} live resources"),
            });
        }
        let id = Self::mint_id(&mut inner)?;
        let class = spec.class;
        inner.resources.insert(
            id.clone(),
            LiveSandbox {
                class,
                status: ResourceStatus::Active,
                spec,
            },
        );
        Self::journal_transition(&mut inner, &id, class, "created")?;
        Ok(ResourceHandle {
            resource: id,
            class,
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
        let live = self.require_owned(&inner, handle)?.clone();
        let class = live.class;
        let mut updated = live;
        updated.status = ResourceStatus::Released;
        inner.resources.insert(handle.resource.clone(), updated);
        Self::journal_transition(&mut inner, &handle.resource, class, "released")?;
        Ok(ResourceHandle {
            resource: handle.resource.clone(),
            class,
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
        let live = self.require_owned(&inner, handle)?.clone();
        let capability = self
            .descriptor
            .capability_for(live.class)
            .ok_or_else(|| self.resource_lost(&handle.resource))?;
        if !capability.lifecycle.supports(&op) {
            return Err(ResourceProviderError::Unsupported {
                provider: self.provider_id().clone(),
                operation: op,
            });
        }
        // Non-create lifecycle ops require a live resource; `Create`
        // (re-provision from the handle's shape) stays usable on any
        // known resource, which is exactly the recovery path.
        if !matches!(op, LifecycleOp::Create) && !live.status.is_bindable() {
            return Err(self.resource_lost(&handle.resource));
        }
        match op {
            LifecycleOp::Create => {
                let id = Self::mint_id(&mut inner)?;
                let class = live.class;
                inner.resources.insert(
                    id.clone(),
                    LiveSandbox {
                        class,
                        status: ResourceStatus::Active,
                        spec: live.spec,
                    },
                );
                Self::journal_transition(&mut inner, &id, class, "recreated")?;
                Ok(LifecycleOutcome::Transitioned(ResourceHandle {
                    resource: id,
                    class,
                    provider: self.provider_id().clone(),
                    status: ResourceStatus::Active,
                }))
            }
            LifecycleOp::Pause => {
                let class = live.class;
                let mut updated = live;
                updated.status = ResourceStatus::Paused;
                inner.resources.insert(handle.resource.clone(), updated);
                Self::journal_transition(&mut inner, &handle.resource, class, "paused")?;
                Ok(LifecycleOutcome::Transitioned(ResourceHandle {
                    resource: handle.resource.clone(),
                    class,
                    provider: self.provider_id().clone(),
                    status: ResourceStatus::Paused,
                }))
            }
            LifecycleOp::Resume => {
                let class = live.class;
                let mut updated = live;
                updated.status = ResourceStatus::Active;
                inner.resources.insert(handle.resource.clone(), updated);
                Self::journal_transition(&mut inner, &handle.resource, class, "resumed")?;
                Ok(LifecycleOutcome::Transitioned(ResourceHandle {
                    resource: handle.resource.clone(),
                    class,
                    provider: self.provider_id().clone(),
                    status: ResourceStatus::Active,
                }))
            }
            LifecycleOp::Snapshot => {
                let artifact = format!("snap-{:04}", inner.next_snapshot);
                inner.next_snapshot += 1;
                let snapshot = ResourceSnapshot {
                    resource: handle.resource.clone(),
                    artifact: artifact.clone(),
                };
                snapshot.validate()?;
                let payload = serde_json::json!({
                    "provider": SANDBOX_CLOUD_PROVIDER_ID,
                    "fact": "resource_snapshot",
                    "resource": handle.resource.as_ref(),
                    "class": live.class.as_str(),
                    "artifact": artifact,
                });
                inner.journal.record(EvidenceKind::Artifact, &payload)?;
                Ok(LifecycleOutcome::Snapshotted(snapshot))
            }
            LifecycleOp::Fork => {
                let id = Self::mint_id(&mut inner)?;
                let class = live.class;
                inner.resources.insert(
                    id.clone(),
                    LiveSandbox {
                        class,
                        status: ResourceStatus::Active,
                        spec: live.spec,
                    },
                );
                Self::journal_transition(&mut inner, &id, class, "forked")?;
                Ok(LifecycleOutcome::Transitioned(ResourceHandle {
                    resource: id,
                    class,
                    provider: self.provider_id().clone(),
                    status: ResourceStatus::Active,
                }))
            }
            // The fixture never declares resize support, so the support
            // check above already rejected it; keep the arm explicit and
            // panic-free anyway.
            LifecycleOp::Resize(request) => Err(ResourceProviderError::Unsupported {
                provider: self.provider_id().clone(),
                operation: LifecycleOp::Resize(request),
            }),
        }
    }

    fn evidence_records(&self) -> Vec<ResourceEvidenceRecord> {
        self.lock_inner().journal.records().to_vec()
    }
}

/// Builds a canonical transition payload for the fixture journal.
fn transition_payload(
    resource: &ResourceId,
    class: ResourceClass,
    op: &str,
    detail: &str,
) -> serde_json::Value {
    serde_json::json!({
        "provider": SANDBOX_CLOUD_PROVIDER_ID,
        "fact": "resource_transition",
        "resource": resource.as_ref(),
        "class": class.as_str(),
        "op": op,
        "detail": detail,
    })
}

#[cfg(test)]
#[path = "sandbox_cloud_tests.rs"]
mod tests;
