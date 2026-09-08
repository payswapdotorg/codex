//! Native computer use runtime bridge boundary.
//!
//! The adapter never implements a desktop agent runtime. The native Codex
//! Computer Use feature/skill/runtime path (configured through `codex-config`
//! computer use settings and surfaced through Codex tools, skills, approvals,
//! sandboxing, sessions, and tracing) is reached exclusively through this
//! trait boundary. The host execution integration supplies the
//! [`ComputerUseBridgeFactory`] implementation backed by the real native
//! runtime; this crate ships the contract, the lazy provisioning lifecycle,
//! and explicit readiness diagnostics.

use serde::Deserialize;
use serde::Serialize;

use crate::action::NormalizedAction;
use crate::action::NormalizedActionResult;
use crate::observation::ScreenObservation;

/// Identity of a provisioned bridge.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BridgeIdentity {
    /// Bridge kind, for example `codex-native-computer-use`.
    pub bridge_kind: String,
    /// Runtime backing the bridge, for example `node_repl`.
    pub runtime: String,
    /// Runtime/bridge version string reported by the native surface.
    pub version: String,
}

/// Why a bridge is not ready.
///
/// Every variant renders a remediation-oriented diagnostic via
/// [`BridgeReadinessFailure::diagnostic`] so missing `node_repl`, bridge, or
/// runtime provisioning is detected explicitly instead of silently
/// degrading.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum BridgeReadinessFailure {
    /// No bridge was provisioned for this Codex installation.
    #[serde(rename_all = "camelCase")]
    NotProvisioned {
        /// Expected bridge kind.
        expected_bridge_kind: String,
    },
    /// The runtime backing the bridge is missing.
    #[serde(rename_all = "camelCase")]
    RuntimeMissing {
        /// Runtime that is missing, for example `node_repl`.
        runtime: String,
    },
    /// The runtime answered but its version is unsupported.
    #[serde(rename_all = "camelCase")]
    VersionUnsupported {
        /// Version reported by the runtime.
        found: String,
        /// Minimum supported version.
        required: String,
    },
    /// The readiness probe failed for another reason.
    #[serde(rename_all = "camelCase")]
    ProbeFailed {
        /// Reason reported by the probe.
        reason: String,
    },
}

impl BridgeReadinessFailure {
    /// Remediation-oriented diagnostic for this failure.
    pub fn diagnostic(&self) -> String {
        match self {
            Self::NotProvisioned {
                expected_bridge_kind,
            } => format!(
                "computer use bridge '{expected_bridge_kind}' is not provisioned; \
                 provision the native Codex computer use bridge (node_repl/runtime) \
                 before binding workflow steps"
            ),
            Self::RuntimeMissing { runtime } => format!(
                "computer use runtime '{runtime}' is missing; install or enable the \
                 native runtime provisioning path"
            ),
            Self::VersionUnsupported { found, required } => format!(
                "computer use runtime version {found} is unsupported; \
                 {required} or newer is required"
            ),
            Self::ProbeFailed { reason } => {
                format!("computer use bridge readiness probe failed: {reason}")
            }
        }
    }
}

/// Why a bridge call could not produce an action result at all.
///
/// Action-level failures are `Ok` results carrying
/// [`crate::action::ActionStatus::Failed`]; this type is reserved for the
/// bridge itself being gone or failing transport.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum BridgeCallFailure {
    /// The bridge is gone (process exit, disconnect, runtime crash).
    #[serde(rename_all = "camelCase")]
    BridgeUnavailable {
        /// Details for diagnostics.
        details: String,
    },
    /// The bridge could not complete the call for another reason.
    #[serde(rename_all = "camelCase")]
    CallFailed {
        /// Details for diagnostics.
        details: String,
    },
}

impl BridgeCallFailure {
    /// Diagnostic for this failure.
    pub fn diagnostic(&self) -> String {
        match self {
            Self::BridgeUnavailable { details } => {
                format!("bridge unavailable: {details}")
            }
            Self::CallFailed { details } => format!("bridge call failed: {details}"),
        }
    }
}

/// Normalized observation/action boundary to the native computer use path.
pub trait ComputerUseBridge: Send + Sync {
    /// Identity reported by the provisioned bridge.
    fn identity(&self) -> BridgeIdentity;

    /// Probes readiness; `Ok(())` means actions may be executed.
    fn probe(&self) -> Result<(), BridgeReadinessFailure>;

    /// Executes one normalized action and returns its normalized result.
    fn execute(
        &self,
        action: NormalizedAction,
    ) -> Result<NormalizedActionResult, BridgeCallFailure>;

    /// Captures one normalized screen/application/window observation.
    fn observe(&self) -> Result<ScreenObservation, BridgeCallFailure>;
}

/// Provisions bridges lazily; implemented by the host integration over the
/// native Codex computer use runtime path.
pub trait ComputerUseBridgeFactory: Send + Sync {
    /// Provisions and returns a bridge ready to be probed.
    fn provision(&self) -> Result<Box<dyn ComputerUseBridge>, BridgeReadinessFailure>;
}

#[cfg(test)]
mod tests {
    use super::BridgeCallFailure;
    use super::BridgeReadinessFailure;

    #[test]
    fn readiness_failures_render_remediation_diagnostics() {
        let not_provisioned = BridgeReadinessFailure::NotProvisioned {
            expected_bridge_kind: "codex-native-computer-use".to_string(),
        };
        let diagnostic = not_provisioned.diagnostic();
        assert!(
            diagnostic.contains("not provisioned"),
            "diagnostic: {diagnostic}"
        );
        assert!(
            diagnostic.contains("node_repl"),
            "diagnostic must name the runtime: {diagnostic}"
        );

        let runtime_missing = BridgeReadinessFailure::RuntimeMissing {
            runtime: "node_repl".to_string(),
        };
        assert!(runtime_missing.diagnostic().contains("node_repl"));

        let unsupported = BridgeReadinessFailure::VersionUnsupported {
            found: "0.9".to_string(),
            required: "1.0".to_string(),
        };
        assert!(unsupported.diagnostic().contains("0.9"));

        let probe = BridgeReadinessFailure::ProbeFailed {
            reason: "timeout".to_string(),
        };
        assert!(probe.diagnostic().contains("timeout"));
    }

    #[test]
    fn bridge_call_failures_render_diagnostics() {
        let lost = BridgeCallFailure::BridgeUnavailable {
            details: "process exited".to_string(),
        };
        assert!(lost.diagnostic().contains("process exited"));
        let call = BridgeCallFailure::CallFailed {
            details: "ipc error".to_string(),
        };
        assert!(call.diagnostic().contains("ipc error"));
    }
}
