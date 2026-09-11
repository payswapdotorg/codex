//! Marketplace discovery: queries, listings, and pure filtering.
//!
//! Discovery is a metadata-only view over published releases: search
//! results never carry workflow definitions or dependency locks — only
//! the immutable release reference, its publication metadata, the
//! distribution state, and the commercial policy class. Installing
//! requires fetching the full sealed version record (see
//! [`crate::DistributionPort::fetch_version`]).
//!
//! Visibility discipline mirrors the plugin-catalog patterns the
//! capability map calls out as references (listed/unlisted/private
//! discoverability, explicit share audiences), implemented over
//! workflow semantics:
//!
//! - anonymous searches see **public** releases only;
//! - a principal sees public releases, releases shared with it, and
//!   its own releases in any state;
//! - unlisted (`Published`) releases are addressable by identity but
//!   never surface in discovery;
//! - forks are owner-visible until explicitly shared or listed.

use codex_workflow_contracts::CapabilityId;
use codex_workflow_contracts::ResourceTypeId;
use codex_workflow_contracts::SemanticVersion;
use codex_workflow_contracts::WorkflowDefinitionId;
use codex_workflow_contracts::WorkflowVersionId;
use serde::Deserialize;
use serde::Serialize;

use crate::CommercialPolicy;
use crate::DistributionState;
use crate::MarketplacePrincipal;
use crate::PublicationMetadata;

/// A marketplace search query. Unset fields match anything.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct MarketplaceQuery {
    /// The principal searching. `None` is an anonymous search, which
    /// sees publicly discoverable releases only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requesting: Option<MarketplacePrincipal>,
    /// Match releases of this workflow.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflow: Option<WorkflowDefinitionId>,
    /// Match releases whose compatibility declares this capability.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capability: Option<CapabilityId>,
    /// Match releases classified with this capability class.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capability_class: Option<crate::CapabilityClass>,
    /// Match releases whose compatibility requires this resource type.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resource: Option<ResourceTypeId>,
    /// Match releases licensed under this exact SPDX-style identifier.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
    /// Match releases whose lineage points back to this immutable
    /// version (direct forks of it).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub forked_from: Option<WorkflowVersionId>,
    /// Match releases whose declared minimum runtime is compatible
    /// with this runtime version (`minimum_runtime <= runtime`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub minimum_runtime: Option<SemanticVersion>,
    /// Match releases whose workflow identity or owning principal
    /// contains this fragment (case-insensitive).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name_contains: Option<String>,
}

/// One marketplace listing: a metadata-only view of a published
/// release.
///
/// The listing carries the release reference (inside
/// [`Self::metadata`]), the entry's distribution state, and the
/// commercial policy class. It carries no workflow definition and no
/// dependency lock; the full sealed record is fetched separately when
/// installing.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MarketplaceListing {
    /// The publication metadata document of the release.
    pub metadata: PublicationMetadata,
    /// The release entry's distribution state.
    pub state: DistributionState,
    /// The commercial policy class, when the release carries
    /// commercial terms.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub commercial: Option<CommercialPolicy>,
}

/// Whether a release entry is visible to a requesting principal.
///
/// Visibility is the marketplace's own scope gate and runs before any
/// facet matching:
///
/// - anonymous requests see only publicly discoverable entries;
/// - the owner sees their entries in every state;
/// - audience members see shared entries;
/// - everyone sees public entries.
pub fn listing_visible(
    metadata: &PublicationMetadata,
    state: DistributionState,
    audience: &[MarketplacePrincipal],
    requesting: Option<&MarketplacePrincipal>,
) -> bool {
    match requesting {
        None => state.is_publicly_discoverable(),
        Some(principal) => {
            &metadata.ownership == principal
                || state.is_publicly_discoverable()
                || (state == DistributionState::Shared && audience.contains(principal))
        }
    }
}

/// Whether a release entry may be **installed** by a principal.
///
/// Install addressability is deliberately wider than discovery
/// visibility: unlisted (`Published`) releases are installable by
/// anyone who can name their immutable identity (secret-link
/// distribution), while discovery never surfaces them. Private and
/// forked entries are owner-only; shared entries additionally serve
/// their audience; public entries serve everyone. The access policy
/// still evaluates license terms after this scope gate.
pub fn release_installable(
    metadata: &PublicationMetadata,
    state: DistributionState,
    audience: &[MarketplacePrincipal],
    requesting: &MarketplacePrincipal,
) -> bool {
    &metadata.ownership == requesting
        || state.is_publicly_discoverable()
        || state == DistributionState::Published
        || (state == DistributionState::Shared && audience.contains(requesting))
}

/// Whether a visible release entry matches every set facet of a
/// query.
///
/// Facet rules:
///
/// - `workflow` — the release publishes that workflow;
/// - `capability` / `resource` — the compatibility metadata declares
///   the requirement (and compatibility is validated to cover the
///   version's own declarations at publish time, so the facet cannot
///   understate requirements);
/// - `capability_class` — the taxonomy classification matches;
/// - `license` — exact SPDX-style identifier match;
/// - `forked_from` — the release's lineage points back to that exact
///   immutable version (direct forks only, like the WO-009 repository
///   discovery's direct-origin rule);
/// - `minimum_runtime` — the release runs on the given runtime
///   (`minimum_runtime <= runtime`);
/// - `name_contains` — case-insensitive fragment of the workflow
///   identity or owning principal.
pub fn listing_matches(metadata: &PublicationMetadata, query: &MarketplaceQuery) -> bool {
    if let Some(workflow) = &query.workflow
        && &metadata.release.identity.workflow != workflow
    {
        return false;
    }
    if let Some(capability) = &query.capability
        && !metadata
            .compatibility
            .required_capabilities
            .contains(capability)
    {
        return false;
    }
    if let Some(class) = &query.capability_class
        && !metadata.compatibility.capability_classes.contains(class)
    {
        return false;
    }
    if let Some(resource) = &query.resource
        && !metadata.compatibility.required_resources.contains(resource)
    {
        return false;
    }
    if let Some(license) = &query.license
        && &metadata.licensing.identifier != license
    {
        return false;
    }
    if let Some(upstream) = &query.forked_from
        && metadata
            .provenance
            .forked_from
            .as_ref()
            .is_none_or(|lineage| &lineage.version_id != upstream)
    {
        return false;
    }
    if let Some(runtime) = &query.minimum_runtime
        && metadata.compatibility.minimum_runtime > *runtime
    {
        return false;
    }
    if let Some(fragment) = &query.name_contains {
        let needle = fragment.to_lowercase();
        let workflow = metadata
            .release
            .identity
            .workflow
            .to_string()
            .to_lowercase();
        let owner = metadata.ownership.as_ref().to_lowercase();
        if !workflow.contains(&needle) && !owner.contains(&needle) {
            return false;
        }
    }
    true
}
