//! Multi-environment execution contracts for the Codex universal workflow
//! platform.
//!
//! This crate is the execution-plane boundary defined by WO-005: one
//! execution abstraction capable of routing workflow steps across
//! Codex-native and external environments without duplicating the workflow
//! engine. It is a contracts-plus-decision crate in the same family as
//! `codex-model-contract` (WO-002) and `codex-workflow-contracts`
//! (WO-003): it contains no environment runtime, no browser or computer
//! control internals, no durable storage, and no workflow engine. Those
//! surfaces are owned by later Work Orders:
//!
//! - WO-006 implements the browser adapter over Codex Browser Use;
//! - WO-007 implements the computer adapter over Codex Computer Use;
//! - the workflow control plane (roadmap M4) owns durable instance state;
//! - the evidence plane stores the payloads referenced here.
//!
//! # Contract surface
//!
//! - [`ExecutionEnvironment`] — the seven peer environment classes
//!   (`BROWSER`, `COMPUTER`, `TERMINAL`, `API`, `TOOL`, `MCP`, `HUMAN`)
//!   plus reserved `MOBILE`/`REMOTE_DESKTOP`.
//! - [`EnvironmentAdapter`] — the single adapter boundary every
//!   environment implements.
//! - [`CapabilityBinding`] — one pairing of a semantic capability with an
//!   adapter, ordered in the fallback chain by [`BindingClass`].
//! - [`ResourceBinding`] — the binding of a resource requirement to an
//!   opaque, credential-free resource instance.
//! - [`Action`], [`Observation`], [`ActionResult`] — the normalized
//!   execution currency between the engine and adapters.
//! - [`Recovery`], [`TakeoverRequest`], [`TakeoverGrant`] — the recovery
//!   and takeover contracts.
//! - [`ReadinessState`] — the frozen lifecycle
//!   `DECLARED -> AVAILABLE -> READY -> AUTHORIZED -> BOUND -> EXECUTING`
//!   with diagnosable `FAILED`/`UNAVAILABLE` states.
//! - [`CapabilityRegistry`] — the in-process binding decision authority:
//!   registration, readiness, authorization, resource binding, resolution
//!   with evidenced fallback, planning, diagnostics, and single-action
//!   dispatch.
//!
//! # Frozen boundary rules
//!
//! - **One execution abstraction, no second engine.** The registry routes
//!   bindings and dispatches single actions. It never walks a workflow
//!   graph, sequences steps, or owns durable state; workflow semantics and
//!   durable transitions remain WO-003 contracts and control-plane
//!   authority. Graph traversal belongs to the workflow engine, which this
//!   crate deliberately does not contain.
//! - **Environments are peers; adapters are executors, never
//!   authorities.** No environment class is privileged, and no adapter can
//!   mutate workflow semantics, select its own work, or escalate its own
//!   permissions. Binding decisions belong to the registry under policy.
//! - **Availability is not authorization.** `AVAILABLE`, `READY`, and
//!   `AUTHORIZED` are distinct, separately evidenced gates. A binding
//!   reaches `AUTHORIZED` only through an [`AuthorizationGrant`] that
//!   references a recorded approval decision, and `BOUND` only after its
//!   required resources are attached.
//! - **Fallback is policy-checked and evidenced.** Resolution orders
//!   candidates by the frozen fallback chain (Codex-native, compatible,
//!   human) and records why any preferred candidate was skipped. Fallback
//!   decisions convert to [`codex_workflow_contracts::EvidenceReference`]
//!   records (kind `Recovery`, the rebind family) that the control plane
//!   must store.
//! - **Credentials never cross the boundary.** Resource bindings carry
//!   opaque instance identities only; adapters resolve them to secret
//!   material internally.
//! - **External outputs are untrusted.** Observations, results, outputs,
//!   and diagnostics are bounded and treated as untrusted input by
//!   default.
//! - **Reserved environments fail loudly.** `MOBILE` and `REMOTE_DESKTOP`
//!   exist on the wire but reject registration, resources, and
//!   observations until WO-015 activates them.
//! - **Provider and forge neutrality.** Nothing in this crate names a
//!   model provider, MCP server, plugin, connector, or forge. Those are
//!   adapter-side binding details.
//!
//! # Integration with WO-003 contracts
//!
//! Requirements come from
//! [`codex_workflow_contracts::CapabilityRequirement`] and
//! [`codex_workflow_contracts::ResourceRequirement`]; steps come from
//! [`codex_workflow_contracts::WorkflowIr`]; evidence flows through
//! [`codex_workflow_contracts::EvidenceReference`] with kinds
//! `Observation`, `Approval`, and `Recovery`. The same workflow step
//! semantics can therefore bind to different compatible environments, and
//! one plan (and one execution) can contain multiple environments.

#![deny(missing_docs)]

mod action;
mod adapter;

pub(crate) use adapter::preparation_diagnostic;
pub(crate) use adapter::probe_error_diagnostic;
pub(crate) use observation::validate_locator;
mod authorization;
mod binding;
mod decision;
mod diagnostic;
mod environment;
mod error;
mod id;
mod observation;
mod policy;
mod readiness;
mod recovery;
mod registry;
mod resolution;
mod resource;
mod result;

pub use action::ACTION_INPUTS_MAX_BYTES;
pub use action::ACTION_INPUTS_MAX_DEPTH;
pub use action::ACTION_TARGET_MAX_BYTES;
pub use action::Action;
pub use action::ActionInputs;
pub use action::ActionTarget;
pub use action::OperationId;
pub use adapter::AdapterDescriptor;
pub use adapter::AdapterExecuteFuture;
pub use adapter::AdapterPrepareFuture;
pub use adapter::AdapterProbeFuture;
pub use adapter::EnvironmentAdapter;
pub use adapter::PrepareRequest;
pub use adapter::PreparedSession;
pub use adapter::ProbeReport;
pub use adapter::ProbeStatus;
pub use adapter::ProvidedCapability;
pub use authorization::AuthorizationGrant;
pub use binding::BindingClass;
pub use binding::CapabilityBinding;
pub use decision::BindingDecision;
pub use decision::BindingPlan;
pub use decision::FallbackReason;
pub use decision::FallbackRecord;
pub use decision::SelectedBinding;
pub use decision::StepBinding;
pub use diagnostic::BindingDiagnostic;
pub use diagnostic::DiagnosticCode;
pub use environment::ExecutionEnvironment;
pub use error::ExecutionContractError;
pub use id::AdapterId;
pub use id::CapabilityBindingId;
pub use id::ResourceId;
pub use id::SessionHandle;
pub use observation::OBSERVATION_MAX_BYTES;
pub use observation::OBSERVATION_MAX_DEPTH;
pub use observation::Observation;
pub use policy::BindingPolicy;
pub use policy::EnvironmentScope;
pub use policy::HumanFallbackPolicy;
pub use readiness::ReadinessState;
pub use readiness::ReadinessTracker;
pub use readiness::ReadinessTransition;
pub use readiness::TransitionCause;
pub use recovery::Recovery;
pub use recovery::RecoveryOutcome;
pub use recovery::RecoveryStrategy;
pub use recovery::TakeoverGrant;
pub use recovery::TakeoverReason;
pub use recovery::TakeoverRequest;
pub use registry::CapabilityRegistry;
pub use registry::RegisteredBinding;
pub use resource::ResourceBinding;
pub use result::ACTION_OUTPUTS_MAX_BYTES;
pub use result::ActionOutcome;
pub use result::ActionResult;
pub use result::ExecutionFailure;
pub use result::FAILURE_MESSAGE_MAX_BYTES;
pub use result::FailureKind;
