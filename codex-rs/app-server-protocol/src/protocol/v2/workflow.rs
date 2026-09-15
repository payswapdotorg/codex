//! Experimental workflow teaching and control-plane APIs (RWO-001).
//!
//! These contracts mount the workflow family behind user-facing surfaces:
//! a normal person can teach a workflow (DEMONSTRATE, INSTRUCT, or HYBRID),
//! compile it, review it, approve it, and publish an immutable version, fork
//! a published version into a new immutable release that carries its
//! lineage (RWO-005), propose an improvement candidate from recorded
//! execution evidence and publish it only after validation and an explicit
//! approval decision (RWO-006), then operate durable instances. Every
//! response carries the Workflow Identity fields that apply to it: workflow
//! definition id, semantic version, version digest, and dependency-lock
//! identity where they exist.
//!
//! All methods are experimental and require the `experimentalApi` capability.
//! Identifiers and digests are opaque strings at this boundary: session and
//! candidate ids are control-plane allocated, version ids and digests use
//! the frozen `sha256:<hex>` form of the workflow contracts.

use crate::JsonSchema;
use crate::TS;
use serde::Deserialize;
use serde::Serialize;

/// How a workflow is being taught.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase", export_to = "v2/")]
pub enum WorkflowTeachMode {
    /// The user demonstrates the workflow by performing it.
    Demonstrate,
    /// The user describes the workflow in instruction statements.
    Instruct,
    /// Demonstrated steps and instructed steps are interleaved.
    Hybrid,
}

/// Whether a teaching session still accepts records.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase", export_to = "v2/")]
pub enum WorkflowTeachSessionStatus {
    /// The session accepts records.
    Open,
    /// The session is closed for recording and ready to compile.
    Closed,
}

/// The kind of one demonstration event.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase", export_to = "v2/")]
pub enum WorkflowDemonstrationKind {
    /// Observed environment state during a demonstration.
    Observation,
    /// A concrete action performed during a demonstration.
    Action,
    /// The observed outcome of a demonstration action.
    Result,
    /// A recovery from failure observed during a demonstration.
    Recovery,
}

/// Lifecycle state of a compiled workflow candidate.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase", export_to = "v2/")]
pub enum WorkflowCandidateStatus {
    /// Built but not yet validated.
    Compiled,
    /// Validation passed with no error-severity findings.
    Validated,
    /// An approval decision has been recorded for the current epoch.
    Approved,
    /// Finalized into an approved, publication-ready definition.
    PublicationReady,
}

/// The decision conveyed by a candidate approval.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase", export_to = "v2/")]
pub enum WorkflowApprovalDecision {
    /// The reviewer approves publication of exactly this content.
    Approved,
    /// The reviewer rejects the candidate; it stays unapproved.
    Rejected,
}

/// Severity of one validation finding.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase", export_to = "v2/")]
pub enum WorkflowValidationSeverity {
    /// Blocks approval and publication.
    Error,
    /// Surfaced for review only; does not block.
    Warning,
}

/// Terminal outcome of one deterministic simulation.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase", export_to = "v2/")]
pub enum WorkflowSimulationOutcomeKind {
    /// The forward path reached a terminal state with no pending work.
    Completed,
    /// The path paused at a wait or human-gate node.
    Paused,
    /// The path reached a point with no deterministic continuation.
    Indeterminate,
    /// The simulation aborted before reaching a terminal state.
    Aborted,
}

/// Where a compiled step came from.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase", export_to = "v2/")]
pub enum WorkflowStepOrigin {
    /// Derived from a demonstrated action.
    Observed,
    /// Derived from an instruction statement.
    Instructed,
}

/// Lifecycle status of a workflow instance.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase", export_to = "v2/")]
pub enum WorkflowInstanceStatus {
    /// Allocated but not yet started.
    Pending,
    /// Executing.
    Running,
    /// Paused, for example at a human gate.
    Paused,
    /// Completed successfully.
    Succeeded,
    /// Completed with failure.
    Failed,
    /// Cancelled before completion.
    Cancelled,
}

/// Normalized trigger class that may start or advance a workflow.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase", export_to = "v2/")]
pub enum WorkflowTriggerClass {
    /// Direct user action.
    User,
    /// Time-based schedule.
    Schedule,
    /// Inbound webhook.
    Webhook,
    /// Event raised by a connector.
    ConnectorEvent,
    /// Event raised in a browser session.
    BrowserEvent,
    /// Event raised on a computer or desktop session.
    ComputerEvent,
    /// Event raised by another workflow.
    WorkflowEvent,
    /// Event raised by a human participant.
    HumanEvent,
}

/// Why one workflow instance run stopped.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase", export_to = "v2/")]
pub enum WorkflowRunTerminalKind {
    /// Every step completed and the walk reached a terminal state.
    Completed,
    /// The run paused at a wait or human-gate node.
    Paused,
    /// The run settled with failure.
    Failed,
}

/// A digest-bearing pointer to externally stored teaching evidence.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export_to = "v2/")]
pub struct WorkflowTeachEvidenceInput {
    /// Short human-facing label for the evidence.
    pub label: String,
    /// Where the evidence payload is stored; never dereferenced here.
    pub locator: String,
    /// SHA-256 digest of the evidence payload, 64 lowercase hex characters.
    pub sha256: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export_to = "v2/")]
pub struct WorkflowTeachStartParams {
    /// How the workflow will be taught.
    pub mode: WorkflowTeachMode,
    /// Name of the workflow being taught; becomes its definition id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub name: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct WorkflowTeachStartResponse {
    /// Control-plane allocated teaching session id.
    pub session_id: String,
    /// The name the workflow will be published under.
    pub name: String,
    /// The teaching mode the session was opened in.
    pub mode: WorkflowTeachMode,
    /// Whether the session still accepts records.
    pub status: WorkflowTeachSessionStatus,
    /// Number of records in the trajectory so far.
    pub record_count: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export_to = "v2/")]
pub struct WorkflowTeachInstructParams {
    /// The teaching session to record into.
    pub session_id: String,
    /// One instruction statement.
    pub text: String,
    /// Evidence references attached to the statement.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub evidence: Vec<WorkflowTeachEvidenceInput>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export_to = "v2/")]
pub struct WorkflowTeachDemonstrateParams {
    /// The teaching session to record into.
    pub session_id: String,
    /// The kind of demonstration event.
    pub kind: WorkflowDemonstrationKind,
    /// Bounded human-facing text describing the event.
    pub text: String,
    /// Evidence references attached to the event.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub evidence: Vec<WorkflowTeachEvidenceInput>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct WorkflowTeachRecordResponse {
    /// The teaching session that was recorded into.
    pub session_id: String,
    /// The teaching mode of the session.
    pub mode: WorkflowTeachMode,
    /// Whether the session still accepts records.
    pub status: WorkflowTeachSessionStatus,
    /// The sequence number the recorded event received.
    pub sequence: u64,
    /// Number of records in the trajectory after recording.
    pub record_count: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export_to = "v2/")]
pub struct WorkflowTeachReconcileParams {
    /// The teaching session to close and reconcile.
    pub session_id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct WorkflowTeachReconcileResponse {
    /// The teaching session that was reconciled.
    pub session_id: String,
    /// The teaching mode of the session.
    pub mode: WorkflowTeachMode,
    /// The session is closed and its trajectory is frozen for compilation.
    pub status: WorkflowTeachSessionStatus,
    /// Number of demonstration-origin records in the trajectory.
    pub demonstration_records: u64,
    /// Number of instruction-origin records in the trajectory.
    pub instruction_records: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export_to = "v2/")]
pub struct WorkflowCompileParams {
    /// The closed teaching session to compile.
    pub session_id: String,
}

/// One structural or policy finding about a compiled candidate.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct WorkflowValidationFinding {
    /// How severe the finding is.
    pub severity: WorkflowValidationSeverity,
    /// A stable diagnostic code.
    pub code: String,
    /// Human-facing explanation.
    pub message: String,
}

/// Summary of one validation pass over a candidate.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct WorkflowValidationSummary {
    /// Whether the pass produced no error-severity findings.
    pub clean: bool,
    /// Number of error-severity findings.
    pub error_count: u64,
    /// Number of warning-severity findings.
    pub warning_count: u64,
    /// All findings in deterministic order.
    pub findings: Vec<WorkflowValidationFinding>,
}

/// Compact view of one deterministic simulation of a candidate.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct WorkflowSimulationSummary {
    /// Terminal outcome of the simulation.
    pub outcome: WorkflowSimulationOutcomeKind,
    /// The node the simulation paused or stopped at, when one exists.
    pub node: Option<String>,
    /// Why the simulation paused, became indeterminate, or aborted.
    pub reason: Option<String>,
    /// The node ids the simulation visited, in order.
    pub path: Vec<String>,
    /// How many nodes the simulation visited.
    pub steps_taken: u64,
    /// The candidate content epoch the simulation ran against.
    pub epoch: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct WorkflowCompileResponse {
    /// Control-plane allocated candidate id.
    pub candidate_id: String,
    /// Lifecycle state of the candidate after compilation.
    pub status: WorkflowCandidateStatus,
    /// The teaching mode the candidate was compiled from.
    pub origin: WorkflowTeachMode,
    /// The candidate content epoch at compilation time.
    pub epoch: u64,
    /// Number of compiled step nodes.
    pub step_count: u64,
    /// Validation summary of the compilation pass.
    pub validation: WorkflowValidationSummary,
    /// Simulation summary of the compilation pass.
    pub simulation: WorkflowSimulationSummary,
}

/// One compiled step of a candidate, as presented for review.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct WorkflowStepReview {
    /// The step node id, for example step-001.
    pub node_id: String,
    /// Where the step came from.
    pub origin: WorkflowStepOrigin,
    /// Human-facing intent of the step, when recorded.
    pub description: Option<String>,
    /// Number of evidence references attached to the step.
    pub evidence_count: u64,
}

/// A lexical capability hint derived from one step's intent.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct WorkflowCapabilityInference {
    /// The step the hint applies to.
    pub node_id: String,
    /// A neutral hint label, for example verb:navigate.
    pub hint: String,
    /// Why the hint was produced.
    pub rationale: String,
}

/// A binding proposal produced by the compiler; binding is the execution plane's decision.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct WorkflowBindingProposal {
    /// The step the proposal addresses.
    pub node_id: String,
    /// The plane the proposal addresses, for example capability.
    pub kind: String,
    /// The unresolved proposed reference.
    pub reference: String,
    /// Why the proposal was produced.
    pub rationale: String,
    /// Whether resolving the proposal requires approval.
    pub requires_approval: bool,
}

/// An unresolved trigger intent surfaced for control-plane review.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct WorkflowTriggerIntent {
    /// A coarse class hint, for example unresolved.
    pub class_hint: String,
    /// The taught trigger description.
    pub description: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export_to = "v2/")]
pub struct WorkflowReviewParams {
    /// The candidate to review.
    pub candidate_id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct WorkflowReviewResponse {
    /// The candidate under review.
    pub candidate_id: String,
    /// Lifecycle state of the candidate.
    pub status: WorkflowCandidateStatus,
    /// The teaching mode the candidate was compiled from.
    pub origin: WorkflowTeachMode,
    /// The candidate content epoch.
    pub epoch: u64,
    /// Human-facing description of the compiled workflow, when recorded.
    pub description: Option<String>,
    /// The compiled steps in walk order.
    pub steps: Vec<WorkflowStepReview>,
    /// Capability hints produced by the compiler.
    pub capability_inferences: Vec<WorkflowCapabilityInference>,
    /// Binding proposals produced by the compiler.
    pub binding_proposals: Vec<WorkflowBindingProposal>,
    /// Trigger intents produced by the compiler.
    pub trigger_intents: Vec<WorkflowTriggerIntent>,
    /// Validation summary of the last pass.
    pub validation: WorkflowValidationSummary,
    /// Simulation summary of the last pass.
    pub simulation: WorkflowSimulationSummary,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export_to = "v2/")]
pub struct WorkflowApproveParams {
    /// The candidate to decide on.
    pub candidate_id: String,
    /// Approver label recorded with the decision.
    pub approver: String,
    /// External review reference recorded with the decision.
    pub reference: String,
    /// The decision.
    pub decision: WorkflowApprovalDecision,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct WorkflowApproveResponse {
    /// The candidate that was decided on.
    pub candidate_id: String,
    /// Lifecycle state of the candidate after the decision.
    pub status: WorkflowCandidateStatus,
    /// The candidate content epoch the decision covers.
    pub epoch: u64,
    /// The decision that was recorded.
    pub decision: WorkflowApprovalDecision,
    /// The approver label that was recorded.
    pub approver: String,
    /// The review reference that was recorded.
    pub reference: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export_to = "v2/")]
pub struct WorkflowPublishParams {
    /// The approved candidate to publish.
    pub candidate_id: String,
    /// Canonical, credential-free repository identity to publish from.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub repository: Option<String>,
    /// Full commit SHA (40 or 64 lowercase hex) anchoring the version.
    pub commit_sha: String,
    /// Semantic version of this revision; defaults to 1.0.0.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub semantic_version: Option<String>,
}

/// Audit record of binding resolution during publication.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct WorkflowBindingResolution {
    /// Digest of the approved definition the resolution started from.
    pub approved_digest: String,
    /// Digest of the executable definition the resolution produced.
    pub executable_digest: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct WorkflowPublishResponse {
    /// The workflow definition id of the published version.
    pub workflow: String,
    /// The immutable version identity digest.
    pub version_id: String,
    /// The semantic version of the published revision.
    pub semantic_version: String,
    /// Digest of the frozen workflow definition.
    pub definition_digest: String,
    /// Digest of the resolved dependency lock.
    pub dependency_lock_digest: String,
    /// Canonical repository identity the version was published from.
    pub repository: String,
    /// The immutable commit SHA the version is anchored at.
    pub commit_sha: String,
    /// The binding-resolution audit record.
    pub binding_resolution: WorkflowBindingResolution,
}

/// One attribution record a fork release carries forward from its
/// upstream (RWO-005).
///
/// Attribution preserves authorship across the fork boundary: the fork
/// request must carry at least one record, and the derived release's
/// lineage renders them. Verifying attribution CONTENT is RWO-016's
/// scope, not this surface's.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export_to = "v2/")]
pub struct WorkflowForkAttribution {
    /// The attributed author or contributor name.
    pub name: String,
    /// A contact reference, when one is recorded.
    pub contact: Option<String>,
}

/// The lineage record a fork release pins to its upstream (RWO-005):
/// the forked-from version identity plus the upstream digests.
///
/// Every field is copied from the engine-sealed lineage record of the
/// derived release, so inspection of the fork provably shows where it
/// came from: which immutable version, at which digests, from which
/// repository, at which commit.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export_to = "v2/")]
pub struct WorkflowForkLineage {
    /// The workflow definition id of the forked-from upstream release.
    pub workflow: String,
    /// The immutable version identity digest of the upstream release.
    pub version_id: String,
    /// The semantic version of the upstream release.
    pub semantic_version: String,
    /// The canonical repository identity of the upstream release.
    pub repository: String,
    /// Digest of the upstream's frozen workflow definition.
    pub definition_digest: String,
    /// Digest of the upstream's resolved dependency lock.
    pub dependency_lock_digest: String,
    /// The immutable commit the upstream release was anchored at.
    pub commit_sha: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export_to = "v2/")]
pub struct WorkflowForkParams {
    /// The published workflow version to fork, in sha256-hex form.
    pub version_id: String,
    /// The fork's own repository identity (the `--as` identity); must
    /// differ from the upstream's.
    pub fork_repository: String,
    /// Semantic version of the derived release; defaults to the
    /// upstream's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub semantic_version: Option<String>,
    /// Full commit SHA (40 or 64 lowercase hex) the fork stands at;
    /// defaults to the upstream's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub commit_sha: Option<String>,
    /// The owning principal of the fork (opaque, credential-free).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub owner: Option<String>,
    /// SPDX-style license identifier for the derived release.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub license: Option<String>,
    /// Attribution carried forward from the upstream. Required non-empty
    /// in spirit: the engine refuses a fork whose carried attribution is
    /// empty ("a fork must carry upstream attribution").
    pub attribution: Vec<WorkflowForkAttribution>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export_to = "v2/")]
pub struct WorkflowForkResponse {
    /// The workflow definition id of the fork release (inherited from
    /// the upstream's frozen definition).
    pub workflow: String,
    /// The immutable version identity digest of the fork release.
    pub version_id: String,
    /// The semantic version of the fork release.
    pub semantic_version: String,
    /// Digest of the frozen workflow definition (inherited from the
    /// upstream).
    pub definition_digest: String,
    /// Digest of the resolved dependency lock (inherited from the
    /// upstream).
    pub dependency_lock_digest: String,
    /// The fork's repository identity.
    pub repository: String,
    /// The immutable commit the fork stands at.
    pub commit_sha: String,
    /// The lineage record pinning the upstream release.
    pub lineage: WorkflowForkLineage,
    /// The attribution the fork release carries forward from its
    /// upstream, as recorded in its sealed lineage.
    pub attribution: Vec<WorkflowForkAttribution>,
}

/// The category of one proposed improvement change (RWO-006).
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase", export_to = "v2/")]
pub enum WorkflowChangeKind {
    /// A delta to the workflow definition.
    DefinitionDelta,
    /// A change to the declared capability bindings of step nodes.
    CapabilityBinding,
    /// An adjustment to the binding/recovery policy.
    RecoveryPolicy,
    /// A change to the resolved dependency lock.
    DependencyChoice,
    /// A tuning of the trigger schedule.
    ScheduleTuning,
}

/// The content-addressed identity of one recorded execution run an
/// improvement candidate cites (RWO-006).
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export_to = "v2/")]
pub struct WorkflowRunProvenance {
    /// The immutable version the cited run pinned.
    pub version_id: String,
    /// The semantic fingerprint digest of the run record.
    pub fingerprint: String,
    /// The settled instance status of the run.
    pub status: WorkflowInstanceStatus,
}

/// The execution evidence summary attached to an improvement candidate
/// (RWO-006): the evidence references the candidate cites plus the
/// content-addressed identities of the recorded runs it was derived from.
///
/// This is the minimal evidence surface of the improvement lifecycle: the
/// dedicated monitoring surface is a later work order's scope.
#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export_to = "v2/")]
pub struct WorkflowEvidenceSummary {
    /// The execution evidence references backing the proposal.
    pub references: Vec<WorkflowEvidenceReference>,
    /// The recorded runs the proposal was derived from.
    pub runs: Vec<WorkflowRunProvenance>,
}

/// One proposed improvement candidate derived from recorded execution
/// evidence (RWO-006): what it evolves, what it proposes, why, and the
/// evidence it cites.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export_to = "v2/")]
pub struct WorkflowImprovementCandidate {
    /// The candidate's control-plane identity.
    pub candidate_id: String,
    /// The workflow the candidate evolves.
    pub workflow: String,
    /// The immutable published version the candidate evolves.
    pub incumbent_version_id: String,
    /// The category of the proposed change.
    pub change_kind: WorkflowChangeKind,
    /// Why the candidate proposes the change (bounded, derived text).
    pub rationale: String,
    /// The execution evidence the candidate cites.
    pub evidence: WorkflowEvidenceSummary,
}

/// One stage of the improvement validation pipeline (RWO-006).
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase", export_to = "v2/")]
pub enum WorkflowValidationStageName {
    /// The deterministic replay of incumbent and successor.
    Replay,
    /// The differential comparison of the two replay records.
    Differential,
    /// The authorization, resource, and compatibility checks.
    Policy,
}

/// The outcome of one validation stage (RWO-006).
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export_to = "v2/")]
pub struct WorkflowValidationStage {
    /// The stage.
    pub stage: WorkflowValidationStageName,
    /// Whether the stage passed.
    pub passed: bool,
}

/// An explicit approval decision on an improvement candidate (RWO-006):
/// the governed gate publication refuses to cross without.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase", export_to = "v2/")]
pub enum WorkflowImprovementDecision {
    /// Approve the candidate for publication.
    Approved,
    /// Reject the candidate.
    Rejected,
}

/// The governed lineage record of one published improvement (RWO-006):
/// predecessor to successor with the full decision trail.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export_to = "v2/")]
pub struct WorkflowImprovementLineage {
    /// The workflow that evolved.
    pub workflow: String,
    /// The immutable version identity of the predecessor.
    pub predecessor_version_id: String,
    /// The semantic version of the predecessor.
    pub predecessor_semantic_version: String,
    /// The immutable version identity of the successor.
    pub successor_version_id: String,
    /// The semantic version of the successor.
    pub successor_semantic_version: String,
    /// The candidate that proposed the change.
    pub candidate_id: String,
    /// The digest of the validation report that gated the succession.
    pub validation_digest: String,
    /// The validation stage outcomes.
    pub validation_stages: Vec<WorkflowValidationStage>,
    /// The principal that approved the succession.
    pub approver: String,
    /// The release tag the successor was published under.
    pub release_tag: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export_to = "v2/")]
pub struct WorkflowImproveProposeParams {
    /// The published workflow version to improve, in sha256-hex form.
    pub version_id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export_to = "v2/")]
pub struct WorkflowImproveProposeResponse {
    /// The workflow the candidates evolve.
    pub workflow: String,
    /// The immutable version the candidates were derived from.
    pub incumbent_version_id: String,
    /// The semantic version of the incumbent.
    pub incumbent_semantic_version: String,
    /// The execution evidence recorded for the proposal.
    pub evidence: WorkflowEvidenceSummary,
    /// The improvement candidates the evidence supports.
    pub candidates: Vec<WorkflowImprovementCandidate>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export_to = "v2/")]
pub struct WorkflowImproveValidateParams {
    /// The improvement candidate to validate.
    pub candidate_id: String,
    /// The semantic version of the proposed successor.
    pub successor_version: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export_to = "v2/")]
pub struct WorkflowImproveValidateResponse {
    /// The validated candidate.
    pub candidate_id: String,
    /// The workflow the candidate evolves.
    pub workflow: String,
    /// The semantic version validated for the successor.
    pub successor_version: String,
    /// Whether every validation gate passed explicitly.
    pub passed: bool,
    /// The validation stage outcomes, in pipeline order.
    pub stages: Vec<WorkflowValidationStage>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export_to = "v2/")]
pub struct WorkflowImproveApproveParams {
    /// The validated candidate the decision covers.
    pub candidate_id: String,
    /// The approving or rejecting principal (human or policy identity).
    pub approver: String,
    /// The explicit decision.
    pub decision: WorkflowImprovementDecision,
    /// Optional note recorded with an approval.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub note: Option<String>,
    /// Why the candidate was rejected; required when the decision is
    /// rejected.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub reason: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export_to = "v2/")]
pub struct WorkflowImproveApproveResponse {
    /// The candidate the decision covers.
    pub candidate_id: String,
    /// The workflow the candidate evolves.
    pub workflow: String,
    /// The incumbent version the approval binds to.
    pub incumbent_version_id: String,
    /// The digest of the validation report the approval covers.
    pub validation_digest: String,
    /// Whether the decision approves.
    pub approved: bool,
    /// The principal that decided.
    pub approver: String,
    /// The approval note, when one was recorded.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub note: Option<String>,
    /// The rejection reason, when the candidate was rejected.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub reason: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export_to = "v2/")]
pub struct WorkflowImprovePublishParams {
    /// The validated and approved candidate to publish.
    pub candidate_id: String,
    /// The release tag the successor is published under.
    pub release_tag: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export_to = "v2/")]
pub struct WorkflowImprovePublishResponse {
    /// The workflow that evolved.
    pub workflow: String,
    /// The immutable version identity of the successor.
    pub version_id: String,
    /// The semantic version of the successor.
    pub semantic_version: String,
    /// Digest of the successor's frozen workflow definition.
    pub definition_digest: String,
    /// Digest of the successor's resolved dependency lock.
    pub dependency_lock_digest: String,
    /// The successor's repository identity.
    pub repository: String,
    /// The immutable commit the successor anchors at.
    pub commit_sha: String,
    /// The governed lineage connecting predecessor to successor.
    pub lineage: WorkflowImprovementLineage,
    /// The execution evidence the published improvement cites.
    pub evidence: WorkflowEvidenceSummary,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export_to = "v2/")]
pub struct WorkflowInstanceRunParams {
    /// The published workflow version to run, in sha256-hex form.
    pub version_id: String,
}

/// Why one instance run stopped.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct WorkflowRunTerminal {
    /// Terminal outcome of the run.
    pub kind: WorkflowRunTerminalKind,
    /// The node the run paused at, when it paused.
    pub node: Option<String>,
    /// Why the run paused or failed.
    pub reason: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct WorkflowInstanceRunResponse {
    /// The durable instance id of the run.
    pub instance_id: String,
    /// The workflow the instance executes.
    pub workflow: String,
    /// The immutable version the instance is pinned to.
    pub version_id: String,
    /// Terminal status of the settled instance.
    pub status: WorkflowInstanceStatus,
    /// Why the run stopped.
    pub terminal: WorkflowRunTerminal,
    /// The step nodes the run visited, in order.
    pub path: Vec<String>,
}

#[derive(
    Serialize, Deserialize, Debug, Default, Clone, Copy, PartialEq, Eq, Hash, JsonSchema, TS,
)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export_to = "v2/")]
pub struct WorkflowInstanceListParams {}

/// The trigger that started an instance, when recorded.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct WorkflowTriggerSource {
    /// The normalized trigger class that fired.
    pub class: WorkflowTriggerClass,
    /// Opaque identifier of the triggering event, when allocated.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_id: Option<String>,
}

/// Compact view of the persisted run position of one active instance.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct WorkflowRunPositionSummary {
    /// The node the run visits next, when one is pending.
    pub current_node: Option<String>,
    /// The node budget the run already consumed.
    pub steps_taken: u64,
    /// The nodes the run visited so far, in order.
    pub path: Vec<String>,
}

/// One durable workflow instance as seen by the control plane.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct WorkflowInstanceRecord {
    /// The durable instance id.
    pub instance_id: String,
    /// The workflow the instance executes.
    pub workflow: String,
    /// The immutable version the instance is pinned to.
    pub version_id: String,
    /// Contract-level lifecycle status.
    pub status: WorkflowInstanceStatus,
    /// The trigger that started the instance, when recorded.
    pub trigger: Option<WorkflowTriggerSource>,
    /// The persisted run position, when one exists.
    pub position: Option<WorkflowRunPositionSummary>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct WorkflowInstanceListResponse {
    /// Every durable instance, ordered by instance id.
    pub instances: Vec<WorkflowInstanceRecord>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export_to = "v2/")]
pub struct WorkflowInstanceGetParams {
    /// The durable instance to read.
    pub instance_id: String,
}

/// A reference from an instance to evidence held by the evidence plane.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct WorkflowEvidenceReference {
    /// What kind of evidence is referenced.
    pub kind: String,
    /// Opaque locator allocated by the evidence plane.
    pub locator: String,
    /// Digest of the referenced evidence for integrity checking.
    pub digest: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct WorkflowInstanceGetResponse {
    /// The durable instance record.
    pub instance: WorkflowInstanceRecord,
    /// Evidence references recorded on the instance, in order.
    pub evidence: Vec<WorkflowEvidenceReference>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export_to = "v2/")]
pub struct WorkflowInstanceResumeParams {
    /// The paused instance to resume.
    pub instance_id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct WorkflowInstanceResumeResponse {
    /// The resumed durable instance record.
    pub instance: WorkflowInstanceRecord,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export_to = "v2/")]
pub struct WorkflowInstanceCancelParams {
    /// The instance to cancel.
    pub instance_id: String,
    /// Operator-visible reason recorded with the cancellation.
    pub reason: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct WorkflowInstanceCancelResponse {
    /// The cancelled durable instance record.
    pub instance: WorkflowInstanceRecord,
}

#[cfg(test)]
#[path = "workflow_tests.rs"]
mod tests;
