//! Tests for the pack revision identity tuple and compatibility contract.

use super::*;
use crate::PackProvenance;
use crate::ProvenanceProducer;
use pretty_assertions::assert_eq;
use pretty_assertions::assert_ne;
use serde_json::json;

fn digest_of(payload: &str) -> ContentDigest {
    ContentDigest::of(&json!({ "payload": payload })).expect("digest computation succeeds")
}

fn semantic_version(value: &str) -> SemanticVersion {
    value.parse().expect("valid semantic version")
}

fn base_identity() -> PackRevisionIdentity {
    PackRevisionIdentity {
        pack: PackId::parse("atlas").expect("valid pack id"),
        semantic_version: semantic_version("0.2.0"),
        system_state_digest: digest_of("system-state"),
        mission_digest: digest_of("mission"),
        policy_digest: digest_of("policy"),
        dependency_lock_digest: digest_of("dependency-lock"),
        parent_revision: None,
        composition_digest: None,
    }
}

#[test]
fn identical_tuples_produce_identical_revision_ids() {
    let identity = base_identity();
    assert_eq!(
        identity.revision_id().expect("revision id computes"),
        identity
            .revision_id()
            .expect("revision id computes deterministically")
    );
    let independently_built = base_identity();
    assert_eq!(
        identity.revision_id().expect("revision id computes"),
        independently_built
            .revision_id()
            .expect("revision id computes")
    );
}

#[test]
fn every_identity_covered_mutation_changes_the_revision_id() {
    let base = base_identity();
    let base_id = base.revision_id().expect("revision id computes");

    let mutated: Vec<(&str, PackRevisionIdentity)> = vec![
        (
            "pack lineage",
            PackRevisionIdentity {
                pack: PackId::parse("atlas-2").expect("valid pack id"),
                ..base.clone()
            },
        ),
        (
            "semantic version",
            PackRevisionIdentity {
                semantic_version: semantic_version("0.3.0"),
                ..base.clone()
            },
        ),
        (
            "system-state digest",
            PackRevisionIdentity {
                system_state_digest: digest_of("system-state-tampered"),
                ..base.clone()
            },
        ),
        (
            "mission digest",
            PackRevisionIdentity {
                mission_digest: digest_of("mission-tampered"),
                ..base.clone()
            },
        ),
        (
            "policy digest",
            PackRevisionIdentity {
                policy_digest: digest_of("policy-tampered"),
                ..base.clone()
            },
        ),
        (
            "dependency lock digest",
            PackRevisionIdentity {
                dependency_lock_digest: digest_of("dependency-lock-tampered"),
                ..base.clone()
            },
        ),
        (
            "parent revision",
            PackRevisionIdentity {
                parent_revision: Some(PackRevisionId::from_digest(digest_of("parent"))),
                ..base
            },
        ),
    ];

    assert_eq!(mutated.len(), 7, "the sweep must cover every tuple field");
    for (covered_field, mutation) in mutated {
        let mutated_id = mutation.revision_id().expect("revision id computes");
        assert_ne!(
            base_id, mutated_id,
            "mutating {covered_field} must change the revision identity"
        );
    }
}

#[test]
fn child_revisions_retain_parent_lineage_in_the_identity_tuple() {
    let parent = base_identity();
    let parent_id = parent.revision_id().expect("revision id computes");

    let child = PackRevisionIdentity {
        semantic_version: semantic_version("0.3.0"),
        parent_revision: Some(parent_id.clone()),
        ..base_identity()
    };
    assert_eq!(child.parent_revision, Some(parent_id.clone()));
    let child_id = child.revision_id().expect("revision id computes");
    assert_ne!(parent_id, child_id);

    // The recorded provenance and the identity tuple agree on the lineage:
    // both name the same parent revision.
    let provenance = PackProvenance::child_of(
        parent_id,
        ProvenanceProducer::agent("pack-architect-worker").expect("valid agent"),
    );
    assert_eq!(child.parent_revision, provenance.parent_revision);
}

#[test]
fn production_provenance_never_influences_revision_identity() {
    // The identity tuple has no producer field: the same governed content
    // proposed by different principals is the same revision identity. The
    // two provenance records below differ, yet neither participates in
    // `revision_id`, which is computed from the tuple alone.
    let identity = base_identity();
    let by_user = PackProvenance::root(ProvenanceProducer::user("org:atlas").expect("valid user"));
    let by_agent =
        PackProvenance::root(ProvenanceProducer::agent("mission-architect").expect("valid agent"));
    assert_ne!(by_user, by_agent);

    let tuple_id = identity.revision_id().expect("revision id computes");
    let same_content_again = base_identity();
    assert_eq!(
        tuple_id,
        same_content_again
            .revision_id()
            .expect("revision id computes")
    );
}

#[test]
fn revision_identity_round_trips_through_serde_and_rejects_unknown_fields() {
    let identity = PackRevisionIdentity {
        parent_revision: Some(PackRevisionId::from_digest(digest_of("parent"))),
        ..base_identity()
    };
    let serialized = serde_json::to_string(&identity).expect("serializable");
    let parsed: PackRevisionIdentity = serde_json::from_str(&serialized).expect("deserializable");
    assert_eq!(parsed, identity);
    assert_eq!(
        parsed.revision_id().expect("revision id computes"),
        identity.revision_id().expect("revision id computes")
    );

    let tampered = serialized.replace('{', "{\"injected\":1,");
    assert!(serde_json::from_str::<PackRevisionIdentity>(&tampered).is_err());
}

#[test]
fn root_revisions_omit_the_parent_field_on_the_wire() {
    let serialized = serde_json::to_string(&base_identity()).expect("serializable");
    let parsed: serde_json::Value = serde_json::from_str(&serialized).expect("valid json");
    assert!(parsed.get("parentRevision").is_none());
}

#[test]
fn compatibility_records_round_trip_and_reject_unknown_fields() {
    let compatibility = PackCompatibility::new(
        [
            ContractVersionRequirement {
                contract: PlatformContract::WorkflowContracts,
                minimum_version: semantic_version("1.4.0"),
            },
            ContractVersionRequirement {
                contract: PlatformContract::PackContracts,
                minimum_version: semantic_version("0.1.0"),
            },
        ],
        Some(ParentRevisionRelation::Breaking {
            rationale: CompatibilityRationale::parse("mission scope widened beyond billing")
                .expect("valid rationale"),
        }),
    );
    compatibility.validate().expect("current record validates");

    let serialized = serde_json::to_string(&compatibility).expect("serializable");
    let parsed: PackCompatibility = serde_json::from_str(&serialized).expect("deserializable");
    assert_eq!(parsed, compatibility);

    let tampered = serialized.replace('{', "{\"injected\":1,");
    assert!(serde_json::from_str::<PackCompatibility>(&tampered).is_err());
}

#[test]
fn compatibility_wire_shape_is_explicit_and_camel_cased() {
    let compatibility = PackCompatibility::new(
        [ContractVersionRequirement {
            contract: PlatformContract::WorkflowContracts,
            minimum_version: semantic_version("1.4.0"),
        }],
        Some(ParentRevisionRelation::Compatible),
    );
    let serialized = serde_json::to_string(&compatibility).expect("serializable");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&serialized).expect("valid json"),
        json!({
            "contractVersion": 1,
            "requiredContracts": [
                { "contract": "workflowContracts", "minimumVersion": "1.4.0" }
            ],
            "parentRelation": { "relation": "compatible" },
        })
    );
}

#[test]
fn compatibility_rejects_foreign_contract_versions_and_duplicate_requirements() {
    let foreign = PackCompatibility {
        contract_version: PACK_REVISION_CONTRACT_VERSION + 1,
        ..PackCompatibility::new([], None)
    };
    let error = foreign.validate().expect_err("foreign version must fail");
    assert!(
        error.to_string().contains("contract version"),
        "error must explain the version mismatch, got: {error}"
    );

    let duplicated = PackCompatibility {
        required_contracts: vec![
            ContractVersionRequirement {
                contract: PlatformContract::WorkflowContracts,
                minimum_version: semantic_version("1.4.0"),
            },
            ContractVersionRequirement {
                contract: PlatformContract::WorkflowContracts,
                minimum_version: semantic_version("2.0.0"),
            },
        ],
        ..PackCompatibility::new([], None)
    };
    let error = duplicated.validate().expect_err("duplicates must fail");
    assert!(
        error.to_string().contains("duplicate requirement"),
        "error must name the duplicate, got: {error}"
    );
}

#[test]
fn breaking_relations_require_a_non_empty_rationale() {
    for blank in ["", "   "] {
        assert!(
            CompatibilityRationale::parse(blank).is_err(),
            "blank rationale must be rejected"
        );
    }
    assert!(CompatibilityRationale::parse(" edge-space ").is_err());

    let serialized = serde_json::to_string(&ParentRevisionRelation::Breaking {
        rationale: CompatibilityRationale::parse("dependencies were re-pinned")
            .expect("valid rationale"),
    })
    .expect("serializable");
    let parsed: ParentRevisionRelation = serde_json::from_str(&serialized).expect("deserializable");
    assert_eq!(
        parsed,
        ParentRevisionRelation::Breaking {
            rationale: CompatibilityRationale::parse("dependencies were re-pinned")
                .expect("valid rationale")
        }
    );

    let blank_rationale = serialized.replace("dependencies were re-pinned", " ");
    assert!(
        serde_json::from_str::<ParentRevisionRelation>(&blank_rationale).is_err(),
        "blank rationales must be rejected on the wire"
    );
}
