use super::*;
use crate::CapabilityId;
use pretty_assertions::assert_eq;

fn capability(name: &str) -> CapabilityRequirement {
    CapabilityRequirement {
        capability: CapabilityId::parse(name).expect("valid capability"),
        purpose: None,
    }
}

fn node_id(name: &str) -> IrNodeId {
    IrNodeId::parse(name).expect("valid node id")
}

fn condition(name: &str) -> ConditionId {
    ConditionId::parse(name).expect("valid condition id")
}

fn role(name: &str) -> RoleId {
    RoleId::parse(name).expect("valid role id")
}

/// A mixed-environment release workflow exercising every composition
/// primitive.
fn release_workflow_ir() -> WorkflowIr {
    let mut nodes = BTreeMap::new();
    let mut conditions = BTreeMap::new();

    conditions.insert(
        condition("release_green"),
        WorkflowCondition {
            condition: condition("release_green"),
            description: "All checks passed and the release is approved.".to_owned(),
        },
    );
    conditions.insert(
        condition("retry_warranted"),
        WorkflowCondition {
            condition: condition("retry_warranted"),
            description: "A transient failure is worth retrying.".to_owned(),
        },
    );

    nodes.insert(
        node_id("prepare"),
        WorkflowIrNode::Step(StepNode {
            capabilities: vec![capability("run_terminal_command")],
            roles: vec![role("release_engineer")],
            description: Some("Prepare the release notes".to_owned()),
            next: Some(node_id("gather")),
        }),
    );
    nodes.insert(
        node_id("gather"),
        WorkflowIrNode::ParallelFork(ParallelForkNode {
            branches: vec![node_id("browser_checks"), node_id("desktop_checks")],
        }),
    );
    nodes.insert(
        node_id("browser_checks"),
        WorkflowIrNode::Step(StepNode {
            capabilities: vec![capability("navigate_web")],
            roles: vec![role("verifier")],
            description: Some("Verify release pages in the browser".to_owned()),
            next: Some(node_id("join")),
        }),
    );
    nodes.insert(
        node_id("desktop_checks"),
        WorkflowIrNode::Step(StepNode {
            capabilities: vec![capability("control_desktop_app")],
            roles: vec![role("verifier")],
            description: Some("Smoke-test the desktop build".to_owned()),
            next: Some(node_id("join")),
        }),
    );
    nodes.insert(
        node_id("join"),
        WorkflowIrNode::ParallelJoin(ParallelJoinNode {
            policy: JoinPolicy::All,
            next: Some(node_id("verify")),
        }),
    );
    nodes.insert(
        node_id("verify"),
        WorkflowIrNode::Loop(LoopNode {
            condition: condition("retry_warranted"),
            body: node_id("retry_once"),
            bound: LoopBound { max_iterations: 3 },
            next: Some(node_id("wait_for_ci")),
        }),
    );
    nodes.insert(
        node_id("retry_once"),
        WorkflowIrNode::Sequence(SequenceNode {
            steps: vec![node_id("rerun_checks"), node_id("assess")],
        }),
    );
    nodes.insert(
        node_id("rerun_checks"),
        WorkflowIrNode::Step(StepNode {
            capabilities: vec![capability("read_screen")],
            roles: vec![role("verifier")],
            description: Some("Re-run the failing check".to_owned()),
            next: Some(node_id("assess")),
        }),
    );
    nodes.insert(
        node_id("assess"),
        WorkflowIrNode::Step(StepNode {
            capabilities: vec![capability("run_terminal_command")],
            roles: vec![role("release_engineer")],
            description: None,
            next: None,
        }),
    );
    nodes.insert(
        node_id("wait_for_ci"),
        WorkflowIrNode::Wait(WaitNode {
            wait_for: WaitFor::Trigger(TriggerClass::ConnectorEvent),
            next: Some(node_id("publish_or_not")),
        }),
    );
    nodes.insert(
        node_id("publish_or_not"),
        WorkflowIrNode::ConditionalBranch(ConditionalBranchNode {
            arms: vec![ConditionalBranchArm {
                condition: condition("release_green"),
                target: node_id("gate"),
            }],
            default: Some(node_id("abort")),
        }),
    );
    nodes.insert(
        node_id("gate"),
        WorkflowIrNode::HumanGate(HumanGateNode {
            approver: role("release_manager"),
            instruction: "Approve the public release.".to_owned(),
            next: Some(node_id("publish")),
        }),
    );
    nodes.insert(
        node_id("publish"),
        WorkflowIrNode::Subworkflow(SubworkflowNode {
            dependency: crate::SubworkflowDependencyId::parse("publish_notes")
                .expect("valid dependency id"),
            description: Some("Publish notes through the pinned subworkflow".to_owned()),
            next: Some(node_id("cleanup")),
        }),
    );
    nodes.insert(
        node_id("cleanup"),
        WorkflowIrNode::Compensation(CompensationNode {
            target: node_id("publish"),
            handler: node_id("abort"),
        }),
    );
    nodes.insert(
        node_id("abort"),
        WorkflowIrNode::Step(StepNode {
            capabilities: vec![capability("send_email")],
            roles: vec![role("release_engineer")],
            description: Some("Notify owners that the release was aborted".to_owned()),
            next: None,
        }),
    );

    WorkflowIr {
        ir_format: IR_FORMAT_VERSION,
        entry: node_id("prepare"),
        nodes,
        conditions,
    }
}

#[test]
fn mixed_environment_ir_round_trips_through_serde() {
    let ir = release_workflow_ir();

    let serialized = serde_json::to_string(&ir).expect("IR serializes");
    let parsed: WorkflowIr = serde_json::from_str(&serialized).expect("IR deserializes");

    assert_eq!(ir, parsed);
}

#[test]
fn mixed_environment_ir_validates() {
    release_workflow_ir()
        .validate()
        .expect("complete mixed-environment graph must validate");
}

#[test]
fn ir_reports_referenced_subworkflow_dependencies() {
    let ir = release_workflow_ir();

    assert_eq!(
        ir.referenced_subworkflow_dependencies(),
        vec![crate::SubworkflowDependencyId::parse("publish_notes").expect("valid id")]
    );
}

#[test]
fn ir_rejects_missing_entry() {
    let ir = release_workflow_ir();
    let broken = WorkflowIr {
        entry: node_id("does_not_exist"),
        ..ir
    };

    let error = broken.validate().expect_err("missing entry must fail");
    assert!(
        error.to_string().contains("does_not_exist"),
        "error must name the entry, got: {error}"
    );
}

#[test]
fn ir_rejects_missing_node_reference() {
    let mut ir = release_workflow_ir();
    let WorkflowIrNode::Step(prepare) = ir
        .nodes
        .get_mut(&node_id("prepare"))
        .expect("prepare node exists")
    else {
        panic!("prepare must be a step");
    };
    prepare.next = Some(node_id("ghost"));

    let error = ir.validate().expect_err("dangling reference must fail");
    assert!(
        error.to_string().contains("ghost"),
        "error must name the missing node, got: {error}"
    );
}

#[test]
fn ir_rejects_missing_condition_reference() {
    let mut ir = release_workflow_ir();
    ir.conditions.remove(&condition("release_green"));

    let error = ir.validate().expect_err("undeclared condition must fail");
    assert!(
        error.to_string().contains("release_green"),
        "error must name the missing condition, got: {error}"
    );
}

#[test]
fn ir_rejects_self_references() {
    let mut ir = release_workflow_ir();
    let WorkflowIrNode::Step(abort) = ir
        .nodes
        .get_mut(&node_id("abort"))
        .expect("abort node exists")
    else {
        panic!("abort must be a step");
    };
    abort.next = Some(node_id("abort"));

    let error = ir.validate().expect_err("self reference must fail");
    assert!(
        error.to_string().contains("references itself"),
        "error must describe the self reference, got: {error}"
    );
}

#[test]
fn ir_rejects_unsupported_format_versions() {
    let ir = release_workflow_ir();
    let future = WorkflowIr {
        ir_format: IR_FORMAT_VERSION + 1,
        ..ir
    };

    let error = future.validate().expect_err("future IR format must fail");
    assert!(
        error.to_string().contains("unsupported IR format version"),
        "error must describe the version mismatch, got: {error}"
    );
}

#[test]
fn node_kinds_serialize_with_their_tag() {
    let serialized = serde_json::to_value(WorkflowIrNode::Sequence(SequenceNode {
        steps: vec![node_id("one")],
    }))
    .expect("sequence serializes");

    assert_eq!(
        serialized,
        serde_json::json!({ "node": "sequence", "steps": ["one"] })
    );
}
