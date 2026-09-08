use super::*;
use pretty_assertions::assert_eq;

#[test]
fn repository_id_strips_embedded_credentials() {
    let plain = WorkflowRepositoryId::parse("https://github.com/acme/release-bot")
        .expect("plain https URL is accepted");
    let credentialed =
        WorkflowRepositoryId::parse("https://user:secret@example.com/acme/release-bot.git")
            .expect("credentialed URL parses");

    let sanitized = credentialed.as_str();
    assert!(
        !sanitized.contains("secret") && !sanitized.contains("user:"),
        "credentials must never enter repository identity, got `{sanitized}`"
    );
    assert_eq!(plain.as_str(), "github.com/acme/release-bot");
    assert_eq!(credentialed.as_str(), "example.com/acme/release-bot");
}

#[test]
fn repository_identity_is_transport_independent() {
    let https = WorkflowRepositoryId::parse("https://github.com/acme/release-bot.git")
        .expect("https remote parses");
    let ssh = WorkflowRepositoryId::parse("git@github.com:acme/release-bot.git")
        .expect("scp-like remote parses");

    assert_eq!(
        https, ssh,
        "the same repository over different transports is one workflow repository"
    );
}

#[test]
fn repository_id_round_trips_through_serde() {
    let repository = WorkflowRepositoryId::parse("https://github.com/acme/release-bot")
        .expect("valid repository id");

    let serialized = serde_json::to_string(&repository).expect("repository id serializes");
    let parsed: WorkflowRepositoryId =
        serde_json::from_str(&serialized).expect("repository id deserializes");

    assert_eq!(repository, parsed);
}

#[test]
fn repository_id_rejects_invalid_urls() {
    let result = WorkflowRepositoryId::parse("not a git url at all");
    assert!(result.is_err(), "malformed remote URLs must be rejected");
}

#[test]
fn forge_kind_is_opaque_data() {
    let github = ForgeKind::parse(ForgeKind::GITHUB).expect("github kind parses");
    let local = ForgeKind::parse("local").expect("local kind parses");

    assert_eq!(github.as_ref(), "github");
    assert_eq!(local.as_ref(), "local");
}

#[test]
fn instance_ids_round_trip_and_display() {
    let instance = WorkflowInstanceId::generate();

    let serialized = serde_json::to_string(&instance).expect("instance id serializes");
    let parsed: WorkflowInstanceId =
        serde_json::from_str(&serialized).expect("instance id deserializes");

    assert_eq!(instance, parsed);
    assert_eq!(instance.to_string(), parsed.as_uuid().to_string());
}

#[test]
fn semantic_versions_are_semver_types() {
    let version: SemanticVersion = "1.4.2".parse().expect("semantic version parses");
    assert_eq!(version.major, 1);
    assert_eq!(version.minor, 4);
    assert_eq!(version.patch, 2);
}

#[test]
fn workflow_definition_ids_reject_empty_and_padded_values() {
    assert!(WorkflowDefinitionId::parse("release-notes").is_ok());
    assert!(WorkflowDefinitionId::parse("").is_err());
    assert!(WorkflowDefinitionId::parse(" padded ").is_err());
}
