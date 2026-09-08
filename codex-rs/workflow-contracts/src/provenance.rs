//! Workflow provenance, attribution, licensing, and upgrade policy.
//!
//! These fields preserve authorship and distribution metadata across forks
//! and installs. They deliberately exclude commercial settlement data
//! (pricing, payment entitlements) and credentials: both are marketplace
//! and installation concerns owned by later Work Orders, never workflow
//! semantics.

use serde::Deserialize;
use serde::Serialize;

use crate::RevisionSha;
use crate::WorkflowRepositoryId;

/// Attribution of one contributor to a workflow.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Attribution {
    /// Contributor display name or handle.
    pub name: String,
    /// Optional contact reference, for example an email or profile URL.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contact: Option<String>,
}

/// License terms of a workflow.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LicenseInfo {
    /// SPDX license identifier, for example `Apache-2.0` or `LicenseRef-Proprietary`.
    pub spdx: String,
    /// Optional URL of the full license text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

/// How an installed workflow may upgrade to newer published versions.
///
/// Upgrade policy is distribution metadata. Applying an upgrade always
/// produces a new pinned version binding; it never mutates the currently
/// pinned immutable version.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub enum UpgradePolicy {
    /// Stay on the pinned version until the user upgrades explicitly.
    Manual,
    /// Offer upgrades within the declared compatibility range.
    Prompt,
    /// Apply upgrades within the declared compatibility range automatically.
    AutomaticWithinRange,
}

/// Fork lineage: where a workflow repository was forked from.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ForkLineage {
    /// The upstream repository this repository was forked from.
    pub upstream: WorkflowRepositoryId,
    /// Upstream commit the fork diverged at, when known.
    ///
    /// Provenance only: upstream refs move and are never execution
    /// authority.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub forked_at: Option<RevisionSha>,
}

/// Provenance of a workflow: authorship, origin, license, and upgrade
/// policy.
///
/// Commercial entitlements (pricing, settlement, entitlement grants) and
/// credentials are intentionally not representable here. A future
/// marketplace Work Order owns commercial metadata as a separate layer that
/// prices access to immutable releases without changing their executable
/// definitions.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields, default)]
pub struct WorkflowProvenance {
    /// Attributed authors and contributors.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub authors: Vec<Attribution>,
    /// Upstream lineage when the workflow was forked.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub forked_from: Option<ForkLineage>,
    /// License terms, when declared.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub license: Option<LicenseInfo>,
    /// Upgrade policy for installed copies of this workflow.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub upgrade_policy: Option<UpgradePolicy>,
}

impl WorkflowProvenance {
    /// Returns `true` when the workflow carries no provenance at all.
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }
}

#[cfg(test)]
#[path = "provenance_tests.rs"]
mod tests;
