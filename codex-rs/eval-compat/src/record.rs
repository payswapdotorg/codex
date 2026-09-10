//! Evaluation records and semantic fingerprints (WO-013).
//!
//! Records are the evidence currency of the evaluation harnesses: they
//! capture what a run observed (outcome, path, actions, evidence
//! histograms, logical counters) and project it onto a **semantic
//! equivalence class** — a canonical JSON projection with
//! content-addressed digest ([`ContentDigest`]) that deliberately excludes
//! run- and provider-specific identities (instance ids, evidence locators,
//! server response ids, provider keys).
//!
//! Two runs with equal [`fingerprint`](WorkflowRunRecord::fingerprint_digest)
//! digests are observably equivalent within the equivalence class; any
//! difference is enumerated explicitly by
//! [`crate::differential`] instead of failing silently.

use std::collections::BTreeMap;

use codex_execution_contracts::Action;
use codex_model_contract::ModelCapabilities;
use codex_model_contract::ModelDescriptor;
use codex_model_contract::ModelError;
use codex_model_contract::ModelFeature;
use codex_model_contract::ModelRequest;
use codex_model_contract::UnsupportedModelCapability;
use codex_model_provider::ProviderSelectionError;
use codex_protocol::models::ResponseItem;
use codex_workflow_app::RunTerminal;
use codex_workflow_contracts::ContentDigest;
use codex_workflow_contracts::EvidenceKind;
use codex_workflow_contracts::IrNodeId;
use codex_workflow_contracts::WorkflowContractError;
use codex_workflow_contracts::WorkflowDefinitionId;
use codex_workflow_contracts::WorkflowInstanceStatus;
use codex_workflow_contracts::WorkflowVersionId;

/// The neutral provider key used when digesting request semantics.
///
/// Request digests must be independent of the serving provider (the
/// provider swap is the differential variable), so the projection replaces
/// the descriptor's provider identity with this fixed marker.
pub const NEUTRAL_PROVIDER_KEY: &str = "eval-semantic";

/// One executed action, as observed by the environment.
///
/// Inputs serialize from the ordered [`Action`] input map, so identical
/// parameter sets always digest identically.
#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ActionFootprint {
    /// The environment-scoped operation that ran.
    pub operation: String,
    /// The opaque target the operation applied to, when any.
    pub target: Option<String>,
    /// The canonical structured inputs of the action.
    pub inputs: serde_json::Value,
    /// Scripted (logical) duration in milliseconds, when measured.
    pub duration_ms: Option<u64>,
}

impl ActionFootprint {
    /// Projects an [`Action`] (plus its scripted duration) to a footprint.
    pub fn of(action: &Action, duration_ms: Option<u64>) -> Self {
        Self {
            operation: action.operation.as_ref().to_string(),
            target: action
                .target
                .as_ref()
                .map(|target| target.as_ref().to_string()),
            inputs: serde_json::to_value(&action.inputs).unwrap_or(serde_json::Value::Null),
            duration_ms,
        }
    }
}

/// The recorded outcome of one workflow evaluation run.
///
/// The record deliberately excludes instance identities, evidence locators,
/// and the model provider key from its semantic fingerprint (see
/// [`WorkflowRunRecord::fingerprint_digest`]); those are run- and
/// provider-specific, while the fingerprint pins the *behavior*.
#[derive(Clone, Debug, PartialEq)]
pub struct WorkflowRunRecord {
    /// The model provider that served the action-proposal seam.
    pub provider_id: String,
    /// The workflow that ran.
    pub workflow: WorkflowDefinitionId,
    /// The immutable version the run pinned.
    pub version: WorkflowVersionId,
    /// Why the run stopped.
    pub terminal: RunTerminal,
    /// The settled instance status.
    pub status: WorkflowInstanceStatus,
    /// The nodes the walk visited, in order.
    pub path: Vec<IrNodeId>,
    /// The actions dispatched to the environment, in order.
    pub actions: Vec<ActionFootprint>,
    /// Recorded evidence, counted per kind (stable kind keys).
    pub evidence_histogram: BTreeMap<String, usize>,
    /// Verified recoveries, counted per strategy (`retry`, `rebind`, ...).
    pub recovery_histogram: BTreeMap<String, usize>,
    /// Control-plane escalations during the run.
    pub escalations: usize,
    /// Model invocations that served action proposals.
    pub model_requests: usize,
}

impl WorkflowRunRecord {
    /// The semantic equivalence class of this run, as canonical JSON.
    ///
    /// Excludes: the provider key (the differential variable), instance
    /// identities, evidence locators and digests (store-allocated), and any
    /// wall-clock data. Includes: terminal, settled status, visited path,
    /// ordered action footprints, evidence and recovery histograms,
    /// escalation count, and the model invocation count.
    pub fn semantic_projection(&self) -> serde_json::Value {
        serde_json::json!({
            "workflow": self.workflow.to_string(),
            "version": self.version.to_string(),
            "terminal": terminal_projection(&self.terminal),
            "status": status_projection(self.status),
            "path": self
                .path
                .iter()
                .map(std::string::ToString::to_string)
                .collect::<Vec<_>>(),
            "actions": &self.actions,
            "evidence": &self.evidence_histogram,
            "recovery": &self.recovery_histogram,
            "escalations": self.escalations,
            "modelRequests": self.model_requests,
        })
    }

    /// The content-addressed digest of the semantic projection.
    ///
    /// Equal digests mean observably equivalent runs (within the documented
    /// equivalence class); unequal digests mean the differential comparison
    /// must enumerate the divergence.
    pub fn fingerprint_digest(&self) -> Result<ContentDigest, WorkflowContractError> {
        ContentDigest::of(&self.semantic_projection())
    }
}

/// One model-compatibility evaluation case.
#[derive(Clone, Debug)]
pub struct ModelCompatCase {
    /// The provider/model identity the case targets.
    pub descriptor: ModelDescriptor,
    /// The catalog entry the capabilities are negotiated against.
    pub model: codex_protocol::openai_models::ModelInfo,
    /// The features the case requires.
    pub required: Vec<ModelFeature>,
    /// The request content (semantic fields are digested; the descriptor is
    /// re-addressed to the case's provider at invocation time).
    pub request: ModelRequest,
}

/// The normalized failure taxonomy class of a [`ModelError`].
///
/// Provider-specific messages are untrusted, derived detail; the taxonomy
/// class is the equivalence-relevant identity of a failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ModelErrorClass {
    /// Transport-level failure.
    Transport,
    /// Authentication failure.
    Auth,
    /// The request was rejected as invalid.
    InvalidRequest,
    /// The context window was exceeded.
    ContextWindowExceeded,
    /// Usage, quota, or plan limits were reached.
    UsageLimitExceeded,
    /// The provider rate-limited the request.
    RateLimited,
    /// The provider was overloaded.
    Overloaded,
    /// A required model capability is unsupported.
    UnsupportedCapability,
    /// The stream ended before completion.
    StreamIncomplete,
    /// The request was canceled.
    Canceled,
    /// An unclassified provider failure.
    Provider,
}

impl ModelErrorClass {
    /// Classifies a normalized model error.
    pub fn of(error: &ModelError) -> Self {
        match error {
            ModelError::Transport { .. } => Self::Transport,
            ModelError::Auth { .. } => Self::Auth,
            ModelError::InvalidRequest { .. } => Self::InvalidRequest,
            ModelError::ContextWindowExceeded { .. } => Self::ContextWindowExceeded,
            ModelError::UsageLimitExceeded { .. } => Self::UsageLimitExceeded,
            ModelError::RateLimited { .. } => Self::RateLimited,
            ModelError::Overloaded { .. } => Self::Overloaded,
            ModelError::UnsupportedCapability(_) => Self::UnsupportedCapability,
            ModelError::StreamIncomplete { .. } => Self::StreamIncomplete,
            ModelError::Canceled { .. } => Self::Canceled,
            ModelError::Provider { .. } => Self::Provider,
        }
    }

    /// The stable class name.
    pub fn key(&self) -> &'static str {
        match self {
            Self::Transport => "transport",
            Self::Auth => "auth",
            Self::InvalidRequest => "invalid-request",
            Self::ContextWindowExceeded => "context-window-exceeded",
            Self::UsageLimitExceeded => "usage-limit-exceeded",
            Self::RateLimited => "rate-limited",
            Self::Overloaded => "overloaded",
            Self::UnsupportedCapability => "unsupported-capability",
            Self::StreamIncomplete => "stream-incomplete",
            Self::Canceled => "canceled",
            Self::Provider => "provider",
        }
    }
}

/// A negotiation failure normalized to its provider-independent content:
/// the unsupported feature, the model it failed for, and the supported
/// alternatives for recovery.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NormalizedNegotiationFailure {
    /// The feature that was requested and unsupported.
    pub feature: ModelFeature,
    /// The model the negotiation ran against.
    pub model_id: String,
    /// Supported values for enumerable features, for recovery.
    pub supported_alternatives: Vec<String>,
}

impl NormalizedNegotiationFailure {
    /// Normalizes a negotiation failure.
    pub fn of(failure: &UnsupportedModelCapability) -> Self {
        Self {
            feature: failure.feature.clone(),
            model_id: failure.model_id.clone(),
            supported_alternatives: failure.supported_alternatives.clone(),
        }
    }

    /// The provider-independent projection of the failure.
    fn projection(&self) -> serde_json::Value {
        serde_json::json!({
            "feature": self.feature.to_string(),
            "model": self.model_id,
            "supported": self.supported_alternatives,
        })
    }
}

/// The semantic equivalence class of a model invocation's output: the
/// completed output items and the end-turn marker.
///
/// Server response ids, token usage, and server-side model substitution are
/// excluded: they are provider-owned observability, not semantics.
#[derive(Clone, Debug, PartialEq)]
pub struct NormalizedResponse {
    /// Completed output items.
    pub output: Vec<ResponseItem>,
    /// Whether the model affirmatively ended its turn, when reported.
    pub end_turn: Option<bool>,
}

impl NormalizedResponse {
    fn projection(&self) -> serde_json::Value {
        serde_json::json!({
            "output": self.output,
            "endTurn": self.end_turn,
        })
    }
}

/// The recorded outcome of one model-compatibility evaluation case.
///
/// Every failure (selection, negotiation, invocation) is captured in the
/// record instead of being silently swallowed; a record with
/// `response: None` and no failure fields would be a harness bug.
#[derive(Clone, Debug)]
pub struct ModelCompatRecord {
    /// The provider/model identity the case targeted.
    pub descriptor: ModelDescriptor,
    /// Whether provider selection succeeded.
    pub selected: bool,
    /// The selection failure, when selection failed.
    pub selection_failure: Option<ProviderSelectionError>,
    /// The negotiated capabilities, when negotiation succeeded.
    pub negotiated: Option<ModelCapabilities>,
    /// The negotiation failure (normalized), when negotiation failed.
    pub negotiation_failure: Option<NormalizedNegotiationFailure>,
    /// The invocation failure class, when streaming failed.
    pub invocation_failure: Option<ModelErrorClass>,
    /// The normalized response, when the invocation completed.
    pub response: Option<NormalizedResponse>,
    /// The content digest of the request's semantic content
    /// (provider-independent).
    pub request_digest: Option<ContentDigest>,
}

impl ModelCompatRecord {
    /// The semantic equivalence class of this record, as canonical JSON.
    ///
    /// Excludes the descriptor (provider identity is the differential
    /// variable) and provider-owned capability/response fields; includes
    /// selection and negotiation outcomes (normalized), the invocation
    /// failure class, the normalized response, and the request digest.
    pub fn semantic_projection(&self) -> serde_json::Value {
        serde_json::json!({
            "selected": self.selected,
            "selectionFailure": self
                .selection_failure
                .as_ref()
                .map(std::string::ToString::to_string),
            "negotiated": self.negotiated.as_ref().map(capabilities_projection),
            "negotiationFailure": self
                .negotiation_failure
                .as_ref()
                .map(NormalizedNegotiationFailure::projection),
            "invocationFailure": self.invocation_failure.map(|class| class.key()),
            "response": self.response.as_ref().map(NormalizedResponse::projection),
            "requestDigest": self.request_digest.as_ref().map(ContentDigest::as_str),
        })
    }

    /// The content-addressed digest of the semantic projection.
    pub fn fingerprint_digest(&self) -> Result<ContentDigest, WorkflowContractError> {
        ContentDigest::of(&self.semantic_projection())
    }
}

/// The provider-independent projection of negotiated capabilities.
fn capabilities_projection(capabilities: &ModelCapabilities) -> serde_json::Value {
    serde_json::json!({
        "model": capabilities.model_id,
        "supportedReasoningEfforts": capabilities
            .supported_reasoning_efforts
            .iter()
            .map(std::string::ToString::to_string)
            .collect::<Vec<_>>(),
        "supportsReasoningSummary": capabilities.supports_reasoning_summary,
        "supportsVerbosity": capabilities.supports_verbosity,
        "inputModalities": capabilities
            .input_modalities
            .iter()
            .map(std::string::ToString::to_string)
            .collect::<Vec<_>>(),
        "contextWindow": capabilities.context_window,
        "serviceTiers": &capabilities.service_tiers,
    })
}

/// The canonical projection of a run terminal.
fn terminal_projection(terminal: &RunTerminal) -> serde_json::Value {
    match terminal {
        RunTerminal::Completed => serde_json::json!({ "completed": true }),
        RunTerminal::Paused { node, reason } => serde_json::json!({
            "paused": { "node": node.to_string(), "reason": reason }
        }),
        RunTerminal::Failed { reason } => {
            serde_json::json!({ "failed": { "reason": reason } })
        }
    }
}

/// The canonical projection of an instance status.
fn status_projection(status: WorkflowInstanceStatus) -> serde_json::Value {
    match status {
        WorkflowInstanceStatus::Pending => serde_json::json!("pending"),
        WorkflowInstanceStatus::Running => serde_json::json!("running"),
        WorkflowInstanceStatus::Succeeded => serde_json::json!("succeeded"),
        WorkflowInstanceStatus::Paused => serde_json::json!("paused"),
        WorkflowInstanceStatus::Failed => serde_json::json!("failed"),
        WorkflowInstanceStatus::Cancelled => serde_json::json!("cancelled"),
    }
}

/// The stable key of one evidence kind.
///
/// Histograms key on these strings (not the enum) so projections are
/// canonical and order-independent without requiring ordering on
/// [`EvidenceKind`].
fn evidence_kind_key(kind: EvidenceKind) -> &'static str {
    match kind {
        EvidenceKind::Observation => "observation",
        EvidenceKind::Artifact => "artifact",
        EvidenceKind::Approval => "approval",
        EvidenceKind::Trace => "trace",
        EvidenceKind::TestResult => "test-result",
        EvidenceKind::Recovery => "recovery",
    }
}

/// Builds an evidence histogram from a run's recorded references.
///
/// Public for the workflow harness; keys are the stable kind names.
pub fn evidence_histogram(
    references: &[codex_workflow_contracts::EvidenceReference],
) -> BTreeMap<String, usize> {
    let mut histogram = BTreeMap::new();
    for reference in references {
        *histogram
            .entry(evidence_kind_key(reference.kind).to_string())
            .or_insert(0) += 1;
    }
    histogram
}
