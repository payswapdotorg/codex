//! Workflow application integration and end-to-end runtime (WO-010).
//!
//! This crate is the application-facing composition layer of the Codex
//! universal workflow platform: it wires the four existing workflow planes
//! into one lifecycle without duplicating any of them.
//!
//! ```text
//! teaching/compiler (WO-008)  ->  publish   ->  workflow version (WO-003)
//! git collaboration (WO-009)  ->  release/install
//! execution contracts (WO-005) + registry  ->  binding decisions, dispatch
//! browser use (WO-006) + computer use (WO-007)  ->  environment adapters
//! ```
//!
//! ## What this crate is
//!
//! - A [`WorkflowLifecycle`] application service: select an immutable
//!   [`WorkflowVersion`], instantiate (validate -> approve -> bind), run
//!   across environment classes, observe/emit events, and recover or
//!   escalate.
//! - [`port`] seams for everything durable or host-owned: version records,
//!   instance records, evidence payloads, approvals, action proposals, and
//!   events. In-memory implementations for tests live in [`memory`]; the
//!   Codex application supplies the durable ones.
//! - Environment wiring ([`browser_env`], [`computer_env`]): the WO-006 and
//!   WO-007 capability adapters exposed behind the WO-005
//!   [`EnvironmentAdapter`] boundary, so one mixed-environment run routes
//!   browser and desktop steps through the same capability registry.
//!
//! ## What this crate deliberately is not
//!
//! - **No second agent runtime and no second workflow engine.** Every
//!   semantic decision delegates to a frozen crate: versions, instances,
//!   IR, evidence, and integrity come from `codex-workflow-contracts`;
//!   binding decisions, readiness, authorization, fallback, and
//!   single-action dispatch come from `codex-execution-contracts`;
//!   teaching, validation, simulation, and approval gating come from
//!   `codex-teaching-compiler`; collaboration and installation records come
//!   from `codex-workflow-forge`. The run path walks the graph with the
//!   same deterministic policy the teaching compiler froze for publication
//!   replay (see [`walk`]); it sequences steps and dispatches one action at
//!   a time, exactly the control-plane responsibility WO-005's registry
//!   docs assign to this Work Order.
//! - **No durable state owned here.** Instances, versions, evidence
//!   payloads, approvals, and events cross [`port`] traits; this crate
//!   holds only in-flight run state.
//! - **No credential, no settlement, no forge specifics.** Repository
//!   identity is canonical and credential-free; approvals arrive as
//!   evidence references; the GitHub forge is one adapter behind
//!   `codex-workflow-forge`.
//! - **No bypass of Codex approvals, sandboxing, policy, sessions, or
//!   tracing.** Bindings only reach `AUTHORIZED` through an
//!   [`AuthorizationGrant`] resting on recorded approval evidence, and the
//!   browser/computer adapters keep their native policy gates.
//!
//! ## Ordinary Codex compatibility
//!
//! Nothing in this crate executes unless a workflow version is explicitly
//! selected through [`WorkflowLifecycle::select_version`]. Adapters are
//! constructed lazily by the host, probing happens only during
//! instantiation, and with no active workflow every run-path operation
//! returns [`WorkflowAppError::NoActiveWorkflow`] without touching
//! adapters, evidence, or events. The ordinary coding-agent behavior of
//! Codex is untouched: this crate adds no code to `codex-core` and the
//! application only mounts the lifecycle when a workflow surface is used.
//!
//! ## Pipeline
//!
//! ```text
//! TeachingSession -> compile -> validate -> simulate -> approve -> finalize
//!   -> publish (bindings resolved into declared, digested requirements)
//!   -> seal immutable WorkflowVersion -> forge release -> install
//!   -> select -> instantiate (validate -> approve -> bind)
//!   -> run (mixed environments, evidence, recovery) -> verify
//! ```

#![deny(missing_docs)]

pub mod approval;
pub mod browser_env;
pub mod computer_env;
pub mod error;
pub mod event;
pub mod lifecycle;
pub mod memory;
pub mod port;
pub mod publish;
pub mod run;
pub mod walk;

pub use approval::{ApprovalEvidence, ApprovalRequest, ApprovalVerdict};
pub use browser_env::{
    BROWSER_CAPABILITY, BROWSER_PROFILE_RESOURCE, BrowserActionExecutor, BrowserTurn,
    BrowserUseEnvironmentAdapter,
};
pub use computer_env::{
    COMPUTER_CAPABILITY, ComputerUseEnvironmentAdapter, DESKTOP_SESSION_RESOURCE,
};
pub use error::WorkflowAppError;
pub use event::WorkflowEvent;
pub use lifecycle::{InstantiateRequest, LifecycleDeps, VerifiedRun, WorkflowLifecycle};
pub use memory::{
    InMemoryApprovalSource, InMemoryEvidenceStore, InMemoryInstanceStore, InMemoryVersionStore,
    RecordingEventSink, ScriptedAction, ScriptedActionSource,
};
pub use port::{
    ActionRequest, ApprovalSource, EventSink, EvidenceStore, StepActionSource,
    WorkflowInstanceStore, WorkflowVersionStore,
};
pub use publish::{
    BindingResolution, PublishRequest, PublishedArtifact, StepCapabilityBindings, install_version,
    publish,
};
pub use run::{RunOutcome, RunTerminal};
pub use walk::{WalkConfig, WalkTerminal};
