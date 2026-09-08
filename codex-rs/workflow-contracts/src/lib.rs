//! Git-native workflow contracts for the Codex Universal workflow platform.
//!
//! This crate defines workflows as first-class Git-native software
//! artifacts while keeping semantic authority in the workflow control
//! plane. It is a contracts-only crate: it contains no agent runtime, no
//! workflow engine, no durable storage, no forge client, and no execution
//! implementation. Those surfaces are owned by later Work Orders:
//!
//! - the universal model contract (WO-002) keeps provider specifics out of
//!   these types;
//! - the execution plane and capability/resource registry (WO-005) bind
//!   requirements to Codex-native capabilities;
//! - the workflow control plane (roadmap M4) owns durable instance
//!   lifecycle and legal transitions;
//! - forge adapters (WO-009) implement [`WorkflowForge`] for concrete
//!   hosting forges, starting with GitHub.
//!
//! ## Core model
//!
//! ```text
//! WorkflowRepository  ── forge identity, fork lineage, branches, manifests
//! WorkflowManifest    ── declares one workflow inside a repository
//! WorkflowDefinition  ── pure semantics: IR graph, roles, triggers, deps
//! WorkflowVersion     ── immutable published artifact + integrity rules
//! WorkflowInstance    ── execution record pinned to a version
//! ```
//!
//! ## Invariants held by these contracts
//!
//! - **Execution identity** is the tuple `semantic version + repository +
//!   immutable source revision + definition digest + dependency lock
//!   digest`, digested into a [`WorkflowVersionId`]. Moving branches and
//!   agent actions cannot change it; mutation is detected by
//!   [`WorkflowVersion::verify_integrity`].
//! - **Development refs are never authority.** [`DevelopmentRef`] values
//!   appear only as provenance; execution anchors are
//!   [`ImmutableSourceRevision`] commit SHAs.
//! - **Forge neutrality.** No GitHub-specific type exists here;
//!   [`WorkflowForge`] is the binding boundary.
//! - **Dependency independence.** Subworkflow, skill, plugin, and MCP
//!   dependencies are explicit, separately modeled, and locked
//!   independently via [`DependencyLock`].
//! - **No credentials, no settlement.** Repository identity is canonical
//!   and credential-free by construction
//!   (see [`WorkflowRepositoryId`]); provenance carries attribution,
//!   license, and upgrade policy only.
//! - **External outputs are untrusted.** Triggers and evidence enter as
//!   normalized, digest-bearing references.

#![deny(missing_docs)]

mod definition;
mod dependency;
mod digest;
mod error;
mod evidence;
mod identity;
mod instance;
mod ir;
mod lock;
mod manifest;
mod provenance;
mod repository;
mod revision;
mod role;
mod trigger;
mod version;

pub use definition::WorkflowDefinition;
pub use dependency::CapabilityId;
pub use dependency::CapabilityRequirement;
pub use dependency::McpCapabilityId;
pub use dependency::McpDependency;
pub use dependency::PluginDependency;
pub use dependency::PluginName;
pub use dependency::ResourceRequirement;
pub use dependency::ResourceTypeId;
pub use dependency::SkillDependency;
pub use dependency::SkillName;
pub use dependency::SubworkflowDependency;
pub use dependency::SubworkflowDependencyId;
pub use dependency::SubworkflowVersionPin;
pub use dependency::WorkflowDependencies;
pub use digest::ContentDigest;
pub use error::WorkflowContractError;
pub use evidence::EvidenceKind;
pub use evidence::EvidenceReference;
pub use identity::ForgeKind;
pub use identity::SemanticVersion;
pub use identity::WorkflowDefinitionId;
pub use identity::WorkflowInstanceId;
pub use identity::WorkflowRepositoryId;
pub use instance::WorkflowInstance;
pub use instance::WorkflowInstanceStatus;
pub use ir::CompensationNode;
pub use ir::ConditionId;
pub use ir::ConditionalBranchArm;
pub use ir::ConditionalBranchNode;
pub use ir::HumanGateNode;
pub use ir::IR_FORMAT_VERSION;
pub use ir::IrNodeId;
pub use ir::JoinPolicy;
pub use ir::LoopBound;
pub use ir::LoopNode;
pub use ir::ParallelForkNode;
pub use ir::ParallelJoinNode;
pub use ir::SequenceNode;
pub use ir::StepNode;
pub use ir::SubworkflowNode;
pub use ir::WaitFor;
pub use ir::WaitNode;
pub use ir::WorkflowCondition;
pub use ir::WorkflowIr;
pub use ir::WorkflowIrNode;
pub use lock::DependencyKey;
pub use lock::DependencyLock;
pub use lock::DependencyProvenance;
pub use lock::ResolvedDependency;
pub use lock::ResolvedDependencyIdentity;
pub use manifest::MANIFEST_FORMAT_VERSION;
pub use manifest::RepositoryRelativePath;
pub use manifest::WorkflowManifest;
pub use provenance::Attribution;
pub use provenance::ForkLineage;
pub use provenance::LicenseInfo;
pub use provenance::UpgradePolicy;
pub use provenance::WorkflowProvenance;
pub use repository::ReviewId;
pub use repository::ReviewState;
pub use repository::WorkflowBranchState;
pub use repository::WorkflowForge;
pub use repository::WorkflowRelease;
pub use repository::WorkflowRepository;
pub use repository::WorkflowReview;
pub use revision::DevelopmentRef;
pub use revision::ImmutableSourceRevision;
pub use revision::RevisionSha;
pub use revision::WorkflowVersionId;
pub use role::ApprovalRequirement;
pub use role::RoleId;
pub use role::WorkflowRole;
pub use trigger::TriggerClass;
pub use trigger::TriggerSource;
pub use trigger::WorkflowTrigger;
pub use version::ExecutionVersionIdentity;
pub use version::WorkflowVersion;
