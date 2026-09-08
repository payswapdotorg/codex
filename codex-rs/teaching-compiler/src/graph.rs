//! Structural helpers over the workflow IR.
//!
//! The contracts crate intentionally keeps graph analysis minimal; the
//! compiler needs deterministic successor, reachability, and reference
//! rewrite traversals for validation, simulation, and optimization. These
//! helpers read and write only public contract fields and add no
//! semantics of their own.

use std::collections::BTreeMap;
use std::collections::BTreeSet;

use codex_workflow_contracts::IrNodeId;
use codex_workflow_contracts::WorkflowIrNode;

/// The node ids a node points at, in deterministic order.
pub(crate) fn successors(node: &WorkflowIrNode) -> Vec<IrNodeId> {
    match node {
        WorkflowIrNode::Step(step) => optional(step.next.as_ref()),
        WorkflowIrNode::Sequence(sequence) => sequence.steps.to_vec(),
        WorkflowIrNode::ParallelFork(fork) => fork.branches.to_vec(),
        WorkflowIrNode::ParallelJoin(join) => optional(join.next.as_ref()),
        WorkflowIrNode::ConditionalBranch(branch) => {
            let mut references: Vec<IrNodeId> =
                branch.arms.iter().map(|arm| arm.target.clone()).collect();
            if let Some(default) = branch.default.as_ref() {
                references.push(default.clone());
            }
            references
        }
        WorkflowIrNode::Loop(_loop) => {
            let mut references = vec![_loop.body.clone()];
            if let Some(next) = _loop.next.as_ref() {
                references.push(next.clone());
            }
            references
        }
        WorkflowIrNode::Wait(wait) => optional(wait.next.as_ref()),
        WorkflowIrNode::HumanGate(gate) => optional(gate.next.as_ref()),
        WorkflowIrNode::Subworkflow(subworkflow) => optional(subworkflow.next.as_ref()),
        WorkflowIrNode::Compensation(compensation) => {
            vec![compensation.target.clone(), compensation.handler.clone()]
        }
    }
}

/// Builds a single optional successor.
fn optional(target: Option<&IrNodeId>) -> Vec<IrNodeId> {
    target
        .map(|target| vec![target.clone()])
        .unwrap_or_default()
}

/// All node ids reachable from `entry`, including `entry` itself.
pub(crate) fn reachable_from(
    entry: &IrNodeId,
    nodes: &BTreeMap<IrNodeId, WorkflowIrNode>,
) -> BTreeSet<IrNodeId> {
    let mut visited = BTreeSet::new();
    let mut frontier = vec![entry.clone()];
    while let Some(node_id) = frontier.pop() {
        if !visited.insert(node_id.clone()) {
            continue;
        }
        if let Some(node) = nodes.get(&node_id) {
            for successor in successors(node) {
                if !visited.contains(&successor) {
                    frontier.push(successor);
                }
            }
        }
    }
    visited
}

/// Rewrites every reference from `from` to `to` in the node map.
pub(crate) fn rewrite_reference(
    nodes: &mut BTreeMap<IrNodeId, WorkflowIrNode>,
    from: &IrNodeId,
    to: &IrNodeId,
) {
    for node in nodes.values_mut() {
        rewrite_node(node, from, to);
    }
}

/// Rewrites one node's references.
fn rewrite_node(node: &mut WorkflowIrNode, from: &IrNodeId, to: &IrNodeId) {
    match node {
        WorkflowIrNode::Step(step) => rewrite_optional(&mut step.next, from, to),
        WorkflowIrNode::Sequence(sequence) => {
            for step in &mut sequence.steps {
                rewrite_id(step, from, to);
            }
        }
        WorkflowIrNode::ParallelFork(fork) => {
            for branch in &mut fork.branches {
                rewrite_id(branch, from, to);
            }
        }
        WorkflowIrNode::ParallelJoin(join) => rewrite_optional(&mut join.next, from, to),
        WorkflowIrNode::ConditionalBranch(branch) => {
            for arm in &mut branch.arms {
                rewrite_id(&mut arm.target, from, to);
            }
            rewrite_optional(&mut branch.default, from, to);
        }
        WorkflowIrNode::Loop(_loop) => {
            rewrite_id(&mut _loop.body, from, to);
            rewrite_optional(&mut _loop.next, from, to);
        }
        WorkflowIrNode::Wait(wait) => rewrite_optional(&mut wait.next, from, to),
        WorkflowIrNode::HumanGate(gate) => rewrite_optional(&mut gate.next, from, to),
        WorkflowIrNode::Subworkflow(subworkflow) => {
            rewrite_optional(&mut subworkflow.next, from, to)
        }
        WorkflowIrNode::Compensation(compensation) => {
            rewrite_id(&mut compensation.target, from, to);
            rewrite_id(&mut compensation.handler, from, to);
        }
    }
}

/// Rewrites one required reference.
fn rewrite_id(id: &mut IrNodeId, from: &IrNodeId, to: &IrNodeId) {
    if id == from {
        *id = to.clone();
    }
}

/// Rewrites one optional reference.
fn rewrite_optional(id: &mut Option<IrNodeId>, from: &IrNodeId, to: &IrNodeId) {
    if let Some(id) = id {
        rewrite_id(id, from, to);
    }
}
