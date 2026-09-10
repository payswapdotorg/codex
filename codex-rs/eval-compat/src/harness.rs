//! Evaluation harnesses (WO-013).
//!
//! Two harnesses drive the merged surfaces end to end, deterministically:
//!
//! - [`ModelCompatHarness`] evaluates one provider/model substitution case
//!   through the real WO-002 selection helpers
//!   (`select_provider_for_descriptor`,
//!   `negotiate_model_capabilities`) and the real session/stream contract,
//!   capturing every failure in the returned record instead of swallowing
//!   it.
//! - [`WorkflowEvalHarness`] evaluates one workflow run through the real
//!   WO-010 lifecycle (select → instantiate → run → verify) over the
//!   in-memory control-plane seams, with action proposals served by a
//!   scripted model provider and environment execution served by a
//!   scripted environment adapter.
//!
//! Both harnesses record outcomes as semantic-fingerprinted records
//! ([`crate::record`]) so runs can be replayed and compared with
//! [`crate::differential`].

use std::collections::BTreeMap;
use std::sync::Arc;

use codex_execution_contracts::BindingPolicy;
use codex_execution_contracts::CapabilityRegistry;
use codex_model_contract::ModelDescriptor;
use codex_model_contract::ModelProvider;
use codex_model_contract::ModelRequest;
use codex_model_contract::ModelResponse;
use codex_model_contract::ModelStreamItem;
use codex_model_provider::negotiate_model_capabilities;
use codex_model_provider::select_provider_for_descriptor;
use codex_workflow_app::InMemoryApprovalSource;
use codex_workflow_app::InMemoryEvidenceStore;
use codex_workflow_app::InMemoryInstanceStore;
use codex_workflow_app::InMemoryVersionStore;
use codex_workflow_app::InstantiateRequest;
use codex_workflow_app::LifecycleDeps;
use codex_workflow_app::RecordingEventSink;
use codex_workflow_app::RunOutcome;
use codex_workflow_app::VerifiedRun;
use codex_workflow_app::WalkConfig;
use codex_workflow_app::WorkflowAppError;
use codex_workflow_app::WorkflowEvent;
use codex_workflow_app::WorkflowLifecycle;
use codex_workflow_app::WorkflowVersionStore;
use codex_workflow_contracts::ContentDigest;
use codex_workflow_contracts::TriggerClass;
use codex_workflow_contracts::TriggerSource;
use codex_workflow_contracts::WorkflowInstanceId;
use codex_workflow_contracts::WorkflowVersion;
use codex_workflow_contracts::WorkflowVersionId;

use crate::action_source::ModelBackedActionSource;
use crate::env_adapter::ScriptedEnvTurn;
use crate::env_adapter::ScriptedEnvironmentAdapter;
use crate::model_fixture::EVAL_MODEL_SLUG;
use crate::model_fixture::ScriptedModelProvider;
use crate::record::ModelCompatCase;
use crate::record::ModelCompatRecord;
use crate::record::ModelErrorClass;
use crate::record::NEUTRAL_PROVIDER_KEY;
use crate::record::NormalizedNegotiationFailure;
use crate::record::NormalizedResponse;
use crate::record::WorkflowRunRecord;
use crate::record::evidence_histogram;

/// The approver identity every evaluation run records approvals under.
pub const EVAL_APPROVER: &str = "eval-compat-approver";

/// The model-compatibility harness.
///
/// Holds the registered providers (the substitution registry) and evaluates
/// cases against them through the real WO-002 selection helpers. The
/// harness never retries, never falls back to another provider, and never
/// drops a failure: everything lands in the
/// [`ModelCompatRecord`](crate::record::ModelCompatRecord).
pub struct ModelCompatHarness {
    providers: Vec<Arc<dyn ModelProvider>>,
}

impl ModelCompatHarness {
    /// Creates the harness over the registered providers.
    pub fn new(providers: Vec<Arc<dyn ModelProvider>>) -> Self {
        Self { providers }
    }

    /// Evaluates one case.
    ///
    /// Pipeline: select the provider for the case's descriptor, negotiate
    /// the case's required features, then (only when both succeeded)
    /// create a session and stream the request. A negotiation failure
    /// short-circuits: the runtime must not invoke a model whose required
    /// features are unsupported (no silent degradation).
    pub async fn run(&self, case: &ModelCompatCase) -> ModelCompatRecord {
        let mut record = ModelCompatRecord {
            descriptor: case.descriptor.clone(),
            selected: false,
            selection_failure: None,
            negotiated: None,
            negotiation_failure: None,
            invocation_failure: None,
            response: None,
            request_digest: None,
        };
        // The request digest is provider-independent: the projection swaps
        // the descriptor's provider key for the neutral marker before
        // digesting.
        let neutral = case.request.for_provider(NEUTRAL_PROVIDER_KEY);
        if let Ok(digest) = ContentDigest::of(&neutral) {
            record.request_digest = Some(digest);
        }
        let provider = match select_provider_for_descriptor(&self.providers, &case.descriptor) {
            Ok(provider) => provider,
            Err(failure) => {
                record.selection_failure = Some(failure);
                return record;
            }
        };
        record.selected = true;
        match negotiate_model_capabilities(provider.as_ref(), &case.model, case.required.clone()) {
            Ok(capabilities) => record.negotiated = Some(capabilities),
            Err(failure) => {
                record.negotiation_failure = Some(NormalizedNegotiationFailure::of(&failure));
                // Negotiation failure short-circuits: no invocation.
                return record;
            }
        }
        let session = match provider.create_session() {
            Ok(session) => session,
            Err(error) => {
                record.invocation_failure = Some(ModelErrorClass::of(&error));
                return record;
            }
        };
        // Re-address the request to the selected provider: the semantic
        // content is shared, only the descriptor's provider identity
        // changes (the frozen provider-swap primitive).
        let request = readdress(&case.request, &case.descriptor);
        let stream = match session.stream(request).await {
            Ok(stream) => stream,
            Err(error) => {
                record.invocation_failure = Some(ModelErrorClass::of(&error));
                return record;
            }
        };
        let mut events = Vec::new();
        let mut terminal_failure = None;
        {
            use futures::StreamExt;
            let mut stream = stream;
            while let Some(item) = stream.next().await {
                match item {
                    ModelStreamItem::Ok(event) => events.push(event),
                    ModelStreamItem::Err(error) => {
                        terminal_failure = Some(error);
                        break;
                    }
                }
            }
        }
        if let Some(error) = terminal_failure {
            record.invocation_failure = Some(ModelErrorClass::of(&error));
            return record;
        }
        let response = ModelResponse::from_events(events);
        record.response = Some(NormalizedResponse {
            output: response.output,
            end_turn: response.end_turn,
        });
        record
    }
}

/// Re-addresses `request` to `descriptor`, sharing all semantic content.
fn readdress(request: &ModelRequest, descriptor: &ModelDescriptor) -> ModelRequest {
    let mut addressed = request.clone();
    addressed.model = descriptor.clone();
    addressed
}

/// The workflow evaluation harness.
///
/// One harness executes one workflow version: it owns a
/// [`WorkflowLifecycle`] over the in-memory control-plane seams, registers
/// the scripted environment adapter, and serves action proposals from a
/// scripted model provider. Every run returns a
/// [`WorkflowRunRecord`] fingerprinted for replay and differential
/// comparison.
pub struct WorkflowEvalHarness {
    lifecycle: WorkflowLifecycle,
    versions: InMemoryVersionStore,
    instances: InMemoryInstanceStore,
    evidence: InMemoryEvidenceStore,
    events: RecordingEventSink,
    adapter: Arc<ScriptedEnvironmentAdapter>,
    provider: Arc<ScriptedModelProvider>,
}

impl WorkflowEvalHarness {
    /// Builds the harness: `provider` serves the action-proposal seam and
    /// `env_script` scripts the environment adapter.
    pub fn new(
        provider: Arc<ScriptedModelProvider>,
        env_script: Vec<ScriptedEnvTurn>,
    ) -> Result<Self, WorkflowAppError> {
        let adapter = Arc::new(ScriptedEnvironmentAdapter::new(env_script)?);
        let mut registry = CapabilityRegistry::new();
        registry.register_adapter(adapter.clone())?;
        let versions = InMemoryVersionStore::new();
        let instances = InMemoryInstanceStore::new();
        let evidence = InMemoryEvidenceStore::new();
        let approvals = InMemoryApprovalSource::approving(EVAL_APPROVER, evidence.clone());
        let actions = ModelBackedActionSource::new(provider.clone(), EVAL_MODEL_SLUG);
        let events = RecordingEventSink::new();
        let lifecycle = WorkflowLifecycle::new(LifecycleDeps {
            versions: Box::new(versions.clone()),
            instances: Box::new(instances.clone()),
            evidence: Box::new(evidence.clone()),
            approvals: Box::new(approvals),
            actions: Box::new(actions),
            events: Box::new(events.clone()),
            registry,
        });
        Ok(Self {
            lifecycle,
            versions,
            instances,
            evidence,
            events,
            adapter,
            provider,
        })
    }

    /// Installs (publishes into the version store) one published version.
    ///
    /// The store only ever inserts; installed records are read back
    /// byte-identically across runs (the read-only guarantee tests assert
    /// this).
    pub fn install(
        &mut self,
        artifact: &codex_workflow_app::PublishedArtifact,
    ) -> Result<WorkflowVersionId, WorkflowAppError> {
        self.versions.publish(artifact.version.clone())
    }

    /// Publishes an arbitrary version record into the version store.
    ///
    /// Integrity tests use this to inject tampered records (same version
    /// id, mutated identity-covered content) and assert they never
    /// execute; ordinary evaluation flows use [`Self::install`].
    pub fn publish_version_record(
        &mut self,
        version: WorkflowVersion,
    ) -> Result<WorkflowVersionId, WorkflowAppError> {
        self.versions.publish(version)
    }

    /// Selects, instantiates, and runs `version`, returning the run record.
    ///
    /// The run uses the default binding policy (all peer environments in
    /// scope, human fallback forbidden) and the given walk budget. The
    /// trigger is the normalized `User` source.
    pub async fn run(
        &mut self,
        version: &WorkflowVersionId,
        walk: WalkConfig,
    ) -> Result<WorkflowRunRecord, WorkflowAppError> {
        self.lifecycle.select_version(version)?;
        self.lifecycle
            .instantiate(InstantiateRequest {
                trigger: Some(TriggerSource {
                    trigger: TriggerClass::User,
                    event_id: None,
                }),
                policy: BindingPolicy::default(),
                resources: Vec::new(),
                walk,
            })
            .await?;
        let outcome = self.lifecycle.run().await?;
        Ok(self.record_for(&outcome))
    }

    /// Reconciles a finished run's durable records (the lifecycle's verify
    /// step): the instance is loaded, its pinned version is loaded, and the
    /// version must still verify end to end.
    pub fn verify(&self, instance: &WorkflowInstanceId) -> Result<VerifiedRun, WorkflowAppError> {
        self.lifecycle.verify_run(instance)
    }

    /// Verifies one evidence reference against the stored payload.
    ///
    /// Returns `Ok(false)` when the locator is unknown; referenced evidence
    /// must still digest to the recorded value.
    pub fn verify_evidence(
        &self,
        reference: &codex_workflow_contracts::EvidenceReference,
    ) -> Result<bool, WorkflowAppError> {
        self.evidence.verify(reference)
    }

    /// The stored payload for `locator`, when present (audit convenience).
    pub fn evidence_payload(&self, locator: &str) -> Option<serde_json::Value> {
        self.evidence.payload(locator)
    }

    /// The stored version record for `version`, when present.
    ///
    /// Read-only probes compare snapshots of this record across runs.
    pub fn version_snapshot(
        &self,
        version: &WorkflowVersionId,
    ) -> Result<Option<WorkflowVersion>, WorkflowAppError> {
        self.versions.load(version)
    }

    /// Whether the lifecycle currently holds any workflow state.
    pub fn is_active(&self) -> bool {
        self.lifecycle.is_active()
    }

    /// Attempts to instantiate without a selected version (the
    /// ordinary-Codex no-workflow path).
    pub async fn instantiate_without_workflow(
        &mut self,
    ) -> Result<codex_workflow_contracts::WorkflowInstance, WorkflowAppError> {
        self.lifecycle
            .instantiate(InstantiateRequest {
                trigger: None,
                policy: BindingPolicy::default(),
                resources: Vec::new(),
                walk: WalkConfig::default(),
            })
            .await
    }

    /// Attempts to run without an active instantiation.
    pub async fn run_without_workflow(&mut self) -> Result<RunOutcome, WorkflowAppError> {
        self.lifecycle.run().await
    }

    /// Number of registered bindings in the harness registry.
    pub fn registered_bindings(&self) -> usize {
        self.lifecycle.registry().bindings().count()
    }

    /// The events observed by the event sink (diagnostics).
    pub fn observed_events(&self) -> Vec<WorkflowEvent> {
        self.events.events()
    }

    /// Number of model invocations that served action proposals.
    pub fn model_request_count(&self) -> usize {
        self.provider.request_count()
    }

    /// The actions executed by the environment adapter, in order.
    pub fn adapter_executions(&self) -> Vec<crate::record::ActionFootprint> {
        self.adapter.executions()
    }

    /// Number of unscripted environment turns remaining.
    pub fn adapter_remaining(&self) -> usize {
        self.adapter.remaining()
    }

    /// Number of unscripted model turns remaining.
    pub fn provider_remaining(&self) -> usize {
        self.provider.remaining()
    }

    /// Number of stored evidence payloads.
    pub fn evidence_count(&self) -> usize {
        self.evidence.len()
    }

    /// Number of stored instance records.
    pub fn instance_count(&self) -> usize {
        self.instances.len()
    }

    /// Number of stored workflow versions.
    pub fn version_count(&self) -> usize {
        self.versions.len()
    }

    /// Builds the run record from a settled outcome.
    fn record_for(&self, outcome: &RunOutcome) -> WorkflowRunRecord {
        let mut recovery_histogram: BTreeMap<String, usize> = BTreeMap::new();
        let mut escalations = 0;
        for event in self.events.events() {
            match event {
                WorkflowEvent::Recovered { strategy, .. } => {
                    *recovery_histogram.entry(strategy).or_insert(0) += 1;
                }
                WorkflowEvent::Escalated { .. } => escalations += 1,
                _ => {}
            }
        }
        WorkflowRunRecord {
            provider_id: self.provider.id().to_string(),
            workflow: outcome.instance.workflow.clone(),
            version: outcome.instance.version.clone(),
            terminal: outcome.terminal.clone(),
            status: outcome.instance.status,
            path: outcome.path.clone(),
            actions: self.adapter.executions(),
            evidence_histogram: evidence_histogram(&outcome.instance.evidence),
            recovery_histogram,
            escalations,
            model_requests: self.provider.request_count(),
        }
    }
}
