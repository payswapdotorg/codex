//! Distribution states and explicit state transitions.
//!
//! The six frozen distribution states — [`DistributionState::Private`],
//! [`DistributionState::Shared`], [`DistributionState::Public`],
//! [`DistributionState::Forked`], [`DistributionState::Installed`], and
//! [`DistributionState::Published`] — are contract types over IMMUTABLE
//! workflow versions. They describe *distribution-plane records only*:
//! no transition ever mutates a published version's executable meaning,
//! because a published version is sealed forever (its identity digest
//! recomputes; see `WorkflowVersion::verify_integrity`).
//!
//! Which record carries which state:
//!
//! - a **release entry** (a published version inside distribution) is
//!   [`Private`], [`Shared`], or [`Public`] — its visibility scope;
//!   [`Published`] when it was released without a listing scope (an
//!   unlisted release, installable by those who can address it but not
//!   surfaced in discovery); or [`Forked`] when the release is a derived
//!   work whose provenance points back to an upstream immutable
//!   version;
//! - an **installation record** (a consumer-side pin) is
//!   [`Installed`].
//!
//! Transitions are explicit ([`DistributionTransition`]) and validated
//! against the legal graph by [`validate_transition`]; the marketplace
//! port is the only place they execute:
//!
//! ```text
//! publish (no scope)   ───────────────▶ Published
//! publish (scope)      ───────────────▶ Private | Shared | Public
//! fork                 ───────────────▶ Forked (a NEW immutable version)
//! install              ───────────────▶ Installed (a consumer pin)
//!
//! Published ──share──▶ Shared ──make-public──▶ Public
//! Private ───share──▶ Shared ───make-public──▶ Public
//! Forked ────share──▶ Shared ───make-public──▶ Public
//! Public ────make-private──▶ Private     Shared ──make-private──▶ Private
//! Published ──make-public/make-private──▶ Public / Private
//! ```
//!
//! No-op transitions (for example making a `Public` entry public again)
//! are refused, mirroring the forge's refusal of no-op updates: scope
//! changes are always explicit and observable.

use serde::Deserialize;
use serde::Serialize;

use crate::WorkflowDistributionError;

/// The distribution state of a workflow record: the six frozen states
/// over immutable versions.
///
/// States are data, never authority over published meaning: a scope
/// change (Private → Shared → Public, or narrowing back) changes only
/// *who can discover and address the release*, never what the release
/// is. Installations that already pinned the version keep working when
/// a release is unlisted, exactly like unlisted plugin catalog entries.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub enum DistributionState {
    /// Published and visible to the owning principal only.
    Private,
    /// Published and visible to an explicit audience.
    Shared,
    /// Published and discoverable by anyone through marketplace search.
    Public,
    /// A derived release: forked from an upstream immutable version,
    /// with provenance pointing back. The derived release is itself a
    /// NEW immutable version; the upstream record is never touched.
    /// A fresh fork is owner-visible until explicitly shared or listed.
    Forked,
    /// An installed pin: a consumer-side record binding a workflow to
    /// an immutable published version identity, with its license and
    /// entitlement decisions and install provenance.
    Installed,
    /// Published and sealed into distribution, but not listed under a
    /// visibility scope (an unlisted release): addressable by those who
    /// already know its identity, invisible to discovery.
    Published,
}

impl DistributionState {
    /// Whether the state is one of the release-entry scope states a
    /// visibility transition can start from.
    pub fn is_scoped_entry(self) -> bool {
        matches!(
            self,
            Self::Private | Self::Shared | Self::Public | Self::Forked | Self::Published
        )
    }

    /// Whether a release in this state is discoverable by an anonymous
    /// marketplace search.
    pub fn is_publicly_discoverable(self) -> bool {
        matches!(self, Self::Public)
    }
}

/// One explicit distribution-state transition on a release entry.
///
/// Transitions carry their own data (the audience of a share) so
/// call-sites stay self-documenting; no boolean or `Option` parameters.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub enum DistributionTransition {
    /// Share the release with an explicit audience (state becomes
    /// [`DistributionState::Shared`]; the previous audience, if any, is
    /// replaced).
    Share {
        /// The principals the release becomes visible to.
        audience: Vec<crate::MarketplacePrincipal>,
    },
    /// Make the release publicly discoverable (state becomes
    /// [`DistributionState::Public`]).
    MakePublic,
    /// Withdraw the release to owner-only visibility (state becomes
    /// [`DistributionState::Private`]).
    MakePrivate,
}

impl DistributionTransition {
    /// The state this transition produces.
    pub fn target(&self) -> DistributionState {
        match self {
            Self::Share { .. } => DistributionState::Shared,
            Self::MakePublic => DistributionState::Public,
            Self::MakePrivate => DistributionState::Private,
        }
    }

    /// A short label used in error reporting.
    pub fn label(&self) -> &'static str {
        match self {
            Self::Share { .. } => "share",
            Self::MakePublic => "make-public",
            Self::MakePrivate => "make-private",
        }
    }
}

/// Validates a visibility transition against the legal state graph and
/// returns the state the entry lands in.
///
/// Rules:
///
/// - only release entries (Private, Shared, Public, Forked, Published)
///   can transition — [`DistributionState::Installed`] belongs to
///   installation records, not entries;
/// - a transition that would land on the state the entry is already in
///   is a refused no-op (scope changes must be observable);
/// - `share` from `Public` narrows the release to an audience and is
///   legal;
/// - every legal transition produces exactly the target state.
pub fn validate_transition(
    from: DistributionState,
    transition: &DistributionTransition,
) -> Result<DistributionState, WorkflowDistributionError> {
    if !from.is_scoped_entry() {
        return Err(WorkflowDistributionError::IllegalTransition {
            transition: transition.label().to_owned(),
            from: state_label(from),
        });
    }
    let target = transition.target();
    if from == target {
        return Err(WorkflowDistributionError::IllegalTransition {
            transition: transition.label().to_owned(),
            from: state_label(from),
        });
    }
    if let DistributionTransition::Share { audience } = transition
        && audience.is_empty()
    {
        return Err(WorkflowDistributionError::InvalidRecord {
            reason: "a shared release must name at least one audience principal".to_owned(),
        });
    }
    Ok(target)
}

/// Stable lowercase label for a state, used in error reporting and
/// records.
pub(crate) fn state_label(state: DistributionState) -> String {
    match state {
        DistributionState::Private => "private".to_owned(),
        DistributionState::Shared => "shared".to_owned(),
        DistributionState::Public => "public".to_owned(),
        DistributionState::Forked => "forked".to_owned(),
        DistributionState::Installed => "installed".to_owned(),
        DistributionState::Published => "published".to_owned(),
    }
}

#[cfg(test)]
#[path = "state_tests.rs"]
mod tests;
