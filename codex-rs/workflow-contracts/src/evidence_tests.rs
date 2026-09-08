use super::*;
use pretty_assertions::assert_eq;

#[test]
fn evidence_references_round_trip_through_serde() {
    let reference = EvidenceReference {
        kind: EvidenceKind::Trace,
        locator: "rollout-trace:9b2f".to_owned(),
        digest: ContentDigest::of(&serde_json::json!({ "trace": "9b2f" }))
            .expect("digest succeeds"),
    };

    let serialized = serde_json::to_string(&reference).expect("reference serializes");
    let parsed: EvidenceReference = serde_json::from_str(&serialized).expect("reference parses");

    assert_eq!(reference, parsed);
    assert!(parsed.digest.as_str().starts_with("sha256:"));
}

#[test]
fn evidence_kinds_serialize_as_camel_case() {
    let kinds = [
        (EvidenceKind::Observation, "\"observation\""),
        (EvidenceKind::Artifact, "\"artifact\""),
        (EvidenceKind::Approval, "\"approval\""),
        (EvidenceKind::Trace, "\"trace\""),
        (EvidenceKind::TestResult, "\"testResult\""),
        (EvidenceKind::Recovery, "\"recovery\""),
    ];
    for (kind, expected) in kinds {
        let serialized = serde_json::to_string(&kind).expect("kind serializes");
        assert_eq!(serialized, expected);
    }
}
