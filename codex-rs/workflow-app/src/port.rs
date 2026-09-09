//! Control-plane and host seams for the workflow application layer.
//!
//! WO-010 owns no durable state: every durable or host-owned concern
//! crosses one of these traits. The Codex application implements them
//! against its real control-plane surfaces (thread-store-backed instance
//! records, rollout-trace-backed evidence, the native approval plane, the
//! Codex agent loop that proposes actions); [`crate::memory`] provides
//! in-memory implementations for tests and seeding.
//!
//! The traits are deliberately synchronous: they describe record seams, not
//! runtimes. Hosts bridge to their own concurrency as needed.

use codex_execution_contracts::Action;
use codex_execution_contracts::SelectedBinding;
use codex_workflow_contracts::CapabilityRequirement;
use codex_workflow_contracts::EvidenceKind;
use codex_workflow_contracts::EvidenceReference;
use codex_workflow_contracts::WorkflowInstance;
use codex_workflow_contracts::WorkflowInstanceId;
use codex_workflow_contracts::WorkflowVersion;
use codex_workflow_contracts::WorkflowVersionId;

use crate::WorkflowAppError;
use crate::WorkflowEvent;
use crate::approval::ApprovalRequest;

/// Durable storage for immutable workflow version records.
///
/// The application publishes sealed versions here (typically materialized
/// from a forge release plus its installed record) and the lifecycle loads
/// them by identity, re-verifying integrity on every load.
pub trait WorkflowVersionStore: Send + Sync {
    /// Stores a sealed workflow version, keyed by its identity.
    fn publish(&mut self, version: WorkflowVersion) -> Result<WorkflowVersionId, WorkflowAppError>;
    /// Loads the version record for `version`, when present.
    fn load(
        &self,
        version: &WorkflowVersionId,
    ) -> Result<Option<WorkflowVersion>, WorkflowAppError>;
}

/// Durable storage for workflow instance records.
///
/// Instance lifecycle is control-plane authority; this seam is how the
/// application's store observes instance creation and status changes. The
/// lifecycle never mutates instance semantics, only drives the documented
/// contract statuses.
pub trait WorkflowInstanceStore: Send + Sync {
    /// Creates a new instance record. Duplicate identities are rejected.
    fn create(&mut self, instance: WorkflowInstance) -> Result<(), WorkflowAppError>;
    /// Saves an updated instance record (status changes and appended
    /// evidence).
    fn save(&mut self, instance: WorkflowInstance) -> Result<(), WorkflowAppError>;
    /// Loads the instance record for `instance`, when present.
    fn load(
        &self,
        instance: &WorkflowInstanceId,
    ) -> Result<Option<WorkflowInstance>, WorkflowAppError>;
}

/// The evidence plane seam.
///
/// Payloads are stored by the evidence plane and referenced from workflow
/// instances by kind, locator, and digest. The in-flight run stores
/// normalized observations, recovery records, binding decisions, and
/// approval records here; nothing in this crate keeps payloads.
pub trait EvidenceStore: Send + Sync {
    /// Stores one evidence payload (the canonical JSON serialization of a
    /// contract record) and returns its reference. The store allocates the
    /// locator and digest, mirroring how the evidence plane mints
    /// references from record content.
    fn store(
        &mut self,
        kind: EvidenceKind,
        payload: &serde_json::Value,
    ) -> Result<EvidenceReference, WorkflowAppError>;
    /// Promotes an adapter-supplied canonical payload digest into a
    /// contract reference. Adapters that digest their own canonical bytes
    /// (for example the WO-006 browser evidence records) hand the digest
    /// hex over here so both planes agree on digest identity.
    fn promote(
        &mut self,
        kind: EvidenceKind,
        locator: &str,
        digest_hex: &str,
    ) -> Result<EvidenceReference, WorkflowAppError>;
}

/// The approval plane seam.
///
/// Bindings reach `AUTHORIZED` only through an
/// [`codex_execution_contracts::AuthorizationGrant`] resting on recorded
/// approval evidence. This seam asks the host's approval machinery (Codex
/// approvals remain the authority) to record a decision for one binding.
/// A denial surfaces as [`WorkflowAppError::ApprovalDenied`].
pub trait ApprovalSource: Send + Sync {
    /// Records an approval decision for the requested binding and returns
    /// the approval evidence reference.
    fn approve(&mut self, request: &ApprovalRequest)
    -> Result<EvidenceReference, WorkflowAppError>;
}

/// The action proposal seam.
///
/// Workflow steps declare *what* capability they need; the execution plane
/// decides *which* binding serves it. *What concrete action to perform* is
/// proposed by the Codex agent/model loop — the existing runtime — through
/// this seam. The application layer never invents actions; it validates
/// the proposed envelope against the execution contracts and dispatches
/// one action at a time.
pub trait StepActionSource: Send + Sync {
    /// Proposes the action to dispatch for one step capability
    /// requirement, routed to the selected binding.
    fn action_for(&mut self, request: &ActionRequest) -> Result<Action, WorkflowAppError>;
}

/// The event sink: an append-only observer of lifecycle events.
///
/// Listeners never gain authority over lifecycle state; they observe.
pub trait EventSink: Send + Sync {
    /// Records one lifecycle event.
    fn record(&mut self, event: WorkflowEvent);
}

/// A request for one action proposal.
#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ActionRequest {
    /// The instance the action executes for.
    pub instance: WorkflowInstanceId,
    /// The step node being executed.
    pub node: codex_workflow_contracts::IrNodeId,
    /// The capability requirement being served.
    pub requirement: CapabilityRequirement,
    /// The binding the requirement resolved to.
    pub selected: SelectedBinding,
}
