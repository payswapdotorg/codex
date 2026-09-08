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

pub use adapter::{BrowserUseAdapter, PrepareReport, RecoveryRemediation};
pub use authorization::{
    AuthorizationOutcome, AuthorizationViolation, SessionGovernance, check_authorization,
};
pub use binding::{
    BROWSER_USE_CAPABILITY_ID, BindingIdentity, BrowserAdapterKind, BrowserSessionIdentity,
    FallbackAdapter, FallbackCapabilities, FallbackEvaluation, FallbackRecord, evaluate_fallback,
};
pub use bridge::{EvidenceRecord, EvidenceSink, attach_evidence};
pub use canonical::{Canonical, canonical_digest, sha256_hex};
pub use codex_workflow_contracts::{
    ContentDigest, EvidenceKind, EvidenceReference, WorkflowInstance, WorkflowInstanceStatus,
};
pub use config_snapshot::{
    BrowserUseConfigSnapshot, EffectiveOriginPolicy, OriginPolicySnapshot, PolicySource,
};
pub use diagnostics::{AdapterDiagnostic, BridgeProbe, DiagnosticCode};
pub use error::AdapterError;
pub use lifecycle::{
    CapabilityLifecycle, CapabilityLifecycleState, InterruptionKind, LifecycleTransition,
    TransitionKind,
};
pub use requirements::{
    AccessApprovalLifetime, AllowDeny, BrowserUseRequirement, FallbackNeeds,
    OriginPolicyRequirement,
};
pub use session::{
    BrowserAction, BrowserActionKind, BrowserActionOutcome, BrowserActionResult, BrowserApproval,
    BrowserApprovalDecision, BrowserApprovalSubject, BrowserArtifact, BrowserExecutionSession,
    BrowserObservation, BrowserRecoveryNote, BrowserVerification, ExecutionOutcome, SessionEnd,
    suggested_instance_status,
};
