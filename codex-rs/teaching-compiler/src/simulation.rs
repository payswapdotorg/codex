//! Deterministic structural replay of a workflow IR.
//!
//! Simulation is a conservative, environment-neutral replay with a fixed
//! policy for everything the control plane owns:
//!
//! - conditions are opaque (control-plane authority), so a conditional
//!   branch replays its declared default arm, or its first arm when no
//!   default exists;
//! - parallel forks are replayed sequentially in branch order (the join
//!   may therefore be visited once per incoming branch);
//! - loops enter the body exactly once and then follow the loop exit —
//!   termination is guaranteed semantically by the declared `LoopBound`;
//! - wait and human-gate nodes pause the replay, which is a valid
//!   terminal state for publication;
//! - compensation handlers run only during recovery and are never entered
//!   on the forward path.
//!
//! Simulation is **not** an execution engine: it performs a bounded
//! structural walk over frozen contract types with no I/O, no capability
//! resolution, and no side effects. Exceeding the step budget aborts the
//! run — a non-terminating graph can never support publication.

use codex_workflow_contracts::IrNodeId;
use codex_workflow_contracts::WorkflowIr;
use codex_workflow_contracts::WorkflowIrNode;
use serde::Deserialize;
use serde::Serialize;

use crate::error::TeachingCompilerError;

/// Default step budget for one simulation.
pub const DEFAULT_STEP_BUDGET: u64 = 10_000;

/// Configuration for one simulation run.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SimulationConfig {
    /// Maximum nodes the simulation may visit before aborting.
    pub max_steps: u64,
}

impl Default for SimulationConfig {
    fn default() -> Self {
        Self {
            max_steps: DEFAULT_STEP_BUDGET,
        }
    }
}

/// How the simulation chose among conditional arms.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum BranchPolicy {
    /// Simulated the declared default arm.
    DefaultArm,
    /// No default arm; simulated the first declared arm.
    FirstArm,
}

/// Terminal outcome of a simulation.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub enum SimulationOutcome {
    /// The forward path reached a terminal state with no pending work.
    Completed,
    /// The path paused at a node waiting on an external event or human
    /// decision — a valid terminal state for publication.
    Paused {
        /// The node the simulation paused at.
        at: IrNodeId,
    },
    /// The path reached a point with no deterministic continuation.
    Indeterminate {
        /// The node where determinism ended.
        at: IrNodeId,
        /// Why the path is indeterminate.
        reason: String,
    },
    /// The simulation aborted before reaching a terminal state.
    Aborted {
        /// Why the simulation aborted.
        reason: String,
    },
}

/// Result of one deterministic simulation run.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SimulationReport {
    outcome: SimulationOutcome,
    path: Vec<IrNodeId>,
    steps_taken: u64,
    branch_policy: Option<BranchPolicy>,
    epoch: u64,
}

impl SimulationReport {
    /// The terminal outcome.
    pub fn outcome(&self) -> &SimulationOutcome {
        &self.outcome
    }

    /// The visited node path, in order.
    pub fn path(&self) -> &[IrNodeId] {
        &self.path
    }

    /// How many nodes were visited.
    pub fn steps_taken(&self) -> u64 {
        self.steps_taken
    }

    /// The conditional-arm policy used, if a branch was simulated.
    pub fn branch_policy(&self) -> Option<BranchPolicy> {
        self.branch_policy
    }

    /// The candidate content epoch the simulation ran against.
    pub fn epoch(&self) -> u64 {
        self.epoch
    }

    /// Whether the outcome allows publication (Completed or Paused).
    pub fn allows_publication(&self) -> bool {
        matches!(
            self.outcome,
            SimulationOutcome::Completed | SimulationOutcome::Paused { .. }
        )
    }
}

/// Runs a deterministic structural simulation of a standalone IR at epoch
/// zero. For publication gating use `WorkflowCandidate::simulate`, which
/// binds the report to the candidate's content epoch.
pub fn simulate_ir(
    ir: &WorkflowIr,
    config: SimulationConfig,
) -> Result<SimulationReport, TeachingCompilerError> {
    simulate_with_epoch(ir, config, 0)
}

/// Runs the simulation bound to a candidate content epoch.
pub(crate) fn simulate_with_epoch(
    ir: &WorkflowIr,
    config: SimulationConfig,
    epoch: u64,
) -> Result<SimulationReport, TeachingCompilerError> {
    ir.validate()?;
    let mut path: Vec<IrNodeId> = Vec::new();
    let mut steps_taken: u64 = 0;
    let mut branch_policy: Option<BranchPolicy> = None;
    // Continuation frames: remaining children of a sequence or fork,
    // replayed in order after the current chain ends.
    let mut continuations: Vec<Vec<IrNodeId>> = Vec::new();
    let mut current: Option<IrNodeId> = Some(ir.entry.clone());

    let outcome = loop {
        let node_id = match current.take() {
            Some(node_id) => node_id,
            None => match continuations.last_mut() {
                Some(frame) if !frame.is_empty() => frame.remove(0),
                Some(_) => {
                    continuations.pop();
                    continue;
                }
                None => break SimulationOutcome::Completed,
            },
        };
        steps_taken += 1;
        if steps_taken > config.max_steps {
            return Err(TeachingCompilerError::SimulationStepBudgetExceeded {
                budget: config.max_steps,
            });
        }
        let node = ir
            .nodes
            .get(&node_id)
            .ok_or_else(|| TeachingCompilerError::InvalidNodeId {
                node_id: node_id.to_string(),
                reason: "simulation encountered a node id that is not in the graph".to_string(),
            })?;
        path.push(node_id.clone());
        match node {
            WorkflowIrNode::Step(step) => {
                current = step.next.clone();
            }
            WorkflowIrNode::Sequence(sequence) => {
                let mut children = sequence.steps.clone();
                current = if children.is_empty() {
                    None
                } else {
                    Some(children.remove(0))
                };
                if !children.is_empty() {
                    continuations.push(children);
                }
            }
            WorkflowIrNode::ParallelFork(fork) => {
                let mut branches = fork.branches.clone();
                current = if branches.is_empty() {
                    None
                } else {
                    Some(branches.remove(0))
                };
                if !branches.is_empty() {
                    continuations.push(branches);
                }
            }
            WorkflowIrNode::ParallelJoin(join) => {
                current = join.next.clone();
            }
            WorkflowIrNode::ConditionalBranch(branch) => {
                // Conditions are control-plane authority; use the fixed
                // deterministic policy documented on this module.
                if let Some(default) = branch.default.clone() {
                    if branch_policy.is_none() {
                        branch_policy = Some(BranchPolicy::DefaultArm);
                    }
                    current = Some(default);
                } else if let Some(arm) = branch.arms.first() {
                    if branch_policy.is_none() {
                        branch_policy = Some(BranchPolicy::FirstArm);
                    }
                    current = Some(arm.target.clone());
                } else {
                    break SimulationOutcome::Indeterminate {
                        at: node_id,
                        reason: "conditional branch has no arms and no default".to_string(),
                    };
                }
            }
            WorkflowIrNode::Loop(_loop) => {
                // Bounded by LoopBound in the definition; the replay enters
                // the body exactly once, then follows the loop exit.
                current = Some(_loop.body.clone());
                if let Some(next) = _loop.next.clone() {
                    continuations.push(vec![next]);
                }
            }
            WorkflowIrNode::Wait(_) => {
                // An external trigger or condition ends the replay.
                break SimulationOutcome::Paused { at: node_id };
            }
            WorkflowIrNode::HumanGate(_) => {
                // A human decision ends the replay.
                break SimulationOutcome::Paused { at: node_id };
            }
            WorkflowIrNode::Subworkflow(subworkflow) => {
                current = subworkflow.next.clone();
            }
            WorkflowIrNode::Compensation(_) => {
                // Compensation handlers run only during recovery, never on
                // the forward path.
                current = None;
            }
        }
    };

    Ok(SimulationReport {
        outcome,
        path,
        steps_taken,
        branch_policy,
        epoch,
    })
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::BranchPolicy;
    use super::SimulationConfig;
    use super::SimulationOutcome;
    use super::simulate_ir;
    use codex_workflow_contracts::ConditionId;
    use codex_workflow_contracts::ConditionalBranchArm;
    use codex_workflow_contracts::ConditionalBranchNode;
    use codex_workflow_contracts::IR_FORMAT_VERSION;
    use codex_workflow_contracts::IrNodeId;
    use codex_workflow_contracts::JoinPolicy;
    use codex_workflow_contracts::LoopBound;
    use codex_workflow_contracts::LoopNode;
    use codex_workflow_contracts::ParallelForkNode;
    use codex_workflow_contracts::ParallelJoinNode;
    use codex_workflow_contracts::StepNode;
    use codex_workflow_contracts::WaitFor;
    use codex_workflow_contracts::WaitNode;
    use codex_workflow_contracts::WorkflowCondition;
    use codex_workflow_contracts::WorkflowIr;
    use codex_workflow_contracts::WorkflowIrNode;

    use crate::error::TeachingCompilerError;

    fn node_id(value: &str) -> IrNodeId {
        IrNodeId::parse(value).expect("node id")
    }

    fn step(next: Option<&str>) -> WorkflowIrNode {
        WorkflowIrNode::Step(StepNode {
            capabilities: Vec::new(),
            roles: Vec::new(),
            description: None,
            next: next.map(|next| node_id(next)),
        })
    }

    fn ir(entry: &str, nodes: BTreeMap<IrNodeId, WorkflowIrNode>) -> WorkflowIr {
        ir_full(entry, nodes, BTreeMap::new())
    }

    fn ir_full(
        entry: &str,
        nodes: BTreeMap<IrNodeId, WorkflowIrNode>,
        conditions: BTreeMap<ConditionId, WorkflowCondition>,
    ) -> WorkflowIr {
        WorkflowIr {
            ir_format: IR_FORMAT_VERSION,
            entry: node_id(entry),
            nodes,
            conditions,
        }
    }

    fn declared_condition(name: &str) -> (ConditionId, BTreeMap<ConditionId, WorkflowCondition>) {
        let condition = ConditionId::parse(name).expect("condition id");
        let mut conditions = BTreeMap::new();
        conditions.insert(
            condition.clone(),
            WorkflowCondition {
                condition: condition.clone(),
                description: "declared for tests".to_string(),
            },
        );
        (condition, conditions)
    }

    #[test]
    fn linear_graph_completes_in_order() {
        let mut nodes = BTreeMap::new();
        nodes.insert(node_id("step-001"), step(Some("step-002")));
        nodes.insert(node_id("step-002"), step(None));
        let report =
            simulate_ir(&ir("step-001", nodes), SimulationConfig::default()).expect("simulate");
        assert_eq!(*report.outcome(), SimulationOutcome::Completed);
        assert_eq!(report.path(), &[node_id("step-001"), node_id("step-002")]);
        assert!(report.allows_publication());
    }

    #[test]
    fn wait_pauses_as_a_valid_terminal_state() {
        let (condition, conditions) = declared_condition("cond-deploy");
        let mut nodes = BTreeMap::new();
        nodes.insert(node_id("step-001"), step(Some("wait-deploy")));
        nodes.insert(
            node_id("wait-deploy"),
            WorkflowIrNode::Wait(WaitNode {
                wait_for: WaitFor::Condition(condition),
                next: None,
            }),
        );
        let report = simulate_ir(
            &ir_full("step-001", nodes, conditions),
            SimulationConfig::default(),
        )
        .expect("simulate");
        match report.outcome() {
            SimulationOutcome::Paused { at } => assert_eq!(at, &node_id("wait-deploy")),
            other => panic!("unexpected outcome {other:?}"),
        }
        assert!(report.allows_publication());
    }

    #[test]
    fn conditional_branch_follows_the_declared_default_arm() {
        let (condition, conditions) = declared_condition("cond-green");
        let mut nodes = BTreeMap::new();
        nodes.insert(
            node_id("branch"),
            WorkflowIrNode::ConditionalBranch(ConditionalBranchNode {
                arms: vec![ConditionalBranchArm {
                    condition: condition.clone(),
                    target: node_id("step-fast-path"),
                }],
                default: Some(node_id("step-safe-path")),
            }),
        );
        nodes.insert(node_id("step-fast-path"), step(None));
        nodes.insert(node_id("step-safe-path"), step(None));
        let report = simulate_ir(
            &ir_full("branch", nodes, conditions),
            SimulationConfig::default(),
        )
        .expect("simulate");
        assert_eq!(*report.outcome(), SimulationOutcome::Completed);
        assert_eq!(report.branch_policy(), Some(BranchPolicy::DefaultArm));
        assert!(report.path().contains(&node_id("step-safe-path")));
        assert!(!report.path().contains(&node_id("step-fast-path")));
    }

    #[test]
    fn branch_without_default_or_arms_is_indeterminate() {
        let mut nodes = BTreeMap::new();
        nodes.insert(
            node_id("branch"),
            WorkflowIrNode::ConditionalBranch(ConditionalBranchNode {
                arms: Vec::new(),
                default: None,
            }),
        );
        let report =
            simulate_ir(&ir("branch", nodes), SimulationConfig::default()).expect("simulate");
        match report.outcome() {
            SimulationOutcome::Indeterminate { at, .. } => assert_eq!(at, &node_id("branch")),
            other => panic!("unexpected outcome {other:?}"),
        }
        assert!(!report.allows_publication());
    }

    #[test]
    fn cycles_are_caught_by_the_step_budget() {
        let mut nodes = BTreeMap::new();
        nodes.insert(node_id("step-001"), step(Some("step-002")));
        nodes.insert(node_id("step-002"), step(Some("step-001")));
        let result = simulate_ir(&ir("step-001", nodes), SimulationConfig { max_steps: 8 });
        assert!(matches!(
            result,
            Err(TeachingCompilerError::SimulationStepBudgetExceeded { budget: 8 })
        ));
    }

    #[test]
    fn loop_enters_body_once_then_exits() {
        let (condition, conditions) = declared_condition("cond-loop");
        let mut nodes = BTreeMap::new();
        nodes.insert(
            node_id("loop"),
            WorkflowIrNode::Loop(LoopNode {
                condition,
                body: node_id("step-body"),
                bound: LoopBound { max_iterations: 5 },
                next: Some(node_id("step-after")),
            }),
        );
        nodes.insert(node_id("step-body"), step(None));
        nodes.insert(node_id("step-after"), step(None));
        let report = simulate_ir(
            &ir_full("loop", nodes, conditions),
            SimulationConfig::default(),
        )
        .expect("simulate");
        assert_eq!(*report.outcome(), SimulationOutcome::Completed);
        assert_eq!(
            report.path(),
            &[node_id("loop"), node_id("step-body"), node_id("step-after")]
        );
    }

    #[test]
    fn fork_is_replayed_sequentially_through_the_join() {
        let mut nodes = BTreeMap::new();
        nodes.insert(
            node_id("fork"),
            WorkflowIrNode::ParallelFork(ParallelForkNode {
                branches: vec![node_id("step-a"), node_id("step-b")],
            }),
        );
        nodes.insert(node_id("step-a"), step(Some("join")));
        nodes.insert(node_id("step-b"), step(Some("join")));
        nodes.insert(
            node_id("join"),
            WorkflowIrNode::ParallelJoin(ParallelJoinNode {
                policy: JoinPolicy::All,
                next: Some(node_id("step-end")),
            }),
        );
        nodes.insert(node_id("step-end"), step(None));
        let report =
            simulate_ir(&ir("fork", nodes), SimulationConfig::default()).expect("simulate");
        assert_eq!(*report.outcome(), SimulationOutcome::Completed);
        assert_eq!(
            report.path(),
            &[
                node_id("fork"),
                node_id("step-a"),
                node_id("join"),
                node_id("step-end"),
                node_id("step-b"),
                node_id("join"),
                node_id("step-end"),
            ]
        );
        assert_eq!(report.steps_taken(), 7);
    }
}
