//! The workflow candidate: the compiler's validated, approval-gated
//! precursor to an immutable workflow version.
//!
//! The candidate is the sole hand-off between teaching and publication. It
//! carries the compiled IR, per-node evidence and origins, inference
//! outputs, validation results, the simulation report, and the approval
//! gate. It owns no runtime, persists nothing, and never mutates a
//! published version — publication itself belongs to the workflow control
//! plane.

use std::collections::BTreeMap;
use std::collections::BTreeSet;

use codex_workflow_contracts::ContentDigest;
use codex_workflow_contracts::IrNodeId;
use codex_workflow_contracts::WorkflowDefinition;
use codex_workflow_contracts::WorkflowDefinitionId;
use codex_workflow_contracts::WorkflowDependencies;
use codex_workflow_contracts::WorkflowIr;
use serde::Deserialize;
use serde::Serialize;

use crate::approval::ApprovalDecision;
use crate::approval::ApprovalDecisionKind;
use crate::approval::ApprovalRecord;
use crate::blueprint::StepOrigin;
use crate::error::TeachingCompilerError;
use crate::evidence::TeachingEvidence;
use crate::mode::TeachingMode;
use crate::proposal::BindingProposal;
use crate::proposal::CapabilityInference;
use crate::proposal::TriggerIntent;
use crate::session::SessionPolicy;
use crate::simulation::SimulationConfig;
use crate::simulation::SimulationReport;
use crate::validation::ValidationSummary;

/// Where a candidate's graph came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CandidateOrigin {
    /// Compiled from a teaching session in the carried mode.
    Taught(TeachingMode),
    /// Supplied directly as an IR by the control plane for validation,
    /// simulation, and optimization.
    AuthoredIr,
}

/// Lifecycle state of a candidate.
///
/// Legal transitions, all enforced by the candidate itself:
/// `Compiled -> Validated -> Approved -> PublicationReady`. Any content or
/// policy change clears the approval and returns the candidate to
/// `Compiled` with a new content epoch.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CandidateStatus {
    /// Built but not yet validated.
    Compiled,
    /// Validation passed with no error findings.
    Validated,
    /// An approval decision has been recorded for the current epoch.
    Approved,
    /// Finalized into an approved, publication-ready definition.
    PublicationReady,
}

impl CandidateStatus {
    /// Stable lowercase name for diagnostics.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Compiled => "compiled",
            Self::Validated => "validated",
            Self::Approved => "approved",
            Self::PublicationReady => "publication-ready",
        }
    }
}

/// Inputs assembled by the compiler for a new candidate.
pub(crate) struct CandidateInputs {
    /// Where the graph came from.
    pub origin: CandidateOrigin,
    /// Human-facing description carried into the definition.
    pub description: Option<String>,
    /// The compiled graph.
    pub ir: WorkflowIr,
    /// The policy carried from the session.
    pub policy: SessionPolicy,
    /// Evidence per step node.
    pub node_evidence: BTreeMap<IrNodeId, Vec<TeachingEvidence>>,
    /// Extraction origin per step node.
    pub node_origins: BTreeMap<IrNodeId, StepOrigin>,
    /// Capability hints produced by inference.
    pub capability_inferences: Vec<CapabilityInference>,
    /// Binding proposals produced by inference.
    pub binding_proposals: Vec<BindingProposal>,
    /// Trigger intents produced by inference.
    pub trigger_intents: Vec<TriggerIntent>,
}

/// A compiled, not-yet-published workflow candidate.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkflowCandidate {
    origin: CandidateOrigin,
    description: Option<String>,
    ir: WorkflowIr,
    policy: SessionPolicy,
    status: CandidateStatus,
    epoch: u64,
    node_evidence: BTreeMap<IrNodeId, Vec<TeachingEvidence>>,
    node_origins: BTreeMap<IrNodeId, StepOrigin>,
    capability_inferences: Vec<CapabilityInference>,
    binding_proposals: Vec<BindingProposal>,
    trigger_intents: Vec<TriggerIntent>,
    last_validation: Option<ValidationSummary>,
    last_simulation: Option<SimulationReport>,
    approval: Option<ApprovalRecord>,
}

/// The approved output of the compiler: the frozen definition, its content
/// digest, the binding approval, and the simulation evidence.
///
/// The definition is frozen at hand-off: this crate exposes no way to
/// mutate it, and any post-hand-off modification is detectable because the
/// control plane publishes the version against `digest`.
pub struct ApprovedWorkflow {
    /// The approved workflow definition.
    pub definition: WorkflowDefinition,
    /// Canonical digest of the definition's semantic content.
    pub digest: ContentDigest,
    /// The approval record covering this exact content epoch.
    pub approval: ApprovalRecord,
    /// The simulation evidence gathered before approval.
    pub simulation: SimulationReport,
    /// Where the candidate's graph came from.
    pub origin: CandidateOrigin,
}

impl WorkflowCandidate {
    pub(crate) fn new(inputs: CandidateInputs) -> Self {
        Self {
            origin: inputs.origin,
            description: inputs.description,
            ir: inputs.ir,
            policy: inputs.policy,
            status: CandidateStatus::Compiled,
            epoch: 0,
            node_evidence: inputs.node_evidence,
            node_origins: inputs.node_origins,
            capability_inferences: inputs.capability_inferences,
            binding_proposals: inputs.binding_proposals,
            trigger_intents: inputs.trigger_intents,
            last_validation: None,
            last_simulation: None,
            approval: None,
        }
    }

    /// Wraps a control-plane-authored IR as a candidate for validation,
    /// simulation, and optimization.
    ///
    /// The IR is contract-validated on entry; the candidate starts in
    /// `Compiled` with the default (conservative) policy.
    pub fn from_ir(
        ir: WorkflowIr,
        description: Option<String>,
    ) -> Result<Self, TeachingCompilerError> {
        ir.validate()?;
        Ok(Self {
            origin: CandidateOrigin::AuthoredIr,
            description,
            ir,
            policy: SessionPolicy::default(),
            status: CandidateStatus::Compiled,
            epoch: 0,
            node_evidence: BTreeMap::new(),
            node_origins: BTreeMap::new(),
            capability_inferences: Vec::new(),
            binding_proposals: Vec::new(),
            trigger_intents: Vec::new(),
            last_validation: None,
            last_simulation: None,
            approval: None,
        })
    }

    /// Where the candidate's graph came from.
    pub fn origin(&self) -> CandidateOrigin {
        self.origin
    }

    /// Human-facing description carried into the definition.
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    /// The compiled IR.
    pub fn ir(&self) -> &WorkflowIr {
        &self.ir
    }

    /// The policy governing optimization.
    pub fn policy(&self) -> &SessionPolicy {
        &self.policy
    }

    /// The lifecycle state.
    pub fn status(&self) -> CandidateStatus {
        self.status
    }

    /// The content epoch; it advances on every content or policy change.
    pub fn epoch(&self) -> u64 {
        self.epoch
    }

    /// Evidence attached to a step node, if any.
    pub fn node_evidence(&self, node_id: &IrNodeId) -> Option<&[TeachingEvidence]> {
        self.node_evidence.get(node_id).map(Vec::as_slice)
    }

    /// The extraction origin of a step node, if recorded.
    pub fn node_origin(&self, node_id: &IrNodeId) -> Option<StepOrigin> {
        self.node_origins.get(node_id).copied()
    }

    /// Capability hints produced by inference.
    pub fn capability_inferences(&self) -> &[CapabilityInference] {
        &self.capability_inferences
    }

    /// Binding proposals produced by inference.
    pub fn binding_proposals(&self) -> &[BindingProposal] {
        &self.binding_proposals
    }

    /// Trigger intents produced by inference.
    pub fn trigger_intents(&self) -> &[TriggerIntent] {
        &self.trigger_intents
    }

    /// The last validation summary, if a validation pass has run.
    pub fn last_validation(&self) -> Option<&ValidationSummary> {
        self.last_validation.as_ref()
    }

    /// The last simulation report, if a simulation has run.
    pub fn last_simulation(&self) -> Option<&SimulationReport> {
        self.last_simulation.as_ref()
    }

    /// The recorded approval, if any.
    pub fn approval(&self) -> Option<&ApprovalRecord> {
        self.approval.as_ref()
    }

    /// Replaces the candidate's policy. A policy change is a content-policy
    /// change: it clears any recorded approval and advances the content
    /// epoch.
    pub fn set_policy(&mut self, policy: SessionPolicy) {
        self.policy = policy;
        self.after_content_change();
    }

    /// Builds a [`WorkflowDefinition`] from the candidate under a
    /// control-plane-supplied definition id.
    ///
    /// The definition is assembled with no triggers and no declared
    /// dependencies: teaching never invents them. Dependency resolution
    /// happens through the binding proposals after approval-plane review,
    /// before or by the publisher. The definition id is opaque to this
    /// crate; repository and version identity are control-plane authority.
    pub fn to_definition(
        &self,
        id: WorkflowDefinitionId,
    ) -> Result<WorkflowDefinition, TeachingCompilerError> {
        let definition = WorkflowDefinition {
            id,
            description: self.description.clone(),
            ir: self.ir.clone(),
            roles: BTreeMap::new(),
            triggers: Vec::new(),
            dependencies: WorkflowDependencies::default(),
        };
        definition.validate()?;
        Ok(definition)
    }

    /// Runs validation over the candidate and, when there are no
    /// error-severity findings, advances `Compiled -> Validated`.
    ///
    /// Validation is idempotent while content is unchanged; any content
    /// change resets the candidate to `Compiled`.
    pub fn validate(&mut self) -> Result<ValidationSummary, TeachingCompilerError> {
        if self.status != CandidateStatus::Compiled && self.status != CandidateStatus::Validated {
            return Err(TeachingCompilerError::InvalidState {
                current: self.status.as_str().to_string(),
                required: "compiled or validated".to_string(),
            });
        }
        let summary = crate::validation::validate_candidate(self)?;
        self.last_validation = Some(summary.clone());
        if summary.is_clean() {
            self.status = CandidateStatus::Validated;
        } else {
            self.status = CandidateStatus::Compiled;
        }
        Ok(summary)
    }

    /// Runs a deterministic structural simulation of the current content
    /// and stores the report for finalization.
    ///
    /// Simulation requires the candidate to be validated with no
    /// error-severity findings.
    pub fn simulate(
        &mut self,
        config: SimulationConfig,
    ) -> Result<SimulationReport, TeachingCompilerError> {
        if self.status != CandidateStatus::Validated {
            return Err(TeachingCompilerError::InvalidState {
                current: self.status.as_str().to_string(),
                required: "validated".to_string(),
            });
        }
        let report = crate::simulation::simulate_with_epoch(&self.ir, config, self.epoch)?;
        self.last_simulation = Some(report.clone());
        Ok(report)
    }

    /// Records an approval decision for the current content epoch and
    /// advances `Validated -> Approved`.
    ///
    /// Approvals are single-use and content-bound: any content change
    /// invalidates them. A recorded rejection is not an approval.
    pub fn approve(&mut self, decision: ApprovalDecision) -> Result<(), TeachingCompilerError> {
        if self.status != CandidateStatus::Validated {
            return Err(TeachingCompilerError::InvalidState {
                current: self.status.as_str().to_string(),
                required: "validated".to_string(),
            });
        }
        let record = decision.into_record(self.epoch);
        if record.kind != ApprovalDecisionKind::Approved {
            return Err(TeachingCompilerError::InvalidApproval {
                reason: "the recorded decision is a rejection, not an approval".to_string(),
            });
        }
        self.approval = Some(record);
        self.status = CandidateStatus::Approved;
        Ok(())
    }

    /// Finalizes the candidate into an approved, publication-ready
    /// workflow: the frozen definition plus its canonical digest, the
    /// binding approval record, and the simulation evidence.
    ///
    /// Publication gates enforced here:
    ///
    /// - the candidate is `Approved` for the **current** content epoch
    ///   (any post-approval change invalidates the approval);
    /// - a simulation of the current epoch exists and its outcome allows
    ///   publication;
    /// - the assembled [`WorkflowDefinition`] passes contract validation;
    /// - the frozen content is proven byte-stable (serialized twice,
    ///   identical) before the canonical digest is computed for hand-off.
    ///
    /// The actual immutable workflow version and repository write are
    /// owned by the workflow control plane; this crate only produces the
    /// approved, digest-pinned payload for it.
    pub fn finalize(
        &mut self,
        id: WorkflowDefinitionId,
    ) -> Result<ApprovedWorkflow, TeachingCompilerError> {
        if self.status != CandidateStatus::Approved {
            return Err(TeachingCompilerError::InvalidState {
                current: self.status.as_str().to_string(),
                required: "approved".to_string(),
            });
        }
        let approval =
            self.approval
                .clone()
                .ok_or_else(|| TeachingCompilerError::InvalidState {
                    current: self.status.as_str().to_string(),
                    required: "an approval record".to_string(),
                })?;
        if approval.epoch != self.epoch {
            return Err(TeachingCompilerError::ApprovalEpochMismatch {
                approval_epoch: approval.epoch,
                candidate_epoch: self.epoch,
            });
        }
        let simulation = self
            .last_simulation
            .clone()
            .ok_or(TeachingCompilerError::MissingSimulation)?;
        if simulation.epoch() != self.epoch {
            return Err(TeachingCompilerError::SimulationEpochMismatch {
                simulation_epoch: simulation.epoch(),
                candidate_epoch: self.epoch,
            });
        }
        if !simulation.allows_publication() {
            return Err(TeachingCompilerError::MissingSimulation);
        }
        let definition = self.to_definition(id)?;
        // Publication immutability check: the frozen content must be
        // byte-stable before hand-off.
        let serialized_once = serde_json::to_vec(&definition).map_err(|error| {
            TeachingCompilerError::InvalidState {
                current: "publication-ready".to_string(),
                required: format!("a serializable definition ({error})"),
            }
        })?;
        let serialized_twice = serde_json::to_vec(&definition).map_err(|error| {
            TeachingCompilerError::InvalidState {
                current: "publication-ready".to_string(),
                required: format!("a serializable definition ({error})"),
            }
        })?;
        if serialized_once != serialized_twice {
            return Err(TeachingCompilerError::InvalidState {
                current: "publication-ready".to_string(),
                required: "a byte-stable definition payload".to_string(),
            });
        }
        let digest = definition.digest()?;
        self.status = CandidateStatus::PublicationReady;
        Ok(ApprovedWorkflow {
            definition,
            digest,
            approval,
            simulation,
            origin: self.origin,
        })
    }

    pub(crate) fn ir_mut(&mut self) -> &mut WorkflowIr {
        &mut self.ir
    }

    pub(crate) fn policy_allows_optimization(&self) -> bool {
        self.policy.allow_optimization
    }

    /// Records a content or policy change: the epoch advances, the approval
    /// and stale simulation are dropped, the candidate returns to
    /// `Compiled`, and bookkeeping for removed nodes is pruned.
    pub(crate) fn after_content_change(&mut self) {
        self.epoch = self.epoch.saturating_add(1);
        self.status = CandidateStatus::Compiled;
        self.approval = None;
        self.last_simulation = None;
        let live: BTreeSet<IrNodeId> = self.ir.nodes.keys().cloned().collect();
        self.node_evidence
            .retain(|node_id, _| live.contains(node_id));
        self.node_origins
            .retain(|node_id, _| live.contains(node_id));
    }
}

#[cfg(test)]
mod tests {
    use super::CandidateOrigin;
    use super::CandidateStatus;
    use super::WorkflowCandidate;
    use crate::compiler::compile;
    use crate::error::TeachingCompilerError;
    use crate::mode::TeachingMode;
    use crate::session::TeachingSession;
    use crate::simulation::SimulationConfig;
    use crate::trajectory::RecordOrigin;
    use crate::trajectory::TrajectoryEvent;

    fn taught_candidate() -> WorkflowCandidate {
        let mut session = TeachingSession::new(TeachingMode::Demonstrate);
        session
            .record(
                RecordOrigin::Demonstration,
                TrajectoryEvent::Action {
                    text: "Open the list.".to_string(),
                },
                Vec::new(),
            )
            .expect("record");
        session.close();
        compile(&session).expect("compile")
    }

    #[test]
    fn enforces_the_lifecycle_transitions() {
        let mut candidate = taught_candidate();
        assert_eq!(
            candidate.origin(),
            CandidateOrigin::Taught(TeachingMode::Demonstrate)
        );
        // Simulation requires validation first.
        assert!(matches!(
            candidate.simulate(SimulationConfig::default()),
            Err(TeachingCompilerError::InvalidState { .. })
        ));
        candidate.validate().expect("validate");
        assert_eq!(candidate.status(), CandidateStatus::Validated);
        candidate
            .simulate(SimulationConfig::default())
            .expect("simulate");
        // Approval requires validation; validation is idempotent here.
        candidate.validate().expect("revalidate");
        candidate
            .approve(
                crate::approval::ApprovalDecision::new(
                    "tech-lead",
                    "review",
                    crate::approval::ApprovalDecisionKind::Approved,
                )
                .expect("decision"),
            )
            .expect("approve");
        assert_eq!(candidate.status(), CandidateStatus::Approved);
        // Finalization is covered end-to-end in tests/teaching_pipeline.rs
        // (it needs a control-plane-supplied WorkflowDefinitionId).
        assert!(candidate.approval().is_some());
        assert_eq!(candidate.epoch(), 0);
    }

    #[test]
    fn round_trips_through_json() {
        let candidate = taught_candidate();
        let encoded = serde_json::to_string(&candidate).expect("serialize");
        let decoded: WorkflowCandidate = serde_json::from_str(&encoded).expect("deserialize");
        assert_eq!(candidate, decoded);
    }
}
