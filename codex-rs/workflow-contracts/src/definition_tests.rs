use super::*;
use crate::CapabilityId;
use crate::CapabilityRequirement;
use crate::ConditionId;
use crate::IrNodeId;
use crate::RoleId;
use crate::StepNode;
use crate::TriggerClass;
use crate::WorkflowIr;
use crate::WorkflowIrNode;
use crate::WorkflowRole;
use pretty_assertions::assert_eq;

fn test_role(name: &str) -> WorkflowRole {
    WorkflowRole {
        role: RoleId::parse(name).expect("valid role id"),
        name: format!("{name} role"),
        objective: "Verify the workflow.".to_owned(),
        allowed_capabilities: vec![],
        approval: crate::ApprovalRequirement::NotRequired,
    }
}

fn minimal_definition() -> WorkflowDefinition {
    let entry = IrNodeId::parse("check").expect("valid node id");
    let mut nodes = BTreeMap::new();
    nodes.insert(
        entry.clone(),
        WorkflowIrNode::Step(StepNode {
            capabilities: vec![CapabilityRequirement {
                capability: CapabilityId::parse("navigate_web").expect("valid capability"),
                purpose: None,
            }],
            roles: vec![RoleId::parse("verifier").expect("valid role id")],
            description: Some("Check the release page".to_owned()),
            next: None,
        }),
    );
    let ir = WorkflowIr {
        ir_format: crate::IR_FORMAT_VERSION,
        entry,
        nodes,
        conditions: BTreeMap::new(),
    };
    let mut roles = BTreeMap::new();
    roles.insert(
        RoleId::parse("verifier").expect("valid role id"),
        test_role("verifier"),
    );
    WorkflowDefinition {
        id: WorkflowDefinitionId::parse("release-notes").expect("valid workflow id"),
        description: Some("Draft and publish release notes".to_owned()),
        ir,
        roles,
        triggers: vec![WorkflowTrigger {
            trigger: TriggerClass::User,
            description: None,
        }],
        dependencies: WorkflowDependencies::default(),
    }
}

#[test]
fn definitions_round_trip_through_serde() {
    let definition = minimal_definition();

    let serialized = serde_json::to_string(&definition).expect("definition serializes");
    let parsed: WorkflowDefinition = serde_json::from_str(&serialized).expect("definition parses");

    assert_eq!(definition, parsed);
}

#[test]
fn definition_digests_are_stable_across_serialization() {
    let definition = minimal_definition();
    let reserialized: WorkflowDefinition =
        serde_json::from_str(&serde_json::to_string(&definition).expect("definition serializes"))
            .expect("definition parses");

    assert_eq!(
        definition.digest().expect("digest computes"),
        reserialized.digest().expect("re-digest computes")
    );
}

#[test]
fn definition_digests_ignores_nothing_semantic() {
    let definition = minimal_definition();
    let mutated = WorkflowDefinition {
        description: Some("Different description".to_owned()),
        ..minimal_definition()
    };

    assert_ne!(
        definition.digest().expect("digest computes"),
        mutated.digest().expect("mutated digest computes")
    );
}

#[test]
fn definitions_validate_roles_and_subworkflow_references() {
    minimal_definition()
        .validate()
        .expect("complete definition validates");

    let missing_role = WorkflowDefinition {
        roles: BTreeMap::new(),
        ..minimal_definition()
    };
    let error = missing_role
        .validate()
        .expect_err("undeclared role must fail");
    assert!(
        error.to_string().contains("verifier"),
        "error must name the missing role, got: {error}"
    );
}

#[test]
fn condition_ids_parse_non_empty() {
    assert!(ConditionId::parse("release_green").is_ok());
    assert!(ConditionId::parse("").is_err());
}
