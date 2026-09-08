use super::*;
use pretty_assertions::assert_eq;
use serde::Serialize;

#[derive(Serialize)]
struct FieldOrderFixture {
    zulu: u32,
    alpha: u32,
    nested: NestedFixture,
}

#[derive(Serialize)]
struct NestedFixture {
    second: u32,
    first: u32,
}

#[test]
fn digest_is_stable_across_field_order_and_key_order() {
    let one = serde_json::json!({
        "zulu": 1,
        "alpha": 2,
        "nested": { "second": 3, "first": 4 },
    });
    let two = serde_json::json!({
        "alpha": 2,
        "zulu": 1,
        "nested": { "first": 4, "second": 3 },
    });

    assert_eq!(
        ContentDigest::of(&one).expect("digest one").as_str(),
        ContentDigest::of(&two).expect("digest two").as_str()
    );
}

#[test]
fn digest_is_stable_for_structs_and_equivalent_json() {
    let fixture = FieldOrderFixture {
        zulu: 1,
        alpha: 2,
        nested: NestedFixture {
            second: 3,
            first: 4,
        },
    };
    let equivalent = serde_json::json!({
        "zulu": 1,
        "alpha": 2,
        "nested": { "second": 3, "first": 4 },
    });

    assert_eq!(
        ContentDigest::of(&fixture)
            .expect("digest fixture")
            .as_str(),
        ContentDigest::of(&equivalent)
            .expect("digest equivalent")
            .as_str()
    );
}

#[test]
fn digest_changes_when_content_changes() {
    let one = serde_json::json!({ "alpha": 2 });
    let two = serde_json::json!({ "alpha": 3 });

    assert_ne!(
        ContentDigest::of(&one).expect("digest one").as_str(),
        ContentDigest::of(&two).expect("digest two").as_str()
    );
}

#[test]
fn digest_round_trips_through_serialization() {
    let digest = ContentDigest::of(&serde_json::json!({ "alpha": 2 }))
        .expect("digest of simple value must succeed");

    let serialized = serde_json::to_string(&digest).expect("digest serializes");
    let parsed: ContentDigest = serde_json::from_str(&serialized).expect("digest deserializes");

    assert_eq!(digest, parsed);
}

#[test]
fn digest_rejects_malformed_values() {
    let rejects = [
        "",
        "sha256:",
        "sha256:abc",
        "md5:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
        "sha256:0123456789ABCDEF0123456789abcdef0123456789abcdef0123456789abcdef",
        "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcde",
    ];
    for value in rejects {
        let result = ContentDigest::try_from(value);
        assert!(result.is_err(), "expected `{value}` to be rejected");
    }
}

#[test]
fn digest_accepts_well_formed_values() {
    let accepted = "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    let digest = ContentDigest::try_from(accepted).expect("well-formed digest is accepted");

    assert_eq!(digest.as_str(), accepted);
}
