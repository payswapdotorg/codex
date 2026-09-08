//! Read-only snapshots of the native Browser Use configuration.
//!
//! The adapter never mutates configuration. A snapshot is taken when
//! `prepare` resolves readiness (lazy), and effective origin policies are
//! resolved with explicit precedence: origin override, then default
//! policy, then a conservative `Deny` pre-flight default. The native
//! runtime remains the enforcement authority; resolution here only lets
//! the adapter detect conflicts before binding.

use crate::requirements::AllowDeny;
use codex_config::{BrowserUseConfigToml, BrowserUseOriginPolicyConfigToml};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Snapshot of one origin's configured policy.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct OriginPolicySnapshot {
    /// Configured access stance.
    pub access: Option<AllowDeny>,
    /// Configured downloads stance.
    pub downloads: Option<AllowDeny>,
    /// Configured uploads stance.
    pub uploads: Option<AllowDeny>,
    /// Configured full CDP access stance.
    pub full_cdp_access: Option<AllowDeny>,
}

impl From<&BrowserUseOriginPolicyConfigToml> for OriginPolicySnapshot {
    fn from(value: &BrowserUseOriginPolicyConfigToml) -> Self {
        Self {
            access: value.access.map(AllowDeny::from),
            downloads: value.downloads.map(AllowDeny::from),
            uploads: value.uploads.map(AllowDeny::from),
            full_cdp_access: value.full_cdp_access.map(AllowDeny::from),
        }
    }
}

/// Snapshot of the Browser Use configuration relevant to the adapter.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct BrowserUseConfigSnapshot {
    /// Whether history access is globally granted.
    pub allow_history_access: Option<bool>,
    /// Configured default origin policy.
    pub default_origin_policy: Option<OriginPolicySnapshot>,
    /// Configured per-origin policies.
    pub origins: BTreeMap<String, OriginPolicySnapshot>,
}

impl From<&BrowserUseConfigToml> for BrowserUseConfigSnapshot {
    fn from(value: &BrowserUseConfigToml) -> Self {
        Self {
            allow_history_access: value.allow_history_access,
            default_origin_policy: value
                .default_origin_policy
                .as_ref()
                .map(OriginPolicySnapshot::from),
            origins: value
                .origins
                .as_ref()
                .map(|origins| {
                    origins
                        .iter()
                        .map(|(origin, policy)| {
                            (origin.clone(), OriginPolicySnapshot::from(policy))
                        })
                        .collect()
                })
                .unwrap_or_default(),
        }
    }
}

/// Where an effective policy resolution came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PolicySource {
    /// An explicit override for this origin.
    OriginOverride,
    /// The configured default policy.
    DefaultPolicy,
    /// Nothing is configured; the conservative pre-flight default applies.
    ConservativeDefault,
}

/// Effective policy fields resolved for one origin (or the default policy).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EffectiveOriginPolicy {
    /// Effective access stance.
    pub access: AllowDeny,
    /// Effective downloads stance.
    pub downloads: AllowDeny,
    /// Effective uploads stance.
    pub uploads: AllowDeny,
    /// Effective full CDP access stance.
    pub full_cdp_access: AllowDeny,
    /// Where this resolution came from.
    pub source: PolicySource,
}

fn resolve(override_value: Option<AllowDeny>, default_value: Option<AllowDeny>) -> AllowDeny {
    override_value.or(default_value).unwrap_or(AllowDeny::Deny)
}

impl BrowserUseConfigSnapshot {
    /// Resolves the effective policy for `origin` (exact string match; no
    /// wildcard semantics are invented here).
    pub fn effective_policy(&self, origin: &str) -> EffectiveOriginPolicy {
        let over = self.origins.get(origin);
        let default = self.default_origin_policy.as_ref();
        let source = if over.is_some() {
            PolicySource::OriginOverride
        } else if default.is_some() {
            PolicySource::DefaultPolicy
        } else {
            PolicySource::ConservativeDefault
        };
        EffectiveOriginPolicy {
            access: resolve(
                over.and_then(|policy| policy.access),
                default.and_then(|policy| policy.access),
            ),
            downloads: resolve(
                over.and_then(|policy| policy.downloads),
                default.and_then(|policy| policy.downloads),
            ),
            uploads: resolve(
                over.and_then(|policy| policy.uploads),
                default.and_then(|policy| policy.uploads),
            ),
            full_cdp_access: resolve(
                over.and_then(|policy| policy.full_cdp_access),
                default.and_then(|policy| policy.full_cdp_access),
            ),
            source,
        }
    }

    /// Resolves the effective default policy (used for a workflow's
    /// `default_origin_policy` requirement).
    pub fn effective_default_policy(&self) -> EffectiveOriginPolicy {
        let default = self.default_origin_policy.as_ref();
        EffectiveOriginPolicy {
            access: resolve(default.and_then(|policy| policy.access), None),
            downloads: resolve(default.and_then(|policy| policy.downloads), None),
            uploads: resolve(default.and_then(|policy| policy.uploads), None),
            full_cdp_access: resolve(default.and_then(|policy| policy.full_cdp_access), None),
            source: if default.is_some() {
                PolicySource::DefaultPolicy
            } else {
                PolicySource::ConservativeDefault
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use codex_config::AllowDenyRequirementToml;

    #[test]
    fn snapshot_from_native_config_toml() {
        let toml = BrowserUseConfigToml {
            allow_history_access: Some(true),
            default_origin_policy: Some(BrowserUseOriginPolicyConfigToml {
                access: Some(AllowDenyRequirementToml::Allow),
                downloads: None,
                uploads: None,
                full_cdp_access: None,
            }),
            origins: None,
        };
        let snapshot = BrowserUseConfigSnapshot::from(&toml);
        assert_eq!(snapshot.allow_history_access, Some(true));
        assert_eq!(
            snapshot
                .default_origin_policy
                .as_ref()
                .and_then(|policy| policy.access),
            Some(AllowDeny::Allow)
        );
        assert!(snapshot.origins.is_empty());
    }

    #[test]
    fn effective_policy_prefers_override_then_default_then_deny() {
        let config = BrowserUseConfigSnapshot {
            allow_history_access: None,
            default_origin_policy: Some(OriginPolicySnapshot {
                access: Some(AllowDeny::Allow),
                downloads: None,
                uploads: Some(AllowDeny::Deny),
                full_cdp_access: None,
            }),
            origins: BTreeMap::from([(
                "https://override.example".to_string(),
                OriginPolicySnapshot {
                    access: Some(AllowDeny::Deny),
                    downloads: None,
                    uploads: None,
                    full_cdp_access: Some(AllowDeny::Allow),
                },
            )]),
        };
        let overridden = config.effective_policy("https://override.example");
        assert_eq!(overridden.access, AllowDeny::Deny);
        assert_eq!(overridden.downloads, AllowDeny::Deny);
        assert_eq!(overridden.uploads, AllowDeny::Deny);
        assert_eq!(overridden.full_cdp_access, AllowDeny::Allow);
        assert_eq!(overridden.source, PolicySource::OriginOverride);

        let untouched = config.effective_policy("https://other.example");
        assert_eq!(untouched.access, AllowDeny::Allow);
        assert_eq!(untouched.uploads, AllowDeny::Deny);
        assert_eq!(untouched.source, PolicySource::DefaultPolicy);

        let bare = BrowserUseConfigSnapshot::default();
        let unset = bare.effective_policy("https://x.example");
        assert_eq!(unset.access, AllowDeny::Deny);
        assert_eq!(unset.source, PolicySource::ConservativeDefault);
    }
}
