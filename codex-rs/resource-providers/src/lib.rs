//! Execution resource providers for the Codex universal workflow platform
//! (WO-016).
//!
//! This crate is the provider-neutral **execution resource provider plane**
//! that sits *beneath* the WO-005 execution contracts: one universal
//! contract for provisioning and lifecycle-managing execution resources
//! (isolated compute sandboxes, remote graphical desktops, persistent
//! workspaces, specialized compute) across many interchangeable
//! infrastructure implementations — E2B, Daytona, Modal, Kubernetes,
//! local/container/VM infrastructure, and future providers are all just
//! candidate implementations of the [`ExecutionResourceProvider`] port.
//! It is a contracts-plus-fixtures leaf crate in the family proven by
//! `codex-env-adapters` (WO-015): no real provider SDK is linked, no
//! runtime is launched or provisioned here, and no provider is a
//! mandatory dependency.
//!
//! ## Position in the stack
//!
//! ```text
//! workflow plane (WO-003 contracts, WO-010 app, WO-011 triggers)
//!     -> declares typed resource REQUIREMENTS (logical resource types)
//! execution-contracts (WO-005)
//!     -> ResourceBinding / ResourceId / ExecutionEnvironment: the frozen
//!        resource binding boundary (opaque, credential-free instances)
//! resource-providers (this crate)
//!     -> the provider-neutral provider port: specs, handles, lifecycle
//!        ops, capability declarations, evidence, binding minting
//! concrete providers (E2B, Daytona, Modal, Kubernetes, local, ...)
//!     -> host-supplied implementations of the port; replaceable
//! ```
//!
//! The integration surface is exactly the frozen contract seam: a
//! provider mints an opaque [`ResourceId`] and the bind surface
//! ([`bind::bind`], [`bind::bind_as`]) turns it into a plain
//! `codex_execution_contracts::ResourceBinding` — logical
//! snake_case resource type, opaque instance id, peer holder environment
//! — so a workflow's declared resource requirement binds at
//! install/instantiate time through the **existing** registry path
//! (`workflow-triggers` `InstallRequest::resources` / `workflow-app`
//! `InstantiateRequest::resources`).
//!
//! ## What this crate is
//!
//! - A provider-neutral resource model ([`model`]): resource
//!   classes, validated specs, credential-free handles, and lifecycle
//!   statuses — provider/resource state only, never workflow state.
//! - Capability **declarations** ([`descriptor`]): which resource
//!   classes a provider serves, which lifecycle operations it supports
//!   (create/pause_resume/snapshot/fork/resize, network policy, GPU),
//!   plus health, region, and cost metadata. Declaration never implies
//!   authorization: binding and authorization decisions stay with the
//!   WO-005 `CapabilityRegistry` and the approval plane.
//! - The port ([`provider`]): [`ExecutionResourceProvider`] —
//!   `descriptor`, `probe`, `create`, `status`, `release`, and one
//!   enum-dispatched `lifecycle` method. Unsupported operations are
//!   explicit [`ResourceProviderError::Unsupported`] errors, never
//!   panics, silent no-ops, or provider-specific method names.
//! - A small host wiring registry ([`registry`]):
//!   [`ProviderRegistry`] holds providers plus credential *references*.
//!   It is resource-plane only: it does not duplicate or bypass
//!   `CapabilityRegistry` binding/authorization decisions.
//! - An adapter-side evidence journal ([`evidence`]): canonical-JSON
//!   SHA-256, deterministic `codex-resource-providers/<provider>/<seq>`
//!   locators, in-memory append-only records for host harvesting —
//!   provider/resource facts (lifecycle transitions, provider health,
//!   capacity) that stay clearly distinguished from workflow semantic
//!   facts through provider-scoped locators and payloads.
//! - Two materially different in-process conformance fixtures
//!   ([`sandbox_cloud`], [`local_containers`]) plus a shared
//!   provider-neutral conformance harness ([`conformance`]) run against
//!   both with identical operations and identical assertions — the
//!   proof that the contract is not shaped around any single vendor.
//!
//! ## What this crate deliberately is not
//!
//! - **No provider is mandatory.** With no provider registered, nothing
//!   in the workflow plane changes: resource requirements still bind
//!   through opaque ids exactly as WO-005/WO-010 already define, and
//!   ordinary Codex behavior is untouched.
//! - **No workflow-semantic change.** No provider name, resource class,
//!   region, snapshot id, or infrastructure identifier ever enters
//!   `codex-workflow-contracts` types, `WorkflowIR`, or
//!   `ResourceTypeId` values. Provider names appear only in
//!   provider-scoped evidence locators and payloads.
//! - **No long-running task semantics.** Codex/workflow orchestration
//!   owns tasks, sessions, and durable state. A provider may offer
//!   persistence, pause/resume, snapshots, or forks — resource
//!   lifecycle capabilities only — and a task can theoretically move
//!   between providers without changing its semantic identity.
//!   Resource lifecycle state never becomes workflow semantic state.
//! - **No second runtime, no second engine, no durable state.** Bridge
//!   provisioning, protocol handling, graph traversal, and evidence
//!   storage stay with the host, the control plane, and the evidence
//!   plane; this crate only normalizes specs, handles, lifecycle
//!   outcomes, and evidence records.
//! - **No credential values.** Credentials are runtime configuration
//!   references only ([`registry::ProviderCredentials`], the
//!   `env_key` precedent from `codex-model-provider-info`): an
//!   environment-variable *name*, never a value. Resolution is
//!   host-owned — the secrets crate's `SecretsManager` is the approved
//!   store. Handles, specs, descriptors, evidence payloads, fixtures,
//!   and tests are credential-free by construction.
//! - **No authorization.** The reserved environments (`MOBILE`,
//!   `REMOTE_DESKTOP`) stay untouched; binding/authorization authority
//!   remains with the WO-005 registry and the approval plane.
//!
//! ## Error and recovery mapping
//!
//! [`ResourceProviderError`] maps onto the execution-contract
//! `FailureKind` family (`Unavailable`/`Timeout`/`PolicyDenied`/
//! `Permanent`) through [`ResourceProviderError::failure_kind`] and
//! [`ResourceProviderError::normalized_failure`], so provider failure
//! flows through the existing recovery model (environment loss ->
//! `Unavailable` -> rebind) without mutating workflow meaning. Rebinding
//! always mints a **new** opaque resource id while the workflow's
//! semantic identity — version, source revision, dependency pins —
//! stays untouched.
//!
//! ## Ordinary Codex compatibility
//!
//! Nothing in this crate executes unless a host explicitly registers a
//! provider and asks it to provision a resource. The no-provider
//! default is a strict no-op: no adapter, registry, or workflow code
//! path in any other crate references this crate.

#![deny(missing_docs)]

pub mod bind;
pub mod conformance;
pub mod descriptor;
pub mod error;
pub mod evidence;
pub mod local_containers;
pub mod model;
pub mod provider;
pub mod registry;
pub mod sandbox_cloud;

pub use bind::{bind, bind_as, canonical_resource_type, holder_environment};
pub use conformance::assert_provider_conformance;
pub use descriptor::{
    CostMetadata, ProviderDescriptor, ProviderHealth, ProviderHealthStatus, ResourceCapability,
    ResourceLifecycleSupport, ResourceQuota,
};
pub use error::ResourceProviderError;
pub use evidence::{EVIDENCE_LOCATOR_PREFIX, ResourceEvidenceJournal, ResourceEvidenceRecord};
pub use local_containers::{LOCAL_CONTAINERS_PROVIDER_ID, LocalContainersFixture};
pub use model::{
    NetworkPolicy, ResourceClass, ResourceHandle, ResourceSnapshot, ResourceSpec, ResourceStatus,
};
pub use provider::{ExecutionResourceProvider, LifecycleOp, LifecycleOutcome, ResizeRequest};
pub use registry::{ProviderCredentials, ProviderRegistry};
pub use sandbox_cloud::{SANDBOX_CLOUD_PROVIDER_ID, SandboxCloudFixture};
