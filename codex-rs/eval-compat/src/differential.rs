//! Differential comparison with explicit divergences (WO-013).
//!
//! Differential testing pins the variable that matters — the model
//! provider serving the action-proposal seam, or the provider serving a
//! model invocation — and compares the resulting records field by field.
//! Every difference is enumerated in the returned report; comparisons
//! never fail silently and never early-return on the first mismatch.
//!
//! The comparisons use the same equivalence classes as the records'
//! semantic fingerprints ([`crate::record`]): provider identity, instance
//! identities, and store-allocated locators are ignored; outcomes, paths,
//! action sequences, evidence histograms, and normalized model outputs are
//! compared exactly.

use codex_model_contract::ModelCapabilities;
use codex_protocol::models::ResponseItem;
use codex_workflow_app::RunTerminal;
use codex_workflow_contracts::ContentDigest;
use codex_workflow_contracts::IrNodeId;
use codex_workflow_contracts::WorkflowInstanceStatus;

use crate::record::ActionFootprint;
use crate::record::ModelCompatRecord;
use crate::record::ModelErrorClass;
use crate::record::NormalizedNegotiationFailure;
use crate::record::WorkflowRunRecord;

/// One workflow-plane divergence between a baseline and a candidate run.
#[derive(Clone, Debug, PartialEq)]
pub enum WorkflowDivergence {
    /// The runs settled with different terminals.
    Terminal {
        /// The baseline terminal.
        baseline: RunTerminal,
        /// The candidate terminal.
        candidate: RunTerminal,
    },
    /// The instances settled with different statuses.
    Status {
        /// The baseline status.
        baseline: WorkflowInstanceStatus,
        /// The candidate status.
        candidate: WorkflowInstanceStatus,
    },
    /// The walks visited different nodes.
    Path {
        /// The baseline path.
        baseline: Vec<IrNodeId>,
        /// The candidate path.
        candidate: Vec<IrNodeId>,
    },
    /// The runs dispatched a different number of actions.
    ActionCount {
        /// The baseline count.
        baseline: usize,
        /// The candidate count.
        candidate: usize,
    },
    /// The runs dispatched different actions at the same position.
    Action {
        /// The zero-based dispatch position.
        index: usize,
        /// The baseline action.
        baseline: ActionFootprint,
        /// The candidate action.
        candidate: ActionFootprint,
    },
    /// The runs recorded a different amount of one evidence kind.
    Evidence {
        /// The evidence kind key.
        kind: String,
        /// The baseline count.
        baseline: usize,
        /// The candidate count.
        candidate: usize,
    },
    /// The runs verified a different amount of one recovery strategy.
    Recovery {
        /// The recovery strategy.
        strategy: String,
        /// The baseline count.
        baseline: usize,
        /// The candidate count.
        candidate: usize,
    },
    /// The runs escalated a different number of times.
    Escalations {
        /// The baseline count.
        baseline: usize,
        /// The candidate count.
        candidate: usize,
    },
    /// The models served a different number of action proposals.
    ModelRequests {
        /// The baseline count.
        baseline: usize,
        /// The candidate count.
        candidate: usize,
    },
}

/// The differential report of two workflow runs.
#[derive(Clone, Debug)]
pub struct WorkflowDifferential {
    /// The provider that served the baseline run.
    pub baseline_provider: String,
    /// The provider that served the candidate run.
    pub candidate_provider: String,
    /// Whether the runs are observably equivalent.
    pub equivalent: bool,
    /// Every enumerated divergence, in comparison order.
    pub divergences: Vec<WorkflowDivergence>,
}

/// Compares two workflow run records.
///
/// All differences are collected; none is silently dropped.
pub fn compare_workflow_runs(
    baseline: &WorkflowRunRecord,
    candidate: &WorkflowRunRecord,
) -> WorkflowDifferential {
    let mut divergences = Vec::new();
    if baseline.terminal != candidate.terminal {
        divergences.push(WorkflowDivergence::Terminal {
            baseline: baseline.terminal.clone(),
            candidate: candidate.terminal.clone(),
        });
    }
    if baseline.status != candidate.status {
        divergences.push(WorkflowDivergence::Status {
            baseline: baseline.status,
            candidate: candidate.status,
        });
    }
    if baseline.path != candidate.path {
        divergences.push(WorkflowDivergence::Path {
            baseline: baseline.path.clone(),
            candidate: candidate.path.clone(),
        });
    }
    if baseline.actions.len() != candidate.actions.len() {
        divergences.push(WorkflowDivergence::ActionCount {
            baseline: baseline.actions.len(),
            candidate: candidate.actions.len(),
        });
    }
    for (index, (baseline_action, candidate_action)) in baseline
        .actions
        .iter()
        .zip(candidate.actions.iter())
        .enumerate()
    {
        if baseline_action != candidate_action {
            divergences.push(WorkflowDivergence::Action {
                index,
                baseline: baseline_action.clone(),
                candidate: candidate_action.clone(),
            });
        }
    }
    compare_histograms(
        &baseline.evidence_histogram,
        &candidate.evidence_histogram,
        |kind, baseline_count, candidate_count| WorkflowDivergence::Evidence {
            kind,
            baseline: baseline_count,
            candidate: candidate_count,
        },
        &mut divergences,
    );
    compare_histograms(
        &baseline.recovery_histogram,
        &candidate.recovery_histogram,
        |strategy, baseline_count, candidate_count| WorkflowDivergence::Recovery {
            strategy,
            baseline: baseline_count,
            candidate: candidate_count,
        },
        &mut divergences,
    );
    if baseline.escalations != candidate.escalations {
        divergences.push(WorkflowDivergence::Escalations {
            baseline: baseline.escalations,
            candidate: candidate.escalations,
        });
    }
    if baseline.model_requests != candidate.model_requests {
        divergences.push(WorkflowDivergence::ModelRequests {
            baseline: baseline.model_requests,
            candidate: candidate.model_requests,
        });
    }
    WorkflowDifferential {
        baseline_provider: baseline.provider_id.clone(),
        candidate_provider: candidate.provider_id.clone(),
        equivalent: divergences.is_empty(),
        divergences,
    }
}

/// One model-plane divergence between a baseline and a candidate record.
#[derive(Clone, Debug, PartialEq)]
pub enum ModelDivergence {
    /// The records selected differently.
    Selection {
        /// Whether the baseline selected a provider.
        baseline: bool,
        /// Whether the candidate selected a provider.
        candidate: bool,
    },
    /// The records negotiated different capabilities (provider identity
    /// normalized out).
    Negotiation {
        /// The baseline capabilities, when negotiated.
        baseline: Option<ModelCapabilities>,
        /// The candidate capabilities, when negotiated.
        candidate: Option<ModelCapabilities>,
    },
    /// The negotiations failed differently (normalized failures).
    NegotiationFailure {
        /// The baseline failure, when it failed.
        baseline: Option<NormalizedNegotiationFailure>,
        /// The candidate failure, when it failed.
        candidate: Option<NormalizedNegotiationFailure>,
    },
    /// The invocations failed with different taxonomy classes.
    Invocation {
        /// The baseline failure class, when it failed.
        baseline: Option<ModelErrorClass>,
        /// The candidate failure class, when it failed.
        candidate: Option<ModelErrorClass>,
    },
    /// The invocations produced different output items.
    Output {
        /// The baseline output items.
        baseline: Vec<ResponseItem>,
        /// The candidate output items.
        candidate: Vec<ResponseItem>,
    },
    /// The invocations reported different end-turn state.
    EndTurn {
        /// The baseline end-turn marker.
        baseline: Option<bool>,
        /// The candidate end-turn marker.
        candidate: Option<bool>,
    },
    /// The requests' semantic content digests differ (a fixture drift, not
    /// a provider behavior difference).
    RequestDigest {
        /// The baseline digest, when it was computed.
        baseline: Option<ContentDigest>,
        /// The candidate digest, when it was computed.
        candidate: Option<ContentDigest>,
    },
}

/// The differential report of two model-compatibility records.
#[derive(Clone, Debug)]
pub struct ModelDifferential {
    /// The provider the baseline case targeted.
    pub baseline_provider: String,
    /// The provider the candidate case targeted.
    pub candidate_provider: String,
    /// Whether the records are observably equivalent.
    pub equivalent: bool,
    /// Every enumerated divergence, in comparison order.
    pub divergences: Vec<ModelDivergence>,
}

/// Compares two model-compatibility records.
///
/// Provider identity is the differential variable, so it is excluded
/// (capabilities are compared with the provider key normalized out);
/// everything else (selection, negotiation, invocation, normalized
/// response, request digest) is compared exactly.
pub fn compare_model_runs(
    baseline: &ModelCompatRecord,
    candidate: &ModelCompatRecord,
) -> ModelDifferential {
    let mut divergences = Vec::new();
    if baseline.selected != candidate.selected {
        divergences.push(ModelDivergence::Selection {
            baseline: baseline.selected,
            candidate: candidate.selected,
        });
    }
    let baseline_capabilities = baseline.negotiated.as_ref().map(normalize_capabilities);
    let candidate_capabilities = candidate.negotiated.as_ref().map(normalize_capabilities);
    if baseline_capabilities != candidate_capabilities {
        divergences.push(ModelDivergence::Negotiation {
            baseline: baseline_capabilities,
            candidate: candidate_capabilities,
        });
    }
    if baseline.negotiation_failure != candidate.negotiation_failure {
        divergences.push(ModelDivergence::NegotiationFailure {
            baseline: baseline.negotiation_failure.clone(),
            candidate: candidate.negotiation_failure.clone(),
        });
    }
    if baseline.invocation_failure != candidate.invocation_failure {
        divergences.push(ModelDivergence::Invocation {
            baseline: baseline.invocation_failure,
            candidate: candidate.invocation_failure,
        });
    }
    let baseline_output = baseline
        .response
        .as_ref()
        .map(|response| response.output.clone());
    let candidate_output = candidate
        .response
        .as_ref()
        .map(|response| response.output.clone());
    if baseline_output != candidate_output {
        divergences.push(ModelDivergence::Output {
            baseline: baseline_output.unwrap_or_default(),
            candidate: candidate_output.unwrap_or_default(),
        });
    }
    let baseline_end_turn = baseline
        .response
        .as_ref()
        .and_then(|response| response.end_turn);
    let candidate_end_turn = candidate
        .response
        .as_ref()
        .and_then(|response| response.end_turn);
    if baseline_end_turn != candidate_end_turn {
        divergences.push(ModelDivergence::EndTurn {
            baseline: baseline_end_turn,
            candidate: candidate_end_turn,
        });
    }
    if baseline.request_digest != candidate.request_digest {
        divergences.push(ModelDivergence::RequestDigest {
            baseline: baseline.request_digest.clone(),
            candidate: candidate.request_digest.clone(),
        });
    }
    ModelDifferential {
        baseline_provider: baseline.descriptor.provider_id.clone(),
        candidate_provider: candidate.descriptor.provider_id.clone(),
        equivalent: divergences.is_empty(),
        divergences,
    }
}

/// Normalizes the provider key out of a capability record so two providers
/// serving the same model capabilities compare equal.
fn normalize_capabilities(capabilities: &ModelCapabilities) -> ModelCapabilities {
    let mut normalized = capabilities.clone();
    // Empty provider key: the provider identity is the differential
    // variable, not a capability difference.
    normalized.provider_id = String::new();
    normalized
}

/// Compares two histograms key by key (union of keys).
fn compare_histograms<T>(
    baseline: &std::collections::BTreeMap<String, usize>,
    candidate: &std::collections::BTreeMap<String, usize>,
    build: impl Fn(String, usize, usize) -> T,
    divergences: &mut Vec<T>,
) {
    for (key, baseline_count) in baseline {
        let candidate_count = candidate.get(key).copied().unwrap_or(0);
        if baseline_count != &candidate_count {
            divergences.push(build(key.clone(), *baseline_count, candidate_count));
        }
    }
    for (key, candidate_count) in candidate {
        if !baseline.contains_key(key) && *candidate_count != 0 {
            divergences.push(build(key.clone(), 0, *candidate_count));
        }
    }
}
