//! The execution resource provider port (WO-016).
//!
//! [`ExecutionResourceProvider`] is the single provider-neutral boundary
//! every infrastructure implementation satisfies: E2B, Daytona, Modal,
//! Kubernetes, local/container/VM infrastructure, and future providers
//! are all just implementations of this port. It follows the proven
//! host-owns-the-runtime convention of the WO-015 bridge ports
//! (`RemoteDesktopBridge`): the trait is synchronous and
//! `Send + Sync`, no provider runtime is linked by this crate, and the
//! host bridges to its own concurrency as needed.
//!
//! ## Boundary rules
//!
//! - Providers are **resource lifecycle executors, never authorities**.
//!   They declare capabilities, provision resources, and report status;
//!   binding, policy, authorization, and workflow decisions stay with
//!   the WO-005 registry, the approval plane, and the control plane.
//! - **Unsupported operations stay explicitly unsupported.** A lifecycle
//!   operation the provider's descriptor does not declare returns
//!   [`ResourceProviderError::Unsupported`]; implementations never
//!   panic, never silently no-op, and never grow provider-specific
//!   method names.
//! - **Long-running task semantics stay with Codex/workflow
//!   orchestration.** Persistence, pause/resume, snapshots, and forks are
//!   resource lifecycle capabilities only: a task may move between
//!   providers without changing its semantic identity.
//! - **Handles are credential-free.** Providers resolve secret material
//!   through host-owned configuration references and never surface it in
//!   handles, specs, outcomes, or evidence.
//! - Implementations MUST reject a handle whose provider id is not their
//!   own with [`ResourceProviderError::ResourceLost`], so cross-provider
//!   handle confusion is a diagnosable error, never silent reuse.
//! - All provider outputs (health, handles, snapshots, evidence payloads)
//!   are untrusted input by default.

use std::fmt;

use serde::Deserialize;
use serde::Serialize;

use crate::ResourceProviderError;
use crate::descriptor::ProviderHealth;
use crate::evidence::ResourceEvidenceRecord;
use crate::model::CPU_MILLIS_MAX;
use crate::model::DISK_MIB_MAX;
use crate::model::MEMORY_MIB_MAX;
use crate::model::ResourceHandle;
use crate::model::ResourceSnapshot;
use crate::model::ResourceSpec;
use crate::model::ResourceStatus;

/// One lifecycle operation, dispatched through
/// [`ExecutionResourceProvider::lifecycle`].
///
/// The enum is the entire lifecycle vocabulary: there are no
/// provider-specific methods, so a provider that does not declare an
/// operation returns [`ResourceProviderError::Unsupported`] for its
/// variant instead of growing a bespoke API.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub enum LifecycleOp {
    /// Provision a fresh resource with the same shape as the handle's
    /// resource, returning a **new** handle with a **new** opaque
    /// resource id. This is the recovery-friendly re-create variant; the
    /// primary provisioning entry remains
    /// [`ExecutionResourceProvider::create`], which takes a full spec.
    Create,
    /// Pause the resource, preserving state where the provider supports
    /// it.
    Pause,
    /// Resume a paused resource.
    Resume,
    /// Capture a snapshot/checkpoint artifact of the resource's current
    /// execution state.
    Snapshot,
    /// Fork the resource's current state into a new resource, returning
    /// a new handle while leaving the source resource live.
    Fork,
    /// Resize the resource to the requested capacities.
    Resize(ResizeRequest),
}

impl LifecycleOp {
    /// The operation's stable name, matching the serde form, used for
    /// error reporting.
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Create => "create",
            Self::Pause => "pause",
            Self::Resume => "resume",
            Self::Snapshot => "snapshot",
            Self::Fork => "fork",
            Self::Resize(_) => "resize",
        }
    }
}

impl fmt::Display for LifecycleOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// A validated request to resize one live resource.
///
/// Absent fields keep the resource's current allocation; at least one
/// field must be present.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResizeRequest {
    /// The new CPU allocation in milli-CPU, when changing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cpu_millis: Option<u32>,
    /// The new memory allocation in MiB, when changing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memory_mib: Option<u32>,
    /// The new disk allocation in MiB, when changing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disk_mib: Option<u64>,
}

impl ResizeRequest {
    /// Validates the request's bounds.
    pub fn validate(self) -> Result<(), ResourceProviderError> {
        if self.cpu_millis.is_none() && self.memory_mib.is_none() && self.disk_mib.is_none() {
            return Err(ResourceProviderError::InvalidSpec {
                reason: "resize requests carry at least one new capacity".to_owned(),
            });
        }
        if let Some(cpu) = self.cpu_millis
            && (cpu == 0 || cpu > CPU_MILLIS_MAX)
        {
            return Err(ResourceProviderError::InvalidSpec {
                reason: format!("resize cpuMillis must be 1..={CPU_MILLIS_MAX}"),
            });
        }
        if let Some(memory) = self.memory_mib
            && (memory == 0 || memory > MEMORY_MIB_MAX)
        {
            return Err(ResourceProviderError::InvalidSpec {
                reason: format!("resize memoryMiB must be 1..={MEMORY_MIB_MAX}"),
            });
        }
        if let Some(disk) = self.disk_mib
            && (disk == 0 || disk > DISK_MIB_MAX)
        {
            return Err(ResourceProviderError::InvalidSpec {
                reason: format!("resize diskMiB must be 1..={DISK_MIB_MAX}"),
            });
        }
        Ok(())
    }
}

/// The outcome of one dispatched lifecycle operation.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub enum LifecycleOutcome {
    /// The operation produced an updated handle: a state transition
    /// (`Pause`/`Resume`/`Resize`), a newly minted resource
    /// (`Create`/`Fork`), or a release confirmation.
    Transitioned(ResourceHandle),
    /// The operation captured a snapshot artifact. Snapshots are
    /// execution artifacts — they never replace immutable workflow
    /// identity.
    Snapshotted(ResourceSnapshot),
}

impl LifecycleOutcome {
    /// The handle this outcome carries, when it is a transition.
    pub fn handle(&self) -> Option<&ResourceHandle> {
        match self {
            Self::Transitioned(handle) => Some(handle),
            Self::Snapshotted(_) => None,
        }
    }
}

/// The provider-neutral execution resource provider boundary.
///
/// Implementations are expected to:
///
/// - keep [`ExecutionResourceProvider::descriptor`] cheap and
///   synchronous: it is the registration surface;
/// - treat [`ExecutionResourceProvider::probe`] as a health check, not a
///   long-running operation;
/// - validate specs explicitly at create time, rejecting undeclared
///   features (network policy, GPU) instead of silently ignoring them;
/// - mint fresh, opaque resource ids for every provisioning (including
///   the `Create`/`Fork` lifecycle ops), so rebinding always produces a
///   new instance identity;
/// - return [`ResourceProviderError::Unsupported`] for lifecycle
///   operations their descriptor does not declare;
/// - journal lifecycle transitions, health, and capacity facts into an
///   append-only provider-side evidence journal and expose the records
///   through [`ExecutionResourceProvider::evidence_records`] for host
///   harvesting;
/// - never panic across the boundary.
pub trait ExecutionResourceProvider: Send + Sync {
    /// The provider's registration declaration.
    fn descriptor(&self) -> &crate::descriptor::ProviderDescriptor;

    /// Probes the provider's live health.
    ///
    /// An unreachable provider surfaces as an
    /// [`ResourceProviderError::Unreachable`] error so it is a
    /// diagnosable condition, never a silently weaker mechanism.
    fn probe(&self) -> Result<ProviderHealth, ResourceProviderError>;

    /// Provisions one resource from a validated spec.
    ///
    /// Returns an active, credential-free handle naming the new opaque
    /// resource id. Specs carrying features the provider does not
    /// declare are rejected with [`ResourceProviderError::InvalidSpec`].
    fn create(&self, spec: ResourceSpec) -> Result<ResourceHandle, ResourceProviderError>;

    /// Reports the resource-plane status of the resource named by the
    /// handle.
    ///
    /// Handles owned by other providers, or resources the provider no
    /// longer knows, surface as [`ResourceProviderError::ResourceLost`].
    fn status(&self, handle: &ResourceHandle) -> Result<ResourceStatus, ResourceProviderError>;

    /// Releases the resource named by the handle.
    ///
    /// Returns the handle updated to
    /// [`crate::model::ResourceStatus::Released`]. Persistent providers
    /// may keep the underlying state; the instance identity is no longer
    /// live.
    fn release(&self, handle: &ResourceHandle) -> Result<ResourceHandle, ResourceProviderError>;

    /// Dispatches one lifecycle operation on the resource named by the
    /// handle.
    ///
    /// The single enum-dispatched lifecycle entry: providers translate
    /// [`LifecycleOp`] variants into their own infrastructure semantics
    /// and return the normalized outcome. Operations the descriptor does
    /// not declare return [`ResourceProviderError::Unsupported`], naming
    /// both the provider and the operation.
    fn lifecycle(
        &self,
        handle: &ResourceHandle,
        op: LifecycleOp,
    ) -> Result<LifecycleOutcome, ResourceProviderError>;

    /// The evidence records recorded so far, oldest first.
    ///
    /// The host harvests these provider/resource facts (lifecycle
    /// transitions, provider health, capacity) and the evidence plane
    /// promotes them into contract `EvidenceReference` values.
    fn evidence_records(&self) -> Vec<ResourceEvidenceRecord>;
}
