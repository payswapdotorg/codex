//! Tests for the mission model contracts.

use super::*;
use pretty_assertions::assert_eq;
use pretty_assertions::assert_ne;
use serde_json::json;

fn statement(text: &str) -> MissionStatement {
    MissionStatement::parse(text).expect("valid statement")
}

fn user_author() -> MissionAuthor {
    MissionAuthor::user("org:atlas").expect("valid principal")
}

fn base_mission() -> Mission {
    Mission {
        id: MissionId::parse("atlas-mission").expect("valid mission id"),
        statement: statement("Run billing operations end to end."),
        author: user_author(),
        value_model: ValueModel {
            objectives: vec![
                ValueObjective {
                    statement: statement("Invoice accuracy above all else."),
                },
                ValueObjective {
                    statement: statement("Minimize time to issue an invoice."),
                },
            ],
        },
        context_model: ContextModel {
            notes: vec![ContextNote {
                statement: statement("Organization of 40 people, EU-based customers."),
            }],
        },
        hard_constraints: vec![HardConstraint {
            statement: statement("Never store customer card numbers."),
        }],
        preferences: vec![UserPreference {
            statement: statement("Prefer EU-hosted services."),
        }],
        success_measures: vec![SuccessMeasure {
            statement: statement("Zero billing disputes per quarter."),
        }],
    }
}

#[test]
fn mission_ids_accept_valid_slugs() {
    for valid in ["atlas-mission", "billing", "m1", "a"] {
        let parsed =
            MissionId::parse(valid).unwrap_or_else(|error| panic!("`{valid}` rejected: {error}"));
        assert_eq!(parsed.as_str(), valid);
        assert_eq!(parsed.to_string(), valid);
    }
}

#[test]
fn mission_ids_reject_invalid_slugs() {
    let invalid = [
        "".to_owned(),
        "-atlas".to_owned(),
        "atlas-".to_owned(),
        "Atlas".to_owned(),
        "atlas_mission".to_owned(),
        "atlas mission".to_owned(),
        "atlas.mission".to_owned(),
        "a".repeat(129),
    ];
    for invalid in invalid {
        let result = MissionId::parse(invalid.clone());
        assert!(
            matches!(result, Err(PackContractError::InvalidIdentifier { .. })),
            "`{invalid}` should be rejected"
        );
    }
}

#[test]
fn mission_statements_reject_blank_and_edge_whitespace() {
    for blank in ["", "   ", " \t "] {
        assert!(
            MissionStatement::parse(blank).is_err(),
            "blank statement must be rejected"
        );
    }
    assert!(MissionStatement::parse(" edge-space ").is_err());

    let parsed = statement("Invoice accuracy above all else.");
    assert_eq!(parsed.as_str(), "Invoice accuracy above all else.");

    // Wire-level construction is validated identically.
    assert!(serde_json::from_str::<MissionStatement>("\" \"").is_err());
}

#[test]
fn mission_authors_round_trip_and_flag_authority_explicitly() {
    let user = MissionAuthor::user("org:atlas").expect("valid principal");
    let agent = MissionAuthor::agent_proposal("mission-architect-worker").expect("valid principal");
    assert!(user.is_user_authority());
    assert!(!agent.is_user_authority());

    let serialized = serde_json::to_string(&user).expect("serializable");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&serialized).expect("valid json"),
        json!({ "kind": "user", "subject": "org:atlas" })
    );
    assert_eq!(
        serde_json::from_str::<MissionAuthor>(&serialized).expect("deserializable"),
        user
    );

    let serialized = serde_json::to_string(&agent).expect("serializable");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&serialized).expect("valid json"),
        json!({ "kind": "agentProposal", "agent": "mission-architect-worker" })
    );
    assert_eq!(
        serde_json::from_str::<MissionAuthor>(&serialized).expect("deserializable"),
        agent
    );

    for blank in ["", "  "] {
        assert!(MissionAuthor::user(blank).is_err());
        assert!(MissionAuthor::agent_proposal(blank).is_err());
    }
    // Blank principals are rejected on the wire, not only in constructors.
    assert!(
        serde_json::from_str::<MissionAuthor>(
            &json!({
                "kind": "user",
                "subject": ""
            })
            .to_string()
        )
        .is_err()
    );
}

#[test]
fn missions_round_trip_through_serde_and_reject_unknown_fields() {
    let mission = base_mission();
    let serialized = serde_json::to_string(&mission).expect("serializable");
    let parsed: Mission = serde_json::from_str(&serialized).expect("deserializable");
    assert_eq!(parsed, mission);

    let tampered = serialized.replace('{', "{\"injected\":1,");
    assert!(serde_json::from_str::<Mission>(&tampered).is_err());
}

#[test]
fn mission_wire_shape_distinguishes_every_model_element() {
    let serialized = serde_json::to_string(&base_mission()).expect("serializable");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&serialized).expect("valid json"),
        json!({
            "id": "atlas-mission",
            "statement": "Run billing operations end to end.",
            "author": { "kind": "user", "subject": "org:atlas" },
            "valueModel": {
                "objectives": [
                    { "statement": "Invoice accuracy above all else." },
                    { "statement": "Minimize time to issue an invoice." }
                ]
            },
            "contextModel": {
                "notes": [ { "statement": "Organization of 40 people, EU-based customers." } ]
            },
            "hardConstraints": [ { "statement": "Never store customer card numbers." } ],
            "preferences": [ { "statement": "Prefer EU-hosted services." } ],
            "successMeasures": [ { "statement": "Zero billing disputes per quarter." } ],
        })
    );
}

#[test]
fn identical_missions_produce_identical_digests() {
    let mission = base_mission();
    assert_eq!(
        mission.digest().expect("digest computes"),
        mission.digest().expect("digest computes deterministically")
    );
    assert_eq!(
        base_mission().digest().expect("digest computes"),
        mission.digest().expect("digest computes")
    );
}

#[test]
fn every_mission_content_mutation_changes_the_digest() {
    let base = base_mission();
    let base_digest = base.digest().expect("digest computes");

    let mutated: Vec<(&str, Mission)> = vec![
        (
            "mission id",
            Mission {
                id: MissionId::parse("atlas-mission-2").expect("valid mission id"),
                ..base.clone()
            },
        ),
        (
            "mission statement",
            Mission {
                statement: statement("Run billing and reporting end to end."),
                ..base.clone()
            },
        ),
        (
            "value model",
            Mission {
                value_model: ValueModel {
                    objectives: vec![ValueObjective {
                        statement: statement("Speed above all else."),
                    }],
                },
                ..base.clone()
            },
        ),
        (
            "context model",
            Mission {
                context_model: ContextModel {
                    notes: vec![ContextNote {
                        statement: statement("Organization of 400 people."),
                    }],
                },
                ..base.clone()
            },
        ),
        (
            "hard constraints",
            Mission {
                hard_constraints: vec![],
                ..base.clone()
            },
        ),
        (
            "preferences",
            Mission {
                preferences: vec![UserPreference {
                    statement: statement("Prefer US-hosted services."),
                }],
                ..base.clone()
            },
        ),
        (
            "success measures",
            Mission {
                success_measures: vec![SuccessMeasure {
                    statement: statement("Under one dispute per quarter."),
                }],
                ..base
            },
        ),
    ];

    assert_eq!(
        mutated.len(),
        7,
        "the sweep must cover every mission record field"
    );
    for (covered_field, mutation) in mutated {
        let mutated_digest = mutation.digest().expect("digest computes");
        assert_ne!(
            base_digest, mutated_digest,
            "mutating {covered_field} must change the mission digest"
        );
    }
}

#[test]
fn agent_proposals_are_never_silent_user_authority() {
    // Identical mission text, different authorship: the agent proposal is a
    // distinct mission revision, and the record says so explicitly.
    let user_authored = base_mission();
    let agent_proposed = Mission {
        author: MissionAuthor::agent_proposal("mission-architect-worker").expect("valid principal"),
        ..base_mission()
    };

    assert_ne!(user_authored.author, agent_proposed.author);
    assert!(user_authored.author.is_user_authority());
    assert!(!agent_proposed.author.is_user_authority());
    assert_ne!(
        user_authored.digest().expect("digest computes"),
        agent_proposed.digest().expect("digest computes"),
        "authorship is mission content: agent proposals must not collide with user authority"
    );
}
