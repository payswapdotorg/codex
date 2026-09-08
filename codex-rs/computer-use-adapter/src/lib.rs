//! Codex Computer Use workflow adapter (WO-007).
//!
//! This crate binds workflow execution to the native Codex Computer Use
//! capability without creating a second desktop-agent runtime or a second
//! workflow engine. It is a leaf adapter crate:
//!
//! - workflow semantics stay in `codex-workflow-contracts`; this adapter only
//!   produces lifecycle records, binding identity, and normalized evidence
//!   that the execution plane (WO-005) attaches to a
//!   `WorkflowInstance`;
//! - the native Codex Computer Use feature/skill/runtime path is reached
//!   exclusively through the [`bridge::ComputerUseBridge`] boundary; the host
//!   integration supplies the factory backed by the real runtime;
//! - policy inputs (default application access, macOS bundle ids, Windows
//!   AUMIDs and exe identities) are normalized projections of the native
//!   `codex-config` computer use settings (`config/src/computer_use.rs`,
//!   `config/src/browser_computer_use_requirements.rs`). Per the capability
//!   implementation map they are resource/policy inputs, never workflow
//!   semantics;
//! - the browser use adapter surface (WO-006) is untouched. Mixed
//!   computer-use + browser-use runs compose in the universal execution
//!   plane over `codex-workflow-contracts` abstractions; every record this
//!   adapter emits is capability-scoped and carries no browser semantics.
//!
//! ## Capability lifecycle
//!
//! `declared -> available -> ready -> authorized -> bound -> executing` with
//! settling (`executing -> bound`), release (`bound -> authorized`),
//! readiness regression after bridge loss (`bound -> ready`), failure from
//! any live state, and reset (`failed -> declared`). The bridge is
//! provisioned lazily at the `ready` boundary; missing `node_repl`, bridge,
//! or runtime provisioning is detected explicitly with remediation
//! diagnostics instead of silent degradation.
//!
//! ## Session ownership, takeover, recovery, fallback
//!
//! Sessions are owned by opaque, credential-free owner tokens supplied by the
//! execution plane. Takeover rules, recovery records, and fallback records to
//! compatible alternate desktop adapters are explicit first-class records; a
//! fallback is accepted only when the alternate honors the same application
//! policy tables and Codex host approvals, and it is only ever recorded, not
//! executed.

#![deny(missing_docs)]

pub mod action;
pub mod adapter;
pub mod bridge;
pub mod capability;
pub mod error;
pub mod evidence;
pub mod observation;
pub mod policy;
pub mod session;

#[cfg(test)]
pub(crate) mod test_support;

pub(crate) mod util;

pub use action::ActionFailureReason;
pub use action::ActionStatus;
pub use action::KeyModifier;
pub use action::MouseButton;
pub use action::NormalizedAction;
pub use action::NormalizedActionResult;
pub use action::PointerTarget;
pub use action::SensitiveText;
pub use adapter::AlternateDesktopAdapter;
pub use adapter::CapabilityKind;
pub use adapter::ComputerUseWorkflowAdapter;
pub use adapter::FallbackReason;
pub use adapter::FallbackRecord;
pub use adapter::RecoveryPolicy;
pub use adapter::StepExecutionRecord;
pub use adapter::StepExecutionStatus;
pub use adapter::StepPlan;
pub use bridge::BridgeCallFailure;
pub use bridge::BridgeIdentity;
pub use bridge::BridgeReadinessFailure;
pub use bridge::ComputerUseBridge;
pub use bridge::ComputerUseBridgeFactory;
pub use capability::ADAPTER_IDENTITY;
pub use capability::COMPUTER_USE_CAPABILITY_ID;
pub use capability::CapabilityBinding;
pub use capability::CapabilityLifecycle;
pub use capability::CapabilityLifecycleState;
pub use capability::LifecycleTransition;
pub use error::ComputerUseAdapterError;
pub use evidence::AdapterEvidenceRecord;
pub use evidence::EVIDENCE_LOCATOR_PREFIX;
pub use evidence::EvidencePromotion;
pub use observation::ApplicationObservation;
pub use observation::ScreenIdentity;
pub use observation::ScreenObservation;
pub use observation::WindowBounds;
pub use observation::WindowObservation;
pub use policy::AccessDecision;
pub use policy::AccessRequirement;
pub use policy::ApplicationIdentity;
pub use policy::ApplicationPlatformIdentity;
pub use policy::AuthorizationRecord;
pub use policy::AuthorizationSource;
pub use policy::ComputerUsePolicy;
pub use policy::HostApproval;
pub use policy::WindowsExeIdentity;
pub use session::ComputerUseSessionId;
pub use session::OwnerToken;
pub use session::RecoveryOutcome;
pub use session::RecoveryRecord;
pub use session::RecoveryStrategy;
pub use session::RecoveryTrigger;
pub use session::SessionOwnership;
pub use session::TakeoverMode;
pub use session::TakeoverReceipt;
pub use session::UnixMsClock;
pub use session::system_clock;
