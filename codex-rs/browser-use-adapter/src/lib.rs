//! Codex Browser Use workflow adapter (WO-006).
//!
//! This crate binds workflow execution to the existing Codex Browser Use
//! capability without creating a parallel browser-agent implementation. It
//! is a leaf adapter: it contains no browser runtime, no agent loop, no
//! workflow engine, and no durable storage. Browsers are driven by the
//! native Codex Browser Use capability; this crate
//!
//! - models the capability lifecycle `declared -> available -> ready ->
//!   authorized -> bound -> executing` with explicit interruption,
//!   recovery, and take-over paths ([`lifecycle`], [`adapter`]);
//! - normalizes browser observation / action / result / recovery
//!   artifacts into digest-bearing evidence records shaped for the
//!   `codex-workflow-contracts` evidence and instance model ([`session`],
//!   [`bridge`]);
//! - pre-flights workflow browser requirements against the native
//!   Browser Use configuration surfaces ([`requirements`],
//!   [`config_snapshot`], [`authorization`]);
//! - detects a missing native bridge/runtime explicitly and produces
//!   actionable, credential-free diagnostics ([`diagnostics`]);
//! - permits compatible fallback browser adapters only when the
//!   workflow's semantic/policy requirements remain satisfied, and always
//!   records the fallback ([`binding`]).
//!
//! ## Invariants
//!
//! - **No second runtime.** The adapter never launches browsers or
//!   daemons; readiness is reported by the host through
//!   [`diagnostics::BridgeProbe`].
//! - **Lazy initialization.** Constructing
//!   [`adapter::BrowserUseAdapter`] has no side effects; probing and
//!   configuration resolution happen only when `prepare` is called, so
//!   unrelated workflows and chat startup are never blocked.
//! - **Evidence, not authority.** Browser state enters workflow records
//!   only as digest-bearing evidence; instance lifecycle remains owned by
//!   the workflow control plane, and [`session::suggested_instance_status`]
//!   is advisory.
//! - **Approvals stay native.** The requirement-vs-policy check is a
//!   pre-flight only; it never grants access and never bypasses Codex
//!   approvals or sandboxing.
//! - **No credentials.** Diagnostics, bindings, and evidence payloads are
//!   credential-free by construction.
//!
//! ## Evidence identity convention
//!
//! Every artifact is canonically serialized ([`canonical`]) and digested
//! with SHA-256. The digest input bytes are exposed on each
//! [`bridge::EvidenceRecord`]; the evidence plane allocates its own
//! `ContentDigest` objects from exactly those bytes, so both sides agree
//! on digest identity without this crate depending on evidence-plane
//! constructor APIs. [`bridge::attach_evidence`] appends the resulting
//! references onto any sink, including `codex_workflow_contracts::
//! WorkflowInstance`.

#![deny(missing_docs)]

pub mod adapter;
pub mod authorization;
pub mod binding;
pub mod bridge;
pub mod canonical;
pub mod config_snapshot;
pub mod diagnostics;
pub mod error;
pub mod lifecycle;
pub mod requirements;
pub mod session;

pub use adapter::BrowserUseAdapter;
pub use adapter::PrepareReport;
pub use adapter::RecoveryRemediation;
pub use authorization::AuthorizationOutcome;
pub use authorization::AuthorizationViolation;
pub use authorization::SessionGovernance;
pub use authorization::check_authorization;
pub use binding::BROWSER_USE_CAPABILITY_ID;
pub use binding::BindingIdentity;
pub use binding::BrowserAdapterKind;
pub use binding::BrowserSessionIdentity;
pub use binding::FallbackAdapter;
pub use binding::FallbackCapabilities;
pub use binding::FallbackEvaluation;
pub use binding::FallbackRecord;
pub use binding::evaluate_fallback;
pub use bridge::EvidenceRecord;
pub use bridge::EvidenceSink;
pub use bridge::attach_evidence;
pub use canonical::Canonical;
pub use canonical::canonical_digest;
pub use canonical::sha256_hex;
pub use codex_workflow_contracts::ContentDigest;
pub use codex_workflow_contracts::EvidenceKind;
pub use codex_workflow_contracts::EvidenceReference;
pub use codex_workflow_contracts::WorkflowInstance;
pub use codex_workflow_contracts::WorkflowInstanceStatus;
pub use config_snapshot::BrowserUseConfigSnapshot;
pub use config_snapshot::EffectiveOriginPolicy;
pub use config_snapshot::OriginPolicySnapshot;
pub use config_snapshot::PolicySource;
pub use diagnostics::AdapterDiagnostic;
pub use diagnostics::BridgeProbe;
pub use diagnostics::DiagnosticCode;
pub use error::AdapterError;
pub use lifecycle::CapabilityLifecycle;
pub use lifecycle::CapabilityLifecycleState;
pub use lifecycle::InterruptionKind;
pub use lifecycle::LifecycleTransition;
pub use lifecycle::TransitionKind;
pub use requirements::AccessApprovalLifetime;
pub use requirements::AllowDeny;
pub use requirements::BrowserUseRequirement;
pub use requirements::FallbackNeeds;
pub use requirements::OriginPolicyRequirement;
pub use session::BrowserAction;
pub use session::BrowserActionKind;
pub use session::BrowserActionOutcome;
pub use session::BrowserActionResult;
pub use session::BrowserApproval;
pub use session::BrowserApprovalDecision;
pub use session::BrowserApprovalSubject;
pub use session::BrowserArtifact;
pub use session::BrowserExecutionSession;
pub use session::BrowserObservation;
pub use session::BrowserRecoveryNote;
pub use session::BrowserVerification;
pub use session::ExecutionOutcome;
pub use session::SessionEnd;
pub use session::suggested_instance_status;
