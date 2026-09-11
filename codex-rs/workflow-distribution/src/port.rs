//! The distribution/marketplace port seam.
//!
//! [`DistributionPort`] is the boundary every distribution operation
//! crosses: publish, search, fetch, fork, install, scope transitions,
//! and the explicit upgrade path. Hosts implement it against their
//! real marketplace (transport, durable stores, commercial systems);
//! [`crate::InMemoryMarketplace`] is the reference implementation and
//! test double.
//!
//! Two further seams cross the port's install path:
//!
//! - [`crate::AccessPolicy`] evaluates license and sharing terms for
//!   an installer. Its shape deliberately reuses the WO-011
//!   trigger-plane authorization discipline (a synchronous record
//!   seam whose decision is *data* — a denial is a recorded outcome
//!   attributed to a source mechanism, never a credential). The
//!   concrete WO-011 port authorizes resource bindings; this one
//!   authorizes distribution access. Authorization semantics are
//!   referenced, not duplicated.
//! - [`crate::EntitlementPort`] (see [`crate::entitlement`]) checks
//!   commercial entitlements. Checks only.
//!
//! The required install gate order, which every implementation must
//! preserve:
//!
//! ```text
//! visibility (scope) -> version integrity -> access policy ->
//! commercial entitlement -> record
//! ```
//!
//! A denial anywhere before the record step installs nothing and
//! mutates nothing: denials gate distribution only, never executable
//! semantics.

use codex_workflow_contracts::Attribution;
use codex_workflow_contracts::ImmutableSourceRevision;
use codex_workflow_contracts::SemanticVersion;
use codex_workflow_contracts::WorkflowDefinitionId;
use codex_workflow_contracts::WorkflowRepositoryId;
use codex_workflow_contracts::WorkflowVersion;
use codex_workflow_contracts::WorkflowVersionId;
use serde::Deserialize;
use serde::Serialize;

use crate::CommercialTerms;
use crate::DistributionState;
use crate::EntitlementDecision;
use crate::LicenseTerms;
use crate::MarketplaceListing;
use crate::MarketplacePrincipal;
use crate::MarketplaceQuery;
use crate::PublicationMetadata;
use crate::UpgradePolicySetting;
use crate::WorkflowDistributionError;
use crate::state::DistributionTransition;
use codex_workflow_forge::PublishedVersionRef;

/// The visibility scope a release enters distribution under.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub enum PublicationScope {
    /// Unlisted: sealed into distribution and addressable by identity,
    /// but invisible to discovery (secret-link distribution).
    Unlisted,
    /// Owner-only: invisible to discovery, installable by the owner.
    Private,
    /// An explicit audience: invisible to public discovery, visible
    /// and installable for the named principals.
    Shared {
        /// The principals the release is shared with.
        audience: Vec<MarketplacePrincipal>,
    },
    /// Publicly discoverable through marketplace search.
    Public,
}

impl PublicationScope {
    /// The entry state this scope creates.
    pub fn entry_state(&self) -> DistributionState {
        match self {
            Self::Unlisted => DistributionState::Published,
            Self::Private => DistributionState::Private,
            Self::Shared { .. } => DistributionState::Shared,
            Self::Public => DistributionState::Public,
        }
    }
}

/// An author-side submission of a sealed release into distribution.
#[derive(Clone, Debug)]
pub struct PublishSubmission {
    /// The sealed, immutable version to publish. Its integrity is
    /// re-verified before anything is recorded.
    pub version: WorkflowVersion,
    /// The publication metadata document bound to the release.
    pub metadata: PublicationMetadata,
    /// Commercial terms, when the release carries a commercial
    /// policy. `None` means the release is free.
    pub commercial: Option<CommercialTerms>,
    /// The visibility scope the release enters distribution under.
    pub scope: PublicationScope,
}

/// A request to fork a published release into a new repository.
///
/// Forking derives a **new immutable version**: the upstream's frozen
/// definition and dependency lock are re-sealed under the fork's
/// repository, revision, and version, with provenance pointing back
/// to the upstream. The upstream release is never mutated.
#[derive(Clone, Debug)]
pub struct ForkRequest {
    /// The upstream release to fork, by its immutable reference (the
    /// listing's `metadata.release`).
    pub upstream: PublishedVersionRef,
    /// The fork's repository identity (must differ from the
    /// upstream's).
    pub fork_repository: WorkflowRepositoryId,
    /// The fork's own immutable source revision (the commit the fork
    /// stands at).
    pub fork_revision: ImmutableSourceRevision,
    /// The semantic version of the derived release.
    pub fork_version: SemanticVersion,
    /// The owning principal of the fork.
    pub owner: MarketplacePrincipal,
    /// The fork's license terms. License obligations may require
    /// carrying the upstream license; the carried attribution below
    /// preserves authorship.
    pub licensing: LicenseTerms,
    /// Attribution carried forward from the upstream (required: a
    /// fork must preserve authorship).
    pub carried_attribution: Vec<Attribution>,
    /// The fork's upgrade policy (pin vs follow).
    pub upgrade_policy: UpgradePolicySetting,
}

/// A request to install a published release from the marketplace.
#[derive(Clone, Debug)]
pub struct MarketplaceInstallRequest {
    /// The workflow to install.
    pub workflow: WorkflowDefinitionId,
    /// The immutable version to pin.
    pub version: WorkflowVersionId,
    /// The installing principal (opaque, credential-free).
    pub principal: MarketplacePrincipal,
    /// When the install is requested, in unix milliseconds (supplied
    /// by the caller so the plane owns no clock).
    pub at_unix_ms: u64,
}

/// A marketplace installation: a distribution-plane pin of a workflow
/// to an immutable published version.
///
/// The record carries the full sealed version so the host can drive
/// the operational installation through the WO-011 trigger plane
/// (`codex_workflow_triggers::InstallRequest::new(version)` plus
/// explicit bindings): the distribution plane owns the gates and the
/// pin, the trigger plane owns configuration and firing. Both planes
/// share the forge install-registry discipline — no silent
/// over-installs, explicit reviewable upgrades.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MarketplaceInstall {
    /// The installed workflow.
    pub workflow: WorkflowDefinitionId,
    /// The principal that installed (opaque, credential-free); upgrade
    /// gates re-run for this principal.
    pub principal: MarketplacePrincipal,
    /// The immutable version pin.
    pub installed: PublishedVersionRef,
    /// The full sealed version record, for the host's operational
    /// installation.
    pub version: WorkflowVersion,
    /// The recorded access decision that admitted the install.
    pub access: crate::AccessDecision,
    /// The recorded entitlement decision, when the release carries
    /// commercial terms.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entitlement: Option<EntitlementDecision>,
    /// The upgrade policy in effect (pin vs follow).
    pub upgrade_policy: UpgradePolicySetting,
    /// When the install was recorded, in unix milliseconds.
    pub at_unix_ms: u64,
}

/// A proposed upgrade of an installed workflow, surfaced by the
/// upgrade-policy evaluation when a followed workflow publishes a new
/// release.
///
/// A proposal is pure data: surfacing it never changes the
/// installation. Advancing requires an explicit
/// [`UpgradeDecision::Approve`] through
/// [`DistributionPort::decide_upgrade`], which re-runs every install
/// gate for the new release — an upgrade that the gates would refuse
/// is never applied.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UpgradeProposal {
    /// The workflow to update.
    pub workflow: WorkflowDefinitionId,
    /// The version the installation is pinned to.
    pub from: WorkflowVersionId,
    /// The newly published release to advance to.
    pub to: PublishedVersionRef,
    /// The policy that surfaced the proposal (always
    /// [`UpgradePolicySetting::Follow`]; pins never surface
    /// proposals).
    pub policy: UpgradePolicySetting,
}

/// An explicit decision on an upgrade proposal.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub enum UpgradeDecision {
    /// Approve: advance the pin to the proposed release (after the
    /// gates re-run). Future instances run the new version; the
    /// previously pinned version stays a valid immutable record.
    Approve,
    /// Reject: keep the pin on the current version.
    Reject,
}

/// The audit record of a decided upgrade: immutable history that
/// makes installed-workflow changes reviewable after the fact.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UpgradeRecord {
    /// The workflow that was (not) updated.
    pub workflow: WorkflowDefinitionId,
    /// The version the update started from.
    pub from: WorkflowVersionId,
    /// The version the update moved to (identical to `from` when
    /// rejected).
    pub to: WorkflowVersionId,
    /// Whether the update was applied.
    pub applied: bool,
    /// When the decision was recorded, in unix milliseconds.
    pub at_unix_ms: u64,
}

/// The distribution/marketplace seam: every operation of the
/// publication, distribution, licensing, and marketplace plane.
///
/// Implementations must preserve the documented invariants:
///
/// - **Immutability.** Published versions and their metadata are
///   sealed; every operation re-verifies integrity before recording
///   anything. Nothing this port does ever mutates a published
///   version's content, identity, or meaning.
/// - **No silent changes.** Installing over an existing installation
///   is rejected; upgrades flow through proposal + explicit decision;
///   republishing an existing version identity is rejected.
/// - **Gates before records.** Installs evaluate visibility,
///   integrity, access, and entitlement *before* any record is
///   written; a denial writes nothing.
/// - **No credentials, no payment semantics.** Records carry opaque
///   principals, license identifiers, and digests only.
pub trait DistributionPort: Send + Sync {
    /// Publishes a sealed release with its metadata into
    /// distribution under a visibility scope.
    ///
    /// The version must verify, the metadata must verify and its
    /// compatibility claims must cover the version's own
    /// declarations, commercial terms (when present) must verify and
    /// bind to the same release, and the version identity must be
    /// new. The returned listing is the entry's metadata-only view.
    fn publish(
        &mut self,
        submission: PublishSubmission,
    ) -> Result<MarketplaceListing, WorkflowDistributionError>;

    /// Searches published releases by facets. Results are
    /// metadata-only listings of immutable versions, filtered by the
    /// requesting principal's visibility.
    fn search(
        &self,
        query: &MarketplaceQuery,
    ) -> Result<Vec<MarketplaceListing>, WorkflowDistributionError>;

    /// Fetches the full sealed version record by workflow and
    /// immutable version identity.
    fn fetch_version(
        &self,
        workflow: &WorkflowDefinitionId,
        version: &WorkflowVersionId,
    ) -> Result<Option<WorkflowVersion>, WorkflowDistributionError>;

    /// Fetches a release entry's marketplace listing (metadata-only
    /// view) by workflow and version identity.
    fn fetch_listing(
        &self,
        workflow: &WorkflowDefinitionId,
        version: &WorkflowVersionId,
    ) -> Result<Option<MarketplaceListing>, WorkflowDistributionError>;

    /// Forks a published release: derives a new immutable version in
    /// a new repository with provenance pointing back to the
    /// upstream.
    ///
    /// The upstream release is never mutated; the derived release is
    /// a new sealed version whose metadata lineage points at the
    /// upstream immutable version reference. The fork entry starts in
    /// the [`DistributionState::Forked`] state (owner-visible) and is
    /// shared or listed through explicit transitions.
    fn fork(
        &mut self,
        request: ForkRequest,
    ) -> Result<MarketplaceListing, WorkflowDistributionError>;

    /// Installs a published release under the license/access and
    /// entitlement gates.
    ///
    /// Gate order: visibility → integrity → access policy →
    /// commercial entitlement → record. Re-installing a workflow that
    /// is already installed is rejected: changes flow through the
    /// explicit upgrade path. The returned record carries the sealed
    /// version for the host's operational installation.
    fn install(
        &mut self,
        request: MarketplaceInstallRequest,
    ) -> Result<MarketplaceInstall, WorkflowDistributionError>;

    /// Applies an explicit distribution-state transition to a release
    /// entry (share, make-public, make-private).
    ///
    /// The transition must be legal for the entry's current state and
    /// must not be a no-op; it changes only the entry's scope record —
    /// never the published version, its metadata document, or any
    /// installation.
    fn apply_transition(
        &mut self,
        workflow: &WorkflowDefinitionId,
        version: &WorkflowVersionId,
        transition: DistributionTransition,
    ) -> Result<MarketplaceListing, WorkflowDistributionError>;

    /// Surfaces an upgrade proposal for an installed workflow.
    ///
    /// `None` when the installation's policy is
    /// [`UpgradePolicySetting::Pin`] (pins never surface proposals)
    /// or when no other release of the workflow is published. A
    /// proposal is pure data; nothing changes until
    /// [`Self::decide_upgrade`].
    fn evaluate_upgrade(
        &self,
        workflow: &WorkflowDefinitionId,
    ) -> Result<Option<UpgradeProposal>, WorkflowDistributionError>;

    /// Decides an upgrade proposal explicitly.
    ///
    /// The proposal must still match the installation (its `from`
    /// equals the installed version); a superseded proposal is
    /// refused. Approval re-runs every install gate for the new
    /// release — an upgrade the gates would refuse is refused with
    /// the decision data and changes nothing. Rejections keep the pin
    /// and are still recorded.
    fn decide_upgrade(
        &mut self,
        proposal: &UpgradeProposal,
        decision: UpgradeDecision,
        at_unix_ms: u64,
    ) -> Result<UpgradeRecord, WorkflowDistributionError>;

    /// The distribution-plane installation record for a workflow,
    /// when installed.
    fn installed(
        &self,
        workflow: &WorkflowDefinitionId,
    ) -> Result<Option<MarketplaceInstall>, WorkflowDistributionError>;
}
