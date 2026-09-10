//! The mobile-device environment domain (WO-015).
//!
//! This module owns the mobile vocabulary: normalized device actions,
//! device observations, action outcomes, the host device-bridge port, and
//! the device-access policy. The vocabulary is adapter-owned — workflow
//! semantics never reference it. A mobile session is a host-bridged
//! interaction with a handset or emulator (adb/appium-style device
//! bridges); the bridge itself is supplied by the host
//! ([`MobileDeviceBridge`]), never provisioned here.

use serde::Deserialize;
use serde::Serialize;

use crate::bridge::BridgeCallFailure;
use crate::bridge::BridgeHealth;
use crate::policy::AccessTable;

/// The adapter identity registered with the capability registry.
pub const MOBILE_DEVICE_ADAPTER_ID: &str = "codex-mobile-device-env";
/// The semantic capability naming mobile device control.
pub const MOBILE_DEVICE_CAPABILITY: &str = "control_mobile_device";
/// The typed resource this environment requires before execution: an
/// opaque, credential-free device identity.
pub const MOBILE_DEVICE_RESOURCE: &str = "mobile_device";

/// One normalized mobile-device action.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub enum MobileDeviceAction {
    /// Tap at a screen position.
    Tap {
        /// Horizontal position in pixels.
        x: i32,
        /// Vertical position in pixels.
        y: i32,
    },
    /// Swipe between two screen positions.
    Swipe {
        /// Horizontal start position in pixels.
        start_x: i32,
        /// Vertical start position in pixels.
        start_y: i32,
        /// Horizontal end position in pixels.
        end_x: i32,
        /// Vertical end position in pixels.
        end_y: i32,
        /// Gesture duration in milliseconds.
        duration_ms: u64,
    },
    /// Enter text into the focused mobile application.
    TypeText {
        /// The text to enter (untrusted-derived bound).
        text: String,
    },
    /// Press one device key, for example `home` or `back`.
    PressKey {
        /// The key name.
        key: String,
    },
    /// Launch a mobile application by identifier.
    LaunchApplication {
        /// The application identifier the device should launch.
        app_id: String,
    },
    /// Capture the device screen.
    Screenshot,
    /// Report device information as an observation.
    DeviceInfo,
}

/// The class of device a bridge is attached to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MobileDeviceClass {
    /// A physical handset.
    Phone,
    /// A physical tablet.
    Tablet,
    /// An emulated device.
    Emulator,
}

/// One normalized mobile-device observation.
///
/// Observations are untrusted-derived: they are bounded by the execution
/// contracts when attached to action results and never trusted for
/// control decisions.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MobileDeviceObservation {
    /// The opaque device label the observation came from.
    pub device: String,
    /// The device class reported by the bridge.
    pub device_class: MobileDeviceClass,
    /// The foreground mobile application, when known.
    pub foreground_app: Option<String>,
    /// Screen width in pixels.
    pub width: u32,
    /// Screen height in pixels.
    pub height: u32,
    /// Bridge-supplied capture timestamp in unix milliseconds.
    pub captured_at_unix_ms: u64,
}

/// The outcome of one mobile-device action.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MobileDeviceOutcome {
    /// The observation captured after the action, when available.
    pub observation: Option<MobileDeviceObservation>,
    /// Bridge-supplied detail (untrusted-derived).
    pub detail: Option<String>,
}

/// Device-access policy for mobile sessions.
///
/// The table gates the devices named by attached `mobile_device` resource
/// bindings. Unspecified devices require explicit host authorization, so
/// the adapter refuses rather than silently defaulting to allow.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MobileDevicePolicy {
    /// Which devices the adapter may drive.
    pub device_access: AccessTable,
    /// Whether cross-owner session takeover may be recorded; denied when
    /// absent, mirroring the WO-007 session policy shape.
    pub allow_session_takeover: Option<bool>,
}

/// The host-provided mobile-device bridge.
///
/// The adapter never attaches to, launches, or re-implements a device
/// runtime: the host supplies this port backed by the real device bridge,
/// the same host-owns-the-runtime boundary WO-006 and WO-007 use. A
/// bridge that cannot answer [`MobileDeviceBridge::health`] surfaces as
/// an explicit not-installed readiness diagnostic.
pub trait MobileDeviceBridge: Send + Sync {
    /// Liveness probe of the bridge endpoint.
    fn health(&self) -> Result<BridgeHealth, BridgeCallFailure>;

    /// Attaches to the device named by `device` and returns the initial
    /// device observation.
    fn attach(&self, device: &str) -> Result<MobileDeviceObservation, BridgeCallFailure>;

    /// Executes one normalized action on the attached `device`.
    fn execute(
        &self,
        device: &str,
        action: MobileDeviceAction,
    ) -> Result<MobileDeviceOutcome, BridgeCallFailure>;
}
