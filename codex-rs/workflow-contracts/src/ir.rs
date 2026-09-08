//! The workflow intermediate representation (IR).
//!
//! `WorkflowIr` is the versioned execution graph of a workflow. It uses the
//! composition primitives frozen by the architecture: `SEQUENCE`,
//! `PARALLEL_FORK`, `PARALLEL_JOIN`, `CONDITIONAL_BRANCH`, `LOOP`, `WAIT`,
//! `HUMAN_GATE`, `SUBWORKFLOW`, and `COMPENSATION`.
//!
//! The IR is environment- and provider-neutral:
//!
//! - steps declare **semantic capability requirements**, never concrete
//!   browsers, tools, models, or MCP servers;
//! - subworkflow invocations reference declared dependency ids, never inline
//!   repository details;
//! - branch and loop predicates are referenced by `ConditionId`; their
//!   evaluation semantics are owned by the workflow control plane
//!   (a later Work Order) and are deliberately not part of this contract.
//!
//! Every node carries a stable `IrNodeId` so evidence, instances, and
//! recovery can reference exact graph positions across versions.

use std::collections::BTreeMap;
use std::fmt;

use serde::Deserialize;
use serde::Serialize;

use crate::CapabilityRequirement;
use crate::RoleId;
use crate::SubworkflowDependencyId;
use crate::TriggerClass;
use crate::WorkflowContractError;

/// Current format version of the IR serialization.
pub const IR_FORMAT_VERSION: u32 = 1;

/// Stable identity of one node within a workflow IR.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct IrNodeId(String);

/// Reference to a condition declaration within a workflow IR.
///
/// Conditions are declared alongside the graph. Their evaluation semantics
/// are control-plane authority and are intentionally opaque here; the
/// declaration carries a human-facing description only.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct ConditionId(String);

/// A condition declaration referenced by branch and loop nodes.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkflowCondition {
    /// The condition's local identity.
    pub condition: ConditionId,
    /// Human-facing description of when the condition holds.
    pub description: String,
}

/// The workflow intermediate representation.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkflowIr {
    /// IR serialization format version.
    pub ir_format: u32,
    /// The graph entry point.
    pub entry: IrNodeId,
    /// All nodes, keyed by node id.
    pub nodes: BTreeMap<IrNodeId, WorkflowIrNode>,
    /// Condition declarations referenced by branch and loop nodes.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub conditions: BTreeMap<ConditionId, WorkflowCondition>,
}

/// One node of the workflow IR.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "node", rename_all = "camelCase")]
pub enum WorkflowIrNode {
    /// Executes one semantic step.
    Step(StepNode),
    /// Executes child nodes in order.
    Sequence(SequenceNode),
    /// Starts parallel branches.
    ParallelFork(ParallelForkNode),
    /// Joins parallel branches.
    ParallelJoin(ParallelJoinNode),
    /// Selects a branch by condition.
    ConditionalBranch(ConditionalBranchNode),
    /// Repeats a body while a condition holds.
    Loop(LoopNode),
    /// Waits for a condition or trigger class.
    Wait(WaitNode),
    /// Pauses for a human decision.
    HumanGate(HumanGateNode),
    /// Invokes a pinned subworkflow.
    Subworkflow(SubworkflowNode),
    /// Compensates a target node with a handler node.
    Compensation(CompensationNode),
}

/// A semantic execution step.
///
/// Steps name capabilities, not implementations: the runtime binds each
/// capability to a compatible Codex capability at execution time.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StepNode {
    /// Capabilities the step requires.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub capabilities: Vec<CapabilityRequirement>,
    /// Roles that may execute or supervise this step.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub roles: Vec<RoleId>,
    /// Human-facing description of the step's intent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Node to execute after this step completes successfully.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next: Option<IrNodeId>,
}

/// Executes children in order.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SequenceNode {
    /// Children in execution order.
    pub steps: Vec<IrNodeId>,
}

/// Starts parallel branches.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ParallelForkNode {
    /// Branches that run concurrently.
    pub branches: Vec<IrNodeId>,
}

/// Join policy for parallel branches.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum JoinPolicy {
    /// Waits for every incoming branch.
    All,
    /// Continues when any incoming branch completes.
    Any,
    /// Continues when at least `branches` incoming branches complete.
    AtLeast {
        /// Minimum number of completed incoming branches.
        branches: u32,
    },
}

/// Joins parallel branches.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ParallelJoinNode {
    /// How completion of incoming branches is decided.
    pub policy: JoinPolicy,
    /// Node to execute after the join completes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next: Option<IrNodeId>,
}

/// One guarded arm of a conditional branch.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ConditionalBranchArm {
    /// The condition guarding this arm.
    pub condition: ConditionId,
    /// The arm's target node.
    pub target: IrNodeId,
}

/// Selects an arm by condition.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ConditionalBranchNode {
    /// Arms evaluated in order.
    pub arms: Vec<ConditionalBranchArm>,
    /// Arm taken when no condition holds, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<IrNodeId>,
}

/// Bound on loop iterations, part of the versioned definition.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LoopBound {
    /// Maximum number of iterations; a loop without a bound is invalid.
    pub max_iterations: u32,
}

/// Repeats a body while a condition holds.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LoopNode {
    /// The condition evaluated before each iteration.
    pub condition: ConditionId,
    /// The loop body.
    pub body: IrNodeId,
    /// Iteration bound.
    pub bound: LoopBound,
    /// Node to execute after the loop exits.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next: Option<IrNodeId>,
}

/// What a wait node waits for.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub enum WaitFor {
    /// Waits until a declared condition holds.
    Condition(ConditionId),
    /// Waits for a normalized trigger class.
    Trigger(TriggerClass),
}

/// Waits for a condition or a trigger class.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WaitNode {
    /// What the node waits for.
    pub wait_for: WaitFor,
    /// Node to execute after the wait completes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next: Option<IrNodeId>,
}

/// Pauses execution for a human decision.
///
/// Human gates are explicit execution participants: the gate records which
/// role decides and what the approver is asked. Human action is
/// evidence-bearing execution, never an authorization bypass.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HumanGateNode {
    /// Role that must decide at the gate.
    pub approver: RoleId,
    /// The question or instruction presented to the approver.
    pub instruction: String,
    /// Node to execute after the gate is passed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next: Option<IrNodeId>,
}

/// Invokes a subworkflow through its declared dependency.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SubworkflowNode {
    /// The declared subworkflow dependency this node invokes.
    pub dependency: SubworkflowDependencyId,
    /// Human-facing description of the invocation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Node to execute after the subworkflow completes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next: Option<IrNodeId>,
}

/// Compensates a target node by executing a handler node.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CompensationNode {
    /// The node being compensated.
    pub target: IrNodeId,
    /// The compensation handler.
    pub handler: IrNodeId,
}

impl WorkflowIr {
    /// Validates the graph's structural invariants:
    ///
    /// - the IR format version is supported;
    /// - the entry node exists;
    /// - every referenced node id exists;
    /// - no node points at itself;
    /// - every referenced condition is declared.
    pub fn validate(&self) -> Result<(), WorkflowContractError> {
        if self.ir_format != IR_FORMAT_VERSION {
            return Err(WorkflowContractError::MalformedIr {
                reason: format!(
                    "unsupported IR format version {} (expected {IR_FORMAT_VERSION})",
                    self.ir_format
                ),
            });
        }
        if !self.nodes.contains_key(&self.entry) {
            return Err(WorkflowContractError::MalformedIr {
                reason: format!("entry node `{}` does not exist", self.entry),
            });
        }
        for (node_id, node) in &self.nodes {
            for (field, target) in node.referenced_nodes() {
                if &target == node_id {
                    return Err(WorkflowContractError::MalformedIr {
                        reason: format!("node `{node_id}` references itself through `{field}`"),
                    });
                }
                if !self.nodes.contains_key(&target) {
                    return Err(WorkflowContractError::MalformedIr {
                        reason: format!(
                            "node `{node_id}` references missing node `{target}` through `{field}`"
                        ),
                    });
                }
            }
            for (field, condition) in node.referenced_conditions() {
                if !self.conditions.contains_key(&condition) {
                    return Err(WorkflowContractError::MalformedIr {
                        reason: format!(
                            "node `{node_id}` references missing condition `{condition}` through `{field}`"
                        ),
                    });
                }
            }
        }
        Ok(())
    }

    /// Lists every subworkflow dependency id referenced by the graph.
    ///
    /// The definition uses this to cross-check that every invocation matches
    /// a declared dependency.
    pub fn referenced_subworkflow_dependencies(&self) -> Vec<SubworkflowDependencyId> {
        let mut dependencies: Vec<SubworkflowDependencyId> = self
            .nodes
            .values()
            .filter_map(|node| match node {
                WorkflowIrNode::Subworkflow(subworkflow) => Some(subworkflow.dependency.clone()),
                _ => None,
            })
            .collect();
        dependencies.sort();
        dependencies.dedup();
        dependencies
    }
}

impl WorkflowIrNode {
    /// The node ids this node points at, with the field doing the pointing.
    fn referenced_nodes(&self) -> Vec<(&'static str, IrNodeId)> {
        match self {
            Self::Step(step) => optional_reference("next", &step.next),
            Self::Sequence(sequence) => position_references("steps", &sequence.steps),
            Self::ParallelFork(fork) => position_references("branches", &fork.branches),
            Self::ParallelJoin(join) => optional_reference("next", &join.next),
            Self::ConditionalBranch(branch) => {
                let mut references = branch
                    .arms
                    .iter()
                    .map(|arm| ("arms.target", arm.target.clone()))
                    .collect::<Vec<_>>();
                if let Some(default) = &branch.default {
                    references.push(("default", default.clone()));
                }
                references
            }
            Self::Loop(_loop) => {
                let mut references = vec![("body", _loop.body.clone())];
                if let Some(next) = &_loop.next {
                    references.push(("next", next.clone()));
                }
                references
            }
            Self::Wait(wait) => optional_reference("next", &wait.next),
            Self::HumanGate(gate) => optional_reference("next", &gate.next),
            Self::Subworkflow(subworkflow) => optional_reference("next", &subworkflow.next),
            Self::Compensation(compensation) => {
                vec![
                    ("target", compensation.target.clone()),
                    ("handler", compensation.handler.clone()),
                ]
            }
        }
    }

    /// The conditions this node references, with the referencing field.
    fn referenced_conditions(&self) -> Vec<(&'static str, ConditionId)> {
        match self {
            Self::ConditionalBranch(branch) => branch
                .arms
                .iter()
                .map(|arm| ("arms.condition", arm.condition.clone()))
                .collect(),
            Self::Loop(_loop) => vec![("condition", _loop.condition.clone())],
            Self::Wait(wait) => match &wait.wait_for {
                WaitFor::Condition(condition) => vec![("waitFor", condition.clone())],
                WaitFor::Trigger(_) => vec![],
            },
            _ => vec![],
        }
    }
}

impl IrNodeId {
    /// Parses a node identifier.
    pub fn parse(value: impl Into<String>) -> Result<Self, WorkflowContractError> {
        let value = value.into();
        if value.is_empty() {
            Err(WorkflowContractError::InvalidIdentifier {
                kind: "ir node id",
                value,
                reason: "must be non-empty",
            })
        } else {
            Ok(Self(value))
        }
    }
}

impl TryFrom<String> for IrNodeId {
    type Error = WorkflowContractError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl TryFrom<&str> for IrNodeId {
    type Error = WorkflowContractError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::parse(value.to_owned())
    }
}

impl From<IrNodeId> for String {
    fn from(value: IrNodeId) -> Self {
        value.0
    }
}

impl AsRef<str> for IrNodeId {
    fn as_ref(&self) -> &str {
        self.0.as_str()
    }
}

impl fmt::Display for IrNodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0.as_str())
    }
}

impl ConditionId {
    /// Parses a condition identifier.
    pub fn parse(value: impl Into<String>) -> Result<Self, WorkflowContractError> {
        let value = value.into();
        if value.is_empty() {
            Err(WorkflowContractError::InvalidIdentifier {
                kind: "condition id",
                value,
                reason: "must be non-empty",
            })
        } else {
            Ok(Self(value))
        }
    }
}

impl TryFrom<String> for ConditionId {
    type Error = WorkflowContractError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl TryFrom<&str> for ConditionId {
    type Error = WorkflowContractError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::parse(value.to_owned())
    }
}

impl From<ConditionId> for String {
    fn from(value: ConditionId) -> Self {
        value.0
    }
}

impl AsRef<str> for ConditionId {
    fn as_ref(&self) -> &str {
        self.0.as_str()
    }
}

impl fmt::Display for ConditionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0.as_str())
    }
}

/// Builds a single optional node reference.
fn optional_reference(
    field: &'static str,
    target: &Option<IrNodeId>,
) -> Vec<(&'static str, IrNodeId)> {
    target
        .as_ref()
        .map(|target| vec![(field, target.clone())])
        .unwrap_or_default()
}

/// Builds positional node references.
fn position_references(field: &'static str, targets: &[IrNodeId]) -> Vec<(&'static str, IrNodeId)> {
    targets
        .iter()
        .map(|target| (field, target.clone()))
        .collect()
}

#[cfg(test)]
#[path = "ir_tests.rs"]
mod tests;
