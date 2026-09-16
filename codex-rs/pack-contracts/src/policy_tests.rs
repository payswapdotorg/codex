//! Tests for the Pack Policy contracts.

use super::*;
use crate::ConstitutionRule;
use crate::ConstitutionStatement;
use crate::PlatformInvariant;
use pretty_assertions::assert_eq;
use pretty_assertions::assert_ne;
use serde_json::json;
use std::collections::BTreeSet;

fn statement(text: &str) -> PolicyStatement {
    PolicyStatement::parse(text).expect("valid statement")
}

fn sample_policy_id(payload: &str) -> PackPolicyId {
    PackPolicyId::from_digest(
        ContentDigest::of(&json!({ "payload": payload })).expect("digest computes"),
    )
}

fn base_policy() -> PackPolicy {
    PackPolicy::new(
        PackPolicyScope::DependencyUpdates,
        vec![statement(
            "Dependency updates require a fresh dependency lock and a new candidate revision.",
        )],
        "How pack dependencies may be updated.",
        BTreeSet::new(),
    )
    .expect("valid policy")
}

fn base_constitution() -> PackConstitution {
    PackConstitution::new(vec![ConstitutionRule::AuditRequirement {
        statement: ConstitutionStatement::parse("Every promotion records an audit entry.")
            .expect("valid statement"),
    }])
    .expect("valid constitution")
}

#[test]
fn every_policy_scope_round_trips_through_serde() {
    let scopes = [
        PackPolicyScope::EntirePack,
        PackPolicyScope::MissionModel,
        PackPolicyScope::ConstitutionChanges,
        PackPolicyScope::DependencyUpdates,
        PackPolicyScope::SystemStateChanges,
    ];
    for scope in scopes {
        let serialized = serde_json::to_string(&scope).expect("serializable");
        assert_eq!(
            serde_json::from_str::<PackPolicyScope>(&serialized).expect("deserializable"),
            scope
        );
    }
    assert!(serde_json::from_str::<PackPolicyScope>("\"missionCreation\"").is_err());
}

#[test]
fn policies_carry_the_complete_retained_authority_acknowledgment() {
    let policy = base_policy();
    let expected: BTreeSet<RetainedAuthority> = ALL_RETAINED_AUTHORITIES.into_iter().collect();
    assert_eq!(policy.authority_boundary.retained_authorities, expected);
    policy.validate().expect("constructed policy validates");

    let all = [
        RetainedAuthority::WorkflowTransitions,
        RetainedAuthority::Credentials,
        RetainedAuthority::Evidence,
    ];
    assert_eq!(ALL_RETAINED_AUTHORITIES.len(), 3);
    for authority in all {
        assert_eq!(
            ALL_RETAINED_AUTHORITIES
                .iter()
                .filter(|known| **known == authority)
                .count(),
            1,
            "every retained authority must appear exactly once"
        );
    }
}

#[test]
fn policies_cannot_claim_retained_authorities() {
    // A wire-tampered record that acknowledges only a subset deserializes
    // but must fail validation: policies constrain, they never grant
    // authority over workflow transitions, credentials, or evidence.
    let policy = json!({
        "scope": "entirePack",
        "statements": [ "Be careful." ],
        "authorityBoundary": {
            "governs": "Everything the pack does.",
            "retainedAuthorities": ["workflowTransitions"],
        }
    });
    let deserialized: PackPolicy =
        serde_json::from_str(&policy.to_string()).expect("structurally deserializable");
    let error = deserialized
        .validate()
        .expect_err("subset acknowledgment must fail validation");
    assert!(
        error.to_string().contains("may not claim"),
        "error must name the violated authority rule, got: {error}"
    );

    // The constructor cannot produce a subset acknowledgment at all: it
    // always acknowledges every retained authority.
    let constructed = PackPolicy::new(
        PackPolicyScope::EntirePack,
        vec![statement("Be careful.")],
        "Everything the pack does.",
        BTreeSet::new(),
    )
    .expect("constructor always acknowledges every retained authority");
    constructed
        .validate()
        .expect("constructed policy validates");
}

#[test]
fn policies_without_statements_are_rejected() {
    let error = PackPolicy::new(
        PackPolicyScope::MissionModel,
        vec![],
        "Mission governance.",
        BTreeSet::new(),
    )
    .expect_err("statement-less policy must be rejected");
    assert!(
        error.to_string().contains("at least one statement"),
        "error must explain the statement requirement, got: {error}"
    );

    for blank in ["", "  "] {
        assert!(PolicyStatement::parse(blank).is_err());
    }
    assert!(PolicyStatement::parse(" edge-space ").is_err());
    assert!(serde_json::from_str::<PolicyStatement>("\" \"").is_err());
}

#[test]
fn identical_policies_produce_identical_policy_ids() {
    let policy = base_policy();
    assert_eq!(
        policy.policy_id().expect("policy id computes"),
        base_policy().policy_id().expect("policy id computes")
    );
}

#[test]
fn every_policy_content_mutation_changes_the_policy_id() {
    let base = base_policy();
    let base_id = base.policy_id().expect("policy id computes");

    let mutated: Vec<(&str, PackPolicy)> = vec![
        (
            "scope",
            PackPolicy::new(
                PackPolicyScope::SystemStateChanges,
                vec![statement(
                    "Dependency updates require a fresh dependency lock and a new candidate revision.",
                )],
                "How pack dependencies may be updated.",
                BTreeSet::new(),
            )
            .expect("valid policy"),
        ),
        (
            "statements",
            PackPolicy::new(
                PackPolicyScope::DependencyUpdates,
                vec![
                    statement(
                        "Dependency updates require a fresh dependency lock and a new candidate revision.",
                    ),
                    statement("Updates must cite evidence."),
                ],
                "How pack dependencies may be updated.",
                BTreeSet::new(),
            )
            .expect("valid policy"),
        ),
        (
            "authority boundary",
            PackPolicy::new(
                PackPolicyScope::DependencyUpdates,
                vec![statement(
                    "Dependency updates require a fresh dependency lock and a new candidate revision.",
                )],
                "How pack dependencies may be updated, in full.",
                BTreeSet::new(),
            )
            .expect("valid policy"),
        ),
        (
            "referenced policies",
            PackPolicy::new(
                PackPolicyScope::DependencyUpdates,
                vec![statement(
                    "Dependency updates require a fresh dependency lock and a new candidate revision.",
                )],
                "How pack dependencies may be updated.",
                BTreeSet::from([sample_policy_id("assurance-reference")]),
            )
            .expect("valid policy"),
        ),
    ];

    assert_eq!(
        mutated.len(),
        4,
        "the sweep must cover every policy record field"
    );
    for (covered_field, mutation) in mutated {
        let mutated_id = mutation.policy_id().expect("policy id computes");
        assert_ne!(
            base_id, mutated_id,
            "mutating {covered_field} must change the policy identity"
        );
    }
}

#[test]
fn policies_round_trip_through_serde_and_reject_unknown_fields() {
    let policy = PackPolicy::new(
        PackPolicyScope::SystemStateChanges,
        vec![statement("System-state changes require evidence.")],
        "How the pack's system state may change.",
        BTreeSet::from([sample_policy_id("assurance-profile")]),
    )
    .expect("valid policy");

    let serialized = serde_json::to_string(&policy).expect("serializable");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&serialized).expect("valid json"),
        json!({
            "scope": "systemStateChanges",
            "statements": ["System-state changes require evidence."],
            "authorityBoundary": {
                "governs": "How the pack's system state may change.",
                "retainedAuthorities": ["workflowTransitions", "credentials", "evidence"],
            },
            "referencedPolicies": [sample_policy_id("assurance-profile").to_string()],
        })
    );
    let parsed: PackPolicy = serde_json::from_str(&serialized).expect("deserializable");
    assert_eq!(parsed, policy);

    let tampered = serialized.replace('{', "{\"injected\":1,");
    assert!(serde_json::from_str::<PackPolicy>(&tampered).is_err());
}

#[test]
fn policy_sets_digest_constitution_and_governing_policies_together() {
    let set =
        PackPolicySet::new(base_constitution(), vec![base_policy()]).expect("valid policy set");
    let digest = set.digest().expect("digest computes");
    assert_eq!(digest, set.digest().expect("digest computes"));

    // Any governing change — a new policy, a different policy, a changed
    // constitution — produces a different set digest, which becomes a
    // different revision identity through `policy_digest`.
    let with_extra_policy = PackPolicySet::new(
        base_constitution(),
        vec![
            base_policy(),
            PackPolicy::new(
                PackPolicyScope::MissionModel,
                vec![statement("Mission changes require user approval.")],
                "How the mission may change.",
                BTreeSet::new(),
            )
            .expect("valid policy"),
        ],
    )
    .expect("valid policy set");
    assert_ne!(digest, with_extra_policy.digest().expect("digest computes"));

    let with_other_constitution = PackPolicySet::new(
        PackConstitution::new(vec![ConstitutionRule::ForbiddenAction {
            statement: ConstitutionStatement::parse("Never delete customer records.")
                .expect("valid statement"),
        }])
        .expect("valid constitution"),
        vec![base_policy()],
    )
    .expect("valid policy set");
    assert_ne!(
        digest,
        with_other_constitution.digest().expect("digest computes")
    );

    with_extra_policy.validate().expect("set validates");
    with_other_constitution.validate().expect("set validates");
}

#[test]
fn policy_sets_reject_duplicates_and_propagate_validation() {
    let duplicated = PackPolicySet::new(base_constitution(), vec![base_policy(), base_policy()])
        .expect_err("identical policies must be rejected");
    assert!(
        duplicated
            .to_string()
            .contains("duplicate governing policy"),
        "error must name the duplication, got: {duplicated}"
    );

    // Validation propagates from the constitution and the policies.
    let mut weakened_constitution = base_constitution();
    weakened_constitution.protected_invariants = BTreeSet::from([PlatformInvariant::Security]);
    let invalid = PackPolicySet::new(weakened_constitution, vec![base_policy()])
        .expect_err("invalid constitution must invalidate the set");
    assert!(
        invalid.to_string().contains("may not weaken"),
        "error must name the platform-invariant violation, got: {invalid}"
    );

    let statementless = PackPolicy {
        statements: vec![],
        ..base_policy()
    };
    let invalid = PackPolicySet::new(base_constitution(), vec![statementless])
        .expect_err("invalid policy must invalidate the set");
    assert!(
        invalid.to_string().contains("at least one statement"),
        "error must name the statement requirement, got: {invalid}"
    );
}

#[test]
fn policy_sets_round_trip_through_serde_and_reject_unknown_fields() {
    let set =
        PackPolicySet::new(base_constitution(), vec![base_policy()]).expect("valid policy set");
    let serialized = serde_json::to_string(&set).expect("serializable");
    let parsed: PackPolicySet = serde_json::from_str(&serialized).expect("deserializable");
    assert_eq!(parsed, set);
    assert_eq!(
        parsed.digest().expect("digest computes"),
        set.digest().expect("digest computes")
    );

    let tampered = serialized.replace('{', "{\"injected\":1,");
    assert!(serde_json::from_str::<PackPolicySet>(&tampered).is_err());
}
