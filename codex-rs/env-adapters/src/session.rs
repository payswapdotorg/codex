//! Session ownership and takeover receipts for the WO-015 adapters.
//!
//! Sessions are owned by opaque, credential-free owner tokens, mirroring
//! the WO-007 session model: the registry-driven dispatch identity (the
//! binding identity) owns the session it prepared, takeover swaps the
//! recorded owner behind an explicit receipt, and execution after a
//! takeover requires a fresh prepare so the displacement is always
//! observable. Takeover never grants authority beyond what the binding's
//! approval already allowed.

use serde::Deserialize;
use serde::Serialize;

use codex_execution_contracts::CapabilityBindingId;

/// Opaque, credential-free session owner token.
///
/// Tokens never carry credential material; they exist so ownership and
/// takeover boundaries are explicit. The registry-driven dispatch owner is
/// derived deterministically from the binding identity.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OwnerToken(String);

impl OwnerToken {
    /// Creates a token from any credential-free string.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// The opaque token string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Takeover mode requested by a claimant.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TakeoverMode {
    /// Take over while the session is idle (between dispatches).
    Graceful,
    /// Take over unconditionally; requires a recorded reason.
    Forced,
}

/// Receipt of one accepted session takeover.
///
/// The receipt is the audit record of the displacement: which binding's
/// session changed hands, which bridge target it drives, the previous and
/// new owners, the mode, and the recorded reason (required for forced
/// takeovers). Adapters journal receipts as recovery-class evidence.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TakeoverReceipt {
    /// The binding whose session changed ownership.
    pub binding: CapabilityBindingId,
    /// The bridge target the session drives (remote host or device label).
    pub target: String,
    /// The owner recorded before the takeover.
    pub previous_owner: OwnerToken,
    /// The owner recorded after the takeover.
    pub new_owner: OwnerToken,
    /// Mode used.
    pub mode: TakeoverMode,
    /// Recorded reason; required for forced takeovers.
    pub reason: Option<String>,
}

/// The live bridge session one adapter instance holds.
///
/// One adapter instance drives at most one bridge session at a time, the
/// same single-session model as the WO-007 desktop wiring. The session
/// records the binding it serves, the current owner, and the opaque bridge
/// target (remote host or device label) derived from the resource binding.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct BridgeSession {
    /// The binding the session serves.
    pub(crate) binding: CapabilityBindingId,
    /// The current session owner.
    pub(crate) owner: OwnerToken,
    /// The opaque bridge target this session drives.
    pub(crate) target: String,
}

impl BridgeSession {
    /// Opens a session for `binding` driving `target`, owned by the
    /// registry-driven dispatch identity derived from the binding.
    pub(crate) fn new(binding: CapabilityBindingId, target: String) -> Self {
        let owner = OwnerToken::new(binding.to_string());
        Self {
            binding,
            owner,
            target,
        }
    }

    /// Whether the session is currently owned by the registry-driven
    /// dispatch identity of its binding.
    pub(crate) fn owned_by_dispatch(&self) -> bool {
        self.owner.as_str() == self.binding.as_ref()
    }
}

#[cfg(test)]
#[path = "session_tests.rs"]
mod tests;
