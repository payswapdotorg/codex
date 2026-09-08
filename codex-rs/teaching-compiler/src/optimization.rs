//! Deterministic, provably equivalence-preserving optimizations.
//!
//! Per the work order, an optimization of observed steps is permitted only
//! when semantic equivalence, policy, evidence, and resource constraints
//! are all satisfied. The compiler restricts itself to transforms whose
//! semantic equivalence is structural and provable on the IR itself:
//!
//! - **RemoveUnreachableNodes** — deleting nodes that cannot be reached
//!   from the entry changes no execution.
//! - **CollapseSingleChildSequence** — a sequence with exactly one child
//!   executes identically to that child; every reference to the sequence
//!   is redirected to the child, and the child's step content (intent,
//!   capabilities, roles, ordering) is preserved verbatim.
//!
//! The additional gates enforced here:
//!
//! - **policy** — the candidate's session policy must allow optimization;
//! - **evidence** — any observed step touched by a collapse must carry at
//!   least one evidence reference;
//! - **resource constraints** — the transforms never alter capability,
//!   role, or ordering content, which is guaranteed by construction and
//!   re-checked by contract revalidation after the transform.
//!
//! Applying an optimization changes candidate content: the approval is
//! cleared, the content epoch advances, and the candidate returns to
//! `Compiled` until revalidation passes again.

use codex_workflow_contracts::IrNodeId;
use codex_workflow_contracts::WorkflowIrNode;
use serde::Deserialize;
use serde::Serialize;

use crate::blueprint::StepOrigin;
use crate::candidate::WorkflowCandidate;
use crate::error::TeachingCompilerError;
use crate::graph;

/// The class of a deterministic optimization.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum OptimizationKind {
    /// Deletes nodes unreachable from the entry.
    RemoveUnreachableNodes,
    /// Replaces a single-child sequence with its child.
    CollapseSingleChildSequence,
}

/// A proposed, not-yet-applied optimization.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OptimizationProposal {
    kind: OptimizationKind,
    description: String,
    affected_nodes: Vec<IrNodeId>,
    epoch: u64,
}

impl OptimizationProposal {
    /// The optimization class.
    pub fn kind(&self) -> OptimizationKind {
        self.kind
    }

    /// Human-facing description of the transform.
    pub fn description(&self) -> &str {
        &self.description
    }

    /// Nodes the transform touches.
    pub fn affected_nodes(&self) -> &[IrNodeId] {
        &self.affected_nodes
    }

    /// The candidate epoch the proposal was computed against.
    pub fn epoch(&self) -> u64 {
        self.epoch
    }
}

/// Computes all currently applicable optimization proposals for the
/// candidate, in deterministic order.
pub fn propose_optimizations(candidate: &WorkflowCandidate) -> Vec<OptimizationProposal> {
    let ir = candidate.ir();
    let mut proposals = Vec::new();

    let reachable = graph::reachable_from(&ir.entry, &ir.nodes);
    let unreachable: Vec<IrNodeId> = ir
        .nodes
        .keys()
        .filter(|node_id| !reachable.contains(*node_id))
        .cloned()
        .collect();
    if !unreachable.is_empty() {
        proposals.push(OptimizationProposal {
            kind: OptimizationKind::RemoveUnreachableNodes,
            description: format!(
                "remove {} node(s) that are unreachable from the entry node",
                unreachable.len()
            ),
            affected_nodes: unreachable,
            epoch: candidate.epoch(),
        });
    }

    for (node_id, node) in ir.nodes.iter() {
        let WorkflowIrNode::Sequence(sequence) = node else {
            continue;
        };
        if sequence.steps.len() == 1 {
            proposals.push(OptimizationProposal {
                kind: OptimizationKind::CollapseSingleChildSequence,
                description: format!(
                    "collapse single-child sequence `{node_id}` into its only child"
                ),
                affected_nodes: vec![node_id.clone(), sequence.steps[0].clone()],
                epoch: candidate.epoch(),
            });
        }
    }
    proposals
}

/// Applies one optimization proposal to the candidate.
///
/// Gates enforced, in order: proposal freshness (exact content epoch),
/// session policy, and evidence for observed steps touched by the
/// transform. The transform itself preserves step content verbatim, and
/// contract validation is re-run before the content epoch advances.
pub fn apply_optimization(
    candidate: &mut WorkflowCandidate,
    proposal: &OptimizationProposal,
) -> Result<(), TeachingCompilerError> {
    if proposal.epoch() != candidate.epoch() {
        return Err(TeachingCompilerError::StaleOptimizationProposal {
            proposal_epoch: proposal.epoch(),
            candidate_epoch: candidate.epoch(),
        });
    }
    if !candidate.policy_allows_optimization() {
        return Err(TeachingCompilerError::OptimizationForbidden {
            reason: "the teaching session policy does not allow optimization".to_string(),
        });
    }
    match proposal.kind() {
        OptimizationKind::RemoveUnreachableNodes => remove_unreachable(candidate),
        OptimizationKind::CollapseSingleChildSequence => {
            collapse_single_child_sequence(candidate, proposal)
        }
    }?;
    candidate.ir().validate()?;
    candidate.after_content_change();
    Ok(())
}

/// Deletes every node unreachable from the entry.
fn remove_unreachable(candidate: &mut WorkflowCandidate) -> Result<(), TeachingCompilerError> {
    let ir = candidate.ir_mut();
    let reachable = graph::reachable_from(&ir.entry, &ir.nodes);
    ir.nodes.retain(|node_id, _| reachable.contains(node_id));
    Ok(())
}

/// Replaces a single-child sequence with its child.
fn collapse_single_child_sequence(
    candidate: &mut WorkflowCandidate,
    proposal: &OptimizationProposal,
) -> Result<(), TeachingCompilerError> {
    let (sequence_id, child_id) = match proposal.affected_nodes() {
        [sequence_id, child_id] => (sequence_id.clone(), child_id.clone()),
        _ => {
            return Err(TeachingCompilerError::OptimizationForbidden {
                reason: "collapse proposal must name exactly one sequence and its child"
                    .to_string(),
            });
        }
    };
    // Evidence gate: an observed step touched by the collapse must carry
    // evidence.
    if candidate.node_origin(&child_id) == Some(StepOrigin::Observed) {
        let has_evidence = match candidate.node_evidence(&child_id) {
            Some(evidence) => !evidence.is_empty(),
            None => false,
        };
        if !has_evidence {
            return Err(TeachingCompilerError::OptimizationForbidden {
                reason: format!(
                    "observed step `{child_id}` carries no evidence; optimization requires \
                     evidence for observed steps"
                ),
            });
        }
    }
    let ir = candidate.ir_mut();
    if !ir.nodes.contains_key(&child_id) {
        return Err(TeachingCompilerError::InvalidNodeId {
            node_id: child_id.to_string(),
            reason: "collapse target child does not exist in the graph".to_string(),
        });
    }
    let valid = match ir.nodes.get(&sequence_id) {
        Some(WorkflowIrNode::Sequence(sequence)) => {
            sequence.steps.len() == 1 && sequence.steps[0] == child_id
        }
        _ => false,
    };
    if !valid {
        return Err(TeachingCompilerError::OptimizationForbidden {
            reason: format!("`{sequence_id}` is not a single-child sequence naming `{child_id}`"),
        });
    }
    graph::rewrite_reference(&mut ir.nodes, &sequence_id, &child_id);
    if ir.entry == sequence_id {
        ir.entry = child_id;
    }
    ir.nodes.remove(&sequence_id);
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::OptimizationKind;
    use super::apply_optimization;
    use super::propose_optimizations;
    use codex_workflow_contracts::IR_FORMAT_VERSION;
    use codex_workflow_contracts::IrNodeId;
    use codex_workflow_contracts::SequenceNode;
    use codex_workflow_contracts::StepNode;
    use codex_workflow_contracts::WorkflowIr;
    use codex_workflow_contracts::WorkflowIrNode;

    use crate::approval::ApprovalDecision;
    use crate::approval::ApprovalDecisionKind;
    use crate::candidate::CandidateStatus;
    use crate::candidate::WorkflowCandidate;
    use crate::error::TeachingCompilerError;
    use crate::session::DEFAULT_MAX_RECORDS;
    use crate::session::SessionPolicy;
    use crate::simulation::SimulationConfig;

    fn node_id(value: &str) -> IrNodeId {
        IrNodeId::parse(value).expect("node id")
    }

    fn step(next: Option<&str>) -> WorkflowIrNode {
        WorkflowIrNode::Step(StepNode {
            capabilities: Vec::new(),
            roles: Vec::new(),
            description: Some("authored step".to_string()),
            next: next.map(|next| node_id(next)),
        })
    }

    fn graph_with_single_child_sequence() -> WorkflowIr {
        let mut nodes = BTreeMap::new();
        nodes.insert(node_id("step-a"), step(Some("step-b")));
        nodes.insert(node_id("step-b"), step(None));
        nodes.insert(
            node_id("sequence-root"),
            WorkflowIrNode::Sequence(SequenceNode {
                steps: vec![node_id("step-a")],
            }),
        );
        WorkflowIr {
            ir_format: IR_FORMAT_VERSION,
            entry: node_id("sequence-root"),
            nodes,
            conditions: BTreeMap::new(),
        }
    }

    fn graph_with_unreachable() -> WorkflowIr {
        let mut nodes = BTreeMap::new();
        nodes.insert(node_id("step-a"), step(None));
        nodes.insert(node_id("step-orphan"), step(None));
        WorkflowIr {
            ir_format: IR_FORMAT_VERSION,
            entry: node_id("step-a"),
            nodes,
            conditions: BTreeMap::new(),
        }
    }

    fn allowed_policy() -> SessionPolicy {
        SessionPolicy {
            allow_optimization: true,
            max_records: DEFAULT_MAX_RECORDS,
        }
    }

    #[test]
    fn default_policy_forbids_optimization() {
        let mut candidate =
            WorkflowCandidate::from_ir(graph_with_unreachable(), None).expect("candidate");
        let proposals = propose_optimizations(&candidate);
        assert_eq!(proposals.len(), 1);
        assert_eq!(
            proposals[0].kind(),
            OptimizationKind::RemoveUnreachableNodes
        );
        let result = apply_optimization(&mut candidate, &proposals[0]);
        assert!(matches!(
            result,
            Err(TeachingCompilerError::OptimizationForbidden { .. })
        ));
    }

    #[test]
    fn removes_unreachable_nodes_deterministically() {
        let mut candidate =
            WorkflowCandidate::from_ir(graph_with_unreachable(), None).expect("candidate");
        candidate.set_policy(allowed_policy());
        let proposals = propose_optimizations(&candidate);
        assert_eq!(proposals.len(), 1);
        apply_optimization(&mut candidate, &proposals[0]).expect("apply");
        assert_eq!(candidate.ir().nodes.len(), 1);
        assert!(candidate.ir().nodes.contains_key(&node_id("step-a")));
        assert_eq!(candidate.status(), CandidateStatus::Compiled);
        let summary = candidate.validate().expect("validate");
        assert!(summary.is_clean());
        assert_eq!(candidate.status(), CandidateStatus::Validated);
    }

    #[test]
    fn collapses_single_child_sequences_preserving_step_content() {
        let mut candidate = WorkflowCandidate::from_ir(graph_with_single_child_sequence(), None)
            .expect("candidate");
        candidate.set_policy(allowed_policy());
        let proposals = propose_optimizations(&candidate);
        assert_eq!(proposals.len(), 1);
        assert_eq!(
            proposals[0].kind(),
            OptimizationKind::CollapseSingleChildSequence
        );
        apply_optimization(&mut candidate, &proposals[0]).expect("apply");
        assert_eq!(candidate.ir().entry, node_id("step-a"));
        assert_eq!(candidate.ir().nodes.len(), 2);
        assert!(!candidate.ir().nodes.contains_key(&node_id("sequence-root")));
        candidate.validate().expect("validate");
        let report = candidate
            .simulate(SimulationConfig::default())
            .expect("simulate");
        assert_eq!(report.path().len(), 2);
    }

    #[test]
    fn stale_proposals_are_rejected() {
        let mut candidate =
            WorkflowCandidate::from_ir(graph_with_unreachable(), None).expect("candidate");
        candidate.set_policy(allowed_policy());
        let proposals = propose_optimizations(&candidate);
        assert_eq!(proposals[0].epoch(), 1);
        candidate.set_policy(allowed_policy());
        assert_eq!(candidate.epoch(), 2);
        assert!(matches!(
            apply_optimization(&mut candidate, &proposals[0]),
            Err(TeachingCompilerError::StaleOptimizationProposal { .. })
        ));
    }

    #[test]
    fn teaching_output_has_nothing_to_optimize() {
        let mut session =
            crate::session::TeachingSession::new(crate::mode::TeachingMode::Demonstrate);
        session
            .record(
                crate::trajectory::RecordOrigin::Demonstration,
                crate::trajectory::TrajectoryEvent::Action {
                    text: "Open the list.".to_string(),
                },
                Vec::new(),
            )
            .expect("record");
        session
            .record(
                crate::trajectory::RecordOrigin::Demonstration,
                crate::trajectory::TrajectoryEvent::Action {
                    text: "Filter the list.".to_string(),
                },
                Vec::new(),
            )
            .expect("record");
        session.close();
        let candidate = crate::compiler::compile(&session).expect("compile");
        assert!(propose_optimizations(&candidate).is_empty());
    }

    #[test]
    fn optimization_after_approval_clears_the_approval() {
        let mut candidate = WorkflowCandidate::from_ir(graph_with_single_child_sequence(), None)
            .expect("candidate");
        candidate.set_policy(allowed_policy());
        candidate.validate().expect("validate");
        candidate
            .simulate(SimulationConfig::default())
            .expect("simulate");
        let decision = ApprovalDecision::new("tech-lead", "review", ApprovalDecisionKind::Approved)
            .expect("decision");
        candidate.approve(decision).expect("approve");
        let proposals = propose_optimizations(&candidate);
        assert_eq!(proposals.len(), 1);
        apply_optimization(&mut candidate, &proposals[0]).expect("apply");
        assert_eq!(candidate.status(), CandidateStatus::Compiled);
        assert!(candidate.approval().is_none());
    }
}
