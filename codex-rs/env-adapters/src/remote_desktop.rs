//! The remote-desktop environment domain (WO-015).
//!
//! This module owns the remote-desktop vocabulary: normalized actions,
//! screen observations, action outcomes, the host-bridge port, and the
//! host-access policy. The vocabulary is adapter-owned, exactly as the
//! execution contracts require — workflow semantics never reference it.
//! A remote-desktop session is an RDP/VNC-style host-bridged interaction
//! with a remote machine's desktop; the bridge itself is supplied by the
//! host ([`RemoteDesktopBridge`]), never launched here.

use serde::Deserialize;
use serde::Serialize;

use crate::bridge::BridgeCallFailure;
use crate::bridge::BridgeHealth;
use crate::policy::AccessTable;

/// The adapter identity registered with the capability registry.
pub const REMOTE_DESKTOP_ADAPTER_ID: &str = "codex-remote-desktop-env";
/// The semantic capability naming remote-desktop session control.
pub const REMOTE_DESKTOP_CAPABILITY: &str = "control_remote_desktop";
/// The WO-007 desktop-application capability this environment also serves
/// as a compatible alternate, so the registry's fallback chain can route
/// desktop steps to a remote bridge when native Computer Use is
/// unavailable.
pub const DESKTOP_APP_CAPABILITY: &str = "control_desktop_app";
/// The typed resource this environment requires before execution: an
/// opaque, credential-free remote host identity.
pub const REMOTE_DESKTOP_SESSION_RESOURCE: &str = "remote_desktop_session";

/// One normalized remote-desktop action.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub enum RemoteDesktopAction {
    /// Press the primary button at a screen position.
    Click {
        /// Horizontal position in pixels.
        x: i32,
        /// Vertical position in pixels.
        y: i32,
    },
    /// Enter text into the focused remote application.
    TypeText {
        /// The text to enter (untrusted-derived bound).
        text: String,
    },
    /// Press one key with optional modifiers.
    PressKey {
        /// The key name, for example `enter`.
        key: String,
        /// Modifier key names, for example `ctrl`.
        modifiers: Vec<String>,
    },
    /// Scroll vertically by a pixel delta.
    Scroll {
        /// Vertical scroll delta in pixels.
        delta_y: i32,
    },
    /// Launch a remote application by name.
    LaunchApplication {
        /// The application the remote desktop should launch.
        application: String,
    },
    /// Capture the remote screen.
    Screenshot,
    /// Close the remote session.
    Disconnect,
}

/// The remote session state reported by the bridge.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RemoteSessionState {
    /// The remote session is connected.
    Connected,
    /// The remote session is disconnected.
    Disconnected,
}

/// One normalized remote screen observation.
///
/// Observations are untrusted-derived: they are bounded by the execution
/// contracts when attached to action results and never trusted for
/// control decisions.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RemoteScreenObservation {
    /// The opaque remote host label the observation came from.
    pub host: String,
    /// The remote session state.
    pub state: RemoteSessionState,
    /// Screen width in pixels.
    pub width: u32,
    /// Screen height in pixels.
    pub height: u32,
    /// The foreground remote application, when known.
    pub foreground_application: Option<String>,
    /// Bridge-supplied capture timestamp in unix milliseconds.
    pub captured_at_unix_ms: u64,
}

/// The outcome of one remote-desktop action.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RemoteDesktopOutcome {
    /// The observation captured after the action, when available.
    pub observation: Option<RemoteScreenObservation>,
    /// Bridge-supplied detail (untrusted-derived).
    pub detail: Option<String>,
}

/// Host-access policy for remote-desktop sessions.
///
/// The table gates the remote hosts named by attached
/// `remote_desktop_session` resource bindings. Unspecified hosts require
/// explicit host authorization, so the adapter refuses rather than
/// silently defaulting to allow.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RemoteDesktopPolicy {
    /// Which remote hosts the adapter may drive.
    pub host_access: AccessTable,
    /// Whether cross-owner session takeover may be recorded; denied when
    /// absent, mirroring the WO-007 session policy shape.
    pub allow_session_takeover: Option<bool>,
}

/// The host-provided remote-desktop bridge (RDP/VNC-style).
///
/// The adapter never connects to, launches, or re-implements a
/// remote-desktop runtime: the host supplies this port backed by the real
/// bridge, the same host-owns-the-runtime boundary WO-006 and WO-007 use.
/// A bridge that cannot answer [`RemoteDesktopBridge::health`] surfaces as
/// an explicit not-installed readiness diagnostic.
pub trait RemoteDesktopBridge: Send + Sync {
    /// Liveness probe of the bridge endpoint.
    fn health(&self) -> Result<BridgeHealth, BridgeCallFailure>;

    /// Opens a session to the remote host named by `host` and returns the
    /// initial screen observation.
    fn connect(&self, host: &str) -> Result<RemoteScreenObservation, BridgeCallFailure>;

    /// Executes one normalized action in the open session for `host`.
    fn execute(
        &self,
        host: &str,
        action: RemoteDesktopAction,
    ) -> Result<RemoteDesktopOutcome, BridgeCallFailure>;
}
