use super::*;
use pretty_assertions::assert_eq;

const COMMIT_40: &str = "0123456789abcdef0123456789abcdef01234567";
const COMMIT_64: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

#[test]
fn revision_sha_accepts_full_sha1_and_sha256_forms() {
    let sha1 = RevisionSha::parse(COMMIT_40).expect("full SHA-1 commit accepted");
    let sha256 = RevisionSha::parse(COMMIT_64).expect("full SHA-256 commit accepted");

    assert_eq!(sha1.as_str(), COMMIT_40);
    assert_eq!(sha256.as_str(), COMMIT_64);
}

#[test]
fn revision_sha_rejects_abbreviated_and_uppercase_forms() {
    let rejects = ["0123456", "", "XYZ"];
    for value in rejects {
        assert!(
            RevisionSha::parse(value).is_err(),
            "abbreviated or malformed SHA `{value}` must be rejected"
        );
    }
    let uppercase = COMMIT_40.to_uppercase();
    assert!(
        RevisionSha::parse(uppercase).is_err(),
        "uppercase SHA must be rejected"
    );
}

#[test]
fn immutable_revision_serializes_with_optional_provenance() {
    let sha = RevisionSha::parse(COMMIT_40).expect("valid sha");
    let with_provenance =
        ImmutableSourceRevision::pin(sha.clone(), DevelopmentRef::branch("main").ok());
    let without_provenance = ImmutableSourceRevision::pin_commit(sha);

    let serialized =
        serde_json::to_value(&with_provenance).expect("revision with provenance serializes");
    assert_eq!(
        serialized,
        serde_json::json!({
            "commitSha": COMMIT_40,
            "resolvedFrom": { "branch": "main" },
        })
    );

    let serialized =
        serde_json::to_value(&without_provenance).expect("revision without provenance serializes");
    assert_eq!(serialized, serde_json::json!({ "commitSha": COMMIT_40 }));
}

#[test]
fn version_id_wraps_and_validates_content_digest() {
    let digest =
        ContentDigest::of(&serde_json::json!({ "alpha": 1 })).expect("digest computation succeeds");
    let version_id = WorkflowVersionId::from_digest(digest);

    let serialized = serde_json::to_string(&version_id).expect("version id serializes");
    let parsed: WorkflowVersionId =
        serde_json::from_str(&serialized).expect("version id deserializes");
    assert_eq!(version_id, parsed);

    let rejected = WorkflowVersionId::try_from("sha256:not-a-digest");
    assert!(rejected.is_err(), "malformed version ids must be rejected");
}

#[test]
fn development_refs_parse_and_display_their_kind() {
    let branch = DevelopmentRef::branch("feature/retry").expect("branch parses");
    let tag = DevelopmentRef::tag("v1").expect("tag parses");

    assert_eq!(branch.to_string(), "branch:feature/retry");
    assert_eq!(tag.to_string(), "tag:v1");
    assert_eq!(branch.name(), "feature/retry");
}

#[test]
fn development_refs_reject_empty_and_padded_values() {
    assert!(DevelopmentRef::branch("").is_err());
    assert!(DevelopmentRef::tag("").is_err());
    assert!(DevelopmentRef::branch(" padded ").is_err());
}
