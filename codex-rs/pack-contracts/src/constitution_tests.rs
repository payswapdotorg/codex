//! Tests for the Pack Constitution contracts.

use super::*;
use pretty_assertions::assert_eq;
use pretty_assertions::assert_ne;
use serde_json::json;

fn statement(text: &str) -> ConstitutionStatement {
    ConstitutionStatement::parse(text).expect("valid statement")
}

fn base_rules() -> Vec<ConstitutionRule> {
    vec![
        ConstitutionRule::ForbiddenAction {
            statement: statement("Never delete customer billing records."),
        },
        ConstitutionRule::RequiredApproval {
            statement: statement("Rate changes require treasurer approval."),
        },
    ]
}

#[test]
fn constitutions_carry_the_full_platform_invariant_acknowledgment() {
    let constitution =
        PackConstitution::new(base_rules()).expect("constitution with rules is valid");
    let expected: BTreeSet<PlatformInvariant> = ALL_PLATFORM_INVARIANTS.into_iter().collect();
    assert_eq!(constitution.protected_invariants, expected);
    assert_eq!(constitution.rules, base_rules());
    constitution
        .validate()
        .expect("constructed record validates");
}

#[test]
fn the_well_known_platform_invariant_list_covers_every_variant() {
    // Enumerated boundary sweep: every discriminant must appear in the list
    // exactly once, and the list length must match the variant count.
    let all = [
        PlatformInvariant::PlatformConstitution,
        PlatformInvariant::Authorization,
        PlatformInvariant::Credentials,
        PlatformInvariant::Evidence,
        PlatformInvariant::Security,
    ];
    assert_eq!(ALL_PLATFORM_INVARIANTS.len(), 5);
    for invariant in all {
        assert_eq!(
            ALL_PLATFORM_INVARIANTS
                .iter()
                .filter(|known| **known == invariant)
                .count(),
            1,
            "every platform invariant must appear exactly once"
        );
    }
}

#[test]
fn constitutions_cannot_weaken_platform_invariants() {
    // A wire-tampered record that acknowledges only a subset of the platform
    // invariants deserializes but must fail validation.
    let weakened = json!({
        "rules": [
            { "forbiddenAction": { "statement": "Never delete customer records." } }
        ],
        "protectedInvariants": ["platformConstitution", "security"],
    });
    let deserialized: PackConstitution =
        serde_json::from_str(&weakened.to_string()).expect("structurally deserializable");
    let error = deserialized
        .validate()
        .expect_err("subset acknowledgment must fail validation");
    assert!(
        error
            .to_string()
            .contains("may not weaken platform invariants"),
        "error must name the violated authority rule, got: {error}"
    );

    let empty = json!({
        "rules": [ { "auditRequirement": { "statement": "Keep audit logs." } } ],
        "protectedInvariants": [],
    });
    let deserialized: PackConstitution =
        serde_json::from_str(&empty.to_string()).expect("structurally deserializable");
    assert!(deserialized.validate().is_err());
}

#[test]
fn empty_constitutions_are_rejected() {
    let error = PackConstitution::new(vec![]).expect_err("empty constitution must be rejected");
    assert!(
        error.to_string().contains("at least one rule"),
        "error must explain the rule requirement, got: {error}"
    );
}

#[test]
fn every_rule_class_round_trips_through_serde() {
    let rules = vec![
        ConstitutionRule::ForbiddenAction {
            statement: statement("Never delete billing records."),
        },
        ConstitutionRule::RequiredApproval {
            statement: statement("Refunds require supervisor approval."),
        },
        ConstitutionRule::ProvenanceRequirement {
            statement: statement("Every revision names its producing principal."),
        },
        ConstitutionRule::DataIntegrity {
            statement: statement("Ledger entries are append-only."),
        },
        ConstitutionRule::DomainStateMachineInvariant {
            statement: statement("An invoice never moves from settled to open."),
        },
        ConstitutionRule::NonDestructiveBehavior {
            statement: statement("Migrations only add columns."),
        },
        ConstitutionRule::AuditRequirement {
            statement: statement("Every promotion records an audit entry."),
        },
        ConstitutionRule::Domain {
            class: DomainRuleClass::parse("medical_dosing_rules").expect("valid domain class"),
            statement: statement("Dose calculations require a second checker."),
        },
    ];
    let constitution = PackConstitution::new(rules.clone()).expect("valid constitution");
    let serialized = serde_json::to_string(&constitution).expect("serializable");
    let parsed: PackConstitution = serde_json::from_str(&serialized).expect("deserializable");
    assert_eq!(parsed, constitution);
    assert_eq!(parsed.rules, rules);
}

#[test]
fn constitutions_reject_unknown_fields_on_the_wire() {
    let constitution =
        PackConstitution::new(base_rules()).expect("constitution with rules is valid");
    let serialized = serde_json::to_string(&constitution).expect("serializable");

    let tampered_record = serialized.replace('{', "{\"injected\":1,");
    assert!(serde_json::from_str::<PackConstitution>(&tampered_record).is_err());

    let tampered_rule = serialized.replace(
        "{\"forbiddenAction\":{\"statement\":\"Never delete customer billing records.\"}}",
        "{\"forbiddenAction\":{\"statement\":\"Never delete customer billing records.\",\"waiver\":true}}",
    );
    assert!(
        serde_json::from_str::<PackConstitution>(&tampered_rule).is_err(),
        "rule payloads must reject unknown fields (no waiver escape hatch)"
    );
}

#[test]
fn constitution_wire_shape_is_camel_cased() {
    let constitution = PackConstitution::new(vec![
        ConstitutionRule::Domain {
            class: DomainRuleClass::parse("medical_dosing_rules").expect("valid domain class"),
            statement: statement("Dose calculations require a second checker."),
        },
        ConstitutionRule::AuditRequirement {
            statement: statement("Every promotion records an audit entry."),
        },
    ])
    .expect("valid constitution");
    let serialized = serde_json::to_string(&constitution).expect("serializable");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&serialized).expect("valid json"),
        json!({
            "rules": [
                {
                    "domain": {
                        "class": "medical_dosing_rules",
                        "statement": "Dose calculations require a second checker."
                    }
                },
                {
                    "auditRequirement": {
                        "statement": "Every promotion records an audit entry."
                    }
                }
            ],
            "protectedInvariants": [
                "platformConstitution",
                "authorization",
                "credentials",
                "evidence",
                "security"
            ],
        })
    );
}

#[test]
fn identical_constitutions_produce_identical_digests() {
    let constitution =
        PackConstitution::new(base_rules()).expect("constitution with rules is valid");
    assert_eq!(
        constitution.digest().expect("digest computes"),
        PackConstitution::new(base_rules())
            .expect("constitution with rules is valid")
            .digest()
            .expect("digest computes")
    );
}

#[test]
fn every_constitution_content_mutation_changes_the_digest() {
    let base = PackConstitution::new(base_rules()).expect("constitution with rules is valid");
    let base_digest = base.digest().expect("digest computes");

    let restated = PackConstitution::new(vec![
        ConstitutionRule::ForbiddenAction {
            statement: statement("Never delete customer records."),
        },
        ConstitutionRule::RequiredApproval {
            statement: statement("Rate changes require treasurer approval."),
        },
    ])
    .expect("valid constitution");
    let added_rule = PackConstitution::new(vec![
        ConstitutionRule::ForbiddenAction {
            statement: statement("Never delete customer billing records."),
        },
        ConstitutionRule::RequiredApproval {
            statement: statement("Rate changes require treasurer approval."),
        },
        ConstitutionRule::AuditRequirement {
            statement: statement("Every promotion records an audit entry."),
        },
    ])
    .expect("valid constitution");
    let dropped_rule =
        PackConstitution::new(vec![base_rules()[0].clone()]).expect("valid constitution");
    let reordered = PackConstitution::new(vec![
        ConstitutionRule::RequiredApproval {
            statement: statement("Rate changes require treasurer approval."),
        },
        ConstitutionRule::ForbiddenAction {
            statement: statement("Never delete customer billing records."),
        },
    ])
    .expect("valid constitution");

    let mutations = [
        ("restated rule", restated),
        ("added rule", added_rule),
        ("dropped rule", dropped_rule),
        ("reordered rules", reordered),
    ];
    for (covered_change, mutation) in mutations {
        assert_ne!(
            base_digest,
            mutation.digest().expect("digest computes"),
            "{covered_change} must change the constitution digest"
        );
    }
}

#[test]
fn domain_rule_classes_and_statements_are_validated() {
    for invalid in ["", "MedicalRules", "medical rules", "_medical", "medical_"] {
        assert!(
            DomainRuleClass::parse(invalid).is_err(),
            "`{invalid}` must be rejected as a domain rule class"
        );
    }
    let parsed = DomainRuleClass::parse("medical_dosing_rules").expect("valid domain class");
    assert_eq!(parsed.as_str(), "medical_dosing_rules");
    assert!(serde_json::from_str::<DomainRuleClass>("\"MedicalRules\"").is_err());

    for blank in ["", "  "] {
        assert!(ConstitutionStatement::parse(blank).is_err());
    }
    assert!(ConstitutionStatement::parse(" edge-space ").is_err());
    assert!(serde_json::from_str::<ConstitutionStatement>("\" \"").is_err());
}
