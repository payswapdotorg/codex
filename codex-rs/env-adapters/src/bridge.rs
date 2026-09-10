//! Shared host-bridge vocabulary for the WO-015 environment adapters.
//!
//! Remote-desktop and mobile-device bridges are host-owned: the real
//! endpoint (an RDP/VNC-style session service, a device bridge such as an
//! adb/appium host) is provisioned by the host, exactly as WO-006 and
//! WO-007 let the host supply the native runtimes. This module defines the
//! vocabulary every such bridge port shares: the endpoint identity, the
//! liveness/health answer, the normalized call-failure classification, and
//! the clamp used when converting bridge output into normalized execution
//! failures.
//!
//! Bridge output is untrusted input by default: details are bounded on use
//! and never trusted for control decisions beyond the normalized
//! classification the host itself provides.

use codex_execution_contracts::ExecutionFailure;
use codex_execution_contracts::FAILURE_MESSAGE_MAX_BYTES;
use codex_execution_contracts::FailureKind;
use serde::Deserialize;
use serde::Serialize;

/// Identity of one host-provided execution bridge endpoint.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BridgeEndpoint {
    /// Bridge kind, for example `rdp`, `vnc`, or `adb`.
    pub bridge_kind: String,
    /// Runtime backing the bridge, for example `codex-device-bridge`.
    pub runtime: String,
    /// Bridge/runtime version reported by the host.
    pub version: String,
}

/// The liveness answer of one bridge endpoint.
///
/// Health is a probe result, not an authorization: a live bridge still
/// crosses the binding policy and adapter-side allow/deny gates before any
/// action executes.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BridgeHealth {
    /// The endpoint that answered the liveness probe.
    pub endpoint: BridgeEndpoint,
    /// Round-trip latency hint reported by the bridge, when measured.
    pub latency_ms: Option<u64>,
}

/// Normalized classification of a bridge call failure.
///
/// The host's classification stays authoritative over text matching, the
/// same convention the WO-006 browser wiring uses for turn failures.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum BridgeFailureKind {
    /// The bridge transport is gone (disconnect, crash, host reboot).
    Lost,
    /// The bridge call exceeded its deadline.
    Timeout,
    /// The bridge refused the call (host policy or target refusal).
    Rejected,
    /// The bridge could not execute the call (protocol error).
    Protocol,
}

/// Why a bridge call could not produce an outcome.
///
/// Call failures are transport-level: the bridge itself is unreachable,
/// too slow, or refused the call. Target-level outcomes (a remote
/// application reporting an error, a device app failing an action) travel
/// inside successful outcomes, where they stay untrusted derived data.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BridgeCallFailure {
    /// Normalized classification of the failure.
    pub kind: BridgeFailureKind,
    /// Untrusted-derived detail from the bridge.
    pub detail: String,
}

impl BridgeCallFailure {
    /// Creates a failure with a classification and untrusted detail.
    pub fn new(kind: BridgeFailureKind, detail: impl Into<String>) -> Self {
        Self {
            kind,
            detail: detail.into(),
        }
    }

    /// Maps the host's classification onto the execution-contract failure
    /// kind, which drives recovery strategy selection.
    pub fn failure_kind(&self) -> FailureKind {
        match self.kind {
            BridgeFailureKind::Lost => FailureKind::Unavailable,
            BridgeFailureKind::Timeout => FailureKind::Timeout,
            BridgeFailureKind::Rejected => FailureKind::PolicyDenied,
            BridgeFailureKind::Protocol => FailureKind::Permanent,
        }
    }

    /// The bounded diagnostic message for this failure.
    pub fn message(&self) -> String {
        let label = match self.kind {
            BridgeFailureKind::Lost => "bridge lost",
            BridgeFailureKind::Timeout => "bridge call timed out",
            BridgeFailureKind::Rejected => "bridge rejected the call",
            BridgeFailureKind::Protocol => "bridge protocol failure",
        };
        format!("{label}: {}", self.detail)
    }
}

/// Builds a normalized execution failure with clamped, non-empty detail.
///
/// [`ExecutionFailure::new`] rejects empty and oversized messages; the
/// adapters clamp instead of failing so a diagnostic is never lost because
/// its text was out of bounds. Truncation happens at a UTF-8 character
/// boundary so untrusted multi-byte output cannot panic the boundary.
pub fn normalized_failure(kind: FailureKind, message: impl Into<String>) -> ExecutionFailure {
    let mut message = message.into();
    if message.is_empty() {
        message = "unspecified environment bridge failure".to_string();
    }
    if message.len() > FAILURE_MESSAGE_MAX_BYTES {
        let mut end = FAILURE_MESSAGE_MAX_BYTES;
        while !message.is_char_boundary(end) {
            end -= 1;
        }
        message.truncate(end);
    }
    ExecutionFailure { kind, message }
}

#[cfg(test)]
#[path = "bridge_tests.rs"]
mod tests;
