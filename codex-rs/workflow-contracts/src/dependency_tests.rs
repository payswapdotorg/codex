use super::*;
use crate::ContentDigest;
use crate::WorkflowDefinitionId;
use crate::WorkflowRepositoryId;
use crate::WorkflowVersionId;
use pretty_assertions::assert_eq;

fn subworkflow_dependency() -> SubworkflowDependency {
    SubworkflowDependency {
        dependency_id: SubworkflowDependencyId::parse("publish_notes").expect("valid id"),
        repository: WorkflowRepositoryId::parse("https://github.com/acme/notes-bot")
            .expect("valid repo"),
        workflow: WorkflowDefinitionId::parse("publish-notes").expect("valid workflow id"),
        pin: SubworkflowVersionPin::Exact("1.4.2".parse().expect("valid semver")),
    }
}

fn version_id() -> WorkflowVersionId {
    WorkflowVersionId::from_digest(
        ContentDigest::of(&serde_json::json!({ "workflow": "publish-notes" })).expect("digest"),
    )
}

#[test]
fn snake_case_ids_accept_the_architecture_examples() {
    for value in [
        "navigate_web",
        "click_element",
        "fill_form",
        "run_terminal_command",
    ] {
        assert!(
            CapabilityId::parse(value).is_ok(),
            "capability `{value}` must parse"
        );
    }
}

#[test]
fn snake_case_ids_reject_invalid_forms() {
    let rejects = [
        "",
        "NavigateWeb",
        "navigate-web",
        "_lead",
        "trail_",
        "sp ace",
    ];
    for value in rejects {
        assert!(
            CapabilityId::parse(value).is_err(),
            "`{value}` must be rejected"
        );
    }
}

#[test]
fn resource_type_ids_cover_architecture_resource_classes() {
    for value in [
        "browser_profile",
        "desktop_session",
        "github_account",
        "slack_workspace",
        "terminal_workspace",
        "human_approver",
        "mobile_device",
    ] {
        assert!(
            ResourceTypeId::parse(value).is_ok(),
            "resource `{value}` must parse"
        );
    }
}

#[test]
fn skill_and_plugin_names_follow_codex_token_rules() {
    assert!(SkillName::parse("browser-use").is_ok());
    assert!(SkillName::parse("web_search").is_ok());
    let skill = SkillName::parse("browser-use")
        .expect("skill name parses")
        .provided_by("acme.plugins");
    assert_eq!(skill.plugin.as_deref(), Some("acme.plugins"));

    assert!(PluginName::parse("my-plugin").is_ok());
    assert!(PluginName::parse("my_plugin.v2").is_ok());
    assert!(PluginName::parse("../traversal").is_err());
    assert!(PluginName::parse(".hidden").is_err());
    assert!(PluginName::parse("").is_err());
}

#[test]
fn dependency_records_round_trip_through_serde() {
    let dependencies = WorkflowDependencies {
        skills: vec![SkillDependency {
            skill: SkillName::parse("browser-use").expect("valid skill"),
            version_requirement: Some(">=1.0.0".parse().expect("valid range")),
        }],
        plugins: vec![PluginDependency {
            plugin: PluginName::parse("acme-connector").expect("valid plugin"),
            version_requirement: None,
        }],
        mcp: vec![McpDependency {
            capability: McpCapabilityId::parse("github_issue_tracker").expect("valid capability"),
            purpose: Some("file release issues".to_owned()),
        }],
        subworkflows: vec![subworkflow_dependency()],
        resources: vec![ResourceRequirement {
            resource_type: ResourceTypeId::parse("github_account").expect("valid resource"),
            purpose: Some("release automation account".to_owned()),
        }],
    };

    let serialized = serde_json::to_string(&dependencies).expect("dependencies serialize");
    let parsed: WorkflowDependencies =
        serde_json::from_str(&serialized).expect("dependencies deserialize");

    assert_eq!(dependencies, parsed);
}

#[test]
fn subworkflow_pins_round_trip_for_all_pin_kinds() {
    let pins = vec![
        SubworkflowVersionPin::Range(">=1.4.0, <2.0.0".parse().expect("valid range")),
        SubworkflowVersionPin::Exact("1.4.2".parse().expect("valid semver")),
        SubworkflowVersionPin::VersionId(version_id()),
    ];
    for pin in pins {
        let dependency = SubworkflowDependency {
            pin: pin.clone(),
            ..subworkflow_dependency()
        };
        let serialized = serde_json::to_string(&dependency).expect("dependency serializes");
        let parsed: SubworkflowDependency =
            serde_json::from_str(&serialized).expect("dependency deserializes");
        assert_eq!(dependency, parsed);
    }
}
