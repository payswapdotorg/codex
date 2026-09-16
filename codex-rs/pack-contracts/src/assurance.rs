//! Policy-scoped assurance and determinism contracts.
//!
//! Owned by the PACK-003 work order (Assurance / Determinism).
//!
//! Pack assurance is policy-scoped, never global. A pack or pack operation
//! may independently require deterministic execution, replay, approval,
//! evidence, model pinning, dependency pinning, or environment pinning. An
//! absent dimension means "no requirement": there is no hidden global
//! default, no platform-wide deterministic execution mode, and no D0–D5
//! execution framework encoded into runtime semantics. The named profiles
//! (see [`AssuranceProfile`]) are convenience presets over these
//! dimensions, not semantic authority.
//!
//! ## Module layout
//!
//! - [`dimensions`] — the four requirement-level dimensions (determinism,
//!   replay, approval, evidence) and their strictness levels.
//! - [`pinning`] — the three pinning dimensions (model, dependency,
//!   environment) and their pin values.
//!
//! ## Authority boundary
//!
//! A [`PackAssurancePolicy`] DECLARES requirements. It does not authorize
//! anything, cannot bypass approvals, cannot manufacture evidence, and
//! cannot perform workflow transitions: the Workflow Control Plane remains
//! the sole authority for legal durable workflow transitions. Enforcement
//! machinery — executors, execution verifiers, replay engines, approval
//! workflows, experiment controllers, and rollback engines — is owned by
//! later Pack work orders.
//!
//! An executor that consults a policy can report exactly which assurance
//! dimensions were in effect through
//! [`PackAssurancePolicy::required_dimensions`]; that report is descriptive
//! only and grants no authority.
//!
//! ## Workflow-semantics boundary
//!
//! This module does not import or model workflow-execution concepts. Its
//! only dependency on Universal workflow machinery is content addressing:
//! policy identity is derived with
//! [`codex_workflow_contracts::ContentDigest`] and expressed as a
//! [`PackPolicyId`](crate::PackPolicyId).
//!
//! ## Non-goals
//!
//! - No rollback requirement dimension: rollback checkpoints are
//!   system-state machinery owned by the PACK-002 system-state contracts,
//!   and rollback automation is a later phase. The declarable assurance
//!   surface here is covered by the evidence and approval dimensions.
//! - No credential material: pin values are identity/digest references
//!   only and never carry tokens, keys, or passwords.

mod dimensions;
mod pinning;

pub use dimensions::*;
pub use pinning::*;

use std::collections::BTreeSet;

use codex_workflow_contracts::ContentDigest;
use serde::Deserialize;
use serde::Serialize;

use crate::PackContractError;
use crate::PackPolicyId;

/// One assurance dimension that a policy may enforce.
///
/// The set of enforceable dimensions is exactly the seven scoped
/// dimensions of this module; adding a dimension is a deliberate contract
/// change, never an implicit platform default.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AssuranceDimension {
    /// Deterministic execution requirements.
    Determinism,
    /// Replay requirements.
    Replay,
    /// Approval requirements.
    Approval,
    /// Evidence requirements.
    Evidence,
    /// Model pinning requirements.
    ModelPinning,
    /// Dependency pinning requirements.
    DependencyPinning,
    /// Environment pinning requirements.
    EnvironmentPinning,
}

/// A typed report of which assurance dimensions a policy enforces, and at
/// what strictness.
///
/// Produced by [`PackAssurancePolicy::required_dimensions`] so an executor
/// can report exactly which assurance dimensions were in effect for a
/// covered operation. The report is descriptive: it carries no authority
/// and cannot approve, waive, or manufacture anything.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AssuranceRequirements {
    /// The enforced determinism strictness, or `None` when not enforced.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub determinism: Option<DeterminismLevel>,
    /// The enforced replay scope, or `None` when not enforced.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub replay: Option<ReplayScope>,
    /// The enforced approval threshold, or `None` when not enforced.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub approval: Option<ApprovalThreshold>,
    /// The enforced evidence level, or `None` when not enforced.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evidence: Option<EvidenceLevel>,
    /// The enforced model pin, or `None` when not enforced.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_pinning: Option<ModelPin>,
    /// The enforced dependency pin, or `None` when not enforced.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dependency_pinning: Option<DependencyPin>,
    /// The enforced environment pin, or `None` when not enforced.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub environment_pinning: Option<EnvironmentPin>,
}

impl AssuranceRequirements {
    /// Reports that no assurance dimensions are enforced.
    pub fn empty() -> Self {
        Self::default()
    }

    /// Whether no assurance dimensions are enforced.
    pub fn is_empty(&self) -> bool {
        self == &Self::empty()
    }

    /// The set of assurance dimensions this report enforces.
    pub fn enforced_dimensions(&self) -> BTreeSet<AssuranceDimension> {
        let mut enforced = BTreeSet::new();
        if self.determinism.is_some() {
            enforced.insert(AssuranceDimension::Determinism);
        }
        if self.replay.is_some() {
            enforced.insert(AssuranceDimension::Replay);
        }
        if self.approval.is_some() {
            enforced.insert(AssuranceDimension::Approval);
        }
        if self.evidence.is_some() {
            enforced.insert(AssuranceDimension::Evidence);
        }
        if self.model_pinning.is_some() {
            enforced.insert(AssuranceDimension::ModelPinning);
        }
        if self.dependency_pinning.is_some() {
            enforced.insert(AssuranceDimension::DependencyPinning);
        }
        if self.environment_pinning.is_some() {
            enforced.insert(AssuranceDimension::EnvironmentPinning);
        }
        enforced
    }
}

/// Named assurance profiles from the Pack architecture.
///
/// Profiles are convenience PRESETS over the assurance dimensions, not
/// semantic authority: the authoritative record is always the full
/// dimension set ([`PackAssurancePolicy`]), and two policies compose the
/// same way regardless of which presets produced them.
///
/// Presets never fabricate pin values: the three pinning dimensions are
/// always `NotRequired` in a preset policy, because pins reference real
/// immutable content digests that only policy authors can supply.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AssuranceProfile {
    /// Exploratory work with no assurance requirements.
    Creative,
    /// Repeated runs converge and record their outcomes.
    Repeatable,
    /// Executions are recorded and re-executable with full traces.
    Replayable,
    /// Identical inputs produce identical outputs, recorded for
    /// verification with full traces.
    Deterministic,
    /// Covered operations require standing approval and full traces.
    Assured,
    /// The strictest preset: exact determinism, re-executable replay, two
    /// distinct human approvals (the four-eyes principle), and full
    /// traces.
    MissionCritical,
}

impl AssuranceProfile {
    /// The preset policy for this profile.
    ///
    /// A plain convenience constructor over the assurance dimensions; it
    /// grants no authority, and callers remain free to declare any other
    /// dimension combination directly.
    ///
    /// Construction is validated like any other policy assembly: the
    /// threshold values the presets declare are statically valid, but they
    /// still pass through [`ApprovalThreshold::new`], so a preset can never
    /// smuggle in an unvalidated threshold.
    pub fn policy(&self) -> Result<PackAssurancePolicy, PackContractError> {
        let policy = match self {
            Self::Creative => PackAssurancePolicy::empty(),
            Self::Repeatable => PackAssurancePolicy::empty()
                .with_determinism(DeterminismPolicy::Required {
                    level: DeterminismLevel::Convergent,
                })
                .with_evidence(EvidencePolicy::Required {
                    level: EvidenceLevel::Outcome,
                }),
            Self::Replayable => PackAssurancePolicy::empty()
                .with_replay(ReplayPolicy::Required {
                    scope: ReplayScope::Reexecute,
                })
                .with_evidence(EvidencePolicy::Required {
                    level: EvidenceLevel::Trace,
                }),
            Self::Deterministic => PackAssurancePolicy::empty()
                .with_determinism(DeterminismPolicy::Required {
                    level: DeterminismLevel::Exact,
                })
                .with_replay(ReplayPolicy::Required {
                    scope: ReplayScope::Record,
                })
                .with_evidence(EvidencePolicy::Required {
                    level: EvidenceLevel::Trace,
                }),
            Self::Assured => PackAssurancePolicy::empty()
                .with_approval(ApprovalPolicy::Required {
                    threshold: ApprovalThreshold::new(1, ApproverClass::StandingApprover)?,
                })
                .with_evidence(EvidencePolicy::Required {
                    level: EvidenceLevel::Trace,
                }),
            Self::MissionCritical => PackAssurancePolicy::empty()
                .with_determinism(DeterminismPolicy::Required {
                    level: DeterminismLevel::Exact,
                })
                .with_replay(ReplayPolicy::Required {
                    scope: ReplayScope::Reexecute,
                })
                .with_approval(ApprovalPolicy::Required {
                    threshold: ApprovalThreshold::new(2, ApproverClass::Human)?,
                })
                .with_evidence(EvidencePolicy::Required {
                    level: EvidenceLevel::Trace,
                }),
        };
        Ok(policy)
    }
}

/// The policy-scoped assurance record for a pack or pack operation.
///
/// Each of the seven dimensions is independently declarable; an absent
/// dimension means "no requirement", never a hidden global default. The
/// record never implies that uncovered operations must be deterministic —
/// or non-deterministic. Identity is content-addressed through
/// [`Self::policy_id`]: identical policy content always yields the same
/// [`PackPolicyId`], and any change to any dimension yields a different
/// one.
///
/// ## Authority
///
/// A `PackAssurancePolicy` DECLARES requirements. It does not authorize
/// anything, cannot bypass approvals, cannot manufacture evidence, and
/// cannot perform workflow transitions. Enforcement — executing against
/// these requirements, verifying them, replaying executions, and running
/// approval workflows — is owned by later Pack work orders and the
/// Workflow Control Plane.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PackAssurancePolicy {
    /// The determinism dimension.
    #[serde(default, skip_serializing_if = "DeterminismPolicy::is_not_required")]
    pub determinism: DeterminismPolicy,
    /// The replay dimension.
    #[serde(default, skip_serializing_if = "ReplayPolicy::is_not_required")]
    pub replay: ReplayPolicy,
    /// The approval dimension.
    #[serde(default, skip_serializing_if = "ApprovalPolicy::is_not_required")]
    pub approval: ApprovalPolicy,
    /// The evidence dimension.
    #[serde(default, skip_serializing_if = "EvidencePolicy::is_not_required")]
    pub evidence: EvidencePolicy,
    /// The model pinning dimension.
    #[serde(default, skip_serializing_if = "ModelPinningPolicy::is_not_required")]
    pub model_pinning: ModelPinningPolicy,
    /// The dependency pinning dimension.
    #[serde(
        default,
        skip_serializing_if = "DependencyPinningPolicy::is_not_required"
    )]
    pub dependency_pinning: DependencyPinningPolicy,
    /// The environment pinning dimension.
    #[serde(
        default,
        skip_serializing_if = "EnvironmentPinningPolicy::is_not_required"
    )]
    pub environment_pinning: EnvironmentPinningPolicy,
}

impl PackAssurancePolicy {
    /// A policy that enforces no assurance dimensions.
    ///
    /// The empty policy is the identity element of [`Self::compose`]:
    /// composing it with any policy yields that policy unchanged. It does
    /// not represent a global "non-deterministic mode"; it simply declares
    /// no requirements.
    pub fn empty() -> Self {
        Self::default()
    }

    /// Sets the determinism dimension (builder style).
    pub fn with_determinism(mut self, determinism: DeterminismPolicy) -> Self {
        self.determinism = determinism;
        self
    }

    /// Sets the replay dimension (builder style).
    pub fn with_replay(mut self, replay: ReplayPolicy) -> Self {
        self.replay = replay;
        self
    }

    /// Sets the approval dimension (builder style).
    pub fn with_approval(mut self, approval: ApprovalPolicy) -> Self {
        self.approval = approval;
        self
    }

    /// Sets the evidence dimension (builder style).
    pub fn with_evidence(mut self, evidence: EvidencePolicy) -> Self {
        self.evidence = evidence;
        self
    }

    /// Sets the model pinning dimension (builder style).
    pub fn with_model_pinning(mut self, model_pinning: ModelPinningPolicy) -> Self {
        self.model_pinning = model_pinning;
        self
    }

    /// Sets the dependency pinning dimension (builder style).
    pub fn with_dependency_pinning(mut self, dependency_pinning: DependencyPinningPolicy) -> Self {
        self.dependency_pinning = dependency_pinning;
        self
    }

    /// Sets the environment pinning dimension (builder style).
    pub fn with_environment_pinning(
        mut self,
        environment_pinning: EnvironmentPinningPolicy,
    ) -> Self {
        self.environment_pinning = environment_pinning;
        self
    }

    /// The content-addressed identity of this policy.
    ///
    /// The digest covers the full declared dimension content in canonical
    /// form: identical policies always produce the same [`PackPolicyId`],
    /// and any change to any dimension produces a different one.
    pub fn policy_id(&self) -> Result<PackPolicyId, PackContractError> {
        let digest = ContentDigest::of(self).map_err(|error| PackContractError::InvalidDigest {
            reason: format!("failed to digest assurance policy content: {error}"),
        })?;
        Ok(PackPolicyId::from_digest(digest))
    }

    /// Reports which assurance dimensions this policy enforces, and at
    /// what strictness.
    ///
    /// This is the report an executor consults to state exactly which
    /// assurance dimensions were in effect. The report is descriptive
    /// only: it grants no authority and never mutates the policy.
    pub fn required_dimensions(&self) -> AssuranceRequirements {
        AssuranceRequirements {
            determinism: self.determinism.level(),
            replay: self.replay.scope(),
            approval: self.approval.threshold().cloned(),
            evidence: self.evidence.level(),
            model_pinning: self.model_pinning.pin().cloned(),
            dependency_pinning: self.dependency_pinning.pin().cloned(),
            environment_pinning: self.environment_pinning.pin().cloned(),
        }
    }

    /// Composes two assurance policies into their joint requirement.
    ///
    /// Composition is dimension-by-dimension and predictable:
    ///
    /// - determinism: the stricter level wins (`exact` over `convergent`);
    /// - replay: the stricter scope wins (`reexecute` over `record`);
    /// - approval: the stricter count wins (the maximum), and the approver
    ///   class narrows (`standingApprover` composes with any other class
    ///   to that class; `human` and `controlPlane` conflict);
    /// - evidence: the stricter level wins (`trace` over `outcome`);
    /// - model, dependency, and environment pinning: equal pins compose to
    ///   the same pin, while different pins are an explicit
    ///   [`PackContractError::PolicyConflict`] because neither pin is
    ///   stricter — they are different immutable targets.
    ///
    /// The operation is commutative — including conflict messages, which
    /// name the conflicting values in a canonical order — and associative
    /// up to conflict identity (different groupings of an irreconcilable
    /// set both fail, possibly naming a different conflicting pair). The
    /// empty policy is the identity element. Composing never escalates
    /// authority: the result is only ever the union of what both sides
    /// already required.
    pub fn compose(&self, other: &Self) -> Result<Self, PackContractError> {
        Ok(Self {
            determinism: self.determinism.compose(&other.determinism)?,
            replay: self.replay.compose(&other.replay)?,
            approval: self.approval.compose(&other.approval)?,
            evidence: self.evidence.compose(&other.evidence)?,
            model_pinning: self.model_pinning.compose(&other.model_pinning)?,
            dependency_pinning: self.dependency_pinning.compose(&other.dependency_pinning)?,
            environment_pinning: self
                .environment_pinning
                .compose(&other.environment_pinning)?,
        })
    }
}

#[cfg(test)]
#[path = "assurance_tests.rs"]
mod tests;
