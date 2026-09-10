//! In-memory implementations of the trigger-plane seams.
//!
//! These exist for tests, seeding, and hosts that have not yet wired
//! their durable control-plane surfaces. They hold no execution and
//! implement no policy beyond the ports' own contracts: the
//! [`InMemoryTriggerLedger`] is the reference idempotency/audit ledger,
//! [`InMemoryResourceAuthorizer`] is an explicit-allow, default-deny
//! authorization double, [`InMemoryPackageCatalog`] reuses the WO-009
//! repository discovery queries, and [`FixedClock`] makes scheduler
//! time deterministic.
//!
//! Every type here is a shared-state handle: clones observe the same
//! backing records the plane writes, which is what lets tests and hosts
//! audit outcomes while the plane owns the boxed port.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::MutexGuard;
use std::sync::PoisonError;

use codex_workflow_app::InMemoryInstanceStore;
use codex_workflow_app::port::WorkflowInstanceStore;
use codex_workflow_contracts::TriggerClass;
use codex_workflow_contracts::WorkflowDefinitionId;
use codex_workflow_contracts::WorkflowInstanceStatus;
use codex_workflow_contracts::WorkflowRepository;
use codex_workflow_contracts::WorkflowVersion;
use codex_workflow_contracts::WorkflowVersionId;
use codex_workflow_forge::PublishedVersionRef;
use codex_workflow_forge::RepositoryQuery;
use codex_workflow_forge::discover_repositories;
use serde::Deserialize;
use serde::Serialize;

use crate::AwaitRecord;
use crate::FireOutcome;
use crate::IncomingTrigger;
use crate::InstallationStore;
use crate::InstalledConfiguration;
use crate::InstanceControl;
use crate::InstanceSettlement;
use crate::PackageCatalog;
use crate::RebindRecord;
use crate::ResourceAuthorization;
use crate::ResourceAuthorizationRequest;
use crate::ResourceAuthorizer;
use crate::ResumeDirective;
use crate::ScheduleClock;
use crate::TriggerAcceptance;
use crate::TriggerEventKey;
use crate::TriggerLedger;
use crate::WorkflowTriggerError;

/// One durable trigger record of the in-memory ledger.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TriggerRecord {
    /// The workflow the event was accepted for.
    pub workflow: WorkflowDefinitionId,
    /// The control-plane event identity allocated at acceptance.
    pub event_id: String,
    /// The normalized trigger class.
    pub trigger: TriggerClass,
    /// The event's idempotency key.
    pub key: TriggerEventKey,
    /// Attribution label of the event source.
    pub source: String,
    /// Digest of the untrusted external payload, when recorded.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payload_digest: Option<String>,
    /// When the event was first accepted (unix milliseconds).
    pub first_seen_unix_ms: u64,
    /// The settlements of every fire of this event, in order —
    /// including the recorded no-op of duplicate fires.
    pub settlements: Vec<FireOutcome>,
}

/// The backing state of [`InMemoryTriggerLedger`].
#[derive(Clone, Debug, Default)]
struct LedgerState {
    next_event: u64,
    records: BTreeMap<(WorkflowDefinitionId, TriggerEventKey), TriggerRecord>,
    by_event_id: BTreeMap<String, (WorkflowDefinitionId, TriggerEventKey)>,
    awaiting: Vec<AwaitRecord>,
}

/// In-memory trigger ledger: the reference idempotency and audit ledger.
///
/// Acceptance is keyed by `(workflow, event key)` and settles append
/// audit records; await registrations are derived from settlements.
/// Clones share the same backing state.
#[derive(Clone, Debug, Default)]
pub struct InMemoryTriggerLedger {
    state: Arc<Mutex<LedgerState>>,
}

impl InMemoryTriggerLedger {
    /// Creates an empty ledger.
    pub fn new() -> Self {
        Self::default()
    }

    /// Snapshot of every trigger record, ordered by (workflow, key).
    pub fn records(&self) -> Vec<TriggerRecord> {
        self.lock().records.values().cloned().collect()
    }

    /// The trigger record for one (workflow, key), when present.
    pub fn record(
        &self,
        workflow: &WorkflowDefinitionId,
        key: &TriggerEventKey,
    ) -> Option<TriggerRecord> {
        self.lock()
            .records
            .get(&(workflow.clone(), key.clone()))
            .cloned()
    }

    /// Snapshot of the current await registrations.
    pub fn awaits(&self) -> Vec<AwaitRecord> {
        self.lock().awaiting.clone()
    }

    fn lock(&self) -> MutexGuard<'_, LedgerState> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl TriggerLedger for InMemoryTriggerLedger {
    fn accept(
        &mut self,
        workflow: &WorkflowDefinitionId,
        envelope: &IncomingTrigger,
        now_unix_ms: u64,
    ) -> Result<TriggerAcceptance, WorkflowTriggerError> {
        let mut state = self.lock();
        let lookup = (workflow.clone(), envelope.key.clone());
        if let Some(existing) = state.records.get(&lookup) {
            return Ok(TriggerAcceptance::Duplicate {
                event_id: existing.event_id.clone(),
            });
        }
        state.next_event += 1;
        let event_id = format!("evt-{}", state.next_event);
        let record = TriggerRecord {
            workflow: workflow.clone(),
            event_id: event_id.clone(),
            trigger: envelope.trigger,
            key: envelope.key.clone(),
            source: envelope.source.clone(),
            payload_digest: envelope.payload_digest.clone(),
            first_seen_unix_ms: now_unix_ms,
            settlements: Vec::new(),
        };
        state
            .by_event_id
            .insert(event_id.clone(), (workflow.clone(), envelope.key.clone()));
        state.records.insert(lookup, record);
        Ok(TriggerAcceptance::Accepted { event_id })
    }

    fn settle(&mut self, event_id: &str, outcome: FireOutcome) -> Result<(), WorkflowTriggerError> {
        let mut state = self.lock();
        let Some((recorded_workflow, key)) = state.by_event_id.get(event_id).cloned() else {
            return Err(WorkflowTriggerError::UnknownTriggerEvent {
                event_id: event_id.to_string(),
            });
        };
        match &outcome {
            FireOutcome::Started {
                instance,
                status:
                    InstanceSettlement::Paused {
                        node,
                        awaiting: Some(class),
                    },
            } => {
                state.awaiting.push(AwaitRecord {
                    workflow: recorded_workflow.clone(),
                    instance: *instance,
                    node: node.clone(),
                    class: *class,
                });
            }
            FireOutcome::Resumed { instances } => {
                state
                    .awaiting
                    .retain(|record| !instances.contains(&record.instance));
            }
            _ => {}
        }
        if let Some(record) = state.records.get_mut(&(recorded_workflow, key)) {
            record.settlements.push(outcome);
        }
        Ok(())
    }

    fn awaiting(
        &self,
        workflow: &WorkflowDefinitionId,
        class: TriggerClass,
    ) -> Result<Vec<AwaitRecord>, WorkflowTriggerError> {
        let state = self.lock();
        let mut records: Vec<AwaitRecord> = state
            .awaiting
            .iter()
            .filter(|record| &record.workflow == workflow && record.class == class)
            .cloned()
            .collect();
        records.sort_by_key(|record| record.instance);
        Ok(records)
    }
}

/// The backing state of [`InMemoryInstallationStore`].
#[derive(Clone, Debug, Default)]
struct InstallationState {
    configurations: BTreeMap<WorkflowDefinitionId, InstalledConfiguration>,
    audits: Vec<RebindRecord>,
}

/// In-memory installed-configurations store.
///
/// Clones share the same backing records.
#[derive(Clone, Debug, Default)]
pub struct InMemoryInstallationStore {
    state: Arc<Mutex<InstallationState>>,
}

impl InMemoryInstallationStore {
    /// Creates an empty store.
    pub fn new() -> Self {
        Self::default()
    }

    /// Snapshot of the rebind audit history, in order.
    pub fn audits(&self) -> Vec<RebindRecord> {
        self.lock().audits.clone()
    }

    fn lock(&self) -> MutexGuard<'_, InstallationState> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl InstallationStore for InMemoryInstallationStore {
    fn save(&mut self, configuration: InstalledConfiguration) -> Result<(), WorkflowTriggerError> {
        self.lock()
            .configurations
            .insert(configuration.workflow.clone(), configuration);
        Ok(())
    }

    fn load(
        &self,
        workflow: &WorkflowDefinitionId,
    ) -> Result<Option<InstalledConfiguration>, WorkflowTriggerError> {
        Ok(self.lock().configurations.get(workflow).cloned())
    }

    fn list(&self) -> Result<Vec<InstalledConfiguration>, WorkflowTriggerError> {
        Ok(self.lock().configurations.values().cloned().collect())
    }

    fn append_audit(&mut self, record: RebindRecord) -> Result<(), WorkflowTriggerError> {
        self.lock().audits.push(record);
        Ok(())
    }
}

/// In-memory authorization plane: explicit allow list, default deny.
///
/// This is the test double for the host's real connector/account
/// authorization. Denials are decisions, not errors; every check is
/// recorded for audit. Clones share the same backing state.
#[derive(Clone, Debug, Default)]
pub struct InMemoryResourceAuthorizer {
    state: Arc<Mutex<AuthorizerState>>,
}

/// The backing state of [`InMemoryResourceAuthorizer`].
#[derive(Clone, Debug, Default)]
struct AuthorizerState {
    allowed: BTreeMap<(String, String), String>,
    decisions: Vec<ResourceAuthorizationRequest>,
}

impl InMemoryResourceAuthorizer {
    /// Creates a default-deny authorizer with an empty allow list.
    pub fn new() -> Self {
        Self::default()
    }

    /// Allows one resource instance of one type, attributed to `source`.
    pub fn allow(
        &mut self,
        resource_type: impl AsRef<str>,
        resource: impl AsRef<str>,
        source: impl Into<String>,
    ) {
        self.lock().allowed.insert(
            (
                resource_type.as_ref().to_string(),
                resource.as_ref().to_string(),
            ),
            source.into(),
        );
    }

    /// Revokes a previously allowed resource instance.
    pub fn revoke(&mut self, resource_type: impl AsRef<str>, resource: impl AsRef<str>) {
        self.lock().allowed.remove(&(
            resource_type.as_ref().to_string(),
            resource.as_ref().to_string(),
        ));
    }

    /// Snapshot of the authorization requests seen so far, in order.
    pub fn requests(&self) -> Vec<ResourceAuthorizationRequest> {
        self.lock().decisions.clone()
    }

    fn lock(&self) -> MutexGuard<'_, AuthorizerState> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl ResourceAuthorizer for InMemoryResourceAuthorizer {
    fn authorize(
        &mut self,
        request: &ResourceAuthorizationRequest,
    ) -> Result<ResourceAuthorization, WorkflowTriggerError> {
        let lookup = (
            request.binding.resource_type.as_ref().to_string(),
            request.binding.resource.as_ref().to_string(),
        );
        let mut state = self.lock();
        let decision = match state.allowed.get(&lookup) {
            Some(source) => ResourceAuthorization {
                authorized: true,
                source: source.clone(),
                reason: None,
            },
            None => ResourceAuthorization {
                authorized: false,
                source: "in-memory-authorizer".to_string(),
                reason: Some("the resource is not in the authorization list".to_string()),
            },
        };
        state.decisions.push(request.clone());
        Ok(decision)
    }
}

/// In-memory package catalog: repository snapshots plus published
/// versions, reusing the WO-009 repository discovery queries. Clones
/// share the same backing state.
#[derive(Clone, Debug, Default)]
pub struct InMemoryPackageCatalog {
    state: Arc<Mutex<CatalogState>>,
}

/// The backing state of [`InMemoryPackageCatalog`].
#[derive(Clone, Debug, Default)]
struct CatalogState {
    repositories: Vec<WorkflowRepository>,
    versions: BTreeMap<WorkflowDefinitionId, Vec<PublishedVersionRef>>,
    records: BTreeMap<WorkflowVersionId, WorkflowVersion>,
}

impl InMemoryPackageCatalog {
    /// Creates an empty catalog.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a catalog over repository snapshots.
    pub fn with_repositories(repositories: Vec<WorkflowRepository>) -> Self {
        Self {
            state: Arc::new(Mutex::new(CatalogState {
                repositories,
                ..CatalogState::default()
            })),
        }
    }

    /// Publishes a sealed version into the catalog (the release view).
    pub fn publish(&mut self, version: WorkflowVersion) {
        let mut state = self.lock();
        state
            .versions
            .entry(version.definition.id.clone())
            .or_default()
            .push(PublishedVersionRef::of(&version));
        state.records.insert(version.version_id.clone(), version);
    }

    fn lock(&self) -> MutexGuard<'_, CatalogState> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl PackageCatalog for InMemoryPackageCatalog {
    fn search_repositories(
        &self,
        query: &RepositoryQuery,
    ) -> Result<Vec<WorkflowRepository>, WorkflowTriggerError> {
        let repositories = self.lock().repositories.clone();
        discover_repositories(&repositories, query).map_err(WorkflowTriggerError::Forge)
    }

    fn published_versions(
        &self,
        workflow: &WorkflowDefinitionId,
    ) -> Result<Vec<PublishedVersionRef>, WorkflowTriggerError> {
        Ok(self
            .lock()
            .versions
            .get(workflow)
            .cloned()
            .unwrap_or_default())
    }

    fn fetch_version(
        &self,
        version: &WorkflowVersionId,
    ) -> Result<Option<WorkflowVersion>, WorkflowTriggerError> {
        Ok(self.lock().records.get(version).cloned())
    }
}

/// In-memory instance control: performs the legal `Paused -> Running`
/// transition on the shared instance store and records the directives
/// it received.
#[derive(Clone, Debug)]
pub struct InMemoryInstanceControl {
    instances: InMemoryInstanceStore,
    directives: Arc<Mutex<Vec<ResumeDirective>>>,
}

impl InMemoryInstanceControl {
    /// Creates a control plane backed by `instances`.
    pub fn new(instances: InMemoryInstanceStore) -> Self {
        Self {
            instances,
            directives: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Snapshot of the resume directives received so far, in order.
    pub fn directives(&self) -> Vec<ResumeDirective> {
        self.directives
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    fn record_directive(&mut self, directive: &ResumeDirective) {
        self.directives
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(directive.clone());
    }
}

impl InstanceControl for InMemoryInstanceControl {
    fn resume(
        &mut self,
        directive: &ResumeDirective,
    ) -> Result<codex_workflow_contracts::WorkflowInstance, WorkflowTriggerError> {
        let mut instance = self.instances.load(&directive.instance)?.ok_or_else(|| {
            WorkflowTriggerError::InstanceUnavailable {
                instance: directive.instance.to_string(),
            }
        })?;
        if instance.status != WorkflowInstanceStatus::Paused {
            return Err(WorkflowTriggerError::IllegalResume {
                instance: directive.instance.to_string(),
                current: format!("{:?}", instance.status),
            });
        }
        instance.status = WorkflowInstanceStatus::Running;
        self.instances.save(instance.clone())?;
        self.record_directive(directive);
        Ok(instance)
    }
}

/// A deterministic clock for tests and seeding.
#[derive(Clone, Debug)]
pub struct FixedClock {
    now: Arc<Mutex<u64>>,
}

impl FixedClock {
    /// Creates a clock fixed at `start_unix_ms`.
    pub fn new(start_unix_ms: u64) -> Self {
        Self {
            now: Arc::new(Mutex::new(start_unix_ms)),
        }
    }

    /// Moves the clock to `unix_ms`.
    pub fn advance_to(&self, unix_ms: u64) {
        *self.lock() = unix_ms;
    }

    /// Advances the clock by `ms`.
    pub fn advance_by(&self, ms: u64) {
        *self.lock() += ms;
    }

    fn lock(&self) -> MutexGuard<'_, u64> {
        self.now.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl ScheduleClock for FixedClock {
    fn now_unix_ms(&self) -> u64 {
        *self.lock()
    }
}
