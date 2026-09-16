//! Tests for the policy-scoped assurance and determinism contracts.
//!
//! The systematic tests enumerate the full on/off lattice of the seven
//! assurance dimensions (2^7 = 128 combinations) and compose every
//! lattice policy against every other one, so scope, inspectability,
//! identity, and composition algebra are checked pairwise rather than
//! only on hand-picked cases.

use std::collections::BTreeSet;
use std::collections::HashSet;

use codex_workflow_contracts::ContentDigest;
use pretty_assertions::assert_eq;
use pretty_assertions::assert_ne;
use serde::Serialize;
use serde_json::json;

use super::*;
use crate::PackContractError;
use crate::PackPolicyId;

/// A minimal serializable record used to derive deterministic test
/// digests.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SeedRecord {
    seed: u64,
}

/// A deterministic content digest unique to `seed`.
fn digest_for(seed: u64) -> ContentDigest {
    ContentDigest::of(&SeedRecord { seed }).expect("digest computation succeeds")
}

/// The number of assurance dimensions swept by the lattice tests.
const DIMENSION_COUNT: u32 = 7;

/// The seven dimensions in lattice bit order (bit 0 = determinism,
/// bit 6 = environment pinning).
const LATTICE_DIMENSIONS: [AssuranceDimension; 7] = [
    AssuranceDimension::Determinism,
    AssuranceDimension::Replay,
    AssuranceDimension::Approval,
    AssuranceDimension::Evidence,
    AssuranceDimension::ModelPinning,
    AssuranceDimension::DependencyPinning,
    AssuranceDimension::EnvironmentPinning,
];

/// The approval threshold used by lattice policies.
fn lattice_threshold() -> ApprovalThreshold {
    ApprovalThreshold::new(/*minimum_approvals*/ 1, ApproverClass::Human).expect("valid threshold")
}

/// The model pin used by lattice policies.
fn lattice_model_pin() -> ModelPin {
    ModelPin::new("atlas-reasoner", digest_for(1)).expect("valid model pin")
}

/// The dependency pin used by lattice policies.
fn lattice_dependency_pin() -> DependencyPin {
    DependencyPin::new("billing-workflow", digest_for(2)).expect("valid dependency pin")
}

/// The environment pin used by lattice policies.
fn lattice_environment_pin() -> EnvironmentPin {
    EnvironmentPin::new("production", digest_for(3)).expect("valid environment pin")
}

/// The lattice policy for a dimension on/off bitmask.
fn lattice_policy(mask: u32) -> PackAssurancePolicy {
    let mut policy = PackAssurancePolicy::empty();
    if mask & (1 << 0) != 0 {
        policy = policy.with_determinism(DeterminismPolicy::Required {
            level: DeterminismLevel::Exact,
        });
    }
    if mask & (1 << 1) != 0 {
        policy = policy.with_replay(ReplayPolicy::Required {
            scope: ReplayScope::Reexecute,
        });
    }
    if mask & (1 << 2) != 0 {
        policy = policy.with_approval(ApprovalPolicy::Required {
            threshold: lattice_threshold(),
        });
    }
    if mask & (1 << 3) != 0 {
        policy = policy.with_evidence(EvidencePolicy::Required {
            level: EvidenceLevel::Trace,
        });
    }
    if mask & (1 << 4) != 0 {
        policy = policy.with_model_pinning(ModelPinningPolicy::Required {
            pin: lattice_model_pin(),
        });
    }
    if mask & (1 << 5) != 0 {
        policy = policy.with_dependency_pinning(DependencyPinningPolicy::Required {
            pin: lattice_dependency_pin(),
        });
    }
    if mask & (1 << 6) != 0 {
        policy = policy.with_environment_pinning(EnvironmentPinningPolicy::Required {
            pin: lattice_environment_pin(),
        });
    }
    policy
}

/// The requirements the lattice policy for `mask` must report.
fn lattice_requirements(mask: u32) -> AssuranceRequirements {
    let mut requirements = AssuranceRequirements::empty();
    if mask & (1 << 0) != 0 {
        requirements.determinism = Some(DeterminismLevel::Exact);
    }
    if mask & (1 << 1) != 0 {
        requirements.replay = Some(ReplayScope::Reexecute);
    }
    if mask & (1 << 2) != 0 {
        requirements.approval = Some(lattice_threshold());
    }
    if mask & (1 << 3) != 0 {
        requirements.evidence = Some(EvidenceLevel::Trace);
    }
    if mask & (1 << 4) != 0 {
        requirements.model_pinning = Some(lattice_model_pin());
    }
    if mask & (1 << 5) != 0 {
        requirements.dependency_pinning = Some(lattice_dependency_pin());
    }
    if mask & (1 << 6) != 0 {
        requirements.environment_pinning = Some(lattice_environment_pin());
    }
    requirements
}

/// All 128 dimension on/off combinations.
fn lattice_policies() -> Vec<PackAssurancePolicy> {
    (0..(1 << DIMENSION_COUNT)).map(lattice_policy).collect()
}

/// A single-dimension approval policy (used for threshold algebra tests).
fn approval_only(minimum_approvals: u32, approver: ApproverClass) -> PackAssurancePolicy {
    PackAssurancePolicy::empty().with_approval(ApprovalPolicy::Required {
        threshold: ApprovalThreshold::new(minimum_approvals, approver).expect("valid threshold"),
    })
}

/// Extra policies that vary strictness levels, approval classes, and pin
/// targets so the pairwise composition sweeps exercise conflict paths the
/// uniform lattice cannot reach.
fn variant_policies() -> Vec<PackAssurancePolicy> {
    // Strictness variations for the requirement-level dimensions.
    vec![
        PackAssurancePolicy::empty().with_determinism(DeterminismPolicy::Required {
            level: DeterminismLevel::Convergent,
        }),
        PackAssurancePolicy::empty().with_replay(ReplayPolicy::Required {
            scope: ReplayScope::Record,
        }),
        PackAssurancePolicy::empty().with_evidence(EvidencePolicy::Required {
            level: EvidenceLevel::Outcome,
        }),
        // Approval class and count variations.
        approval_only(1, ApproverClass::StandingApprover),
        approval_only(3, ApproverClass::StandingApprover),
        approval_only(2, ApproverClass::Human),
        approval_only(1, ApproverClass::ControlPlane),
        // Pin variations: different digests and different identities.
        PackAssurancePolicy::empty().with_model_pinning(ModelPinningPolicy::Required {
            pin: ModelPin::new("atlas-reasoner", digest_for(101)).expect("valid model pin"),
        }),
        PackAssurancePolicy::empty().with_model_pinning(ModelPinningPolicy::Required {
            pin: ModelPin::new("bertha-coder", digest_for(1)).expect("valid model pin"),
        }),
        PackAssurancePolicy::empty().with_dependency_pinning(DependencyPinningPolicy::Required {
            pin: DependencyPin::new("billing-workflow", digest_for(102))
                .expect("valid dependency pin"),
        }),
        PackAssurancePolicy::empty().with_environment_pinning(EnvironmentPinningPolicy::Required {
            pin: EnvironmentPin::new("production", digest_for(103)).expect("valid environment pin"),
        }),
        // Multi-dimension variations of the full lattice policy.
        lattice_policy((1 << DIMENSION_COUNT) - 1).with_determinism(DeterminismPolicy::Required {
            level: DeterminismLevel::Convergent,
        }),
        lattice_policy(0b0101010),
    ]
}

#[test]
fn empty_policy_reports_no_enforced_dimensions() {
    let empty = PackAssurancePolicy::empty();
    assert_eq!(
        empty.required_dimensions(),
        AssuranceRequirements::empty(),
        "an empty policy enforces nothing"
    );
    // Composing an empty policy with a declared one is the identity: no
    // hidden global default turns an undeclared dimension into a
    // requirement.
    let declared = lattice_policy(0b0000001);
    let composed = empty
        .compose(&declared)
        .expect("empty composes with anything");
    assert_eq!(
        composed.required_dimensions(),
        declared.required_dimensions(),
        "composition with the empty policy must not add requirements"
    );
}

#[test]
fn each_dimension_is_independently_enforceable() {
    for (index, dimension) in LATTICE_DIMENSIONS.iter().enumerate() {
        let report = lattice_policy(1 << index).required_dimensions();
        assert_eq!(
            report.enforced_dimensions(),
            BTreeSet::from([*dimension]),
            "dimension {index} alone must be enforceable"
        );
    }
}

#[test]
fn dimension_lattice_reports_exactly_the_declared_dimensions() {
    for mask in 0..(1 << DIMENSION_COUNT) {
        let report = lattice_policy(mask).required_dimensions();
        assert_eq!(
            report,
            lattice_requirements(mask),
            "requirements for mask {mask:#09b} must reflect the declared dimensions exactly"
        );
        let expected: BTreeSet<AssuranceDimension> = LATTICE_DIMENSIONS
            .into_iter()
            .enumerate()
            .filter(|&(index, _)| mask & (1 << index) != 0)
            .map(|(_, dimension)| dimension)
            .collect();
        assert_eq!(
            report.enforced_dimensions(),
            expected,
            "enforced dimension set for mask {mask:#09b} must match the bitmask"
        );
    }
}

#[test]
fn no_dimension_combination_implies_global_determinism() {
    // Every combination that does not declare determinism reports none —
    // including the combination that declares every other dimension.
    for mask in 0..(1 << DIMENSION_COUNT) {
        let requires_determinism = mask & (1 << 0) != 0;
        let report = lattice_policy(mask).required_dimensions();
        assert_eq!(
            report.determinism.is_some(),
            requires_determinism,
            "mask {mask:#09b} must not imply an undeclared determinism requirement"
        );
    }
    let everything_but_determinism = lattice_policy(0b1111110).required_dimensions();
    assert_eq!(everything_but_determinism.determinism, None);
    assert_eq!(everything_but_determinism.enforced_dimensions().len(), 6);
}

// ---------------------------------------------------------------------------
// Composition
// ---------------------------------------------------------------------------

#[test]
fn composition_is_commutative_across_the_dimension_lattice() {
    let mut corpus = lattice_policies();
    corpus.extend(variant_policies());
    for left in &corpus {
        for right in &corpus {
            let forward = left.compose(right);
            let backward = right.compose(left);
            match (forward, backward) {
                (Ok(forward), Ok(backward)) => assert_eq!(
                    forward, backward,
                    "composition must be commutative for ({left:?}, {right:?})"
                ),
                (Err(forward), Err(backward)) => assert_eq!(
                    forward.to_string(),
                    backward.to_string(),
                    "conflict messages must be order-independent for ({left:?}, {right:?})"
                ),
                (forward, backward) => panic!(
                    "compose({left:?}, {right:?}) disagrees on commutativity: {forward:?} vs {backward:?}"
                ),
            }
        }
    }
}

#[test]
fn composition_is_associative_over_the_dimension_lattice() {
    let mut sample: Vec<PackAssurancePolicy> = (0..(1 << DIMENSION_COUNT))
        .step_by(16)
        .map(lattice_policy)
        .collect();
    sample.push(lattice_policy((1 << DIMENSION_COUNT) - 1));
    sample.extend(variant_policies());
    for first in &sample {
        for second in &sample {
            for third in &sample {
                let left = first
                    .compose(second)
                    .and_then(|composed| composed.compose(third));
                let right = second
                    .compose(third)
                    .and_then(|composed| first.compose(&composed));
                match (left, right) {
                    (Ok(left), Ok(right)) => assert_eq!(
                        left, right,
                        "composition must be associative for ({first:?}, {second:?}, {third:?})"
                    ),
                    // Both groupings conflict; only the conflicting pair
                    // named in the message may differ between groupings.
                    (Err(_), Err(_)) => {}
                    (left, right) => panic!(
                        "associativity disagreement for ({first:?}, {second:?}, {third:?}): {left:?} vs {right:?}"
                    ),
                }
            }
        }
    }
}

#[test]
fn stricter_requirement_wins_per_dimension() {
    // Determinism: exact outranks convergent; absent adopts the other side.
    let exact = PackAssurancePolicy::empty().with_determinism(DeterminismPolicy::Required {
        level: DeterminismLevel::Exact,
    });
    let convergent = PackAssurancePolicy::empty().with_determinism(DeterminismPolicy::Required {
        level: DeterminismLevel::Convergent,
    });
    assert_eq!(exact.compose(&convergent).unwrap(), exact);
    assert_eq!(convergent.compose(&exact).unwrap(), exact);
    assert_eq!(
        PackAssurancePolicy::empty().compose(&convergent).unwrap(),
        convergent
    );
    assert_eq!(
        convergent.compose(&PackAssurancePolicy::empty()).unwrap(),
        convergent
    );

    // Replay: reexecute outranks record.
    let reexecute = PackAssurancePolicy::empty().with_replay(ReplayPolicy::Required {
        scope: ReplayScope::Reexecute,
    });
    let record = PackAssurancePolicy::empty().with_replay(ReplayPolicy::Required {
        scope: ReplayScope::Record,
    });
    assert_eq!(reexecute.compose(&record).unwrap(), reexecute);
    assert_eq!(record.compose(&reexecute).unwrap(), reexecute);

    // Evidence: trace outranks outcome.
    let trace = PackAssurancePolicy::empty().with_evidence(EvidencePolicy::Required {
        level: EvidenceLevel::Trace,
    });
    let outcome = PackAssurancePolicy::empty().with_evidence(EvidencePolicy::Required {
        level: EvidenceLevel::Outcome,
    });
    assert_eq!(trace.compose(&outcome).unwrap(), trace);
    assert_eq!(outcome.compose(&trace).unwrap(), trace);

    // Approval: the stricter count wins and the approver class narrows.
    let standing_one = approval_only(1, ApproverClass::StandingApprover);
    let standing_three = approval_only(3, ApproverClass::StandingApprover);
    let human_one = approval_only(1, ApproverClass::Human);
    let human_two = approval_only(2, ApproverClass::Human);
    let control_plane_one = approval_only(1, ApproverClass::ControlPlane);
    assert_eq!(
        standing_one.compose(&standing_three).unwrap(),
        standing_three
    );
    assert_eq!(standing_one.compose(&human_two).unwrap(), human_two);
    assert_eq!(
        standing_three.compose(&human_one).unwrap(),
        approval_only(3, ApproverClass::Human)
    );
    assert_eq!(
        human_one.compose(&standing_three).unwrap(),
        approval_only(3, ApproverClass::Human)
    );
    // Human and control-plane approvals share no common principal class.
    assert!(human_one.compose(&control_plane_one).is_err());
    assert!(control_plane_one.compose(&human_one).is_err());
    // Composing a policy with itself is idempotent.
    assert_eq!(human_two.compose(&human_two).unwrap(), human_two);
}

#[test]
fn equal_pins_compose_and_different_pins_conflict() {
    // Model pins.
    let pinned = PackAssurancePolicy::empty().with_model_pinning(ModelPinningPolicy::Required {
        pin: lattice_model_pin(),
    });
    let same = PackAssurancePolicy::empty().with_model_pinning(ModelPinningPolicy::Required {
        pin: lattice_model_pin(),
    });
    assert_eq!(pinned.compose(&same).unwrap(), pinned);
    assert_eq!(
        PackAssurancePolicy::empty().compose(&pinned).unwrap(),
        pinned
    );
    let different_digest =
        PackAssurancePolicy::empty().with_model_pinning(ModelPinningPolicy::Required {
            pin: ModelPin::new("atlas-reasoner", digest_for(101)).expect("valid model pin"),
        });
    let different_model =
        PackAssurancePolicy::empty().with_model_pinning(ModelPinningPolicy::Required {
            pin: ModelPin::new("bertha-coder", digest_for(1)).expect("valid model pin"),
        });
    assert!(matches!(
        pinned.compose(&different_digest),
        Err(PackContractError::PolicyConflict { .. })
    ));
    assert!(matches!(
        pinned.compose(&different_model),
        Err(PackContractError::PolicyConflict { .. })
    ));

    // Dependency pins: equal pins compose, a different digest conflicts.
    let dependency_pinned =
        PackAssurancePolicy::empty().with_dependency_pinning(DependencyPinningPolicy::Required {
            pin: lattice_dependency_pin(),
        });
    assert_eq!(
        dependency_pinned.compose(&dependency_pinned).unwrap(),
        dependency_pinned
    );
    assert_eq!(
        PackAssurancePolicy::empty()
            .compose(&dependency_pinned)
            .unwrap(),
        dependency_pinned
    );
    let different_dependency =
        PackAssurancePolicy::empty().with_dependency_pinning(DependencyPinningPolicy::Required {
            pin: DependencyPin::new("billing-workflow", digest_for(102))
                .expect("valid dependency pin"),
        });
    assert!(matches!(
        dependency_pinned.compose(&different_dependency),
        Err(PackContractError::PolicyConflict { .. })
    ));

    // Environment pins: equal pins compose, a different digest conflicts.
    let environment_pinned =
        PackAssurancePolicy::empty().with_environment_pinning(EnvironmentPinningPolicy::Required {
            pin: lattice_environment_pin(),
        });
    assert_eq!(
        environment_pinned.compose(&environment_pinned).unwrap(),
        environment_pinned
    );
    assert_eq!(
        PackAssurancePolicy::empty()
            .compose(&environment_pinned)
            .unwrap(),
        environment_pinned
    );
    let different_environment =
        PackAssurancePolicy::empty().with_environment_pinning(EnvironmentPinningPolicy::Required {
            pin: EnvironmentPin::new("production", digest_for(103)).expect("valid environment pin"),
        });
    assert!(matches!(
        environment_pinned.compose(&different_environment),
        Err(PackContractError::PolicyConflict { .. })
    ));
}

#[test]
fn conflicting_pins_report_clear_conflicts() {
    let pinned = PackAssurancePolicy::empty().with_model_pinning(ModelPinningPolicy::Required {
        pin: lattice_model_pin(),
    });
    let conflicting =
        PackAssurancePolicy::empty().with_model_pinning(ModelPinningPolicy::Required {
            pin: ModelPin::new("atlas-reasoner", digest_for(101)).expect("valid model pin"),
        });
    let error = pinned
        .compose(&conflicting)
        .expect_err("different model pins must conflict");
    assert!(matches!(error, PackContractError::PolicyConflict { .. }));
    let message = error.to_string();
    assert!(message.contains("model pin conflict"), "{message}");
    assert!(message.contains("atlas-reasoner"), "{message}");
    assert!(message.contains(&digest_for(1).to_string()), "{message}");
    assert!(message.contains(&digest_for(101).to_string()), "{message}");

    let approval_error = approval_only(1, ApproverClass::Human)
        .compose(&approval_only(1, ApproverClass::ControlPlane))
        .expect_err("human and control-plane approvals must conflict");
    let approval_message = approval_error.to_string();
    assert!(
        approval_message.contains("approver class conflict"),
        "{approval_message}"
    );
    assert!(approval_message.contains("human"), "{approval_message}");
    assert!(
        approval_message.contains("controlPlane"),
        "{approval_message}"
    );
}

#[test]
fn preset_policies_compose_like_any_other_policy() {
    let composed = AssuranceProfile::Assured
        .policy()
        .expect("preset is valid")
        .compose(
            &AssuranceProfile::Deterministic
                .policy()
                .expect("preset is valid"),
        )
        .expect("presets compose like hand-declared policies");
    assert_eq!(
        composed.required_dimensions(),
        AssuranceRequirements {
            determinism: Some(DeterminismLevel::Exact),
            replay: Some(ReplayScope::Record),
            approval: Some(
                ApprovalThreshold::new(
                    /*minimum_approvals*/ 1,
                    ApproverClass::StandingApprover
                )
                .expect("valid threshold")
            ),
            evidence: Some(EvidenceLevel::Trace),
            ..AssuranceRequirements::empty()
        }
    );
}

// ---------------------------------------------------------------------------
// Identity
// ---------------------------------------------------------------------------

#[test]
fn same_policy_content_yields_the_same_identity() {
    let composed = PackAssurancePolicy::empty()
        .with_evidence(EvidencePolicy::Required {
            level: EvidenceLevel::Trace,
        })
        .with_determinism(DeterminismPolicy::Required {
            level: DeterminismLevel::Exact,
        });
    let rebuilt = PackAssurancePolicy::empty()
        .with_determinism(DeterminismPolicy::Required {
            level: DeterminismLevel::Exact,
        })
        .with_evidence(EvidencePolicy::Required {
            level: EvidenceLevel::Trace,
        });
    assert_eq!(composed, rebuilt);
    assert_eq!(
        composed.policy_id().expect("identity computation succeeds"),
        rebuilt.policy_id().expect("identity computation succeeds")
    );
    // Identity derivation is stable across repeated computation.
    assert_eq!(
        composed.policy_id().expect("identity computation succeeds"),
        composed.policy_id().expect("identity computation succeeds")
    );
    // Presets produce the same identity regardless of construction path.
    assert_eq!(
        AssuranceProfile::Deterministic
            .policy()
            .expect("preset is valid")
            .policy_id()
            .expect("identity computation succeeds"),
        AssuranceProfile::Deterministic
            .policy()
            .expect("preset is valid")
            .policy_id()
            .expect("identity computation succeeds")
    );
}

#[test]
fn dimension_lattice_yields_distinct_identities() {
    let identities: HashSet<PackPolicyId> = lattice_policies()
        .iter()
        .map(|policy| policy.policy_id().expect("identity computation succeeds"))
        .collect();
    assert_eq!(identities.len(), (1 << DIMENSION_COUNT) as usize);
}

#[test]
fn any_dimension_mutation_changes_policy_identity() {
    let base = lattice_policy((1 << DIMENSION_COUNT) - 1);
    let base_identity = base.policy_id().expect("identity computation succeeds");
    let mutations = vec![
        base.clone().with_determinism(DeterminismPolicy::Required {
            level: DeterminismLevel::Convergent,
        }),
        base.clone().with_replay(ReplayPolicy::Required {
            scope: ReplayScope::Record,
        }),
        base.clone().with_approval(ApprovalPolicy::Required {
            threshold: ApprovalThreshold::new(
                /*minimum_approvals*/ 3,
                ApproverClass::StandingApprover,
            )
            .expect("valid threshold"),
        }),
        base.clone().with_evidence(EvidencePolicy::Required {
            level: EvidenceLevel::Outcome,
        }),
        base.clone()
            .with_model_pinning(ModelPinningPolicy::Required {
                pin: ModelPin::new("atlas-reasoner", digest_for(201)).expect("valid model pin"),
            }),
        base.clone()
            .with_dependency_pinning(DependencyPinningPolicy::Required {
                pin: DependencyPin::new("billing-workflow", digest_for(202))
                    .expect("valid dependency pin"),
            }),
        base.with_environment_pinning(EnvironmentPinningPolicy::Required {
            pin: EnvironmentPin::new("production", digest_for(203)).expect("valid environment pin"),
        }),
    ];
    for mutation in &mutations {
        assert_ne!(
            mutation.policy_id().expect("identity computation succeeds"),
            base_identity,
            "mutating a dimension must change the policy identity"
        );
    }
    // The seven mutations are also pairwise distinct.
    let identities: HashSet<PackPolicyId> = mutations
        .iter()
        .map(|mutation| mutation.policy_id().expect("identity computation succeeds"))
        .collect();
    assert_eq!(identities.len(), mutations.len());
}

// ---------------------------------------------------------------------------
// Presets
// ---------------------------------------------------------------------------

#[test]
fn named_profiles_are_dimension_presets() {
    let expectations = [
        (AssuranceProfile::Creative, AssuranceRequirements::empty()),
        (
            AssuranceProfile::Repeatable,
            AssuranceRequirements {
                determinism: Some(DeterminismLevel::Convergent),
                evidence: Some(EvidenceLevel::Outcome),
                ..AssuranceRequirements::empty()
            },
        ),
        (
            AssuranceProfile::Replayable,
            AssuranceRequirements {
                replay: Some(ReplayScope::Reexecute),
                evidence: Some(EvidenceLevel::Trace),
                ..AssuranceRequirements::empty()
            },
        ),
        (
            AssuranceProfile::Deterministic,
            AssuranceRequirements {
                determinism: Some(DeterminismLevel::Exact),
                replay: Some(ReplayScope::Record),
                evidence: Some(EvidenceLevel::Trace),
                ..AssuranceRequirements::empty()
            },
        ),
        (
            AssuranceProfile::Assured,
            AssuranceRequirements {
                approval: Some(
                    ApprovalThreshold::new(
                        /*minimum_approvals*/ 1,
                        ApproverClass::StandingApprover,
                    )
                    .expect("valid threshold"),
                ),
                evidence: Some(EvidenceLevel::Trace),
                ..AssuranceRequirements::empty()
            },
        ),
        (
            AssuranceProfile::MissionCritical,
            AssuranceRequirements {
                determinism: Some(DeterminismLevel::Exact),
                replay: Some(ReplayScope::Reexecute),
                approval: Some(
                    ApprovalThreshold::new(/*minimum_approvals*/ 2, ApproverClass::Human)
                        .expect("valid threshold"),
                ),
                evidence: Some(EvidenceLevel::Trace),
                ..AssuranceRequirements::empty()
            },
        ),
    ];
    for (profile, expected) in expectations {
        let policy = profile.policy().expect("preset is valid");
        assert_eq!(
            policy.required_dimensions(),
            expected,
            "profile {profile:?} must map to its preset dimensions"
        );
        // Presets never fabricate pin values.
        assert!(policy.model_pinning.is_not_required());
        assert!(policy.dependency_pinning.is_not_required());
        assert!(policy.environment_pinning.is_not_required());
        // Preset approval thresholds stay valid.
        if let Some(threshold) = policy.required_dimensions().approval {
            assert!(threshold.minimum_approvals() >= 1);
        }
    }
}

// ---------------------------------------------------------------------------
// No authority escalation
// ---------------------------------------------------------------------------

#[test]
fn policy_inspection_is_descriptive_and_non_mutating() {
    let policy = lattice_policy(0b1011010);
    let before = policy.clone();
    let report = policy.required_dimensions();
    policy.policy_id().expect("identity computation succeeds");
    let composed = policy
        .compose(&lattice_policy(0b0100101))
        .expect("disjoint lattice masks compose");
    // Inspecting, identifying, and composing never mutate the policy.
    assert_eq!(policy, before);
    // The report is a pure descriptive value.
    assert_eq!(report, policy.required_dimensions());
    // Composition of disjoint dimension sets unions them exactly.
    assert_eq!(
        composed.required_dimensions(),
        lattice_requirements(0b1111111)
    );
}

// ---------------------------------------------------------------------------
// Serde
// ---------------------------------------------------------------------------

#[test]
fn full_policy_round_trips_with_camel_case_wire_names() {
    let policy = lattice_policy((1 << DIMENSION_COUNT) - 1);
    let wire = serde_json::to_value(&policy).expect("serializable");
    assert_eq!(
        wire,
        json!({
            "determinism": { "kind": "required", "level": "exact" },
            "replay": { "kind": "required", "scope": "reexecute" },
            "approval": {
                "kind": "required",
                "threshold": { "minimumApprovals": 1, "approver": "human" }
            },
            "evidence": { "kind": "required", "level": "trace" },
            "modelPinning": {
                "kind": "required",
                "pin": { "model": "atlas-reasoner", "digest": digest_for(1).to_string() }
            },
            "dependencyPinning": {
                "kind": "required",
                "pin": { "dependency": "billing-workflow", "digest": digest_for(2).to_string() }
            },
            "environmentPinning": {
                "kind": "required",
                "pin": { "environment": "production", "digest": digest_for(3).to_string() }
            },
        })
    );
    let deserialized: PackAssurancePolicy = serde_json::from_value(wire).expect("deserializable");
    assert_eq!(deserialized, policy);
}

#[test]
fn empty_policy_serializes_to_an_empty_object_and_defaults_are_tolerated() {
    let wire = serde_json::to_value(PackAssurancePolicy::empty()).expect("serializable");
    assert_eq!(wire, json!({}));
    let deserialized: PackAssurancePolicy =
        serde_json::from_str("{}").expect("an empty object is the canonical empty policy");
    assert_eq!(deserialized, PackAssurancePolicy::empty());
    let partial: PackAssurancePolicy =
        serde_json::from_str(r#"{"evidence":{"kind":"required","level":"outcome"}}"#)
            .expect("absent fields default to not-required");
    assert_eq!(
        partial,
        PackAssurancePolicy::empty().with_evidence(EvidencePolicy::Required {
            level: EvidenceLevel::Outcome,
        })
    );
}

#[test]
fn requirement_reports_round_trip_over_the_wire() {
    let report = lattice_policy(0b1000001).required_dimensions();
    let wire = serde_json::to_value(&report).expect("serializable");
    assert_eq!(
        wire,
        json!({
            "determinism": "exact",
            "environmentPinning": {
                "environment": "production",
                "digest": digest_for(3).to_string()
            },
        })
    );
    let deserialized: AssuranceRequirements = serde_json::from_value(wire).expect("deserializable");
    assert_eq!(deserialized, report);
}

#[test]
fn unknown_fields_are_rejected_on_struct_records() {
    // PackAssurancePolicy.
    let mut wire = serde_json::to_value(lattice_policy(1)).expect("serializable");
    wire.as_object_mut()
        .expect("policy serializes to an object")
        .insert("unknownField".to_owned(), serde_json::Value::Bool(true));
    assert!(serde_json::from_value::<PackAssurancePolicy>(wire).is_err());

    // AssuranceRequirements.
    let mut report_wire =
        serde_json::to_value(lattice_policy(1).required_dimensions()).expect("serializable");
    report_wire
        .as_object_mut()
        .expect("report serializes to an object")
        .insert("unknownField".to_owned(), serde_json::Value::Bool(true));
    assert!(serde_json::from_value::<AssuranceRequirements>(report_wire).is_err());

    // ApprovalThreshold.
    let mut threshold_wire = serde_json::to_value(lattice_threshold()).expect("serializable");
    threshold_wire
        .as_object_mut()
        .expect("threshold serializes to an object")
        .insert("unknownField".to_owned(), serde_json::Value::Bool(true));
    assert!(serde_json::from_value::<ApprovalThreshold>(threshold_wire).is_err());

    // Pin records.
    let mut pin_wire = serde_json::to_value(lattice_model_pin()).expect("serializable");
    pin_wire
        .as_object_mut()
        .expect("pin serializes to an object")
        .insert("unknownField".to_owned(), serde_json::Value::Bool(true));
    assert!(serde_json::from_value::<ModelPin>(pin_wire).is_err());
}

#[test]
fn unknown_enum_variants_are_rejected() {
    assert!(serde_json::from_str::<DeterminismPolicy>(r#"{"kind":"banana"}"#).is_err());
    assert!(
        serde_json::from_str::<DeterminismPolicy>(r#"{"kind":"required","level":"banana"}"#)
            .is_err()
    );
    assert!(
        serde_json::from_str::<ReplayPolicy>(r#"{"kind":"required","scope":"banana"}"#).is_err()
    );
    assert!(
        serde_json::from_str::<EvidencePolicy>(r#"{"kind":"required","level":"banana"}"#).is_err()
    );
    assert!(serde_json::from_str::<ApprovalPolicy>(r#"{"kind":"banana"}"#).is_err());
    assert!(
        serde_json::from_str::<ApprovalPolicy>(
            r#"{"kind":"required","threshold":{"minimumApprovals":1,"approver":"banana"}}"#
        )
        .is_err()
    );
    assert!(serde_json::from_str::<ModelPinningPolicy>(r#"{"kind":"banana"}"#).is_err());
    assert!(serde_json::from_str::<DependencyPinningPolicy>(r#"{"kind":"banana"}"#).is_err());
    assert!(serde_json::from_str::<EnvironmentPinningPolicy>(r#"{"kind":"banana"}"#).is_err());
    assert!(serde_json::from_str::<AssuranceProfile>("\"banana\"").is_err());
    assert!(serde_json::from_str::<AssuranceDimension>("\"banana\"").is_err());
}

#[test]
fn profiles_and_dimensions_round_trip_as_camel_case() {
    let profiles = [
        (AssuranceProfile::Creative, "\"creative\""),
        (AssuranceProfile::Repeatable, "\"repeatable\""),
        (AssuranceProfile::Replayable, "\"replayable\""),
        (AssuranceProfile::Deterministic, "\"deterministic\""),
        (AssuranceProfile::Assured, "\"assured\""),
        (AssuranceProfile::MissionCritical, "\"missionCritical\""),
    ];
    for (profile, wire) in profiles {
        assert_eq!(serde_json::to_string(&profile).expect("serializable"), wire);
        assert_eq!(
            serde_json::from_str::<AssuranceProfile>(wire).expect("deserializable"),
            profile
        );
    }
    assert_eq!(
        serde_json::to_string(&AssuranceDimension::DependencyPinning).expect("serializable"),
        "\"dependencyPinning\""
    );
    assert_eq!(
        serde_json::from_str::<AssuranceDimension>("\"dependencyPinning\"")
            .expect("deserializable"),
        AssuranceDimension::DependencyPinning
    );
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

#[test]
fn approval_thresholds_reject_zero_approvals() {
    for class in [
        ApproverClass::StandingApprover,
        ApproverClass::Human,
        ApproverClass::ControlPlane,
    ] {
        assert!(matches!(
            ApprovalThreshold::new(0, class),
            Err(PackContractError::InvalidIdentifier {
                kind: "approval threshold",
                ..
            })
        ));
    }
    let threshold = ApprovalThreshold::new(2, ApproverClass::Human).expect("valid threshold");
    assert_eq!(threshold.minimum_approvals(), 2);
    assert_eq!(threshold.approver(), ApproverClass::Human);
}

#[test]
fn approver_classes_narrow_or_conflict() {
    assert_eq!(
        ApproverClass::StandingApprover
            .compose(ApproverClass::Human)
            .unwrap(),
        ApproverClass::Human
    );
    assert_eq!(
        ApproverClass::Human
            .compose(ApproverClass::StandingApprover)
            .unwrap(),
        ApproverClass::Human
    );
    assert_eq!(
        ApproverClass::Human.compose(ApproverClass::Human).unwrap(),
        ApproverClass::Human
    );
    assert_eq!(
        ApproverClass::StandingApprover
            .compose(ApproverClass::ControlPlane)
            .unwrap(),
        ApproverClass::ControlPlane
    );
    assert_eq!(
        ApproverClass::ControlPlane
            .compose(ApproverClass::StandingApprover)
            .unwrap(),
        ApproverClass::ControlPlane
    );
    assert!(
        ApproverClass::Human
            .compose(ApproverClass::ControlPlane)
            .is_err()
    );
    assert!(
        ApproverClass::ControlPlane
            .compose(ApproverClass::Human)
            .is_err()
    );
}

#[test]
fn pin_constructors_validate_identifiers() {
    let digest = digest_for(1);
    let invalid = [
        "".to_owned(),
        "  ".to_owned(),
        " atlas".to_owned(),
        "atlas ".to_owned(),
        "at las".to_owned(),
        "a".repeat(257),
    ];
    for invalid in invalid {
        assert!(
            ModelPin::new(invalid.clone(), digest.clone()).is_err(),
            "model id `{invalid}` must be rejected"
        );
        assert!(
            DependencyPin::new(invalid.clone(), digest.clone()).is_err(),
            "dependency id `{invalid}` must be rejected"
        );
        assert!(
            EnvironmentPin::new(invalid.clone(), digest.clone()).is_err(),
            "environment id `{invalid}` must be rejected"
        );
    }
    let valid = [
        "a".to_owned(),
        "atlas-reasoner-v2.1".to_owned(),
        "org/atlas/reasoner".to_owned(),
        "a".repeat(256),
    ];
    for valid in valid {
        assert!(
            ModelPin::new(valid.clone(), digest.clone()).is_ok(),
            "model id `{valid}` must be accepted"
        );
        assert!(
            DependencyPin::new(valid.clone(), digest.clone()).is_ok(),
            "dependency id `{valid}` must be accepted"
        );
        assert!(
            EnvironmentPin::new(valid.clone(), digest.clone()).is_ok(),
            "environment id `{valid}` must be accepted"
        );
    }
}

#[test]
fn pins_expose_identity_and_digest_only() {
    let model = ModelPin::new("org/atlas-reasoner-v2.1", digest_for(9)).expect("valid model pin");
    assert_eq!(model.model(), "org/atlas-reasoner-v2.1");
    assert_eq!(model.digest(), &digest_for(9));

    let dependency =
        DependencyPin::new("billing-workflow", digest_for(10)).expect("valid dependency pin");
    assert_eq!(dependency.dependency(), "billing-workflow");
    assert_eq!(dependency.digest(), &digest_for(10));

    let environment =
        EnvironmentPin::new("production", digest_for(11)).expect("valid environment pin");
    assert_eq!(environment.environment(), "production");
    assert_eq!(environment.digest(), &digest_for(11));
}

// ---------------------------------------------------------------------------
// Dimension accessors
// ---------------------------------------------------------------------------

#[test]
fn dimension_accessors_report_the_declared_requirement() {
    assert!(DeterminismPolicy::NotRequired.is_not_required());
    assert_eq!(DeterminismPolicy::NotRequired.level(), None);
    let determinism = DeterminismPolicy::Required {
        level: DeterminismLevel::Exact,
    };
    assert!(!determinism.is_not_required());
    assert_eq!(determinism.level(), Some(DeterminismLevel::Exact));

    assert!(ReplayPolicy::NotRequired.is_not_required());
    assert_eq!(ReplayPolicy::NotRequired.scope(), None);
    let replay = ReplayPolicy::Required {
        scope: ReplayScope::Reexecute,
    };
    assert!(!replay.is_not_required());
    assert_eq!(replay.scope(), Some(ReplayScope::Reexecute));

    assert!(EvidencePolicy::NotRequired.is_not_required());
    assert_eq!(EvidencePolicy::NotRequired.level(), None);
    let evidence = EvidencePolicy::Required {
        level: EvidenceLevel::Trace,
    };
    assert!(!evidence.is_not_required());
    assert_eq!(evidence.level(), Some(EvidenceLevel::Trace));

    assert!(ApprovalPolicy::NotRequired.is_not_required());
    assert_eq!(ApprovalPolicy::NotRequired.threshold(), None);
    let approval = ApprovalPolicy::Required {
        threshold: lattice_threshold(),
    };
    assert!(!approval.is_not_required());
    assert_eq!(approval.threshold(), Some(&lattice_threshold()));

    assert!(ModelPinningPolicy::NotRequired.is_not_required());
    assert_eq!(ModelPinningPolicy::NotRequired.pin(), None);
    let model_pinning = ModelPinningPolicy::Required {
        pin: lattice_model_pin(),
    };
    assert!(!model_pinning.is_not_required());
    assert_eq!(model_pinning.pin(), Some(&lattice_model_pin()));

    assert!(DependencyPinningPolicy::NotRequired.is_not_required());
    assert_eq!(DependencyPinningPolicy::NotRequired.pin(), None);
    let dependency_pinning = DependencyPinningPolicy::Required {
        pin: lattice_dependency_pin(),
    };
    assert!(!dependency_pinning.is_not_required());
    assert_eq!(dependency_pinning.pin(), Some(&lattice_dependency_pin()));

    assert!(EnvironmentPinningPolicy::NotRequired.is_not_required());
    assert_eq!(EnvironmentPinningPolicy::NotRequired.pin(), None);
    let environment_pinning = EnvironmentPinningPolicy::Required {
        pin: lattice_environment_pin(),
    };
    assert!(!environment_pinning.is_not_required());
    assert_eq!(environment_pinning.pin(), Some(&lattice_environment_pin()));
}
