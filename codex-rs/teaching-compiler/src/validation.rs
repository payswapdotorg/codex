//! Candidate validation: contract checks plus compiler-level structural
//! findings.
//!
//! Validation runs the frozen contract validators first (a contract error
//! aborts the pass), then adds compiler-level structural findings.
//! Error-severity findings block approval and publication; warnings are
//! surfaced for review only.

use std::collections::BTreeSet;

use codex_workflow_contracts::IrNodeId;
use codex_workflow_contracts::WorkflowIr;
use codex_workflow_contracts::WorkflowIrNode;
use serde::Deserialize;
use serde::Serialize;

use crate::blueprint::StepOrigin;
use crate::candidate::WorkflowCandidate;
use crate::error::TeachingCompilerError;
use crate::graph;

/// Severity of a validation finding.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Severity {
    /// Blocks approval and publication.
    Error,
    /// Informational; does not block, but is surfaced for review.
    Warning,
}

/// Stable diagnostic codes emitted by candidate validation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FindingCode {
    /// A node is not reachable from the entry node.
    UnreachableNode,
    /// An observed step carries no evidence references; it was recorded
    /// through the no-evidence fallback and cannot be optimized.
    ObservedStepWithoutEvidence,
    /// A step declares no capability requirement and has no open binding
    /// proposal.
    StepWithoutCapability,
    /// A step carries no human-facing intent description.
    StepWithoutIntent,
    /// A conditional branch has neither a default arm nor resolvable
    /// conditions; its simulation is indeterminate.
    IndeterminateBranch,
}

/// One structural or policy finding about a candidate.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ValidationFinding {
    /// How severe the finding is.
    pub severity: Severity,
    /// A stable diagnostic code.
    pub code: FindingCode,
    /// Human-facing explanation.
    pub message: String,
}

/// Summary of one validation pass.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ValidationSummary {
    findings: Vec<ValidationFinding>,
}

impl ValidationSummary {
    pub(crate) fn new(findings: Vec<ValidationFinding>) -> Self {
        Self { findings }
    }

    /// All findings in deterministic order.
    pub fn findings(&self) -> &[ValidationFinding] {
        &self.findings
    }

    /// Number of error-severity findings.
    pub fn error_count(&self) -> usize {
        self.findings
            .iter()
            .filter(|finding| finding.severity == Severity::Error)
            .count()
    }

    /// Number of warning-severity findings.
    pub fn warning_count(&self) -> usize {
        self.findings
            .iter()
            .filter(|finding| finding.severity == Severity::Warning)
            .count()
    }

    /// Whether the pass produced no error-severity findings.
    pub fn is_clean(&self) -> bool {
        self.error_count() == 0
    }
}

/// Runs the contract validators and the compiler-level structural checks
/// over a graph.
pub(crate) fn validate_candidate_ir(
    ir: &WorkflowIr,
) -> Result<Vec<ValidationFinding>, TeachingCompilerError> {
    ir.validate()?;
    let mut findings = Vec::new();
    let reachable = graph::reachable_from(&ir.entry, &ir.nodes);
    let mut unreachable: Vec<&IrNodeId> = ir
        .nodes
        .keys()
        .filter(|node_id| !reachable.contains(*node_id))
        .collect();
    unreachable.sort();
    for node_id in unreachable {
        findings.push(ValidationFinding {
            severity: Severity::Error,
            code: FindingCode::UnreachableNode,
            message: format!("node `{node_id}` is not reachable from the entry node"),
        });
    }
    for (node_id, node) in ir.nodes.iter() {
        match node {
            WorkflowIrNode::ConditionalBranch(branch) if branch.default.is_none() => {
                findings.push(ValidationFinding {
                    severity: Severity::Warning,
                    code: FindingCode::IndeterminateBranch,
                    message: format!(
                        "conditional branch `{node_id}` has no default arm; branch selection \
                         is control-plane authority and simulation will be indeterminate"
                    ),
                });
            }
            WorkflowIrNode::Step(step) if step.description.is_none() => {
                findings.push(ValidationFinding {
                    severity: Severity::Warning,
                    code: FindingCode::StepWithoutIntent,
                    message: format!("step `{node_id}` carries no intent description"),
                });
            }
            _ => {}
        }
    }
    Ok(findings)
}

/// Runs the full validation pass over a candidate: graph-level checks plus
/// per-step evidence and binding-coverage checks.
pub(crate) fn validate_candidate(
    candidate: &WorkflowCandidate,
) -> Result<ValidationSummary, TeachingCompilerError> {
    let mut findings = validate_candidate_ir(candidate.ir())?;
    let covered: BTreeSet<IrNodeId> = candidate
        .binding_proposals()
        .iter()
        .map(|proposal| proposal.node_id.clone())
        .collect();
    for (node_id, node) in candidate.ir().nodes.iter() {
        let WorkflowIrNode::Step(step) = node else {
            continue;
        };
        let has_capability = !step.capabilities.is_empty();
        let has_proposal = covered.contains(node_id);
        if !has_capability && !has_proposal {
            findings.push(ValidationFinding {
                severity: Severity::Warning,
                code: FindingCode::StepWithoutCapability,
                message: format!(
                    "step `{node_id}` declares no capability requirement and has no binding \
                     proposal"
                ),
            });
        }
        let no_evidence = match candidate.node_evidence(node_id) {
            Some(evidence) => evidence.is_empty(),
            None => true,
        };
        if candidate.node_origin(node_id) == Some(StepOrigin::Observed) && no_evidence {
            findings.push(ValidationFinding {
                severity: Severity::Warning,
                code: FindingCode::ObservedStepWithoutEvidence,
                message: format!(
                    "observed step `{node_id}` carries no evidence references; it was recorded \
                     through the no-evidence fallback and cannot be optimized"
                ),
            });
        }
    }
    Ok(ValidationSummary::new(findings))
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::FindingCode;
    use super::Severity;
    use codex_workflow_contracts::IR_FORMAT_VERSION;
    use codex_workflow_contracts::IrNodeId;
    use codex_workflow_contracts::StepNode;
    use codex_workflow_contracts::WorkflowIr;
    use codex_workflow_contracts::WorkflowIrNode;

    use crate::approval::ApprovalDecision;
    use crate::approval::ApprovalDecisionKind;
    use crate::blueprint::StepOrigin;
    use crate::candidate::CandidateStatus;
    use crate::candidate::WorkflowCandidate;
    use crate::compiler::compile;
    use crate::error::TeachingCompilerError;
    use crate::evidence::TeachingEvidence;
    use crate::mode::TeachingMode;
    use crate::session::TeachingSession;
    use crate::trajectory::RecordOrigin;
    use crate::trajectory::TrajectoryEvent;

    fn taught(actions: &[(&str, bool)]) -> WorkflowCandidate {
        let mut session = TeachingSession::new(TeachingMode::Demonstrate);
        for (text, with_evidence) in actions {
            let evidence = if *with_evidence {
                vec![
                    TeachingEvidence::new("step-evidence", "rollout://step", "ab".repeat(32))
                        .expect("evidence"),
                ]
            } else {
                Vec::new()
            };
            session
                .record(
                    RecordOrigin::Demonstration,
                    TrajectoryEvent::Action {
                        text: (*text).to_string(),
                    },
                    evidence,
                )
                .expect("record");
        }
        session.close();
        compile(&session).expect("compile")
    }

    fn step(next: Option<&str>) -> WorkflowIrNode {
        WorkflowIrNode::Step(StepNode {
            capabilities: Vec::new(),
            roles: Vec::new(),
            description: Some("authored step".to_string()),
            next: next.map(|next| IrNodeId::parse(next).expect("node id")),
        })
    }

    fn ir_with_orphan() -> WorkflowIr {
        let mut nodes = BTreeMap::new();
        let a = IrNodeId::parse("step-a").expect("node id");
        let orphan = IrNodeId::parse("step-orphan").expect("node id");
        nodes.insert(a.clone(), step(None));
        nodes.insert(orphan, step(None));
        WorkflowIr {
            ir_format: IR_FORMAT_VERSION,
            entry: a,
            nodes,
            conditions: BTreeMap::new(),
        }
    }

    #[test]
    fn taught_candidate_with_evidence_validates_clean() {
        let mut candidate = taught(&[("Do a.", true), ("Do b.", true)]);
        let summary = candidate.validate().expect("validate");
        assert!(summary.is_clean());
        assert_eq!(summary.warning_count(), 0);
        assert_eq!(candidate.status(), CandidateStatus::Validated);
    }

    #[test]
    fn observed_step_without_evidence_is_a_warning_not_an_error() {
        let mut candidate = taught(&[("Do a.", false)]);
        let summary = candidate.validate().expect("validate");
        assert!(summary.is_clean());
        assert_eq!(summary.warning_count(), 1);
        assert_eq!(
            summary.findings()[0].code,
            FindingCode::ObservedStepWithoutEvidence
        );
        assert_eq!(summary.findings()[0].severity, Severity::Warning);
        assert_eq!(candidate.status(), CandidateStatus::Validated);
        assert_eq!(
            candidate.node_origin(&IrNodeId::parse("step-001").expect("node id")),
            Some(StepOrigin::Observed)
        );
    }

    #[test]
    fn unreachable_nodes_are_errors_and_block_validation() {
        let mut candidate = WorkflowCandidate::from_ir(ir_with_orphan(), None).expect("candidate");
        let summary = candidate.validate().expect("validate");
        assert!(!summary.is_clean());
        assert_eq!(summary.error_count(), 1);
        assert_eq!(summary.findings()[0].code, FindingCode::UnreachableNode);
        assert_eq!(candidate.status(), CandidateStatus::Compiled);
        let decision = ApprovalDecision::new("tech-lead", "review", ApprovalDecisionKind::Approved)
            .expect("decision");
        assert!(matches!(
            candidate.approve(decision),
            Err(TeachingCompilerError::InvalidState { .. })
        ));
    }

    #[test]
    fn authored_step_without_intent_or_capability_is_flagged() {
        let mut nodes = BTreeMap::new();
        let a = IrNodeId::parse("step-a").expect("node id");
        nodes.insert(
            a.clone(),
            WorkflowIrNode::Step(StepNode {
                capabilities: Vec::new(),
                roles: Vec::new(),
                description: None,
                next: None,
            }),
        );
        let mut candidate = WorkflowCandidate::from_ir(
            WorkflowIr {
                ir_format: IR_FORMAT_VERSION,
                entry: a,
                nodes,
                conditions: BTreeMap::new(),
            },
            None,
        )
        .expect("candidate");
        let summary = candidate.validate().expect("validate");
        assert!(summary.is_clean());
        assert_eq!(summary.warning_count(), 2);
        let codes: Vec<_> = summary
            .findings()
            .iter()
            .map(|finding| finding.code)
            .collect();
        assert!(codes.contains(&FindingCode::StepWithoutIntent));
        assert!(codes.contains(&FindingCode::StepWithoutCapability));
    }
}
