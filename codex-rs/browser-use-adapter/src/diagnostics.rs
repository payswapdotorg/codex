//! Explicit, actionable diagnostics for missing capability provisioning.
//!
//! Diagnostics are the adapter's channel for "the native Codex Browser Use
//! path is not usable in this environment". They are credential-free by
//! construction and never suggest bypassing Codex approvals or sandboxing.

use serde::{Deserialize, Serialize};

/// Stable, machine-readable diagnostic codes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DiagnosticCode {
    /// No native Browser Use bridge is present in this environment.
    NativeBridgeMissing,
    /// A bridge exists but the Browser Use runtime is not provisioned.
    NativeRuntimeNotProvisioned,
    /// A workflow browser requirement conflicts with the effective policy.
    OriginPolicyUnsatisfied,
    /// The workflow forbids history access but configuration grants it.
    HistoryAccessConflict,
    /// A fallback adapter does not satisfy the workflow's semantic/policy
    /// needs.
    FallbackIncompatible,
}

/// One actionable, credential-free diagnostic.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdapterDiagnostic {
    /// Stable code for programmatic handling.
    pub code: DiagnosticCode,
    /// Human-readable summary.
    pub message: String,
    /// Ordered remediation steps.
    pub remediation: Vec<String>,
}

/// What the host environment reports about the native Browser Use path.
///
/// The adapter never launches browsers or daemons (that would be a second
/// runtime); probing is a host responsibility reported through this plain
/// value. This is also what makes lazy initialization trivial: nothing is
/// probed until the host decides to call `prepare`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct BridgeProbe {
    /// Whether the native Browser Use bridge/feature is present at all.
    pub native_bridge_present: bool,
    /// Whether the native runtime behind the bridge is provisioned.
    pub native_runtime_provisioned: bool,
    /// Optional host-provided detail; the host must redact credentials.
    pub detail: Option<String>,
}

impl BridgeProbe {
    /// Returns the detected native gap together with explicit, actionable
    /// diagnostics, or `None` when the probe reports a healthy native path.
    pub fn gap(&self) -> Option<(DiagnosticCode, Vec<AdapterDiagnostic>)> {
        if !self.native_bridge_present {
            let mut remediation = vec![
                "enable the native Codex Browser Use capability/bridge for this environment"
                    .to_string(),
                "provision the browser runtime the native bridge expects, then re-run readiness"
                    .to_string(),
"or supply a fallback browser adapter that satisfies the workflow's browser  policy requirements; a selected fallback is always recorded as evidence"
                    .to_string(),
"browser workflows stay blocked until then; unrelated workflows and chat  startup are never blocked by this check"
                    .to_string(),
            ];
            if let Some(detail) = &self.detail {
                remediation.insert(0, format!("environment detail: {detail}"));
            }
            return Some((
                DiagnosticCode::NativeBridgeMissing,
                vec![AdapterDiagnostic {
                    code: DiagnosticCode::NativeBridgeMissing,
                    message:
                        "native Codex Browser Use bridge not found; refusing to start a second browser agent"
                            .to_string(),
                    remediation,
                }],
            ));
        }
        if !self.native_runtime_provisioned {
            let mut remediation = vec![
"provision the Browser Use runtime (bridge runtime/browser profile) for this  environment"
                    .to_string(),
                "verify native Codex Browser Use readiness before retrying".to_string(),
"or supply a fallback browser adapter that satisfies the workflow's browser  policy requirements; a selected fallback is always recorded as evidence"
                    .to_string(),
            ];
            if let Some(detail) = &self.detail {
                remediation.insert(0, format!("environment detail: {detail}"));
            }
            return Some((
                DiagnosticCode::NativeRuntimeNotProvisioned,
                vec![AdapterDiagnostic {
                    code: DiagnosticCode::NativeRuntimeNotProvisioned,
                    message:
                        "native Codex Browser Use bridge found but its runtime is not provisioned"
                            .to_string(),
                    remediation,
                }],
            ));
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn healthy_probe_reports_no_gap() {
        let probe = BridgeProbe {
            native_bridge_present: true,
            native_runtime_provisioned: true,
            detail: None,
        };
        assert!(probe.gap().is_none());
    }

    #[test]
    fn missing_bridge_reports_explicit_actionable_diagnostics() {
        let probe = BridgeProbe {
            native_bridge_present: false,
            native_runtime_provisioned: false,
            detail: Some("no bridge registered".to_string()),
        };
        let (code, diagnostics) = probe.gap().expect("gap expected");
        assert_eq!(code, DiagnosticCode::NativeBridgeMissing);
        assert_eq!(diagnostics.len(), 1);
        assert!(!diagnostics[0].remediation.is_empty());
        assert!(diagnostics[0].remediation[0].contains("no bridge registered"));
    }

    #[test]
    fn unprovisioned_runtime_is_reported_distinctly() {
        let probe = BridgeProbe {
            native_bridge_present: true,
            native_runtime_provisioned: false,
            detail: None,
        };
        let (code, _) = probe.gap().expect("gap expected");
        assert_eq!(code, DiagnosticCode::NativeRuntimeNotProvisioned);
    }
}
