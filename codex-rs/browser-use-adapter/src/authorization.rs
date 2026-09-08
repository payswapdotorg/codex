//! Requirement-vs-policy authorization pre-flight.
//!
//! `check_authorization` compares a workflow's normalized Browser Use
//! requirement against the resolved configuration snapshot and reports
//! violations with actionable remediation, plus items that must be
//! governed at session level (approvals, review controls), which the
//! native runtime enforces. This is a pre-flight only: it never grants
//! access and never bypasses Codex approvals or sandboxing.

use crate::config_snapshot::{BrowserUseConfigSnapshot, EffectiveOriginPolicy};
use crate::diagnostics::{AdapterDiagnostic, DiagnosticCode};
use crate::requirements::{
    AccessApprovalLifetime, AllowDeny, BrowserUseRequirement, OriginPolicyRequirement,
};
use serde::{Deserialize, Serialize};

/// An unsatisfied requirement, with remediation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuthorizationViolation {
    /// Origin the violation applies to; `None` for the default policy.
    pub origin: Option<String>,
    /// The unsatisfied field.
    pub field: &'static str,
    /// The required stance.
    pub required: String,
    /// The effective stance.
    pub effective: String,
    /// Actionable remediation; credential-free.
    pub remediation: String,
}

/// Items a session must govern at runtime. The native runtime enforces
/// these; the adapter records them so they can become approval evidence.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SessionGovernance {
    /// History access must be withheld during the session.
    HistoryAccessWithheld,
    /// WebMCP must be withheld during the session.
    WebmcpWithheld,
    /// Auto review suppression was requested.
    AutoReviewSuppressionRequested,
    /// Global persistent approval was requested.
    PersistentApprovalRequested,
    /// Origin-scoped auto review stance.
    OriginAutoReview {
        /// The origin.
        origin: String,
        /// Required stance.
        required: AllowDeny,
    },
    /// Origin-scoped persistent approval stance.
    OriginPersistentApproval {
        /// The origin.
        origin: String,
        /// Required stance.
        required: bool,
    },
    /// Origin-scoped approval lifetime.
    OriginApprovalLifetime {
        /// The origin.
        origin: String,
        /// Required lifetime.
        lifetime: AccessApprovalLifetime,
    },
}

/// The outcome of the authorization pre-flight.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AuthorizationOutcome {
    /// `true` when no violation was found.
    pub authorized: bool,
    /// Unsatisfied requirements.
    pub violations: Vec<AuthorizationViolation>,
    /// Session-level governance items derived from the requirement.
    pub session_governed: Vec<SessionGovernance>,
}

impl AuthorizationOutcome {
    /// Builds one actionable diagnostic per violation.
    pub fn diagnostics(&self) -> Vec<AdapterDiagnostic> {
        self.violations
            .iter()
            .map(|violation| {
                let code = if violation.field == "allow_history_access" {
                    DiagnosticCode::HistoryAccessConflict
                } else {
                    DiagnosticCode::OriginPolicyUnsatisfied
                };
                let scope = violation.origin.as_deref().unwrap_or("<default policy>");
                AdapterDiagnostic {
                    code,
                    message: format!(
                        "browser requirement not satisfied for {scope}: field {} requires {} \
                        but the effective policy is {}",
                        violation.field, violation.required, violation.effective
                    ),
                    remediation: vec![
                        violation.remediation.clone(),
                        "the native Codex Browser Use runtime remains the enforcement authority; \
                        this check is a pre-flight and never grants access"
                            .to_string(),
                    ],
                }
            })
            .collect()
    }
}

/// Checks a workflow requirement against the resolved configuration.
pub fn check_authorization(
    requirement: &BrowserUseRequirement,
    config: &BrowserUseConfigSnapshot,
) -> AuthorizationOutcome {
    let mut violations = Vec::new();
    let mut session_governed = Vec::new();

    if requirement.allow_history_access == Some(false) {
        if config.allow_history_access == Some(true) {
            violations.push(AuthorizationViolation {
                origin: None,
                field: "allow_history_access",
                required: "deny".to_string(),
                effective: "allow".to_string(),
                remediation: "remove the global browser use history-access grant, or scope it \
                    away from this workflow's origins"
                    .to_string(),
            });
        } else {
            session_governed.push(SessionGovernance::HistoryAccessWithheld);
        }
    }
    if requirement.allow_webmcp == Some(false) {
        session_governed.push(SessionGovernance::WebmcpWithheld);
    }
    if requirement.disable_auto_review == Some(true) {
        session_governed.push(SessionGovernance::AutoReviewSuppressionRequested);
    }
    if requirement.allow_global_persistent_approval == Some(true) {
        session_governed.push(SessionGovernance::PersistentApprovalRequested);
    }

    if let Some(default_requirement) = &requirement.default_origin_policy {
        let effective = config.effective_default_policy();
        check_origin_fields(
            None,
            default_requirement,
            effective,
            &mut violations,
            &mut session_governed,
        );
    }
    for (origin, origin_requirement) in &requirement.origins {
        let effective = config.effective_policy(origin);
        check_origin_fields(
            Some(origin.as_str()),
            origin_requirement,
            effective,
            &mut violations,
            &mut session_governed,
        );
    }

    AuthorizationOutcome {
        authorized: violations.is_empty(),
        violations,
        session_governed,
    }
}

fn check_origin_fields(
    origin: Option<&str>,
    required: &OriginPolicyRequirement,
    effective: EffectiveOriginPolicy,
    violations: &mut Vec<AuthorizationViolation>,
    session_governed: &mut Vec<SessionGovernance>,
) {
    let fields = [
        ("access", required.access, effective.access),
        ("downloads", required.downloads, effective.downloads),
        ("uploads", required.uploads, effective.uploads),
        (
            "full_cdp_access",
            required.full_cdp_access,
            effective.full_cdp_access,
        ),
    ];
    for (field, required_value, effective_value) in fields {
        if let Some(required_value) = required_value
            && required_value != effective_value
        {
            let remediation = match required_value {
                AllowDeny::Allow => format!(
                    "grant \"{field} = allow\" for this origin in the browser use origin policy"
                ),
                AllowDeny::Deny => format!(
                    "set \"{field} = deny\" for this origin in the browser use origin policy"
                ),
            };
            violations.push(AuthorizationViolation {
                origin: origin.map(str::to_string),
                field,
                required: format!("{required_value:?}").to_lowercase(),
                effective: format!("{effective_value:?}").to_lowercase(),
                remediation,
            });
        }
    }
    if let Some(auto_review) = required.auto_review {
        session_governed.push(SessionGovernance::OriginAutoReview {
            origin: origin.unwrap_or("<default>").to_string(),
            required: auto_review,
        });
    }
    if let Some(persistent_approval) = required.persistent_approval {
        session_governed.push(SessionGovernance::OriginPersistentApproval {
            origin: origin.unwrap_or("<default>").to_string(),
            required: persistent_approval,
        });
    }
    if let Some(lifetime) = required.access_approval_lifetime {
        session_governed.push(SessionGovernance::OriginApprovalLifetime {
            origin: origin.unwrap_or("<default>").to_string(),
            lifetime,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config_snapshot::OriginPolicySnapshot;
    use std::collections::BTreeMap;

    #[test]
    fn requirement_matching_default_policy_authorizes() {
        let requirement = BrowserUseRequirement {
            default_origin_policy: Some(OriginPolicyRequirement {
                access: Some(AllowDeny::Allow),
                downloads: Some(AllowDeny::Allow),
                ..Default::default()
            }),
            ..Default::default()
        };
        let config = BrowserUseConfigSnapshot {
            default_origin_policy: Some(OriginPolicySnapshot {
                access: Some(AllowDeny::Allow),
                downloads: Some(AllowDeny::Allow),
                uploads: None,
                full_cdp_access: None,
            }),
            ..Default::default()
        };
        let outcome = check_authorization(&requirement, &config);
        assert!(outcome.authorized);
        assert!(outcome.violations.is_empty());
    }

    #[test]
    fn unsatisfied_origin_requirement_produces_actionable_diagnostics() {
        let requirement = BrowserUseRequirement {
            origins: BTreeMap::from([(
                "https://example.com".to_string(),
                OriginPolicyRequirement {
                    access: Some(AllowDeny::Allow),
                    ..Default::default()
                },
            )]),
            ..Default::default()
        };
        let outcome = check_authorization(&requirement, &BrowserUseConfigSnapshot::default());
        assert!(!outcome.authorized);
        assert_eq!(outcome.violations.len(), 1);
        let violation = &outcome.violations[0];
        assert_eq!(violation.origin.as_deref(), Some("https://example.com"));
        assert_eq!(violation.field, "access");
        assert_eq!(violation.required, "allow");
        assert_eq!(violation.effective, "deny");
        let diagnostics = outcome.diagnostics();
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, DiagnosticCode::OriginPolicyUnsatisfied);
        assert!(!diagnostics[0].remediation.is_empty());
        assert!(diagnostics[0].message.contains("https://example.com"));
    }

    #[test]
    fn history_access_conflict_gets_a_dedicated_code() {
        let requirement = BrowserUseRequirement {
            allow_history_access: Some(false),
            ..Default::default()
        };
        let config = BrowserUseConfigSnapshot {
            allow_history_access: Some(true),
            ..Default::default()
        };
        let outcome = check_authorization(&requirement, &config);
        assert!(!outcome.authorized);
        let diagnostics = outcome.diagnostics();
        assert_eq!(diagnostics[0].code, DiagnosticCode::HistoryAccessConflict);
    }

    #[test]
    fn session_governance_items_are_collected() {
        let requirement = BrowserUseRequirement {
            allow_webmcp: Some(false),
            allow_history_access: Some(false),
            disable_auto_review: Some(true),
            allow_global_persistent_approval: Some(true),
            origins: BTreeMap::from([(
                "https://example.com".to_string(),
                OriginPolicyRequirement {
                    auto_review: Some(AllowDeny::Allow),
                    persistent_approval: Some(true),
                    access_approval_lifetime: Some(AccessApprovalLifetime::Thread),
                    ..Default::default()
                },
            )]),
            ..Default::default()
        };
        let outcome = check_authorization(&requirement, &BrowserUseConfigSnapshot::default());
        assert!(outcome.authorized);
        assert_eq!(outcome.session_governed.len(), 7);
        assert!(outcome.session_governed.iter().any(|item| matches!(
            item,
            SessionGovernance::OriginApprovalLifetime {
                lifetime: AccessApprovalLifetime::Thread,
                ..
            }
        )));
    }
}
