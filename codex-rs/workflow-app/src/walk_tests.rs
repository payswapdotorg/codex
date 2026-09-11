//! Walk tests: the runtime walk mirrors the frozen compiler replay policy.

use std::collections::BTreeMap;

use codex_teaching_compiler::SimulationConfig;
use codex_teaching_compiler::simulate_ir;
use codex_workflow_contracts::ConditionId;
use codex_workflow_contracts::ConditionalBranchArm;
use codex_workflow_contracts::ConditionalBranchNode;
use codex_workflow_contracts::IR_FORMAT_VERSION;
use codex_workflow_contracts::IrNodeId;
use codex_workflow_contracts::LoopBound;
use codex_workflow_contracts::LoopNode;
use codex_workflow_contracts::ParallelForkNode;
use codex_workflow_contracts::ParallelJoinNode;
use codex_workflow_contracts::SequenceNode;
use codex_workflow_contracts::StepNode;
use codex_workflow_contracts::WaitFor;
use codex_workflow_contracts::WaitNode;
use codex_workflow_contracts::WorkflowCondition;
use codex_workflow_contracts::WorkflowIr;
use codex_workflow_contracts::WorkflowIrNode;
use pretty_assertions::assert_eq as pretty_assert_eq;

use super::Walk;
use super::WalkStep;
use super::WalkTerminal;
use crate::WalkConfig;

fn node_id(value: &str) -> IrNodeId {
    IrNodeId::parse(value).expect("node id")
}

fn step(next: Option<&str>) -> WorkflowIrNode {
    WorkflowIrNode::Step(StepNode {
        capabilities: Vec::new(),
        roles: Vec::new(),
        description: None,
        next: next.map(node_id),
    })
}

fn ir(entry: &str, nodes: BTreeMap<IrNodeId, WorkflowIrNode>) -> WorkflowIr {
    WorkflowIr {
        ir_format: IR_FORMAT_VERSION,
        entry: node_id(entry),
        nodes,
        conditions: BTreeMap::new(),
    }
}

/// Drives the walk to its terminal state, returning the visited path.
fn walk_path(ir: &WorkflowIr) -> (Vec<IrNodeId>, WalkTerminal) {
    let mut walk = Walk::new(ir).expect("walk");
    loop {
        match walk.next(ir, WalkConfig::default()).expect("next") {
            WalkStep::Visit(_) => {}
            WalkStep::Terminal(terminal) => return (walk.path().to_vec(), terminal),
        }
    }
}

#[test]
fn walk_path_matches_the_compiler_simulation_on_a_mixed_graph() {
    // Entry sequence -> fork with two branches -> join -> loop -> wait.
    let mut nodes = BTreeMap::new();
    nodes.insert(node_id("seq"), {
        WorkflowIrNode::Sequence(SequenceNode {
            steps: vec![node_id("step-001"), node_id("fork")],
        })
    });
    nodes.insert(node_id("step-001"), step(None));
    nodes.insert(node_id("fork"), {
        WorkflowIrNode::ParallelFork(ParallelForkNode {
            branches: vec![node_id("step-002"), node_id("step-003")],
        })
    });
    nodes.insert(node_id("step-002"), step(Some("join")));
    nodes.insert(node_id("step-003"), step(Some("join")));
    nodes.insert(node_id("join"), {
        WorkflowIrNode::ParallelJoin(ParallelJoinNode {
            policy: codex_workflow_contracts::JoinPolicy::All,
            next: Some(node_id("loop")),
        })
    });
    nodes.insert(node_id("loop"), {
        WorkflowIrNode::Loop(LoopNode {
            condition: ConditionId::parse("cond").expect("condition"),
            body: node_id("step-004"),
            bound: LoopBound { max_iterations: 3 },
            next: Some(node_id("wait")),
        })
    });
    nodes.insert(node_id("step-004"), step(None));
    nodes.insert(node_id("wait"), {
        WorkflowIrNode::Wait(WaitNode {
            wait_for: WaitFor::Trigger(codex_workflow_contracts::TriggerClass::User),
            next: None,
        })
    });
    let mut conditions = BTreeMap::new();
    conditions.insert(
        ConditionId::parse("cond").expect("condition"),
        WorkflowCondition {
            condition: ConditionId::parse("cond").expect("condition"),
            description: "declared for tests".to_string(),
        },
    );
    let mut graph = ir("seq", nodes);
    graph.conditions = conditions;

    let simulation = simulate_ir(&graph, SimulationConfig::default()).expect("simulation");
    let (path, terminal) = walk_path(&graph);
    pretty_assert_eq!(path, simulation.path().to_vec());
    assert!(matches!(terminal, WalkTerminal::Paused { .. }));
    assert_eq!(
        simulation.outcome(),
        &codex_teaching_compiler::SimulationOutcome::Paused {
            at: node_id("wait")
        }
    );
}

#[test]
fn conditional_branch_takes_the_default_arm_then_the_first_arm() {
    let condition = ConditionId::parse("cond").expect("condition");
    let declared = || {
        let mut conditions = BTreeMap::new();
        conditions.insert(
            ConditionId::parse("cond").expect("condition"),
            WorkflowCondition {
                condition: ConditionId::parse("cond").expect("condition"),
                description: "declared for tests".to_string(),
            },
        );
        conditions
    };
    let mut nodes = BTreeMap::new();
    nodes.insert(node_id("branch-1"), {
        WorkflowIrNode::ConditionalBranch(ConditionalBranchNode {
            arms: vec![ConditionalBranchArm {
                condition,
                target: node_id("arm-step"),
            }],
            default: Some(node_id("default-step")),
        })
    });
    nodes.insert(node_id("default-step"), step(None));
    nodes.insert(node_id("arm-step"), step(None));
    let mut graph = ir("branch-1", nodes);
    graph.conditions = declared();
    let (path, terminal) = walk_path(&graph);
    assert_eq!(path, vec![node_id("branch-1"), node_id("default-step")]);
    assert_eq!(terminal, WalkTerminal::Completed);

    // No default arm: the first declared arm applies.
    let mut nodes = BTreeMap::new();
    nodes.insert(node_id("branch-2"), {
        WorkflowIrNode::ConditionalBranch(ConditionalBranchNode {
            arms: vec![ConditionalBranchArm {
                condition: ConditionId::parse("cond").expect("condition"),
                target: node_id("arm-step"),
            }],
            default: None,
        })
    });
    nodes.insert(node_id("arm-step"), step(None));
    let mut graph = ir("branch-2", nodes);
    graph.conditions = declared();
    let (path, terminal) = walk_path(&graph);
    assert_eq!(path, vec![node_id("branch-2"), node_id("arm-step")]);
    assert_eq!(terminal, WalkTerminal::Completed);
}

#[test]
fn budget_exceeded_aborts_the_walk() {
    // A step that loops back to itself is structurally rejected by the
    // contract, so use a chain long enough to exceed a budget of 2.
    let mut nodes = BTreeMap::new();
    nodes.insert(node_id("step-001"), step(Some("step-002")));
    nodes.insert(node_id("step-002"), step(Some("step-003")));
    nodes.insert(node_id("step-003"), step(None));
    let graph = ir("step-001", nodes);
    let mut walk = Walk::new(&graph).expect("walk");
    let config = WalkConfig { max_steps: 2 };
    assert!(walk.next(&graph, config).is_ok());
    assert!(walk.next(&graph, config).is_ok());
    assert!(walk.next(&graph, config).is_err());
}

#[test]
fn a_paused_walk_resumes_at_the_wait_nodes_successor() {
    // step-001 -> wait -> step-002: the pause records the pending
    // re-entry point, so resuming continues at the wait node's successor
    // and reaches the terminal state.
    let mut nodes = BTreeMap::new();
    nodes.insert(node_id("step-001"), step(Some("wait")));
    nodes.insert(node_id("wait"), {
        WorkflowIrNode::Wait(WaitNode {
            wait_for: WaitFor::Trigger(codex_workflow_contracts::TriggerClass::User),
            next: Some(node_id("step-002")),
        })
    });
    nodes.insert(node_id("step-002"), step(None));
    let graph = ir("step-001", nodes);

    let mut walk = Walk::new(&graph).expect("walk");
    let config = WalkConfig::default();
    assert!(matches!(
        walk.next(&graph, config).expect("step"),
        WalkStep::Visit(ref node) if *node == node_id("step-001")
    ));
    let WalkStep::Terminal(WalkTerminal::Paused { node, .. }) =
        walk.next(&graph, config).expect("pause")
    else {
        panic!("expected the walk to pause at the wait node");
    };
    pretty_assert_eq!(node, node_id("wait"));
    // The position at the pause holds the pending re-entry point.
    let position = walk.position();
    pretty_assert_eq!(position.current, Some(node_id("step-002")));
    pretty_assert_eq!(position.path, vec![node_id("step-001"), node_id("wait")]);
    assert_eq!(position.steps_taken, 2);
    // Continuing the same walk visits the successor and completes.
    assert!(matches!(
        walk.next(&graph, config).expect("resume step"),
        WalkStep::Visit(ref node) if *node == node_id("step-002")
    ));
    assert_eq!(
        walk.next(&graph, config).expect("terminal"),
        WalkStep::Terminal(WalkTerminal::Completed)
    );
}

#[test]
fn a_paused_walk_without_a_successor_completes_on_resume() {
    // A wait node that is the final node has no successor: resuming
    // finds no pending work and the walk completes.
    let mut nodes = BTreeMap::new();
    nodes.insert(node_id("step-001"), step(Some("wait")));
    nodes.insert(node_id("wait"), {
        WorkflowIrNode::Wait(WaitNode {
            wait_for: WaitFor::Trigger(codex_workflow_contracts::TriggerClass::User),
            next: None,
        })
    });
    let graph = ir("step-001", nodes);

    let mut walk = Walk::new(&graph).expect("walk");
    let config = WalkConfig::default();
    assert!(matches!(
        walk.next(&graph, config).expect("step"),
        WalkStep::Visit(ref node) if *node == node_id("step-001")
    ));
    let WalkStep::Terminal(WalkTerminal::Paused { .. }) = walk.next(&graph, config).expect("pause")
    else {
        panic!("expected the walk to pause at the wait node");
    };
    pretty_assert_eq!(walk.position().current, None);
    assert_eq!(
        walk.next(&graph, config).expect("terminal"),
        WalkStep::Terminal(WalkTerminal::Completed)
    );
}

#[test]
fn a_position_round_trips_through_a_fresh_walk() {
    // step-001 -> wait -> step-002 -> step-003: snapshot the position at
    // the pause and restore it through a fresh walk; the fresh walk
    // continues from the same pending re-entry point with the same
    // consumed budget.
    let mut nodes = BTreeMap::new();
    nodes.insert(node_id("step-001"), step(Some("wait")));
    nodes.insert(node_id("wait"), {
        WorkflowIrNode::Wait(WaitNode {
            wait_for: WaitFor::Trigger(codex_workflow_contracts::TriggerClass::User),
            next: Some(node_id("step-002")),
        })
    });
    nodes.insert(node_id("step-002"), step(Some("step-003")));
    nodes.insert(node_id("step-003"), step(None));
    let graph = ir("step-001", nodes);

    let mut walk = Walk::new(&graph).expect("walk");
    let config = WalkConfig { max_steps: 3 };
    assert!(matches!(
        walk.next(&graph, config).expect("step"),
        WalkStep::Visit(ref node) if *node == node_id("step-001")
    ));
    let WalkStep::Terminal(WalkTerminal::Paused { .. }) = walk.next(&graph, config).expect("pause")
    else {
        panic!("expected the walk to pause at the wait node");
    };
    let position = walk.position();
    let mut restored = Walk::resume(&graph, position.clone()).expect("restore");
    pretty_assert_eq!(restored.position(), position);
    assert!(matches!(
        restored.next(&graph, config).expect("resume step"),
        WalkStep::Visit(ref node) if *node == node_id("step-002")
    ));
    assert!(
        restored.next(&graph, config).is_err(),
        "the restored walk inherits the consumed budget, not a fresh one"
    );
}
