//! Allow/deny policy tables shared by the WO-015 environment adapters.
//!
//! Availability never implies authorization: besides the registry's
//! binding policy, each adapter keeps a policy table over the opaque
//! targets named by its resource bindings (remote hosts, mobile devices).
//! The table is the adapter-side projection of the host's native access
//! configuration, the same role the WO-007 application tables play for the
//! desktop environment. A target with no applicable entry is
//! **unspecified**: driving it requires explicit host authorization, so
//! the adapter refuses instead of silently defaulting to allow.

use std::collections::BTreeMap;

use serde::Deserialize;
use serde::Serialize;

/// One allow/deny policy entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Access {
    /// The target is allowed.
    Allow,
    /// The target is denied.
    Deny,
}

/// The resolved decision for one policy target.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AccessDecision {
    /// A policy entry explicitly allows the target.
    Allowed,
    /// A policy entry explicitly denies the target.
    Denied,
    /// No policy entry applies; explicit host authorization is required.
    Unspecified,
}

/// An allow/deny table over opaque, credential-free target labels.
///
/// Entries win over the default; an absent default leaves unspecified
/// targets to explicit host authorization.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AccessTable {
    /// The decision applied when no entry matches.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<Access>,
    /// Per-target decisions, keyed by opaque target label.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub entries: BTreeMap<String, Access>,
}

impl AccessTable {
    /// An empty table with no default: every target is unspecified.
    pub fn new() -> Self {
        Self::default()
    }

    /// The decision for `target`.
    pub fn decide(&self, target: &str) -> AccessDecision {
        match self.entries.get(target) {
            Some(Access::Allow) => AccessDecision::Allowed,
            Some(Access::Deny) => AccessDecision::Denied,
            None => match self.default {
                Some(Access::Allow) => AccessDecision::Allowed,
                Some(Access::Deny) => AccessDecision::Denied,
                None => AccessDecision::Unspecified,
            },
        }
    }
}

#[cfg(test)]
#[path = "policy_tests.rs"]
mod tests;
