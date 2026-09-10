//! Control-plane and host seams for the workflow trigger plane.
//!
//! WO-011 owns no durable state: every durable or host-owned concern
//! crosses one of these traits, and the WO-010 application ports
//! (`WorkflowVersionStore`, the lifecycle itself) are composed by
//! reference at operation time. The traits are deliberately synchronous
//! record seams — hosts bridge to their own concurrency as needed, and
//! the in-memory implementations live in the re-exported `InMemory*` types.
//!
//! The seams keep the frozen authority boundaries intact:
//!
//! - [`TriggerLedger`] is the **idempotency and audit authority**: it
//!   deduplicates fires per `(workflow, event key)`, allocates
//!   control-plane event ids, records every settlement (including
//!   duplicate no-ops), and tracks paused instances awaiting a trigger.
//! - [`InstanceControl`] is the resume **seam to the durable control
//!   plane**: only the host's control plane performs the legal
//!   `Paused -> Running` transition.
//! - [`ResourceAuthorizer`] is the authorization seam where the host
//!   binds its real connector/account authorization (Codex connectors,
//!   connected accounts) — decisions are data, never credentials.
//! - [`PackageCatalog`] is the discovery transport seam (forge search,
//!   releases), reusing the WO-009 discovery queries underneath.
//! - [`InstallationStore`] persists installed configurations and rebind
//!   audits.
//! - [`ScheduleClock`] supplies time to the scheduler.

use codex_workflow_contracts::IrNodeId;
use codex_workflow_contracts::TriggerClass;
use codex_workflow_contracts::TriggerSource;
use codex_workflow_contracts::WorkflowDefinitionId;
use codex_workflow_contracts::WorkflowInstanceId;
use codex_workflow_contracts::WorkflowRepository;
use codex_workflow_contracts::WorkflowVersion;
use codex_workflow_contracts::WorkflowVersionId;
use codex_workflow_forge::RepositoryQuery;
use serde::Deserialize;
use serde::Serialize;

use crate::IncomingTrigger;
use crate::InstalledConfiguration;
use crate::RebindRecord;
use crate::TriggerDiagnostic;
use crate::WorkflowTriggerError;
use codex_execution_contracts::ResourceBinding;

/// The result of the ledger's idempotent acceptance check.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub enum TriggerAcceptance {
    /// First sight of this event key: the control plane allocated an
    /// event id and the fire may proceed.
    Accepted {
        /// The allocated control-plane event identity.
        event_id: String,
    },
    /// The event key was already accepted for this workflow: the fire is
    /// a recorded no-op.
    Duplicate {
        /// The event id of the original acceptance.
        event_id: String,
    },
}

/// How one instance settled after a fire.
///
/// This is the durable mirror of the WO-010 run terminal, plus the
/// trigger-await correlation derived from the IR.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub enum InstanceSettlement {
    /// The run completed successfully.
    Completed,
    /// The run paused at a wait or human-gate node.
    Paused {
        /// The node the run paused at.
        node: IrNodeId,
        /// The trigger class the node awaits, when it waits for a
        /// trigger (`WaitFor::Trigger`).
        awaiting: Option<TriggerClass>,
    },
    /// The run (or its instantiation) settled with failure.
    Failed {
        /// Why the run failed.
        reason: String,
    },
}

/// The recorded outcome of one trigger fire.
///
/// Every fire — eligible, ineligible, failed, or duplicate — settles in
/// the ledger with exactly one outcome, which is what makes trigger
/// handling durable and auditable.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub enum FireOutcome {
    /// The fire was accepted and started one instance, which settled with
    /// this status.
    Started {
        /// The instance the fire started.
        instance: WorkflowInstanceId,
        /// How the instance settled.
        status: InstanceSettlement,
    },
    /// The fire was a duplicate of an already-accepted event key: a
    /// recorded no-op that performed no transition.
    Duplicate,
    /// The fire was accepted but not eligible: diagnostics were recorded
    /// and no instance was created.
    NotEligible {
        /// Why the fire was not eligible.
        diagnostics: Vec<TriggerDiagnostic>,
    },
    /// The fire was accepted and instantiation was attempted, but the
    /// instantiation gate (integrity, readiness, policy, approvals)
    /// refused it: the control-plane instance record settled `Failed`.
    InstantiationFailed {
        /// Why instantiation failed.
        diagnostics: Vec<TriggerDiagnostic>,
    },
    /// The fire resumed paused instances awaiting its trigger class.
    Resumed {
        /// The instances that were resumed.
        instances: Vec<WorkflowInstanceId>,
    },
}

/// A resume correlation target: an instance paused at a wait node that
/// awaits a trigger class.
///
/// Await records are derived from settled run outcomes (a fire that
/// paused at a `WaitFor::Trigger` node) and resolved when a matching
/// trigger fires; they are pure correlation data with no authority over
/// instance state.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AwaitRecord {
    /// The workflow the paused instance belongs to.
    pub workflow: WorkflowDefinitionId,
    /// The paused instance.
    pub instance: WorkflowInstanceId,
    /// The node the instance paused at.
    pub node: IrNodeId,
    /// The trigger class the wait node awaits.
    pub class: TriggerClass,
}

/// The durable trigger idempotency and audit ledger: the control-plane
/// seam every fire passes through.
///
/// Implementations (the host's control plane) must make acceptance
/// durable: after a crash, an event key that was accepted once must keep
/// answering [`TriggerAcceptance::Duplicate`] forever, so the same event
/// can never drive a second instance transition. Settlements and await
/// registrations are append-only audit records.
pub trait TriggerLedger: Send + Sync {
    /// Deduplicates the event for `workflow` by its key.
    ///
    /// First sight allocates and records a control-plane event id;
    /// subsequent sights are recorded duplicates carrying the original
    /// id. `now_unix_ms` timestamps the acceptance for audit.
    fn accept(
        &mut self,
        workflow: &WorkflowDefinitionId,
        envelope: &IncomingTrigger,
        now_unix_ms: u64,
    ) -> Result<TriggerAcceptance, WorkflowTriggerError>;

    /// Records the outcome of one fire (audit + await tracking).
    ///
    /// - `Started` with a paused status awaiting a trigger registers the
    ///   await correlation;
    /// - `Resumed` clears the await registrations of the resumed
    ///   instances;
    /// - every other outcome is pure audit.
    ///
    /// The event id must be one this ledger allocated through
    /// [`TriggerLedger::accept`].
    fn settle(&mut self, event_id: &str, outcome: FireOutcome) -> Result<(), WorkflowTriggerError>;

    /// The instances of `workflow` paused awaiting `class`, ordered by
    /// instance identity.
    fn awaiting(
        &self,
        workflow: &WorkflowDefinitionId,
        class: TriggerClass,
    ) -> Result<Vec<AwaitRecord>, WorkflowTriggerError>;
}

/// A request to authorize one resource binding for an installed
/// workflow.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResourceAuthorizationRequest {
    /// The workflow the binding serves.
    pub workflow: WorkflowDefinitionId,
    /// The resource binding to authorize.
    pub binding: ResourceBinding,
}

/// The decision of the authorization plane for one resource binding.
///
/// The decision is data: a denial is a recorded outcome, not an error.
/// The source labels the authorizing mechanism (for example
/// `codex-connectors:composio`) for audit; credentials never appear.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResourceAuthorization {
    /// Whether the binding is authorized.
    pub authorized: bool,
    /// Attribution label of the authorizing mechanism.
    pub source: String,
    /// Why the decision was reached, when applicable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// The authorization seam for resource and account bindings.
///
/// The host implementation binds this to its real connector/plugin/skill
/// authorization surfaces (Codex connectors and connected accounts);
/// [`crate::InMemoryResourceAuthorizer`] is the explicit-allow,
/// default-deny test double.
pub trait ResourceAuthorizer: Send + Sync {
    /// Decides whether `request`'s resource binding is authorized for
    /// the workflow.
    fn authorize(
        &mut self,
        request: &ResourceAuthorizationRequest,
    ) -> Result<ResourceAuthorization, WorkflowTriggerError>;
}

/// The discovery transport seam: forge search and released versions.
///
/// Transport (a forge's search API, release listing) is host-owned, the
/// same boundary WO-009 established; the plane applies the frozen
/// repository discovery queries over the snapshots this seam returns.
pub trait PackageCatalog: Send + Sync {
    /// Searches repositories matching `query`.
    fn search_repositories(
        &self,
        query: &RepositoryQuery,
    ) -> Result<Vec<WorkflowRepository>, WorkflowTriggerError>;

    /// The published versions of `workflow`, newest-first is not
    /// required; ordering is the catalog's choice.
    fn published_versions(
        &self,
        workflow: &WorkflowDefinitionId,
    ) -> Result<Vec<codex_workflow_forge::PublishedVersionRef>, WorkflowTriggerError>;

    /// Fetches the full version record for `version`, when the catalog
    /// holds it.
    fn fetch_version(
        &self,
        version: &WorkflowVersionId,
    ) -> Result<Option<WorkflowVersion>, WorkflowTriggerError>;
}

/// Durable storage for installed configurations and rebind audits.
///
/// The Codex application persists these records through the workflow
/// control plane; the in-memory implementation serves tests and seeding.
pub trait InstallationStore: Send + Sync {
    /// Upserts the configuration of one workflow.
    fn save(&mut self, configuration: InstalledConfiguration) -> Result<(), WorkflowTriggerError>;
    /// Loads the configuration of `workflow`, when installed.
    fn load(
        &self,
        workflow: &WorkflowDefinitionId,
    ) -> Result<Option<InstalledConfiguration>, WorkflowTriggerError>;
    /// Lists every installed configuration, ordered by workflow
    /// identity.
    fn list(&self) -> Result<Vec<InstalledConfiguration>, WorkflowTriggerError>;
    /// Appends one rebind audit record.
    fn append_audit(&mut self, record: RebindRecord) -> Result<(), WorkflowTriggerError>;
}

/// One directive to resume a paused instance, issued when a trigger
/// matching the instance's awaited class fires.
///
/// The directive is a request to the durable control plane, which alone
/// performs the legal `Paused -> Running` transition (and, in the host
/// application, continues the walk from its own resumable run state).
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResumeDirective {
    /// The instance to resume.
    pub instance: WorkflowInstanceId,
    /// The node the instance is paused at.
    pub node: IrNodeId,
    /// The trigger whose firing satisfied the wait.
    pub trigger: TriggerSource,
}

/// The resume seam into the durable control plane.
///
/// Implementations must reject anything but a `Paused` instance and must
/// never mutate workflow semantics; the transition they perform is the
/// documented control-plane transition `Paused -> Running`.
pub trait InstanceControl: Send + Sync {
    /// Resumes one paused instance, returning the updated record.
    fn resume(
        &mut self,
        directive: &ResumeDirective,
    ) -> Result<codex_workflow_contracts::WorkflowInstance, WorkflowTriggerError>;
}

/// The clock seam supplying scheduler time.
///
/// Hosts inject their real clock; tests inject [`crate::FixedClock`].
pub trait ScheduleClock: Send + Sync {
    /// The current instant in unix milliseconds.
    fn now_unix_ms(&self) -> u64;
}
