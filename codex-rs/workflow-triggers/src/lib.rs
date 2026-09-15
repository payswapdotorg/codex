//! Workflow scheduling, triggers, sharing, and installation (WO-011).
//!
//! This crate is the trigger/scheduling/installation plane **over** the
//! WO-010 workflow application ports: it makes immutable, published
//! workflow versions operationally reusable — discoverable, installable,
//! configurable, triggerable, and schedulable — through the same control
//! plane the application lifecycle already owns.
//!
//! ```text
//! workflow-forge (WO-009)      -> discovery + install semantics (reused)
//! workflow-app (WO-010)        -> lifecycle: select -> instantiate -> run
//! workflow-triggers (WO-011)   -> ingest -> dedupe -> gate -> fire/resume
//! ```
//!
//! ## What this crate is
//!
//! - A [`WorkflowTriggerPlane`] application service: ingest normalized
//!   [`IncomingTrigger`] events (all eight frozen trigger classes), dedupe
//!   them durably through a control-plane [`TriggerLedger`] seam, evaluate
//!   capability/resource/policy/authorization readiness **before** any
//!   instance is created, then start one instance per accepted fire by
//!   driving the WO-010 [`WorkflowLifecycle`] (`select_version` ->
//!   `instantiate` -> `run`).
//! - Installation/configuration: [`WorkflowTriggerPlane::install`] pins an
//!   immutable [`WorkflowVersion`] through the WO-009 forge
//!   [`InstallRegistry`] semantics (no silent over-installs or upgrades)
//!   and records an [`InstalledConfiguration`] with **explicit** resource
//!   and dependency bindings. [`WorkflowTriggerPlane::rebind_resource`]
//!   swaps a resource or account binding without touching the immutable
//!   semantic source.
//! - Sharing/discovery: [`WorkflowTriggerPlane::discover`] reuses the
//!   WO-009 repository discovery queries over a host-supplied
//!   [`PackageCatalog`] seam (the connector/forge transport stays
//!   host-owned).
//! - Scheduling: [`WorkflowTriggerPlane::poll_schedules`] computes due
//!   occurrences from explicit [`ScheduleSpec`]s and fires them through
//!   the exact same ingest path — schedule fires are ordinary triggers
//!   with deterministic idempotency keys, so a re-polled or crashed
//!   scheduler can never double-fire.
//! - Port seams (see the re-exported `TriggerLedger`,
//!   `ResourceAuthorizer`, `PackageCatalog`, `InstallationStore`,
//!   `InstanceControl`, and `ScheduleClock` traits) for everything
//!   durable or host-owned, with in-memory implementations (the
//!   `InMemory*` types) for tests and seeding.
//!
//! ## What this crate deliberately is not
//!
//! - **No second control plane and no second scheduler.** Every instance
//!   transition goes through the WO-010 lifecycle (validate -> approve ->
//!   bind -> run) and every dedupe decision through the control-plane
//!   ledger seam. The scheduler only derives triggers; it owns no instance
//!   authority.
//! - **No events mutating workflow meaning.** External trigger payloads
//!   are untrusted input: only the normalized trigger class, the
//!   control-plane event id, and an optional payload digest ever enter
//!   workflow records. Events start, resume, or are recorded as no-ops —
//!   they never rewrite semantics.
//! - **No credentials.** Resource bindings are opaque, credential-free
//!   instance identities; authorization decisions are attributed to a
//!   source mechanism label, never to secret material.
//! - **No silent upgrades.** Installed versions are immutable and pinned;
//!   dependency bindings must match the version's dependency lock exactly;
//!   re-installs and upgrades flow through the forge's explicit,
//!   reviewable update path.
//!
//! ## Readiness gating order
//!
//! ```text
//! ingest:  dedupe (ledger) -> static gate (installation, version
//!          integrity, trigger bindings/declarations, resource bindings,
//!          dependency-lock match, resource authorization) -> resume
//!          correlation -> instantiate (integrity + adapter readiness +
//!          binding plan + approvals) -> run -> settle (ledger)
//! ```
//!
//! The static gate runs before any instance record exists; capability,
//! policy, and binding-plan readiness is enforced inside instantiation
//! (which settles a `Failed` instance with structured diagnostics when a
//! required capability is not ready), so no fire ever dispatches an action
//! without passing every gate in order.
//!
//! ## Ordinary Codex compatibility
//!
//! Nothing in this crate executes unless the application drives it. With
//! no installed configuration, every operation is an inert, recorded
//! rejection: no adapter is probed, no version selected, no instance
//! created, no event emitted on the lifecycle — ordinary Codex behavior
//! is untouched.
//!
//! ## Store-sharing contract
//!
//! The port seams here deliberately reuse the WO-010 port
//! traits. The version store supplied to [`TriggerDeps`] must be backed
//! by the **same durable store** the [`WorkflowLifecycle`] selects from:
//! the plane publishes installed versions into it at install time and
//! re-verifies integrity at fire time.

#![deny(missing_docs)]

mod configuration;
mod diagnostic;
mod discovery;
mod error;
mod event;
mod install;
mod memory;
mod plane;
mod port;
mod schedule;

#[cfg(test)]
#[path = "event_tests.rs"]
mod event_tests;

#[cfg(test)]
#[path = "schedule_tests.rs"]
mod schedule_tests;

pub use configuration::DependencyBinding;
pub use configuration::InstalledConfiguration;
pub use configuration::RebindRecord;
pub use configuration::ScheduleRegistration;
pub use configuration::TriggerBinding;
pub use diagnostic::TriggerDiagnostic;
pub use diagnostic::TriggerDiagnosticCode;
pub use discovery::PackageQuery;
pub use discovery::WorkflowPackageListing;
pub use error::WorkflowTriggerError;
pub use event::IncomingTrigger;
pub use event::TriggerEventKey;
pub use install::InstallRequest;
pub use memory::FixedClock;
pub use memory::InMemoryInstallationStore;
pub use memory::InMemoryInstanceControl;
pub use memory::InMemoryPackageCatalog;
pub use memory::InMemoryResourceAuthorizer;
pub use memory::InMemoryTriggerLedger;
pub use memory::TriggerRecord;
pub use plane::TriggerDeps;
pub use plane::TriggerReport;
pub use plane::WorkflowTriggerPlane;
pub use port::AwaitRecord;
pub use port::FireOutcome;
pub use port::InstallationStore;
pub use port::InstanceControl;
pub use port::InstanceSettlement;
pub use port::PackageCatalog;
pub use port::ResourceAuthorization;
pub use port::ResourceAuthorizationRequest;
pub use port::ResourceAuthorizer;
pub use port::ResumeDirective;
pub use port::ScheduleClock;
pub use port::TriggerAcceptance;
pub use port::TriggerLedger;
pub use schedule::MAX_OCCURRENCES_PER_POLL;
pub use schedule::ScheduleSpec;
pub use schedule::due_occurrences;
