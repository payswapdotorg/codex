//! Binding identity, session identity, and fallback adapter policy.
//!
//! The binding identity is what makes "records exact capability/binding
//! identity" auditable: it names the universal capability, the selected
//! adapter (native, or a recorded compatible fallback), the concrete
//! browser session (profile, session id, tabs, takeover lineage), and the
//! origin scope — and exposes a canonical SHA-256 fingerprint over all of
//! it. Identity strings are credential-free by construction.

use crate::canonical::canonical_digest;
use crate::diagnostics::{AdapterDiagnostic, DiagnosticCode};
use crate::requirements::FallbackNeeds;
use serde::{Deserialize, Serialize};

/// The universal capability id this adapter binds workflow execution to.
pub const BROWSER_USE_CAPABILITY_ID: &str = "codex.browser-use";

/// Which adapter implementation a binding uses. The native Codex Browser
/// Use capability is always preferred.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum BrowserAdapterKind {
    /// The native Codex Browser Use capability.
    Native,
    /// A compatible fallback adapter, recorded explicitly.
    Fallback(FallbackAdapter),
}

/// A fallback browser adapter and the policy capabilities it declares.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FallbackAdapter {
    /// Stable adapter name; never contains credentials.
    pub name: String,
    /// Policy-relevant capabilities the adapter declares.
    pub declared_capabilities: FallbackCapabilities,
}

/// Policy-relevant capabilities a browser adapter can declare. A fallback
/// may only be used when the declared capabilities keep the workflow's
/// semantic/policy requirements satisfied.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct FallbackCapabilities {
    /// Can scope sessions to specific origins.
    pub origin_scoping: bool,
    /// Can withhold history access.
    pub history_governance: bool,
    /// Can withhold WebMCP.
    pub webmcp_governance: bool,
    /// Supports review controls (auto review suppression).
    pub review_controls: bool,
    /// Supports persistent approval controls.
    pub persistent_approval_controls: bool,
    /// Can capture evidence (observations/artifacts/traces).
    pub evidence_capture: bool,
    /// Supports session take-over.
    pub takeover: bool,
    /// Supports browser profiles.
    pub profiles: bool,
    /// Supports tab management.
    pub tabs: bool,
}

/// The result of evaluating a fallback against derived needs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FallbackEvaluation {
    /// `true` when every required capability is declared.
    pub compatible: bool,
    /// Names of unmet capabilities (empty when compatible).
    pub unmet: Vec<String>,
}

/// Evaluates whether a fallback satisfies the requirement-derived needs.
///
/// Evidence capture, profiles, tabs, and take-over are required of every
/// fallback because the workflow contract depends on them; the remaining
/// capabilities are required only when the requirement derives the
/// corresponding need.
pub fn evaluate_fallback(
    needs: FallbackNeeds,
    capabilities: &FallbackCapabilities,
) -> FallbackEvaluation {
    let mut unmet = Vec::new();
    if !capabilities.evidence_capture {
        unmet.push("evidence_capture".to_string());
    }
    if !capabilities.profiles {
        unmet.push("profiles".to_string());
    }
    if !capabilities.tabs {
        unmet.push("tabs".to_string());
    }
    if !capabilities.takeover {
        unmet.push("takeover".to_string());
    }
    if needs.origin_scoping && !capabilities.origin_scoping {
        unmet.push("origin_scoping".to_string());
    }
    if needs.history_governance && !capabilities.history_governance {
        unmet.push("history_governance".to_string());
    }
    if needs.webmcp_governance && !capabilities.webmcp_governance {
        unmet.push("webmcp_governance".to_string());
    }
    if needs.review_controls && !capabilities.review_controls {
        unmet.push("review_controls".to_string());
    }
    if needs.persistent_approval_controls && !capabilities.persistent_approval_controls {
        unmet.push("persistent_approval_controls".to_string());
    }
    FallbackEvaluation {
        compatible: unmet.is_empty(),
        unmet,
    }
}

/// Durable identity of one capability binding.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BindingIdentity {
    /// The universal capability this binding refers to.
    pub capability_id: String,
    /// Which adapter implementation is bound.
    pub adapter: BrowserAdapterKind,
    /// The concrete browser session.
    pub session: BrowserSessionIdentity,
    /// Origins the binding is scoped to.
    pub origin_scope: Vec<String>,
}

impl BindingIdentity {
    /// Canonical SHA-256 fingerprint over the full identity. Recorded in
    /// evidence so audits can pin the exact binding that produced each
    /// artifact.
    pub fn fingerprint_hex(&self) -> String {
        canonical_digest(self).1
    }
}

/// The concrete browser session a binding points at.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserSessionIdentity {
    /// Browser profile, when one is used.
    pub profile: Option<String>,
    /// Native or fallback session identifier.
    pub session_id: String,
    /// Tab identifiers in scope.
    pub tabs: Vec<String>,
    /// Session id this session took over, when created by a take-over.
    pub takeover_of: Option<String>,
}

/// Record that a fallback was selected; always emitted as evidence.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct FallbackRecord {
    /// The selected fallback adapter's name.
    pub adapter_name: String,
    /// Why the fallback was selected (the native gap).
    pub note: String,
}

/// Builds the diagnostic emitted when a fallback cannot be used.
pub fn fallback_incompatible_diagnostic(unmet: &[String]) -> AdapterDiagnostic {
    AdapterDiagnostic {
        code: DiagnosticCode::FallbackIncompatible,
        message: format!(
            "fallback browser adapter does not satisfy the workflow's semantic/policy \
            requirements; unmet: {}",
            unmet.join(", ")
        ),
        remediation: vec![
            "choose a fallback adapter that declares every required capability".to_string(),
            "or provision the native Codex Browser Use bridge/runtime".to_string(),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fallback_evaluation_requires_core_capabilities() {
        let needs = FallbackNeeds::default();
        let strong = FallbackCapabilities {
            evidence_capture: true,
            profiles: true,
            tabs: true,
            takeover: true,
            ..Default::default()
        };
        assert!(evaluate_fallback(needs, &strong).compatible);

        let weak = FallbackCapabilities::default();
        let evaluation = evaluate_fallback(needs, &weak);
        assert!(!evaluation.compatible);
        assert!(
            evaluation
                .unmet
                .iter()
                .any(|name| name == "evidence_capture")
        );
        assert!(evaluation.unmet.iter().any(|name| name == "takeover"));
    }

    #[test]
    fn fallback_evaluation_checks_policy_needs() {
        let needs = FallbackNeeds {
            origin_scoping: true,
            ..Default::default()
        };
        let capabilities = FallbackCapabilities {
            evidence_capture: true,
            profiles: true,
            tabs: true,
            takeover: true,
            origin_scoping: false,
            ..Default::default()
        };
        let evaluation = evaluate_fallback(needs, &capabilities);
        assert!(!evaluation.compatible);
        assert_eq!(evaluation.unmet, ["origin_scoping"]);
    }

    #[test]
    fn binding_fingerprint_is_stable_and_identity_sensitive() {
        let base = BindingIdentity {
            capability_id: BROWSER_USE_CAPABILITY_ID.to_string(),
            adapter: BrowserAdapterKind::Native,
            session: BrowserSessionIdentity {
                profile: Some("work".to_string()),
                session_id: "s1".to_string(),
                tabs: vec!["tab-1".to_string()],
                takeover_of: None,
            },
            origin_scope: vec!["https://example.com".to_string()],
        };
        assert_eq!(base.fingerprint_hex(), base.clone().fingerprint_hex());
        let mut other = base.clone();
        other.session.session_id = "s2".to_string();
        assert_ne!(base.fingerprint_hex(), other.fingerprint_hex());
        let mut swapped = base.clone();
        swapped.adapter = BrowserAdapterKind::Fallback(FallbackAdapter {
            name: "fallback".to_string(),
            declared_capabilities: FallbackCapabilities::default(),
        });
        assert_ne!(base.fingerprint_hex(), swapped.fingerprint_hex());
    }
}
