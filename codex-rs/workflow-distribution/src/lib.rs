//! Workflow distribution, marketplace, and monetization (WO-012).
//!
//! This crate is the publication/distribution/licensing/marketplace
//! plane of the Codex universal workflow platform: it makes immutable,
//! published workflow versions **distributable** — publishable,
//! discoverable, forkable, installable under license and commercial
//! policy — without ever coupling payment semantics to executable
//! workflow meaning.
//!
//! ## Position in the stack
//!
//! ```text
//! workflow-contracts (WO-003)  -> frozen records: versions, digests,
//!                                 provenance, dependency locks
//! workflow-forge (WO-009)      -> publish/install/discovery record
//!                                 semantics reused here
//! workflow-triggers (WO-011)   -> installation/configuration bindings
//!                                 (composed at the host level; the
//!                                 access-policy shape is referenced)
//! workflow-distribution (this) -> publish, scope, search, fetch,
//!                                 fork, install gates, entitlements
//! ```
//!
//! The crate sits **over** `codex-workflow-forge` at runtime: every
//! release is a forge [`PublishedVersionRef`] whose identity digest
//! recomputes, installs reuse the forge's no-silent-over-install
//! discipline, and upgrades follow the forge's explicit
//! proposal/decision/audit shape. It sits **over** the WO-011
//! trigger plane's installation/configuration model at the host
//! orchestration level: a marketplace install carries the full sealed
//! [`WorkflowVersion`] so the host drives
//! `codex_workflow_triggers::InstallRequest` for the operational
//! installation (explicit resource and dependency bindings), and the
//! two planes share the forge install-registry discipline. The
//! [`AccessPolicy`] port reuses the WO-011 authorization port *shape*
//! — a decision-as-data record with source attribution, denial never
//! a credential — because the concrete WO-011 port authorizes
//! resource bindings while this one authorizes distribution access;
//! authorization semantics are referenced, not duplicated.
//!
//! ## What this crate is
//!
//! - **Distribution states** ([`DistributionState`]): the six frozen
//!   states — Private, Shared, Public, Forked, Installed, Published —
//!   as contract types over immutable versions, with explicit,
//!   validated transitions ([`DistributionTransition`]). Published
//!   means unlisted (secret-link distribution); a published version
//!   is sealed forever, and no transition touches its meaning.
//! - **Publication metadata** ([`PublicationMetadata`]): a versioned,
//!   integrity-checkable document carrying attribution, ownership,
//!   licensing (SPDX-style identifiers plus custom terms with digests),
//!   source lineage, compatibility (minimum runtime, capabilities,
//!   taxonomy classes, resources), and upgrade policy (pin vs follow),
//!   hash-chained to the immutable release seal.
//! - **A distribution/marketplace seam** ([`DistributionPort`]):
//!   publish, search with filter facets, fetch by id+version, fork (a
//!   new immutable version with provenance pointing back), and
//!   install with license/access policy evaluation. In-memory
//!   implementations ([`InMemoryMarketplace`], [`InMemoryAccessPolicy`],
//!   [`InMemoryEntitlements`]) serve tests and host seeding.
//! - **A separate commercial boundary** ([`EntitlementPort`]): paid
//!   release / subscription / license-grant **checks only**.
//!   Commercial terms ([`CommercialTerms`]) are a digest-bound policy
//!   class — no prices, no payment processing, no credentials — and
//!   entitlement grants live in a host-owned store.
//! - **Marketplace discovery** ([`MarketplaceQuery`],
//!   [`MarketplaceListing`]): search/filter over published releases
//!   by capability, capability class, resource needs, license,
//!   provenance, and compatibility; results are metadata-only views
//!   of immutable versions.
//!
//! ## What this crate deliberately is not
//!
//! - **No payment semantics anywhere.** No prices, no billing, no
//!   settlement, no payment credentials, and nothing monetization-
//!   shaped enters workflow source, evidence, or executable records.
//!   Denial of an entitlement gates distribution and installation
//!   only — it can never change executable semantics.
//! - **No mutation of published meaning.** A published version is
//!   sealed forever: every operation re-verifies integrity before
//!   recording anything, scope transitions touch only distribution
//!   records, and upgrades move pins without touching versions.
//! - **No forge specifics.** GitHub is one forge behind the frozen
//!   `WorkflowForge` boundary; this crate's contracts are
//!   forge-neutral (repository identity is canonical and
//!   credential-free by construction).
//! - **No durable state and no execution.** The in-memory collections
//!   are reference implementations and test doubles; the Codex
//!   application persists records through the workflow control plane,
//!   and execution belongs to the lifecycle and trigger planes.
//!
//! ## Ordinary Codex compatibility
//!
//! Nothing in this crate executes unless the application drives it.
//! With no marketplace, no publication, and no installation, every
//! operation is an inert `UnknownRelease`/`NotInstalled` result: no
//! adapter is probed, no version selected, no instance created. The
//! ordinary coding-agent behavior of Codex is untouched.
//!
//! ## Module map
//!
//! - `error` — the plane's error type, with decision data carried in
//!   refusal variants.
//! - `state` — distribution states and the explicit transition graph.
//! - `metadata` — the publication metadata document and its hash
//!   chain, plus principal/class/license/lineage/compatibility types.
//! - `commerce` — commercial terms: the separate, digest-bound
//!   commercial policy boundary.
//! - `entitlement` — the checks-only entitlement port and its
//!   in-memory registry.
//! - `access` — the access-policy seam (license/sharing evaluation)
//!   and its in-memory double.
//! - `port` — the distribution/marketplace seam and the
//!   install/upgrade records.
//! - `discovery` — marketplace queries, listings, and pure filtering.
//! - `memory` — the in-memory reference marketplace.

#![deny(missing_docs)]

mod access;
mod commerce;
mod discovery;
mod entitlement;
mod error;
mod memory;
mod metadata;
mod port;
mod state;

pub use access::{AccessDecision, AccessPolicy, AccessRequest, InMemoryAccessPolicy};
pub use commerce::{COMMERCIAL_TERMS_FORMAT_VERSION, CommercialPolicy, CommercialTerms};
pub use discovery::{
    MarketplaceListing, MarketplaceQuery, listing_matches, listing_visible, release_installable,
};
pub use entitlement::{
    EntitlementDecision, EntitlementPort, EntitlementRequest, InMemoryEntitlements,
};
pub use error::WorkflowDistributionError;
pub use memory::InMemoryMarketplace;
pub use metadata::{
    CapabilityClass, CompatibilitySpec, LicenseTerms, MarketplacePrincipal,
    PUBLICATION_METADATA_FORMAT_VERSION, PublicationMetadata, SourceLineage, UpgradePolicySetting,
};
pub use port::{
    DistributionPort, ForkRequest, MarketplaceInstall, MarketplaceInstallRequest, PublicationScope,
    PublishSubmission, UpgradeDecision, UpgradeProposal, UpgradeRecord,
};
pub use state::{DistributionState, DistributionTransition, validate_transition};
