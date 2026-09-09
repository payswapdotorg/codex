//! The teaching compiler entry point.
//!
//! `compile` turns a closed [`TeachingSession`] into a
//! [`WorkflowCandidate`]:
//!
//! 1. ordered step blueprints are extracted deterministically from the
//!    trajectory ([`crate::blueprint`]);
//! 2. each blueprint becomes one IR step node (`step-001`, `step-002`, …)
//!    chained through `next` pointers, all reachable from the entry step;
//! 3. evidence and extraction origins are attached per node;
//! 4. lexical capability hints and neutral binding *proposals* are emitted
//!    per step (the compiler never writes capability requirements itself);
//! 5. instructions beginning with `when` additionally surface as
//!    unresolved trigger intents for control-plane review.
//!
//! Compilation is a pure function of the closed session: identical
//! trajectories produce identical candidates (and identical digests after
//! finalization). Nothing here binds tools, models, browsers, or MCP
//! servers: binding resolution is the execution plane's job, confirmed by
//! approval.

use std::collections::BTreeMap;

use codex_workflow_contracts::IR_FORMAT_VERSION;
use codex_workflow_contracts::IrNodeId;
use codex_workflow_contracts::StepNode;
use codex_workflow_contracts::WorkflowIr;
use codex_workflow_contracts::WorkflowIrNode;

use crate::blueprint::StepOrigin;
use crate::blueprint::extract_blueprints;
use crate::candidate::CandidateInputs;
use crate::candidate::CandidateOrigin;
use crate::candidate::WorkflowCandidate;
use crate::error::TeachingCompilerError;
use crate::proposal::BindingKind;
use crate::proposal::BindingProposal;
use crate::proposal::CapabilityInference;
use crate::proposal::TriggerIntent;
use crate::session::TeachingSession;
use crate::trajectory::RecordOrigin;
use crate::trajectory::TrajectoryEvent;

/// Deterministic step node id scheme: `step-001`, `step-002`, …
fn step_node_id(index: usize) -> Result<IrNodeId, TeachingCompilerError> {
    let raw = format!("step-{index:03}");
    IrNodeId::parse(raw.clone()).map_err(|error| TeachingCompilerError::InvalidNodeId {
        node_id: raw,
        reason: error.to_string(),
    })
}

/// The first word of a step intent, lowercased, as a neutral hint label.
fn verb_of(intent: &str) -> String {
    intent
        .split_whitespace()
        .next()
        .unwrap_or("unspecified")
        .trim_end_matches(['.', ',', ';', ':'])
        .to_lowercase()
}

/// Compiles a closed teaching session into a workflow candidate.
///
/// The session must be closed and non-empty; otherwise
/// [`TeachingCompilerError::SessionNotClosed`] or
/// [`TeachingCompilerError::EmptySession`] is returned (both surfaced by
/// blueprint extraction).
pub fn compile(session: &TeachingSession) -> Result<WorkflowCandidate, TeachingCompilerError> {
    let blueprints = extract_blueprints(session)?;

    // 1. Chain one step node per blueprint, all reachable from the entry.
    let mut nodes: BTreeMap<IrNodeId, WorkflowIrNode> = BTreeMap::new();
    let mut node_evidence: BTreeMap<IrNodeId, Vec<crate::evidence::TeachingEvidence>> =
        BTreeMap::new();
    let mut node_origins: BTreeMap<IrNodeId, StepOrigin> = BTreeMap::new();
    let mut capability_inferences: Vec<CapabilityInference> = Vec::new();
    let mut binding_proposals: Vec<BindingProposal> = Vec::new();

    for (index, blueprint) in blueprints.iter().enumerate() {
        let node_id = step_node_id(index + 1)?;
        let next = if index + 1 < blueprints.len() {
            Some(step_node_id(index + 2)?)
        } else {
            None
        };
        nodes.insert(
            node_id.clone(),
            WorkflowIrNode::Step(StepNode {
                capabilities: Vec::new(),
                roles: Vec::new(),
                description: Some(blueprint.intent().to_string()),
                next,
            }),
        );
        node_evidence.insert(node_id.clone(), blueprint.evidence().to_vec());
        node_origins.insert(node_id.clone(), blueprint.origin());

        // Neutral, deterministic inference: one lexical hint + one open
        // capability binding proposal per step. The proposal (not a
        // declared requirement) is what keeps validation quiet while
        // leaving binding authority with the execution plane.
        let verb = verb_of(blueprint.intent());
        capability_inferences.push(CapabilityInference {
            node_id: node_id.clone(),
            requirement_hint: format!("verb:{verb}"),
            rationale: format!(
                "lexical hint from the {} step's recorded intent",
                blueprint.origin().as_str()
            ),
        });
        binding_proposals.push(BindingProposal {
            node_id: node_id.clone(),
            kind: BindingKind::Capability,
            proposed_reference: format!("unresolved:verb:{verb}"),
            rationale: "derived from the taught step intent; binding is the \
                        execution plane's decision"
                .to_string(),
            requires_approval: true,
        });
    }

    // 2. Trigger intents from `when`-prefixed instruction statements.
    let mut trigger_intents: Vec<TriggerIntent> = Vec::new();
    for record in session.records() {
        if let (RecordOrigin::Instruction, TrajectoryEvent::Instruction { text }) =
            (record.origin(), record.event())
        {
            let trimmed = text.trim();
            if trimmed.to_lowercase().starts_with("when") {
                trigger_intents.push(TriggerIntent {
                    class_hint: "unresolved".to_string(),
                    description: trimmed.to_string(),
                });
            }
        }
    }

    let ir = WorkflowIr {
        ir_format: IR_FORMAT_VERSION,
        entry: step_node_id(1)?,
        nodes,
        conditions: BTreeMap::new(),
    };

    let inputs = CandidateInputs {
        origin: CandidateOrigin::Taught(session.mode()),
        description: Some(format!(
            "taught workflow ({} step(s), mode {})",
            blueprints.len(),
            session.mode().as_str()
        )),
        ir,
        policy: *session.policy(),
        node_evidence,
        node_origins,
        capability_inferences,
        binding_proposals,
        trigger_intents,
    };
    Ok(WorkflowCandidate::new(inputs))
}

#[cfg(test)]
mod tests {
    use super::compile;
    use crate::blueprint::StepOrigin;
    use crate::candidate::CandidateOrigin;
    use crate::evidence::TeachingEvidence;
    use crate::mode::TeachingMode;
    use crate::session::TeachingSession;
    use crate::trajectory::RecordOrigin;
    use crate::trajectory::TrajectoryEvent;
    use codex_workflow_contracts::IrNodeId;
    use codex_workflow_contracts::WorkflowIrNode;

    fn node_id(value: &str) -> IrNodeId {
        IrNodeId::parse(value).expect("node id")
    }

    fn evidence(label: &str) -> TeachingEvidence {
        TeachingEvidence::new(label, "rollout://step", "ab".repeat(32)).expect("evidence")
    }

    #[test]
    fn demonstration_compiles_to_annotated_observed_steps() {
        let mut session = TeachingSession::new(TeachingMode::Demonstrate);
        session
            .record(
                RecordOrigin::Demonstration,
                TrajectoryEvent::Observation {
                    text: "the list view is open".to_string(),
                },
                Vec::new(),
            )
            .expect("record");
        session
            .record(
                RecordOrigin::Demonstration,
                TrajectoryEvent::Action {
                    text: "open the details pane".to_string(),
                },
                vec![evidence("open-details")],
            )
            .expect("record");
        session
            .record(
                RecordOrigin::Demonstration,
                TrajectoryEvent::Result {
                    text: "the pane shows the summary".to_string(),
                },
                Vec::new(),
            )
            .expect("record");
        session.close();

        let candidate = compile(&session).expect("compile");
        assert_eq!(
            candidate.origin(),
            CandidateOrigin::Taught(TeachingMode::Demonstrate)
        );
        let ir = candidate.ir();
        let step = ir.nodes.get(&node_id("step-001")).expect("step node");
        let WorkflowIrNode::Step(node) = step else {
            panic!("expected a step node");
        };
        let description = node.description.as_deref().expect("intent");
        assert!(description.contains("Observed context: the list view is open"));
        assert!(description.contains("Action: open the details pane"));
        assert!(description.contains("Outcome: the pane shows the summary"));
        assert_eq!(ir.entry, node_id("step-001"));
        assert_eq!(
            candidate.node_origin(&node_id("step-001")),
            Some(StepOrigin::Observed)
        );
        assert_eq!(
            candidate.node_evidence(&node_id("step-001")).map(len),
            Some(1)
        );
    }

    #[test]
    fn instruction_compiles_to_instructed_steps_and_trigger_intents() {
        let mut session = TeachingSession::new(TeachingMode::Instruct);
        session
            .record(
                RecordOrigin::Instruction,
                TrajectoryEvent::Instruction {
                    text: "When the build fails, page the on-call engineer.".to_string(),
                },
                Vec::new(),
            )
            .expect("record");
        session.close();

        let candidate = compile(&session).expect("compile");
        assert_eq!(
            candidate.node_origin(&node_id("step-001")),
            Some(StepOrigin::Instructed)
        );
        assert_eq!(candidate.trigger_intents().len(), 1);
        assert_eq!(candidate.trigger_intents()[0].class_hint, "unresolved");
        assert!(
            candidate.trigger_intents()[0]
                .description
                .to_lowercase()
                .starts_with("when")
        );
    }

    #[test]
    fn hybrid_interleaves_observed_and_instructed_steps() {
        let mut session = TeachingSession::new(TeachingMode::Hybrid);
        session
            .record(
                RecordOrigin::Demonstration,
                TrajectoryEvent::Action {
                    text: "open the list".to_string(),
                },
                vec![evidence("hybrid-1")],
            )
            .expect("record");
        session
            .record(
                RecordOrigin::Instruction,
                TrajectoryEvent::Instruction {
                    text: "then archive the item".to_string(),
                },
                Vec::new(),
            )
            .expect("record");
        session
            .record(
                RecordOrigin::Demonstration,
                TrajectoryEvent::Action {
                    text: "confirm the archive".to_string(),
                },
                vec![evidence("hybrid-3")],
            )
            .expect("record");
        session.close();

        let candidate = compile(&session).expect("compile");
        assert_eq!(
            candidate.node_origin(&node_id("step-001")),
            Some(StepOrigin::Observed)
        );
        assert_eq!(
            candidate.node_origin(&node_id("step-002")),
            Some(StepOrigin::Instructed)
        );
        assert_eq!(
            candidate.node_origin(&node_id("step-003")),
            Some(StepOrigin::Observed)
        );
        // chained ordering: step-001 -> step-002 -> step-003
        let ir = candidate.ir();
        for (current, expected_next) in [("step-001", "step-002"), ("step-002", "step-003")] {
            let WorkflowIrNode::Step(node) = ir.nodes.get(&node_id(current)).expect("step") else {
                panic!("expected a step node");
            };
            assert_eq!(node.next.as_ref(), Some(&node_id(expected_next)));
        }
        let WorkflowIrNode::Step(last) = ir.nodes.get(&node_id("step-003")).expect("step") else {
            panic!("expected a step node");
        };
        assert!(last.next.is_none());
    }

    #[test]
    fn emits_neutral_inferences_and_binding_proposals_per_step() {
        let mut session = TeachingSession::new(TeachingMode::Demonstrate);
        session
            .record(
                RecordOrigin::Demonstration,
                TrajectoryEvent::Action {
                    text: "Open the list.".to_string(),
                },
                vec![evidence("open-list")],
            )
            .expect("record");
        session.close();

        let candidate = compile(&session).expect("compile");
        assert_eq!(candidate.capability_inferences().len(), 1);
        let hint = &candidate.capability_inferences()[0];
        assert_eq!(hint.node_id, node_id("step-001"));
        assert_eq!(hint.requirement_hint, "verb:open");
        assert!(!hint.rationale.is_empty());

        assert_eq!(candidate.binding_proposals().len(), 1);
        let proposal = &candidate.binding_proposals()[0];
        assert_eq!(proposal.node_id, node_id("step-001"));
        assert_eq!(proposal.kind, crate::proposal::BindingKind::Capability);
        assert_eq!(proposal.proposed_reference, "unresolved:verb:open");
        assert!(proposal.requires_approval);

        // Environment neutrality: the compiler emits unresolved references
        // only; concrete bindings are the execution plane's decision.
        assert!(
            candidate
                .binding_proposals()
                .iter()
                .all(|binding| binding.proposed_reference.starts_with("unresolved:"))
        );
    }

    fn len(slice: &[TeachingEvidence]) -> usize {
        slice.len()
    }
}
