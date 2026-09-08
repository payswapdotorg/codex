//! Normalized browser use requirements.
//!
//! These are the adapter-neutral forms of the native Codex config surfaces
//! (`codex-config`: `BrowserUseRequirementsToml` and its origin policy
//! types). Conversion is one-directional and lossless: native configuration
//! remains the source of truth and the adapter never writes config back.
//! Keeping the adapter-facing types neutral also keeps browser-specific
//! detail out of workflow semantic types.

use codex_config::{
    AllowDenyRequirementToml, BrowserUseAccessApprovalLifetimeToml, BrowserUseOriginPolicyToml,
    BrowserUseRequirementsToml,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Whether a policy aspect must be allowed or denied.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AllowDeny {
    /// The aspect must be allowed.
    Allow,
    /// The aspect must be denied.
    Deny,
}

impl From<AllowDenyRequirementToml> for AllowDeny {
    fn from(value: AllowDenyRequirementToml) -> Self {
        match value {
            AllowDenyRequirementToml::Allow => AllowDeny::Allow,
            AllowDenyRequirementToml::Deny => AllowDeny::Deny,
        }
    }
}

/// Approval lifetime semantics for origin access approvals.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AccessApprovalLifetime {
    /// Approval lives for one turn.
    Turn,
    /// Approval lives for the whole thread.
    Thread,
}

impl From<BrowserUseAccessApprovalLifetimeToml> for AccessApprovalLifetime {
    fn from(value: BrowserUseAccessApprovalLifetimeToml) -> Self {
        match value {
            BrowserUseAccessApprovalLifetimeToml::Turn => AccessApprovalLifetime::Turn,
            BrowserUseAccessApprovalLifetimeToml::Thread => AccessApprovalLifetime::Thread,
        }
    }
}

/// A workflow requirement over one origin, or over the default policy.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct OriginPolicyRequirement {
    /// Required page/network access stance.
    pub access: Option<AllowDeny>,
    /// Required downloads stance.
    pub downloads: Option<AllowDeny>,
    /// Required uploads stance.
    pub uploads: Option<AllowDeny>,
    /// Required full CDP access stance.
    pub full_cdp_access: Option<AllowDeny>,
    /// Required auto-review stance.
    pub auto_review: Option<AllowDeny>,
    /// Required persistent-approval stance.
    pub persistent_approval: Option<bool>,
    /// Required approval lifetime.
    pub access_approval_lifetime: Option<AccessApprovalLifetime>,
}

impl From<&BrowserUseOriginPolicyToml> for OriginPolicyRequirement {
    fn from(value: &BrowserUseOriginPolicyToml) -> Self {
        Self {
            access: value.access.map(AllowDeny::from),
            downloads: value.downloads.map(AllowDeny::from),
            uploads: value.uploads.map(AllowDeny::from),
            full_cdp_access: value.full_cdp_access.map(AllowDeny::from),
            auto_review: value.auto_review.map(AllowDeny::from),
            persistent_approval: value.persistent_approval,
            access_approval_lifetime: value
                .access_approval_lifetime
                .map(AccessApprovalLifetime::from),
        }
    }
}

/// A workflow's normalized Browser Use requirement.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct BrowserUseRequirement {
    /// Whether WebMCP must be allowed (`false` forbids it).
    pub allow_webmcp: Option<bool>,
    /// Whether history access must be allowed (`false` forbids it).
    pub allow_history_access: Option<bool>,
    /// Whether auto review must be disabled.
    pub disable_auto_review: Option<bool>,
    /// Whether global persistent approval is requested.
    pub allow_global_persistent_approval: Option<bool>,
    /// Requirement over the default origin policy.
    pub default_origin_policy: Option<OriginPolicyRequirement>,
    /// Requirements over specific origins.
    pub origins: BTreeMap<String, OriginPolicyRequirement>,
}

impl From<&BrowserUseRequirementsToml> for BrowserUseRequirement {
    fn from(value: &BrowserUseRequirementsToml) -> Self {
        Self {
            allow_webmcp: value.allow_webmcp,
            allow_history_access: value.allow_history_access,
            disable_auto_review: value.disable_auto_review,
            allow_global_persistent_approval: value.allow_global_persistent_approval,
            default_origin_policy: value
                .default_origin_policy
                .as_ref()
                .map(OriginPolicyRequirement::from),
            origins: value
                .origins
                .as_ref()
                .map(|origins| {
                    origins
                        .iter()
                        .map(|(origin, policy)| {
                            (origin.clone(), OriginPolicyRequirement::from(policy))
                        })
                        .collect()
                })
                .unwrap_or_default(),
        }
    }
}

impl BrowserUseRequirement {
    /// `true` when the requirement constrains nothing.
    pub fn is_empty(&self) -> bool {
        self.allow_webmcp.is_none()
            && self.allow_history_access.is_none()
            && self.disable_auto_review.is_none()
            && self.allow_global_persistent_approval.is_none()
            && self.default_origin_policy.is_none()
            && self.origins.is_empty()
    }

    /// Derives the policy-relevant capabilities a fallback browser adapter
    /// must declare to remain compatible with this requirement.
    pub fn fallback_needs(&self) -> FallbackNeeds {
        let mut policies = self
            .default_origin_policy
            .iter()
            .chain(self.origins.values());
        FallbackNeeds {
            origin_scoping: self.default_origin_policy.is_some() || !self.origins.is_empty(),
            history_governance: self.allow_history_access == Some(false),
            webmcp_governance: self.allow_webmcp == Some(false),
            review_controls: self.disable_auto_review == Some(true)
                || policies.clone().any(|policy| policy.auto_review.is_some()),
            persistent_approval_controls: self.allow_global_persistent_approval == Some(true)
                || policies.any(|policy| policy.persistent_approval.is_some()),
        }
    }
}

/// Policy-relevant capabilities derived from a requirement; a fallback
/// adapter must cover every derived need to be compatible.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct FallbackNeeds {
    /// The adapter must scope sessions to the workflow's origins.
    pub origin_scoping: bool,
    /// The adapter must be able to withhold history access.
    pub history_governance: bool,
    /// The adapter must be able to withhold WebMCP.
    pub webmcp_governance: bool,
    /// The adapter must support review controls (auto review suppression).
    pub review_controls: bool,
    /// The adapter must support persistent approval controls.
    pub persistent_approval_controls: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requirement_from_native_toml_is_lossless() {
        let toml = BrowserUseRequirementsToml {
            allow_webmcp: Some(false),
            allow_history_access: Some(false),
            disable_auto_review: None,
            allow_global_persistent_approval: None,
            default_origin_policy: Some(BrowserUseOriginPolicyToml {
                access: Some(AllowDenyRequirementToml::Allow),
                ..Default::default()
            }),
            origins: Some(BTreeMap::from([(
                "https://example.com".to_string(),
                BrowserUseOriginPolicyToml {
                    access: Some(AllowDenyRequirementToml::Deny),
                    access_approval_lifetime: Some(BrowserUseAccessApprovalLifetimeToml::Thread),
                    ..Default::default()
                },
            )])),
        };
        let requirement = BrowserUseRequirement::from(&toml);
        assert_eq!(requirement.allow_webmcp, Some(false));
        assert_eq!(
            requirement
                .default_origin_policy
                .as_ref()
                .and_then(|policy| policy.access),
            Some(AllowDeny::Allow)
        );
        let origin = requirement
            .origins
            .get("https://example.com")
            .expect("origin");
        assert_eq!(origin.access, Some(AllowDeny::Deny));
        assert_eq!(
            origin.access_approval_lifetime,
            Some(AccessApprovalLifetime::Thread)
        );
        assert!(!requirement.is_empty());
    }

    #[test]
    fn default_requirement_is_empty_and_needs_nothing() {
        let requirement = BrowserUseRequirement::default();
        assert!(requirement.is_empty());
        assert!(!requirement.fallback_needs().origin_scoping);
        assert!(!requirement.fallback_needs().history_governance);
    }

    #[test]
    fn fallback_needs_are_derived_from_constraints() {
        let requirement = BrowserUseRequirement {
            allow_webmcp: Some(false),
            allow_history_access: Some(false),
            origins: BTreeMap::from([(
                "https://example.com".to_string(),
                OriginPolicyRequirement {
                    auto_review: Some(AllowDeny::Deny),
                    persistent_approval: Some(true),
                    ..Default::default()
                },
            )]),
            ..Default::default()
        };
        let needs = requirement.fallback_needs();
        assert!(needs.origin_scoping);
        assert!(needs.history_governance);
        assert!(needs.webmcp_governance);
        assert!(needs.review_controls);
        assert!(needs.persistent_approval_controls);
    }
}
