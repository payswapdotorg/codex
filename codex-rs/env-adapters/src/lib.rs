//! Additional execution-environment adapters for the Codex universal
//! workflow platform (WO-015).
//!
//! This crate extends the single Execution Plane with environment adapters
//! for additional interaction domains — remote desktop (RDP/VNC-style
//! host-bridged sessions) and mobile devices (handset/device bridges) —
//! without touching workflow semantics or adding any orchestration engine.
//! It is a leaf adapter crate in the family proven by
//! `codex-browser-use-adapter` (WO-006) and `codex-computer-use-adapter`
//! (WO-007):
//!
//! - the WO-005 [`codex_execution_contracts::EnvironmentAdapter`] seam is
//!   the only integration surface: both adapters implement it directly, so
//!   they register into the capability/resource readiness registry exactly
//!   like the browser and computer environment wirings in
//!   `codex-workflow-app`, and mixed-environment runs compose through the
//!   same workflow-app run paths with no environment-specific branches;
//! - the host provides the real bridges (remote desktop endpoints, device
//!   bridges). This crate models each bridge as a port
//!   ([`remote_desktop::RemoteDesktopBridge`],
//!   [`mobile_device::MobileDeviceBridge`]) and ships no runtime: nothing
//!   is connected, launched, or provisioned here, so a missing bridge is an
//!   explicit readiness diagnostic, never a silently weaker mechanism;
//! - every adapter action crosses the native policy gates: binding policy
//!   is enforced by the registry during resolution, and each adapter keeps
//!   its own allow/deny table for the targets named by its resource
//!   bindings, so availability never implies authorization;
//! - sessions are owned by opaque, credential-free owner tokens, takeovers
//!   are recorded receipts, and every action, observation, failure, and
//!   takeover lands in an append-only adapter-side evidence journal whose
//!   records carry digest identity for evidence-plane promotion;
//! - credentials never cross the boundary: resource bindings carry opaque
//!   instance identities only.
//!
//! ## Environment classes
//!
//! Both adapters register behind the existing peer environment class
//! [`codex_execution_contracts::ExecutionEnvironment::Computer`], following
//! the repo's own guidance that a remote desktop session "is reached
//! through the existing Computer Use semantic contract" (WO-016 advisory)
//! and that mobile may ride a compatible bridge "without changing workflow
//! semantics" (frozen architecture, environment section). The reserved
//! wire variants `MOBILE`/`REMOTE_DESKTOP` stay reserved: their loud-failure
//! behavior is frozen and tested by WO-005, and activating them as
//! first-class environment classes is a separate, future Work Order that
//! needs no change here. Both adapters sit at
//! [`codex_execution_contracts::BindingClass::Compatible`] in the frozen
//! fallback chain: Codex-native Computer Use is preferred for
//! `control_desktop_app`, and these bridges serve as the compatible
//! alternates — plus their own capabilities (`control_remote_desktop`,
//! `control_mobile_device`) that only they provide today.
//!
//! ## What this crate deliberately is not
//!
//! - **No second runtime and no second engine.** Bridge provisioning,
//!   protocol handling, and graph traversal stay with the host and the
//!   control plane; this crate only normalizes actions, observations,
//!   results, and recovery records.
//! - **No durable state.** Evidence is journaled in memory for host
//!   harvesting; the evidence plane owns storage.
//! - **No bypass of Codex approvals, sandboxing, policy, sessions, or
//!   tracing.** Bindings reach `AUTHORIZED` only through the registry's
//!   approval-gated transitions; adapter-side policy can only *restrict*.

#![deny(missing_docs)]

pub mod bridge;
pub mod error;
pub mod evidence;
pub mod mobile_device;
pub mod mobile_device_adapter;
pub mod policy;
pub mod remote_desktop;
pub mod remote_desktop_adapter;
pub mod session;

pub use bridge::{
    BridgeCallFailure, BridgeEndpoint, BridgeFailureKind, BridgeHealth, normalized_failure,
};
pub use error::EnvAdapterError;
pub use evidence::{AdapterEvidenceRecord, EVIDENCE_LOCATOR_PREFIX, EvidenceJournal};
pub use mobile_device::{
    MOBILE_DEVICE_ADAPTER_ID, MOBILE_DEVICE_CAPABILITY, MOBILE_DEVICE_RESOURCE, MobileDeviceAction,
    MobileDeviceBridge, MobileDeviceClass, MobileDeviceObservation, MobileDeviceOutcome,
    MobileDevicePolicy,
};
pub use mobile_device_adapter::MobileDeviceEnvironmentAdapter;
pub use policy::{Access, AccessDecision, AccessTable};
pub use remote_desktop::{
    DESKTOP_APP_CAPABILITY, REMOTE_DESKTOP_ADAPTER_ID, REMOTE_DESKTOP_CAPABILITY,
    REMOTE_DESKTOP_SESSION_RESOURCE, RemoteDesktopAction, RemoteDesktopBridge,
    RemoteDesktopOutcome, RemoteDesktopPolicy, RemoteScreenObservation, RemoteSessionState,
};
pub use remote_desktop_adapter::RemoteDesktopEnvironmentAdapter;
pub use session::{OwnerToken, TakeoverMode, TakeoverReceipt};
