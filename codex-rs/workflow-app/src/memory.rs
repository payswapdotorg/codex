//! In-memory implementations of the application-layer seams.
//!
//! These exist for tests, seeding, and hosts that have not yet wired their
//! durable control-plane surfaces. They hold no execution and implement no
//! policy beyond the ports' own contracts; the Codex application replaces
//! them with durable implementations (thread-store-backed instances,
//! rollout-trace-backed evidence, the native approval plane).
//!
//! Every type here is a shared-state handle: clones observe the same
//! backing records the lifecycle writes, which is what lets tests and
//! hosts read run results while the lifecycle owns the boxed port.

use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::MutexGuard;
use std::sync::PoisonError;

use codex_execution_contracts::Action;
use codex_execution_contracts::CapabilityBindingId;
use codex_workflow_contracts::CapabilityId;
use codex_workflow_contracts::ContentDigest;
use codex_workflow_contracts::EvidenceKind;
use codex_workflow_contracts::EvidenceReference;
use codex_workflow_contracts::IrNodeId;
use codex_workflow_contracts::WorkflowInstance;
use codex_workflow_contracts::WorkflowInstanceId;
use codex_workflow_contracts::WorkflowVersion;
use codex_workflow_contracts::WorkflowVersionId;

use crate::WorkflowAppError;
use crate::WorkflowEvent;
use crate::approval::ApprovalEvidence;
use crate::approval::ApprovalRequest;
use crate::port::ActionRequest;
use crate::port::ApprovalSource;
use crate::port::EventSink;
use crate::port::EvidenceStore;
use crate::port::StepActionSource;
use crate::port::WorkflowInstanceStore;
use crate::port::WorkflowVersionStore;

/// In-memory workflow version records.
#[derive(Clone, Debug, Default)]
pub struct InMemoryVersionStore {
    state: Arc<Mutex<BTreeMap<WorkflowVersionId, WorkflowVersion>>>,
}

impl InMemoryVersionStore {
    /// Creates an empty store.
    pub fn new() -> Self {
        Self::default()
    }

    /// Number of stored versions.
    pub fn len(&self) -> usize {
        self.lock().len()
    }

    /// Whether the store is empty.
    pub fn is_empty(&self) -> bool {
        self.lock().is_empty()
    }

    fn lock(&self) -> MutexGuard<'_, BTreeMap<WorkflowVersionId, WorkflowVersion>> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl WorkflowVersionStore for InMemoryVersionStore {
    fn publish(&mut self, version: WorkflowVersion) -> Result<WorkflowVersionId, WorkflowAppError> {
        let id = version.version_id.clone();
        self.lock().insert(id.clone(), version);
        Ok(id)
    }

    fn load(
        &self,
        version: &WorkflowVersionId,
    ) -> Result<Option<WorkflowVersion>, WorkflowAppError> {
        Ok(self.lock().get(version).cloned())
    }
}

/// In-memory workflow instance records.
#[derive(Clone, Debug, Default)]
pub struct InMemoryInstanceStore {
    state: Arc<Mutex<BTreeMap<WorkflowInstanceId, WorkflowInstance>>>,
}

impl InMemoryInstanceStore {
    /// Creates an empty store.
    pub fn new() -> Self {
        Self::default()
    }

    /// Number of stored instances.
    pub fn len(&self) -> usize {
        self.lock().len()
    }

    /// Whether the store is empty.
    pub fn is_empty(&self) -> bool {
        self.lock().is_empty()
    }

    /// Snapshot of the instance record for `instance`, when present.
    pub fn snapshot(&self, instance: &WorkflowInstanceId) -> Option<WorkflowInstance> {
        self.lock().get(instance).cloned()
    }

    /// Snapshot of all instance records, ordered by instance id.
    pub fn records(&self) -> Vec<WorkflowInstance> {
        self.lock().values().cloned().collect()
    }

    fn lock(&self) -> MutexGuard<'_, BTreeMap<WorkflowInstanceId, WorkflowInstance>> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl WorkflowInstanceStore for InMemoryInstanceStore {
    fn create(&mut self, instance: WorkflowInstance) -> Result<(), WorkflowAppError> {
        let mut state = self.lock();
        if state.contains_key(&instance.instance_id) {
            return Err(WorkflowAppError::InstanceAlreadyExists {
                instance: instance.instance_id.to_string(),
            });
        }
        state.insert(instance.instance_id, instance);
        Ok(())
    }

    fn save(&mut self, instance: WorkflowInstance) -> Result<(), WorkflowAppError> {
        self.lock().insert(instance.instance_id, instance);
        Ok(())
    }

    fn load(
        &self,
        instance: &WorkflowInstanceId,
    ) -> Result<Option<WorkflowInstance>, WorkflowAppError> {
        Ok(self.lock().get(instance).cloned())
    }
}

/// The backing state of [`InMemoryEvidenceStore`].
#[derive(Clone, Debug, Default)]
struct EvidenceState {
    next_sequence: u64,
    payloads: BTreeMap<String, (EvidenceKind, serde_json::Value)>,
}

/// In-memory evidence plane: stores payloads and allocates deterministic
/// locators and digests.
#[derive(Clone, Debug, Default)]
pub struct InMemoryEvidenceStore {
    state: Arc<Mutex<EvidenceState>>,
}

impl InMemoryEvidenceStore {
    /// Creates an empty store.
    pub fn new() -> Self {
        Self::default()
    }

    /// Number of stored payloads.
    pub fn len(&self) -> usize {
        self.lock().payloads.len()
    }

    /// Whether the store is empty.
    pub fn is_empty(&self) -> bool {
        self.lock().payloads.is_empty()
    }

    /// Verifies one stored payload against its reference digest.
    ///
    /// Returns `Ok(false)` when the locator is unknown. This is the
    /// evidence-plane side of the "verify" step: referenced evidence must
    /// still digest to the recorded value.
    pub fn verify(&self, reference: &EvidenceReference) -> Result<bool, WorkflowAppError> {
        match self.lock().payloads.get(&reference.locator) {
            Some((_, payload)) => Ok(ContentDigest::of(payload)? == reference.digest),
            None => Ok(false),
        }
    }

    /// The stored payload for `locator`, when present (audit convenience).
    pub fn payload(&self, locator: &str) -> Option<serde_json::Value> {
        self.lock()
            .payloads
            .get(locator)
            .map(|(_, payload)| payload.clone())
    }

    fn lock(&self) -> MutexGuard<'_, EvidenceState> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl EvidenceStore for InMemoryEvidenceStore {
    fn store(
        &mut self,
        kind: EvidenceKind,
        payload: &serde_json::Value,
    ) -> Result<EvidenceReference, WorkflowAppError> {
        let mut state = self.lock();
        state.next_sequence += 1;
        let locator = format!("workflow-app/{}/{}", slug(kind), state.next_sequence);
        let digest = ContentDigest::of(payload)?;
        state
            .payloads
            .insert(locator.clone(), (kind, payload.clone()));
        Ok(EvidenceReference {
            kind,
            locator,
            digest,
        })
    }

    fn promote(
        &mut self,
        kind: EvidenceKind,
        locator: &str,
        digest_hex: &str,
    ) -> Result<EvidenceReference, WorkflowAppError> {
        let digest = ContentDigest::try_from(format!("sha256:{digest_hex}"))?;
        Ok(EvidenceReference {
            kind,
            locator: locator.to_string(),
            digest,
        })
    }
}

/// The backing state of [`InMemoryApprovalSource`].
#[derive(Clone, Debug, Default)]
struct ApprovalState {
    denied: BTreeSet<CapabilityBindingId>,
    requests: Vec<ApprovalRequest>,
    records: Vec<ApprovalEvidence>,
}

/// In-memory approval plane: approves every request except bindings
/// explicitly denied, recording each decision as approval evidence
/// through the shared evidence store.
#[derive(Clone, Debug)]
pub struct InMemoryApprovalSource {
    approver: String,
    state: Arc<Mutex<ApprovalState>>,
    evidence: InMemoryEvidenceStore,
}

impl InMemoryApprovalSource {
    /// Creates a source that approves everything as `approver`, recording
    /// decisions as evidence through `evidence`.
    pub fn approving(approver: impl Into<String>, evidence: InMemoryEvidenceStore) -> Self {
        Self {
            approver: approver.into(),
            state: Arc::new(Mutex::new(ApprovalState::default())),
            evidence,
        }
    }

    /// Marks a binding as denied: approving it fails.
    pub fn deny(&mut self, binding: CapabilityBindingId) {
        self.lock().denied.insert(binding);
    }

    /// Snapshot of the approval requests seen so far, in order.
    pub fn requests(&self) -> Vec<ApprovalRequest> {
        self.lock().requests.clone()
    }

    /// Snapshot of the approval decisions recorded so far, in order.
    pub fn records(&self) -> Vec<ApprovalEvidence> {
        self.lock().records.clone()
    }

    fn lock(&self) -> MutexGuard<'_, ApprovalState> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl ApprovalSource for InMemoryApprovalSource {
    fn approve(
        &mut self,
        request: &ApprovalRequest,
    ) -> Result<EvidenceReference, WorkflowAppError> {
        let mut state = self.lock();
        if state.denied.contains(&request.binding) {
            return Err(WorkflowAppError::ApprovalDenied {
                binding: request.binding.to_string(),
                reason: "denied by the in-memory approval source".to_string(),
            });
        }
        let record = ApprovalEvidence::approved(request, self.approver.clone());
        state.requests.push(request.clone());
        state.records.push(record.clone());
        drop(state);
        let payload = serde_json::to_value(&record)?;
        self.evidence.store(EvidenceKind::Approval, &payload)
    }
}

/// Append-only event recorder.
///
/// Clones share the same backing log.
#[derive(Clone, Debug, Default)]
pub struct RecordingEventSink {
    events: Arc<Mutex<Vec<WorkflowEvent>>>,
}

impl RecordingEventSink {
    /// Creates an empty sink.
    pub fn new() -> Self {
        Self::default()
    }

    /// Snapshot of the recorded events, in order.
    pub fn events(&self) -> Vec<WorkflowEvent> {
        self.events
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    /// Number of recorded events.
    pub fn len(&self) -> usize {
        self.events
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .len()
    }

    /// Whether no events were recorded.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl EventSink for RecordingEventSink {
    fn record(&mut self, event: WorkflowEvent) {
        self.events
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(event);
    }
}

/// One scripted action: the action to propose for one step capability.
#[derive(Clone, Debug)]
pub struct ScriptedAction {
    /// The step node the action is proposed for.
    pub node: IrNodeId,
    /// The capability the action serves.
    pub capability: CapabilityId,
    /// The proposed action.
    pub action: Action,
}

impl ScriptedAction {
    /// Builds a scripted action from string identifiers.
    pub fn new(
        node: impl Into<String>,
        capability: impl Into<String>,
        action: Action,
    ) -> Result<Self, WorkflowAppError> {
        Ok(Self {
            node: IrNodeId::parse(node)?,
            capability: CapabilityId::parse(capability)?,
            action,
        })
    }
}

/// A scripted action source: serves queued actions per (step node,
/// capability), in order, and refuses anything unscripted.
///
/// This is the in-flight stand-in for the Codex agent/model loop at the
/// action proposal seam. Clones share the same script.
#[derive(Clone, Debug, Default)]
pub struct ScriptedActionSource {
    script: Arc<Mutex<Vec<ScriptedAction>>>,
}

impl ScriptedActionSource {
    /// Creates a source serving `script` in order.
    pub fn new(script: Vec<ScriptedAction>) -> Self {
        Self {
            script: Arc::new(Mutex::new(script)),
        }
    }

    /// Number of unscripted (remaining) actions.
    pub fn remaining(&self) -> usize {
        self.script
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .len()
    }
}

impl StepActionSource for ScriptedActionSource {
    fn action_for(&mut self, request: &ActionRequest) -> Result<Action, WorkflowAppError> {
        let mut script = self.script.lock().unwrap_or_else(PoisonError::into_inner);
        let position = script
            .iter()
            .position(|scripted| {
                scripted.node == request.node
                    && scripted.capability == request.requirement.capability
            })
            .ok_or_else(|| WorkflowAppError::ActionUnavailable {
                node: request.node.to_string(),
                capability: request.requirement.capability.as_ref().to_string(),
                reason: "no scripted action for this step capability".to_string(),
            })?;
        Ok(script.remove(position).action)
    }
}

/// The locator slug for one evidence kind.
fn slug(kind: EvidenceKind) -> &'static str {
    match kind {
        EvidenceKind::Observation => "observation",
        EvidenceKind::Artifact => "artifact",
        EvidenceKind::Approval => "approval",
        EvidenceKind::Trace => "trace",
        EvidenceKind::TestResult => "test-result",
        EvidenceKind::Recovery => "recovery",
    }
}
