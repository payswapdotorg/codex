//! The bounded deterministic graph walk of the application run path.
//!
//! This walk is **the frozen replay policy of the teaching compiler**
//! (WO-008 `simulation.rs`), applied at runtime with real execution: the
//! application layer does not invent orchestration semantics. Conditions
//! remain control-plane authority, so the walk takes the declared default
//! arm (or the first arm when no default exists); parallel branches are
//! walked sequentially in declaration order; a loop enters its body once
//! and then follows the loop exit; wait and human-gate nodes pause the
//! run; compensation handlers never run on the forward path. The
//! equivalence is asserted by tests against
//! [`codex_teaching_compiler::simulate_ir`].
//!
//! The walk is bounded: exceeding [`WalkConfig::max_steps`] aborts instead
//! of looping forever.

use codex_workflow_contracts::IrNodeId;
use codex_workflow_contracts::WorkflowIr;
use codex_workflow_contracts::WorkflowIrNode;

use crate::WorkflowAppError;

/// The persisted position of one bounded walk.
///
/// This is the walk's control-plane state for persistence and resume:
/// the pending re-entry point, the continuation frames, the budget
/// already consumed, and the visited path. It carries no workflow
/// semantics — the graph it indexes is re-loaded and re-verified from
/// the immutable version at rehydration.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WalkPosition {
    /// The node the walk visits next, when one is pending. `None` means
    /// the walk continues from its continuation frames (or completes
    /// when none remain).
    pub current: Option<IrNodeId>,
    /// The remaining continuation frames (sequence/fork children and
    /// loop exits), innermost frame last.
    pub continuations: Vec<Vec<IrNodeId>>,
    /// The node budget already consumed.
    pub steps_taken: u64,
    /// The nodes visited so far, in order.
    pub path: Vec<IrNodeId>,
}

/// Default node budget for one run, matching the teaching compiler's
/// simulation budget.
pub const DEFAULT_WALK_BUDGET: u64 = 10_000;

/// Configuration for one walk.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WalkConfig {
    /// Maximum nodes the walk may visit before aborting.
    pub max_steps: u64,
}

impl Default for WalkConfig {
    fn default() -> Self {
        Self {
            max_steps: DEFAULT_WALK_BUDGET,
        }
    }
}

/// Why the walk stopped at a node it cannot pass deterministically.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum WalkTerminal {
    /// The forward path reached a terminal state with no pending work.
    Completed,
    /// The path paused at a node waiting on an external event or human
    /// decision — a valid terminal state for a running instance.
    Paused {
        /// The node the walk paused at.
        node: IrNodeId,
        /// Why the walk paused (`wait` or `human gate`).
        reason: &'static str,
    },
    /// The path reached a node with no deterministic continuation in this
    /// Work Order.
    Indeterminate {
        /// The node where determinism ended.
        node: IrNodeId,
        /// Why the path is indeterminate.
        reason: String,
    },
}

/// One step of walk progress: either visit a node or stop.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) enum WalkStep {
    /// Visit this node (execute it if it is a step node).
    Visit(IrNodeId),
    /// The walk is finished with this terminal outcome.
    Terminal(WalkTerminal),
}

/// The resumable walk state over one workflow IR.
///
/// Mirrors the teaching compiler's simulation loop: a `current` node plus
/// continuation frames for remaining sequence/fork children and loop
/// exits.
pub(crate) struct Walk {
    current: Option<IrNodeId>,
    continuations: Vec<Vec<IrNodeId>>,
    steps_taken: u64,
    path: Vec<IrNodeId>,
}

impl Walk {
    /// Starts a walk at the IR entry.
    pub fn new(ir: &WorkflowIr) -> Result<Self, WorkflowAppError> {
        ir.validate()?;
        Ok(Self {
            current: Some(ir.entry.clone()),
            continuations: Vec::new(),
            steps_taken: 0,
            path: Vec::new(),
        })
    }

    /// The nodes visited so far, in order.
    pub fn path(&self) -> &[IrNodeId] {
        &self.path
    }

    /// The walk's current position, for persistence at the documented
    /// checkpoints.
    pub fn position(&self) -> WalkPosition {
        WalkPosition {
            current: self.current.clone(),
            continuations: self.continuations.clone(),
            steps_taken: self.steps_taken,
            path: self.path.clone(),
        }
    }

    /// Restores a walk over `ir` from a persisted position.
    ///
    /// The IR is validated exactly like [`Walk::new`]; the restored
    /// position is control-plane state, never workflow semantics: the
    /// graph, its digest, and the version pin are untouched. The
    /// restored walk consumes its remaining budget from
    /// `position.steps_taken`, so a resumed run does not get a fresh
    /// node budget.
    pub fn resume(ir: &WorkflowIr, position: WalkPosition) -> Result<Self, WorkflowAppError> {
        ir.validate()?;
        Ok(Self {
            current: position.current,
            continuations: position.continuations,
            steps_taken: position.steps_taken,
            path: position.path,
        })
    }

    /// Produces the next walk step under `config`.
    pub fn next(
        &mut self,
        ir: &WorkflowIr,
        config: WalkConfig,
    ) -> Result<WalkStep, WorkflowAppError> {
        let node_id = match self.current.take() {
            Some(node_id) => node_id,
            None => match self.continuations.last_mut() {
                Some(frame) if !frame.is_empty() => frame.remove(0),
                Some(_) => {
                    self.continuations.pop();
                    return self.next(ir, config);
                }
                None => return Ok(WalkStep::Terminal(WalkTerminal::Completed)),
            },
        };
        self.steps_taken += 1;
        if self.steps_taken > config.max_steps {
            return Err(WorkflowAppError::Execution(
                codex_execution_contracts::ExecutionContractError::InvalidRecord {
                    reason: format!(
                        "workflow run exceeded its node budget of {}",
                        config.max_steps
                    ),
                },
            ));
        }
        let node = ir.nodes.get(&node_id).ok_or_else(|| {
            WorkflowAppError::Execution(
                codex_execution_contracts::ExecutionContractError::InvalidRecord {
                    reason: format!("walk encountered node `{node_id}` missing from the graph"),
                },
            )
        })?;
        self.path.push(node_id.clone());
        match node {
            WorkflowIrNode::Step(step) => {
                self.current = step.next.clone();
            }
            WorkflowIrNode::Sequence(sequence) => {
                let mut children = sequence.steps.clone();
                self.current = children.first().cloned();
                if children.len() > 1 {
                    children.remove(0);
                    self.continuations.push(children);
                }
            }
            WorkflowIrNode::ParallelFork(fork) => {
                let mut branches = fork.branches.clone();
                self.current = branches.first().cloned();
                if branches.len() > 1 {
                    branches.remove(0);
                    self.continuations.push(branches);
                }
            }
            WorkflowIrNode::ParallelJoin(join) => {
                self.current = join.next.clone();
            }
            WorkflowIrNode::ConditionalBranch(branch) => {
                // Conditions are control-plane authority: apply the frozen
                // deterministic policy (default arm, else first arm).
                self.current = if let Some(default) = branch.default.clone() {
                    Some(default)
                } else if let Some(arm) = branch.arms.first() {
                    Some(arm.target.clone())
                } else {
                    return Ok(WalkStep::Terminal(WalkTerminal::Indeterminate {
                        node: node_id,
                        reason: "conditional branch has no arms and no default".to_string(),
                    }));
                };
            }
            WorkflowIrNode::Loop(node) => {
                // Termination is guaranteed by the declared `LoopBound`;
                // the deterministic walk enters the body once, then follows
                // the loop exit.
                self.current = Some(node.body.clone());
                if let Some(next) = node.next.clone() {
                    self.continuations.push(vec![next]);
                }
            }
            WorkflowIrNode::Wait(node) => {
                // The pending re-entry point: a resume continues at the
                // wait node's successor (a `None` successor means the
                // wait is the final node, so a resume completes).
                self.current = node.next.clone();
                return Ok(WalkStep::Terminal(WalkTerminal::Paused {
                    node: node_id,
                    reason: "wait",
                }));
            }
            WorkflowIrNode::HumanGate(node) => {
                // The pending re-entry point, mirroring the wait node.
                self.current = node.next.clone();
                return Ok(WalkStep::Terminal(WalkTerminal::Paused {
                    node: node_id,
                    reason: "human gate",
                }));
            }
            WorkflowIrNode::Subworkflow(subworkflow) => {
                self.current = subworkflow.next.clone();
            }
            WorkflowIrNode::Compensation(_) => {
                // Compensation handlers run only during recovery, never on
                // the forward path.
                self.current = None;
            }
        }
        Ok(WalkStep::Visit(node_id))
    }
}

#[cfg(test)]
#[path = "walk_tests.rs"]
mod tests;
