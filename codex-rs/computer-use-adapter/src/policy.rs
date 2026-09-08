//! Normalized computer use policy inputs.
//!
//! These types are the adapter's normalized projection of the native Codex
//! computer use policy surfaces (`codex-config`'s `ComputerUseConfigToml` and
//! `ComputerUseRequirementsToml`, backed by `config/src/computer_use.rs` and
//! `config/src/browser_computer_use_requirements.rs`). Per the capability
//! implementation map, default application access and platform-specific
//! application identity constraints (macOS bundle ids, Windows AUMIDs, exe
//! identities) are resource/policy inputs: they gate adapter behavior but are
//! never workflow semantics and never appear in workflow contract types.

use serde::Deserialize;
use serde::Serialize;
use std::collections::BTreeMap;

use crate::util::canonical_digest;

/// Allow/deny requirement, mirroring the native `AllowDenyRequirementToml`
/// shape.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AccessRequirement {
    /// Access is allowed.
    Allow,
    /// Access is denied.
    Deny,
}

/// Resolved access decision for one application identity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AccessDecision {
    /// A policy table explicitly allows the application.
    Allowed,
    /// A policy table explicitly denies the application.
    Denied,
    /// No policy entry applies; explicit host authorization is required.
    Unspecified,
}

/// Platform-specific application identity.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ApplicationPlatformIdentity {
    /// macOS application identified by bundle id.
    #[serde(rename_all = "camelCase")]
    MacosBundle {
        /// macOS bundle identifier, for example `com.example.app`.
        bundle_id: String,
    },
    /// Windows application identified by AUMID.
    #[serde(rename_all = "camelCase")]
    WindowsAumid {
        /// Windows Application User Model ID.
        aumid: String,
    },
    /// Windows application identified by publisher/product/binary triple.
    #[serde(rename_all = "camelCase")]
    WindowsExe {
        /// Publisher name from the signed binary.
        publisher_name: String,
        /// Product name from the signed binary.
        product_name: String,
        /// Optional binary file name.
        binary_name: Option<String>,
    },
    /// Platform-neutral identity with an optional binary hint.
    #[serde(rename_all = "camelCase")]
    Generic {
        /// Optional binary file name hint.
        binary_name: Option<String>,
    },
}

/// One Windows exe identity policy entry, mirroring the native
/// `ComputerUseWindowsExeConfigToml` shape.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WindowsExeIdentity {
    /// Publisher name from the signed binary.
    pub publisher_name: String,
    /// Product name from the signed binary.
    pub product_name: String,
    /// Optional binary file name.
    pub binary_name: Option<String>,
    /// Allow/deny requirement for this identity.
    pub access: AccessRequirement,
}

/// An application a step may target.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ApplicationIdentity {
    /// Human-readable display name (diagnostics only, never authority).
    pub display_name: String,
    /// Platform-specific identity.
    pub platform: ApplicationPlatformIdentity,
}

impl ApplicationIdentity {
    /// Stable digest of this identity; used in authorization records so
    /// application identity is auditable without embedding raw names.
    pub fn digest(&self) -> String {
        canonical_digest(self)
    }
}

/// Normalized computer use policy input.
///
/// The native `codex-config` computer use settings remain the policy
/// authority; the host projects them into this shape when constructing the
/// adapter. The adapter-level session flag `allow_session_takeover` gates
/// cross-owner forced takeover (denied by default) and is an explicit
/// adapter policy boundary, not a native config field.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ComputerUsePolicy {
    /// Default application access when no table entry applies.
    pub default_app_access: Option<AccessRequirement>,
    /// macOS per-bundle-id requirements.
    pub macos_bundle_ids: BTreeMap<String, AccessRequirement>,
    /// Windows per-AUMID requirements.
    pub windows_aumids: BTreeMap<String, AccessRequirement>,
    /// Windows exe identity requirements.
    pub windows_exe_identities: Vec<WindowsExeIdentity>,
    /// Whether persistent approval is allowed (mirrors native requirement).
    pub allow_persistent_approval: Option<bool>,
    /// Whether computer use while the machine is locked is allowed.
    pub allow_locked_computer_use: Option<bool>,
    /// Adapter session policy: whether cross-owner forced takeover may be
    /// recorded. Denied when absent.
    pub allow_session_takeover: Option<bool>,
}

impl ComputerUsePolicy {
    /// Returns true when no policy input constrains anything.
    pub fn is_empty(&self) -> bool {
        self.default_app_access.is_none()
            && self.macos_bundle_ids.is_empty()
            && self.windows_aumids.is_empty()
            && self.windows_exe_identities.is_empty()
            && self.allow_persistent_approval.is_none()
            && self.allow_locked_computer_use.is_none()
            && self.allow_session_takeover.is_none()
    }

    /// Resolves the access decision for `application`.
    ///
    /// Resolution order: platform-specific exact match (bundle id, AUMID, or
    /// exe identity), then `default_app_access`, then `Unspecified`, which
    /// requires explicit host authorization.
    pub fn decide(&self, application: &ApplicationIdentity) -> AccessDecision {
        let specific = match &application.platform {
            ApplicationPlatformIdentity::MacosBundle { bundle_id } => {
                self.macos_bundle_ids.get(bundle_id).copied()
            }
            ApplicationPlatformIdentity::WindowsAumid { aumid } => {
                self.windows_aumids.get(aumid).copied()
            }
            ApplicationPlatformIdentity::WindowsExe {
                publisher_name,
                product_name,
                binary_name,
            } => self
                .windows_exe_identities
                .iter()
                .find(|entry| {
                    entry.publisher_name == *publisher_name
                        && entry.product_name == *product_name
                        && match (&entry.binary_name, binary_name) {
                            (Some(entry_binary), Some(target_binary)) => {
                                entry_binary == target_binary
                            }
                            _ => true,
                        }
                })
                .map(|entry| entry.access),
            ApplicationPlatformIdentity::Generic { .. } => None,
        };
        match specific {
            Some(AccessRequirement::Allow) => AccessDecision::Allowed,
            Some(AccessRequirement::Deny) => AccessDecision::Denied,
            None => match self.default_app_access {
                Some(AccessRequirement::Allow) => AccessDecision::Allowed,
                Some(AccessRequirement::Deny) => AccessDecision::Denied,
                None => AccessDecision::Unspecified,
            },
        }
    }

    /// Whether a cross-owner takeover is permitted at all. Only forced
    /// takeovers may be granted, and only when the policy allows them.
    pub fn takeover_allowed(&self, forced: bool) -> bool {
        forced && self.allow_session_takeover.unwrap_or(false)
    }

    /// Stable digest of the policy in force; recorded in bindings and
    /// fallback records so audits can reproduce the exact policy identity.
    pub fn policy_digest(&self) -> String {
        canonical_digest(self)
    }
}

/// Source of an authorization decision for a target application.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub enum AuthorizationSource {
    /// The native computer use policy explicitly allows the application.
    PolicyAllow,
    /// A host approval (Codex approvals remain the authority) covers it.
    HostApproval(HostApproval),
}

/// A host-supplied approval covering one application authorization.
///
/// Only the opaque approval identifier is recorded; credential material
/// never enters this crate.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HostApproval {
    /// Opaque approval identifier from the Codex approvals plane.
    pub approval_id: String,
}

/// One recorded authorization decision for a target application.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AuthorizationRecord {
    /// Digest of the authorized application identity.
    pub subject_digest: String,
    /// Display identity of the authorized application.
    pub subject_display: String,
    /// Why the authorization holds.
    pub source: AuthorizationSource,
    /// When the decision was recorded, in unix milliseconds.
    pub recorded_at_unix_ms: u64,
}

#[cfg(test)]
mod tests {
    use super::AccessDecision;
    use super::AccessRequirement;
    use super::ApplicationIdentity;
    use super::ApplicationPlatformIdentity;
    use super::ComputerUsePolicy;
    use super::WindowsExeIdentity;

    fn macos_app(bundle_id: &str) -> ApplicationIdentity {
        ApplicationIdentity {
            display_name: bundle_id.to_string(),
            platform: ApplicationPlatformIdentity::MacosBundle {
                bundle_id: bundle_id.to_string(),
            },
        }
    }

    fn exe_app() -> ApplicationIdentity {
        ApplicationIdentity {
            display_name: "publisher product".to_string(),
            platform: ApplicationPlatformIdentity::WindowsExe {
                publisher_name: "Contoso".to_string(),
                product_name: "Widget".to_string(),
                binary_name: Some("widget.exe".to_string()),
            },
        }
    }

    #[test]
    fn platform_tables_and_default_resolve_access() {
        let mut policy = ComputerUsePolicy::default();
        policy
            .macos_bundle_ids
            .insert("com.allowed".to_string(), AccessRequirement::Allow);
        policy
            .macos_bundle_ids
            .insert("com.denied".to_string(), AccessRequirement::Deny);
        policy
            .windows_aumids
            .insert("Contoso.Widget!App".to_string(), AccessRequirement::Deny);
        policy.windows_exe_identities.push(WindowsExeIdentity {
            publisher_name: "Contoso".to_string(),
            product_name: "Widget".to_string(),
            binary_name: Some("widget.exe".to_string()),
            access: AccessRequirement::Allow,
        });
        policy.default_app_access = Some(AccessRequirement::Deny);

        assert_eq!(
            policy.decide(&macos_app("com.allowed")),
            AccessDecision::Allowed
        );
        assert_eq!(
            policy.decide(&macos_app("com.denied")),
            AccessDecision::Denied
        );
        assert_eq!(
            policy.decide(&macos_app("com.other")),
            AccessDecision::Denied,
            "default deny applies when no table entry matches"
        );
        assert_eq!(
            policy.decide(&ApplicationIdentity {
                display_name: "aumid app".to_string(),
                platform: ApplicationPlatformIdentity::WindowsAumid {
                    aumid: "Contoso.Widget!App".to_string(),
                },
            }),
            AccessDecision::Denied
        );
        assert_eq!(policy.decide(&exe_app()), AccessDecision::Allowed);
        assert_eq!(
            policy.decide(&ApplicationIdentity {
                display_name: "generic".to_string(),
                platform: ApplicationPlatformIdentity::Generic { binary_name: None },
            }),
            AccessDecision::Denied
        );

        policy.default_app_access = None;
        assert_eq!(
            policy.decide(&macos_app("com.other")),
            AccessDecision::Unspecified
        );
    }

    #[test]
    fn policy_digest_is_stable_and_sensitive() {
        let mut first = ComputerUsePolicy::default();
        first
            .macos_bundle_ids
            .insert("com.a".to_string(), AccessRequirement::Allow);
        first.default_app_access = Some(AccessRequirement::Deny);

        let mut second = ComputerUsePolicy::default();
        second.default_app_access = Some(AccessRequirement::Deny);
        second
            .macos_bundle_ids
            .insert("com.a".to_string(), AccessRequirement::Allow);

        assert_eq!(
            first.policy_digest(),
            second.policy_digest(),
            "digest is independent of insertion order"
        );

        let mut third = first.clone();
        third.default_app_access = Some(AccessRequirement::Allow);
        assert_ne!(
            first.policy_digest(),
            third.policy_digest(),
            "digest is sensitive to policy changes"
        );
    }

    #[test]
    fn takeover_is_denied_unless_policy_and_forced_mode_allow() {
        let mut policy = ComputerUsePolicy::default();
        assert!(
            !policy.takeover_allowed(true),
            "default denies forced cross-owner takeover"
        );
        assert!(!policy.takeover_allowed(false));
        policy.allow_session_takeover = Some(true);
        assert!(policy.takeover_allowed(true));
        assert!(
            !policy.takeover_allowed(false),
            "graceful cross-owner takeover is never granted by the policy gate"
        );
    }

    #[test]
    fn is_empty_mirrors_native_semantics() {
        assert!(ComputerUsePolicy::default().is_empty());
        let mut policy = ComputerUsePolicy::default();
        policy.windows_exe_identities.clear();
        assert!(policy.is_empty());
        let mut policy = ComputerUsePolicy::default();
        policy.allow_locked_computer_use = Some(false);
        assert!(!policy.is_empty());
    }
}
