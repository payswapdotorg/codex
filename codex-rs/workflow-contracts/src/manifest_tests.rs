use super::*;
use pretty_assertions::assert_eq;

fn manifest() -> WorkflowManifest {
    WorkflowManifest {
        manifest_version: MANIFEST_FORMAT_VERSION,
        workflow: WorkflowDefinitionId::parse("release-notes").expect("valid workflow id"),
        display_name: Some("Release Notes".to_owned()),
        description: Some("Draft and publish release notes".to_owned()),
        repository: WorkflowRepositoryId::parse("https://github.com/acme/release-bot")
            .expect("valid repo"),
        definition_path: RepositoryRelativePath::parse("workflows/release-notes/definition.json")
            .expect("valid path"),
        provenance: None,
    }
}

#[test]
fn manifests_round_trip_through_serde() {
    let manifest = manifest();

    let serialized = serde_json::to_string(&manifest).expect("manifest serializes");
    let parsed: WorkflowManifest = serde_json::from_str(&serialized).expect("manifest parses");

    assert_eq!(manifest, parsed);
    assert!(serialized.contains("release-notes"));
}

#[test]
fn manifests_validate_their_format_version() {
    manifest().validate().expect("current format validates");

    let future = WorkflowManifest {
        manifest_version: MANIFEST_FORMAT_VERSION + 1,
        ..manifest()
    };
    assert!(
        future.validate().is_err(),
        "future formats must be rejected"
    );
}

#[test]
fn repository_paths_reject_traversal_and_absolute_forms() {
    let rejects = [
        "",
        "/etc/passwd",
        "workflows/../secrets",
        "./workflows/definition.json",
        "workflows//definition.json",
        "C:\\workflows\\definition.json",
    ];
    for value in rejects {
        assert!(
            RepositoryRelativePath::parse(value).is_err(),
            "path `{value}` must be rejected"
        );
    }
}

#[test]
fn repository_paths_accept_normalized_relative_forms() {
    let accepted = "workflows/release-notes/definition.json";
    let path = RepositoryRelativePath::parse(accepted).expect("normalized path accepted");

    assert_eq!(path.as_ref(), accepted);
}
