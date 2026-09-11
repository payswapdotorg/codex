//! Publication metadata: the versioned, integrity-checkable document
//! bound to an immutable workflow release.
//!
//! A [`PublicationMetadata`] document carries the distribution facts
//! the marketplace needs about a sealed release — attribution,
//! ownership, licensing, source lineage, compatibility, and upgrade
//! policy — and is **hash-chained to the version seal**: its digest
//! covers the release reference (whose own version id is the digest of
//! the frozen execution identity tuple) plus every metadata field. A
//! tampered document fails [`PublicationMetadata::verify`] exactly the
//! way a tampered version record fails `WorkflowVersion::verify_integrity`.
//!
//! The document deliberately contains **no commercial entitlement
//! data and no credentials**: commercial terms live in their own
//! digest-bound document (see [`crate::commerce`]) behind the separate
//! entitlement boundary, and every identity here is an opaque,
//! credential-free label.

use codex_workflow_contracts::Attribution;
use codex_workflow_contracts::CapabilityId;
use codex_workflow_contracts::ContentDigest;
use codex_workflow_contracts::ResourceTypeId;
use codex_workflow_contracts::SemanticVersion;
use codex_workflow_contracts::UpgradePolicy;
use codex_workflow_contracts::WorkflowVersion;
use codex_workflow_forge::PublishedVersionRef;
use serde::Deserialize;
use serde::Serialize;

use crate::WorkflowDistributionError;

/// Format version of the publication metadata document.
pub const PUBLICATION_METADATA_FORMAT_VERSION: u32 = 1;

/// Maximum length of an opaque principal label.
const PRINCIPAL_MAX_LEN: usize = 128;

/// An opaque marketplace principal: the owner, installer, or audience
/// identity referenced by distribution records.
///
/// Principals are credential-free by construction: the label must be a
/// non-empty path-segment-safe token without `:` or `/`, so URLs with
/// embedded userinfo (`https://user:secret@host/...`) can never become
/// a principal. Who a principal *is* — and any authentication for it —
/// is host-owned; this plane only sees the label.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct MarketplacePrincipal(String);

impl MarketplacePrincipal {
    /// Parses and validates a principal label.
    pub fn parse(value: impl Into<String>) -> Result<Self, WorkflowDistributionError> {
        let value = value.into();
        let valid = !value.is_empty()
            && value.len() <= PRINCIPAL_MAX_LEN
            && !value.trim().is_empty()
            && value.trim().len() == value.len()
            && value.chars().all(|character| {
                character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | '.')
            });
        if valid {
            Ok(Self(value))
        } else {
            Err(WorkflowDistributionError::InvalidRecord {
                reason: format!(
                    "principal labels must be non-empty, at most {PRINCIPAL_MAX_LEN} bytes, and free of whitespace, `:`, and `/`"
                ),
            })
        }
    }
}

impl TryFrom<String> for MarketplacePrincipal {
    type Error = WorkflowDistributionError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl From<MarketplacePrincipal> for String {
    fn from(value: MarketplacePrincipal) -> Self {
        value.0
    }
}

impl AsRef<str> for MarketplacePrincipal {
    fn as_ref(&self) -> &str {
        self.0.as_str()
    }
}

impl std::fmt::Display for MarketplacePrincipal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0.as_str())
    }
}

/// A coarse capability-class label used as a marketplace discovery
/// facet (for example `browser-automation` or `document-processing`).
///
/// Capability classes are a **distribution taxonomy**: they classify
/// releases for search and are metadata only. They are deliberately
/// distinct from execution environment classes and capability
/// bindings, which are runtime concerns owned by the execution plane.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct CapabilityClass(String);

impl CapabilityClass {
    /// Parses a capability-class label: a non-empty
    /// path-segment-safe token (ASCII letters, digits, `_`, `-`, `.`).
    pub fn parse(value: impl Into<String>) -> Result<Self, WorkflowDistributionError> {
        let value = value.into();
        let characters_ok = !value.is_empty()
            && value.chars().all(|character| {
                character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | '.')
            });
        let dots_ok = !value.starts_with('.') && !value.ends_with('.') && !value.contains("..");
        if characters_ok && dots_ok {
            Ok(Self(value))
        } else {
            Err(WorkflowDistributionError::InvalidMetadata {
                reason: "capability classes must be non-empty path-segment-safe tokens".to_owned(),
            })
        }
    }
}

impl TryFrom<String> for CapabilityClass {
    type Error = WorkflowDistributionError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl From<CapabilityClass> for String {
    fn from(value: CapabilityClass) -> Self {
        value.0
    }
}

impl AsRef<str> for CapabilityClass {
    fn as_ref(&self) -> &str {
        self.0.as_str()
    }
}

impl std::fmt::Display for CapabilityClass {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0.as_str())
    }
}

/// License terms of a published release: an SPDX-style identifier
/// (for example `Apache-2.0` or `LicenseRef-Proprietary`), optionally
/// with the URL of the full text and, for custom terms, the content
/// digest of the terms document so the terms are
/// integrity-checkable.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LicenseTerms {
    /// SPDX-style license identifier or custom license name.
    pub identifier: String,
    /// URL of the full license text, when hosted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// Digest of the full custom terms document, when the license is
    /// custom. The full text lives with the host; only the digest
    /// enters metadata.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub custom_terms_digest: Option<ContentDigest>,
}

impl LicenseTerms {
    /// Validates the identifier: non-empty, no surrounding
    /// whitespace.
    pub fn validate(&self) -> Result<(), WorkflowDistributionError> {
        let trimmed = self.identifier.trim();
        if trimmed.is_empty() || trimmed.len() != self.identifier.len() {
            Err(WorkflowDistributionError::InvalidMetadata {
                reason: "license identifiers must be non-empty without surrounding whitespace"
                    .to_owned(),
            })
        } else {
            Ok(())
        }
    }
}

/// Source lineage of a release: where it came from.
///
/// Lineage is provenance only. A forked release's lineage points back
/// to the upstream **immutable version reference**; the derived
/// release is a new immutable version whose own sealed provenance
/// (repository-level fork lineage) is owned by the frozen contracts.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields, default)]
pub struct SourceLineage {
    /// The upstream release this release derives from, when the
    /// release is a fork.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub forked_from: Option<PublishedVersionRef>,
    /// Attribution carried forward from upstream releases (license
    /// obligations), recorded so forks preserve authorship.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub carried_attribution: Vec<Attribution>,
}

impl SourceLineage {
    /// Whether this lineage records a fork.
    pub fn is_fork(&self) -> bool {
        self.forked_from.is_some()
    }
}

/// Compatibility requirements of a release: what an installation needs
/// before the release can run.
///
/// The declared capabilities and resources must **cover** everything
/// the sealed version itself declares (see
/// [`CompatibilitySpec::validate_against_version`]), so marketplace
/// search facets cannot understate a release's requirements.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CompatibilitySpec {
    /// Minimum workflow runtime version the release requires.
    pub minimum_runtime: SemanticVersion,
    /// Semantic capabilities the release requires.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub required_capabilities: Vec<CapabilityId>,
    /// Capability-class taxonomy labels for discovery.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub capability_classes: Vec<CapabilityClass>,
    /// Typed resources the release needs.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub required_resources: Vec<ResourceTypeId>,
}

impl CompatibilitySpec {
    /// Validates the spec against the sealed version it describes.
    ///
    /// Every capability the version's steps declare and every resource
    /// its dependencies declare must appear in the spec: metadata may
    /// overstate requirements (conservative and safe for installers)
    /// but must never understate them.
    pub fn validate_against_version(
        &self,
        version: &WorkflowVersion,
    ) -> Result<(), WorkflowDistributionError> {
        for (node_id, node) in &version.definition.ir.nodes {
            if let codex_workflow_contracts::WorkflowIrNode::Step(step) = node {
                for requirement in &step.capabilities {
                    if !self.required_capabilities.contains(&requirement.capability) {
                        return Err(WorkflowDistributionError::InvalidMetadata {
                            reason: format!(
                                "compatibility metadata must cover capability `{}` declared by step `{node_id}`",
                                requirement.capability.as_ref()
                            ),
                        });
                    }
                }
            }
        }
        for resource in &version.definition.dependencies.resources {
            if !self.required_resources.contains(&resource.resource_type) {
                return Err(WorkflowDistributionError::InvalidMetadata {
                    reason: format!(
                        "compatibility metadata must cover resource `{}` declared by the version",
                        resource.resource_type.as_ref()
                    ),
                });
            }
        }
        Ok(())
    }
}

/// How an installed release advances to newer published versions:
/// pin or follow.
///
/// This is the distribution-plane setting the Work Order names ("pin
/// vs follow"). It corresponds to the frozen contract
/// [`UpgradePolicy`] (repository provenance): `Manual` is `Pin`,
/// `Prompt` and `AutomaticWithinRange` are `Follow` — the difference
/// between the two follow modes is *who* approves the advance, and in
/// this plane every advance is an explicit, recorded decision
/// (see [`crate::UpgradeProposal`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub enum UpgradePolicySetting {
    /// Pin: the installation stays on its pinned immutable version
    /// until the user upgrades explicitly; new releases never surface
    /// as proposals.
    Pin,
    /// Follow: new published releases surface as upgrade proposals
    /// that advance the pin only through an explicit, recorded
    /// approval. Running instances always keep the version they
    /// pinned.
    Follow,
}

impl UpgradePolicySetting {
    /// Maps the frozen contract upgrade policy onto the distribution
    /// pin/follow setting.
    ///
    /// `AutomaticWithinRange` maps to [`Self::Follow`] because
    /// automatic application is a host approval behavior, not a
    /// distribution distinction — this plane still records every
    /// advance as an explicit decision.
    pub fn from_contract(policy: UpgradePolicy) -> Self {
        match policy {
            UpgradePolicy::Manual => Self::Pin,
            UpgradePolicy::Prompt | UpgradePolicy::AutomaticWithinRange => Self::Follow,
        }
    }

    /// The frozen contract policy this setting corresponds to.
    ///
    /// `Pin` maps to `Manual`; `Follow` maps to `Prompt`, because in
    /// this plane every advance is an explicit, recorded approval.
    pub fn to_contract(self) -> UpgradePolicy {
        match self {
            Self::Pin => UpgradePolicy::Manual,
            Self::Follow => UpgradePolicy::Prompt,
        }
    }
}

/// The publication metadata document: distribution facts bound to an
/// immutable release by a hash chain.
///
/// `metadata_digest` covers the release reference (whose `version_id`
/// is the digest of the frozen execution identity tuple) plus every
/// metadata field, so the document is integrity-checkable end to end:
/// mutating any covered field breaks the chain, and mutating the
/// release breaks the version's own seal.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PublicationMetadata {
    /// Document format version.
    pub format_version: u32,
    /// The immutable release this document is bound to.
    pub release: PublishedVersionRef,
    /// Attributed authors and contributors.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub attribution: Vec<Attribution>,
    /// The owning principal of the release (opaque, credential-free).
    pub ownership: MarketplacePrincipal,
    /// License terms.
    pub licensing: LicenseTerms,
    /// Source lineage.
    #[serde(default)]
    pub provenance: SourceLineage,
    /// Compatibility requirements.
    pub compatibility: CompatibilitySpec,
    /// Upgrade policy (pin vs follow).
    pub upgrade_policy: UpgradePolicySetting,
    /// The hash-chain seal: digest over the release reference and all
    /// covered fields above.
    pub metadata_digest: ContentDigest,
}

impl PublicationMetadata {
    /// Seals a publication metadata document against an immutable
    /// release.
    ///
    /// Sealing validates the release reference (its identity digest
    /// must recompute — tampered releases never publish), the license
    /// identifier, and the document format version, then computes the
    /// hash-chain digest. After sealing, any mutation of covered
    /// content fails [`Self::verify`].
    pub fn seal(
        release: PublishedVersionRef,
        attribution: Vec<Attribution>,
        ownership: MarketplacePrincipal,
        licensing: LicenseTerms,
        provenance: SourceLineage,
        compatibility: CompatibilitySpec,
        upgrade_policy: UpgradePolicySetting,
    ) -> Result<Self, WorkflowDistributionError> {
        release.verify()?;
        licensing.validate()?;
        if provenance.forked_from.is_some() && provenance.carried_attribution.is_empty() {
            return Err(WorkflowDistributionError::InvalidMetadata {
                reason: "a forked release must carry upstream attribution".to_owned(),
            });
        }
        let metadata_digest = ContentDigest::of(&ChainedFields {
            format_version: PUBLICATION_METADATA_FORMAT_VERSION,
            release: &release,
            attribution: &attribution,
            ownership: &ownership,
            licensing: &licensing,
            provenance: &provenance,
            compatibility: &compatibility,
            upgrade_policy,
        })
        .map_err(digest_error)?;
        Ok(Self {
            format_version: PUBLICATION_METADATA_FORMAT_VERSION,
            release,
            attribution,
            ownership,
            licensing,
            provenance,
            compatibility,
            upgrade_policy,
            metadata_digest,
        })
    }

    /// Verifies the document end to end.
    ///
    /// A document passes only when the format version is current, the
    /// release reference still verifies against its recomputed
    /// identity digest, and the recorded metadata digest equals the
    /// recomputed hash chain. This is the check that makes publication
    /// metadata effectively immutable: a tampered document fails
    /// recomputation.
    pub fn verify(&self) -> Result<(), WorkflowDistributionError> {
        if self.format_version != PUBLICATION_METADATA_FORMAT_VERSION {
            return Err(WorkflowDistributionError::InvalidMetadata {
                reason: format!(
                    "unsupported publication metadata format version {} (expected {PUBLICATION_METADATA_FORMAT_VERSION})",
                    self.format_version
                ),
            });
        }
        self.release.verify()?;
        let recomputed = self.compute_digest()?;
        if recomputed != self.metadata_digest {
            return Err(WorkflowDistributionError::InvalidMetadata {
                reason: "metadata digest does not match the hash chain over the release and covered fields".to_owned(),
            });
        }
        Ok(())
    }

    /// Validates the document's compatibility claims against the full
    /// sealed version record (the metadata's declared requirements
    /// must cover the version's own declarations).
    pub fn validate_against_version(
        &self,
        version: &WorkflowVersion,
    ) -> Result<(), WorkflowDistributionError> {
        if self.release.identity != version.identity
            || self.release.version_id != version.version_id
        {
            return Err(WorkflowDistributionError::InvalidMetadata {
                reason: "the metadata document is bound to a different release".to_owned(),
            });
        }
        self.compatibility.validate_against_version(version)
    }

    /// Computes the hash-chain digest over the release reference and
    /// every covered field.
    fn compute_digest(&self) -> Result<ContentDigest, WorkflowDistributionError> {
        ContentDigest::of(&ChainedFields {
            format_version: self.format_version,
            release: &self.release,
            attribution: &self.attribution,
            ownership: &self.ownership,
            licensing: &self.licensing,
            provenance: &self.provenance,
            compatibility: &self.compatibility,
            upgrade_policy: self.upgrade_policy,
        })
        .map_err(digest_error)
    }
}

/// The digest-covered projection of the metadata document: every field
/// except `metadata_digest` itself.
#[derive(Serialize)]
struct ChainedFields<'a> {
    format_version: u32,
    release: &'a PublishedVersionRef,
    attribution: &'a [Attribution],
    ownership: &'a MarketplacePrincipal,
    licensing: &'a LicenseTerms,
    provenance: &'a SourceLineage,
    compatibility: &'a CompatibilitySpec,
    upgrade_policy: UpgradePolicySetting,
}

/// Maps a digest serialization failure onto the metadata error channel.
fn digest_error(
    error: codex_workflow_contracts::WorkflowContractError,
) -> WorkflowDistributionError {
    WorkflowDistributionError::InvalidMetadata {
        reason: error.to_string(),
    }
}

#[cfg(test)]
#[path = "metadata_tests.rs"]
mod tests;
