//! Tests for the shared identity foundations.

use codex_workflow_contracts::ContentDigest;
use pretty_assertions::assert_eq;
use serde::Serialize;
use serde_json::json;

use crate::PackContractError;
use crate::PackId;
use crate::PackPolicyId;
use crate::PackProvenance;
use crate::PackRevisionId;
use crate::ProvenanceProducer;

/// A minimal serializable record used to produce content digests in tests.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SampleRecord {
    mission: &'static str,
    revision: u32,
}

fn sample_digest() -> ContentDigest {
    ContentDigest::of(&SampleRecord {
        mission: "sample-mission",
        revision: 17,
    })
    .expect("digest computation succeeds")
}

#[test]
fn pack_id_parse_accepts_valid_slugs() {
    for valid in ["atlas", "medical-codex", "billing-2", "a", "pack-0"] {
        let parsed =
            PackId::parse(valid).unwrap_or_else(|error| panic!("`{valid}` rejected: {error}"));
        assert_eq!(parsed.as_str(), valid);
        assert_eq!(parsed.to_string(), valid);
    }
}

#[test]
fn pack_id_parse_rejects_invalid_slugs() {
    let invalid = [
        "".to_owned(),
        "-atlas".to_owned(),
        "atlas-".to_owned(),
        "Atlas".to_owned(),
        "atlas_pack".to_owned(),
        "atlas/pack".to_owned(),
        "atlas pack".to_owned(),
        "atlas.pack".to_owned(),
        "a".repeat(129),
    ];
    for invalid in invalid {
        let result = PackId::parse(invalid.clone());
        assert!(
            matches!(result, Err(PackContractError::InvalidIdentifier { .. })),
            "`{invalid}` should be rejected"
        );
    }
}

#[test]
fn pack_id_serde_round_trips() {
    let pack_id = PackId::parse("atlas").expect("valid slug");
    let serialized = serde_json::to_string(&pack_id).expect("serializable");
    assert_eq!(serialized, "\"atlas\"");
    let deserialized: PackId = serde_json::from_str(&serialized).expect("deserializable");
    assert_eq!(deserialized, pack_id);
    assert!(serde_json::from_str::<PackId>("\"Atlas\"").is_err());
}

#[test]
fn pack_revision_id_round_trips_through_digest_form() {
    let revision_id = PackRevisionId::from_digest(sample_digest());
    let serialized = serde_json::to_string(&revision_id).expect("serializable");
    let deserialized: PackRevisionId = serde_json::from_str(&serialized).expect("deserializable");
    assert_eq!(deserialized, revision_id);
    assert_eq!(deserialized.as_ref(), revision_id.as_ref());
    assert_eq!(deserialized.to_string(), revision_id.to_string());
}

#[test]
fn pack_revision_id_rejects_malformed_digests() {
    for malformed in ["", "sha256:", "deadbeef", "sha256:ABC", "not-a-digest"] {
        let result = PackRevisionId::try_from(malformed);
        assert!(
            matches!(result, Err(PackContractError::InvalidDigest { .. })),
            "`{malformed}` should be rejected"
        );
    }
}

#[test]
fn pack_policy_id_round_trips_and_rejects_malformed_digests() {
    let policy_id = PackPolicyId::from_digest(sample_digest());
    let serialized = serde_json::to_string(&policy_id).expect("serializable");
    let deserialized: PackPolicyId = serde_json::from_str(&serialized).expect("deserializable");
    assert_eq!(deserialized, policy_id);

    let result = PackPolicyId::try_from("sha256:short");
    assert!(matches!(
        result,
        Err(PackContractError::InvalidDigest { .. })
    ));
}

#[test]
fn pack_provenance_serde_round_trips_with_parent() {
    let parent = PackRevisionId::from_digest(sample_digest());
    let provenance = PackProvenance::child_of(
        parent.clone(),
        ProvenanceProducer::agent("pack-architect-worker").expect("valid agent"),
    );
    let serialized = serde_json::to_string(&provenance).expect("serializable");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&serialized).expect("valid json"),
        json!({
            "parentRevision": parent.to_string(),
            "producer": { "kind": "agent", "agent": "pack-architect-worker" },
        }),
    );
    let deserialized: PackProvenance = serde_json::from_str(&serialized).expect("deserializable");
    assert_eq!(deserialized, provenance);
}

#[test]
fn pack_provenance_root_omits_parent_field() {
    let provenance = PackProvenance::root(
        ProvenanceProducer::control_plane("workflow-control-plane").expect("valid plane"),
    );
    let serialized = serde_json::to_string(&provenance).expect("serializable");
    assert!(!serialized.contains("parentRevision"));
    let deserialized: PackProvenance = serde_json::from_str(&serialized).expect("deserializable");
    assert_eq!(deserialized, provenance);
}

#[test]
fn pack_provenance_rejects_unknown_fields() {
    let parent = PackRevisionId::from_digest(sample_digest());
    let serialized = serde_json::to_string(&PackProvenance::child_of(
        parent,
        ProvenanceProducer::user("org:atlas").expect("valid subject"),
    ))
    .expect("serializable");
    let tampered = serialized.replace('{', "{\"injected\":1,");
    assert!(serde_json::from_str::<PackProvenance>(&tampered).is_err());
}

#[test]
fn provenance_producers_reject_blank_principals() {
    for blank in ["", "  ", " \t "] {
        assert!(
            ProvenanceProducer::user(blank).is_err(),
            "blank user subject must be rejected"
        );
        assert!(
            ProvenanceProducer::agent(blank).is_err(),
            "blank agent must be rejected"
        );
        assert!(
            ProvenanceProducer::control_plane(blank).is_err(),
            "blank plane must be rejected"
        );
    }
    assert!(ProvenanceProducer::user(" edge-space ").is_err());
}
