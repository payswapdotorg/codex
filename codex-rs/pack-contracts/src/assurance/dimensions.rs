//! The four requirement-level assurance dimensions.
//!
//! Determinism, replay, approval, and evidence declare how strictly a
//! covered operation must behave and what it must record, independent of
//! any pinned identity (the pinning dimensions live in
//! [`super::pinning`]). Each dimension is independently declarable:
//! `NotRequired` means "no requirement", never a global default.
//!
//! Strictness levels are ordered from least to most strict in their
//! variant declaration order, and composition always keeps the stricter
//! requirement. The approval dimension additionally narrows the class of
//! principal that may approve; irreconcilable classes are explicit
//! conflicts.

use serde::Deserialize;
use serde::Serialize;

use crate::PackContractError;

/// How strictly repeated execution must behave.
///
/// Variants are ordered from least to most strict; assurance composition
/// keeps the stricter level.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DeterminismLevel {
    /// Repeated executions of the same inputs must converge on the same
    /// outcome, even if intermediate representations differ.
    Convergent,
    /// Identical inputs must produce identical outputs.
    Exact,
}

/// How deeply execution must be replayable.
///
/// Variants are ordered from least to most strict; assurance composition
/// keeps the stricter scope.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ReplayScope {
    /// Executions must be recorded so their inputs and outputs can be
    /// audited later.
    Record,
    /// Executions must additionally be re-executable from the recorded
    /// state.
    Reexecute,
}

/// How much evidence an execution must record.
///
/// Variants are ordered from least to most strict; assurance composition
/// keeps the stricter level.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum EvidenceLevel {
    /// Executions must record outcome evidence: the result and references
    /// to produced artifacts.
    Outcome,
    /// Executions must record a full trace of steps and decisions, not
    /// just the outcome.
    Trace,
}

/// The class of principal whose approval satisfies an approval
/// requirement.
///
/// Classes are descriptive requirements, not grants of authority: naming
/// `Human` requires a human approver, it does not vest any human with
/// approval rights.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ApproverClass {
    /// Any principal vested with approval rights for the pack (human,
    /// agent, or control plane).
    StandingApprover,
    /// A human principal vested with approval rights for the pack.
    Human,
    /// The governing control-plane principal.
    ControlPlane,
}

impl ApproverClass {
    /// Narrows two approver classes into their joint requirement.
    ///
    /// `StandingApprover` is the least strict class, so composing it with
    /// any other class yields the other class. `Human` and `ControlPlane`
    /// are each stricter than `StandingApprover` but share no common
    /// principal class, so composing them is an explicit
    /// [`PackContractError::PolicyConflict`]. The operation is
    /// commutative.
    pub fn compose(self, other: Self) -> Result<Self, PackContractError> {
        if self == other || other == Self::StandingApprover {
            Ok(self)
        } else if self == Self::StandingApprover {
            Ok(other)
        } else {
            Err(PackContractError::PolicyConflict {
                reason: "approver class conflict: `human` and `controlPlane` approvals have no common principal class"
                    .to_owned(),
            })
        }
    }
}

/// How many approvals, and from which class of principal, satisfy an
/// approval requirement.
///
/// The fields are crate-private so the "at least one approval" invariant
/// is preserved by the validated constructor; the named profile presets in
/// the parent module construct thresholds directly with statically valid
/// counts.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ApprovalThreshold {
    /// Minimum number of distinct approvals; always at least one.
    minimum_approvals: u32,
    /// The class of principal whose approvals satisfy this threshold.
    approver: ApproverClass,
}

impl ApprovalThreshold {
    /// Creates a validated approval threshold.
    ///
    /// Zero approvals is rejected: "required with zero approvals" states
    /// the same requirement as [`ApprovalPolicy::NotRequired`], and
    /// rejecting it keeps the no-requirement representation canonical,
    /// which keeps content-addressed policy identity unambiguous.
    pub fn new(minimum_approvals: u32, approver: ApproverClass) -> Result<Self, PackContractError> {
        if minimum_approvals == 0 {
            Err(PackContractError::InvalidIdentifier {
                kind: "approval threshold",
                value: minimum_approvals.to_string(),
                reason: "minimum approvals must be at least 1; use `NotRequired` when no approval is required",
            })
        } else {
            Ok(Self {
                minimum_approvals,
                approver,
            })
        }
    }

    /// The minimum number of distinct approvals (always at least one).
    pub fn minimum_approvals(&self) -> u32 {
        self.minimum_approvals
    }

    /// The class of principal whose approvals satisfy this threshold.
    pub fn approver(&self) -> ApproverClass {
        self.approver
    }

    /// Merges two thresholds into their joint requirement.
    ///
    /// The stricter count wins (the maximum) and the approver class
    /// narrows per [`ApproverClass::compose`], so the merged threshold is
    /// satisfiable by approvals that satisfy both inputs. Irreconcilable
    /// approver classes surface as [`PackContractError::PolicyConflict`].
    /// The operation is commutative.
    pub fn compose(&self, other: &Self) -> Result<Self, PackContractError> {
        let approver = self.approver.compose(other.approver)?;
        Ok(Self {
            minimum_approvals: self.minimum_approvals.max(other.minimum_approvals),
            approver,
        })
    }
}

/// The determinism assurance dimension.
///
/// Declares whether covered execution must be deterministic, and at what
/// strictness. `NotRequired` means the dimension imposes no requirement;
/// it never implies that execution must be non-deterministic.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum DeterminismPolicy {
    /// Deterministic execution is not required.
    #[default]
    NotRequired,
    /// Deterministic execution is required at the given strictness.
    Required {
        /// The strictness the covered execution must satisfy.
        level: DeterminismLevel,
    },
}

impl DeterminismPolicy {
    /// Whether this dimension imposes no requirement.
    pub fn is_not_required(&self) -> bool {
        matches!(self, Self::NotRequired)
    }

    /// The enforced strictness, or `None` when this dimension imposes no
    /// requirement.
    pub fn level(&self) -> Option<DeterminismLevel> {
        match self {
            Self::NotRequired => None,
            Self::Required { level } => Some(*level),
        }
    }

    /// Composes two determinism requirements.
    ///
    /// The stricter level wins; a `NotRequired` side adopts the other
    /// requirement. Composition never fails: any two determinism
    /// requirements are reconcilable by taking the stricter one.
    pub fn compose(&self, other: &Self) -> Result<Self, PackContractError> {
        match (self.level(), other.level()) {
            (None, None) => Ok(Self::NotRequired),
            (Some(level), None) | (None, Some(level)) => Ok(Self::Required { level }),
            (Some(left), Some(right)) => Ok(Self::Required {
                level: left.max(right),
            }),
        }
    }
}

/// The replay assurance dimension.
///
/// Declares whether covered execution must be recorded, and whether it
/// must be re-executable. The requirement declares what must hold; replay
/// machinery itself is owned by later Pack work orders.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ReplayPolicy {
    /// Replay is not required.
    #[default]
    NotRequired,
    /// Replay is required at the given scope.
    Required {
        /// The replay scope the covered execution must satisfy.
        scope: ReplayScope,
    },
}

impl ReplayPolicy {
    /// Whether this dimension imposes no requirement.
    pub fn is_not_required(&self) -> bool {
        matches!(self, Self::NotRequired)
    }

    /// The enforced replay scope, or `None` when this dimension imposes no
    /// requirement.
    pub fn scope(&self) -> Option<ReplayScope> {
        match self {
            Self::NotRequired => None,
            Self::Required { scope } => Some(*scope),
        }
    }

    /// Composes two replay requirements.
    ///
    /// The stricter scope wins; a `NotRequired` side adopts the other
    /// requirement. Composition never fails.
    pub fn compose(&self, other: &Self) -> Result<Self, PackContractError> {
        match (self.scope(), other.scope()) {
            (None, None) => Ok(Self::NotRequired),
            (Some(scope), None) | (None, Some(scope)) => Ok(Self::Required { scope }),
            (Some(left), Some(right)) => Ok(Self::Required {
                scope: left.max(right),
            }),
        }
    }
}

/// The approval assurance dimension.
///
/// Declares whether approvals are required before a covered operation may
/// proceed, and at what threshold. The requirement constrains the
/// operation; it does not grant anyone the right to approve, and it does
/// not run an approval workflow.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ApprovalPolicy {
    /// No approval is required.
    #[default]
    NotRequired,
    /// Approvals are required before the covered operation may proceed.
    Required {
        /// How many approvals, and from which class of principal.
        threshold: ApprovalThreshold,
    },
}

impl ApprovalPolicy {
    /// Whether this dimension imposes no requirement.
    pub fn is_not_required(&self) -> bool {
        matches!(self, Self::NotRequired)
    }

    /// The enforced threshold, or `None` when this dimension imposes no
    /// requirement.
    pub fn threshold(&self) -> Option<&ApprovalThreshold> {
        match self {
            Self::NotRequired => None,
            Self::Required { threshold } => Some(threshold),
        }
    }

    /// Composes two approval requirements.
    ///
    /// Thresholds merge per [`ApprovalThreshold::compose`]: the stricter
    /// count wins and the approver class narrows. Irreconcilable approver
    /// classes surface as [`PackContractError::PolicyConflict`].
    pub fn compose(&self, other: &Self) -> Result<Self, PackContractError> {
        match (self.threshold(), other.threshold()) {
            (None, None) => Ok(Self::NotRequired),
            (Some(threshold), None) | (None, Some(threshold)) => Ok(Self::Required {
                threshold: threshold.clone(),
            }),
            (Some(left), Some(right)) => Ok(Self::Required {
                threshold: left.compose(right)?,
            }),
        }
    }
}

/// The evidence assurance dimension.
///
/// Declares how much evidence covered execution must record. The
/// requirement declares what must be recorded; evidence collection,
/// storage, and verification are owned by the evidence authority and
/// later Pack work orders.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum EvidencePolicy {
    /// No evidence recording is required.
    #[default]
    NotRequired,
    /// Evidence recording is required at the given level.
    Required {
        /// The evidence level the covered execution must record.
        level: EvidenceLevel,
    },
}

impl EvidencePolicy {
    /// Whether this dimension imposes no requirement.
    pub fn is_not_required(&self) -> bool {
        matches!(self, Self::NotRequired)
    }

    /// The enforced evidence level, or `None` when this dimension imposes
    /// no requirement.
    pub fn level(&self) -> Option<EvidenceLevel> {
        match self {
            Self::NotRequired => None,
            Self::Required { level } => Some(*level),
        }
    }

    /// Composes two evidence requirements.
    ///
    /// The stricter level wins; a `NotRequired` side adopts the other
    /// requirement. Composition never fails.
    pub fn compose(&self, other: &Self) -> Result<Self, PackContractError> {
        match (self.level(), other.level()) {
            (None, None) => Ok(Self::NotRequired),
            (Some(level), None) | (None, Some(level)) => Ok(Self::Required { level }),
            (Some(left), Some(right)) => Ok(Self::Required {
                level: left.max(right),
            }),
        }
    }
}
