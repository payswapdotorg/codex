//! The durable trigger idempotency and audit ledger.
//!
//! [`DurableTriggerLedger`] implements the WO-011 [`TriggerLedger`]
//! port over one append-only JSONL journal. Every first acceptance and
//! every settlement is a committed journal line (fsynced before the
//! port method returns), and the ledger state — accepted records with
//! their settlements, the event-id index, and the derived await
//! registrations — is rebuilt by replaying the journal at open time.
//!
//! The replay applies exactly the mutations the in-memory double
//! performs, so a drop-and-reload observes identical records,
//! identical `evt-<n>` allocation (the counter resumes from the highest
//! committed event number), and — the core M4 property — an event key
//! that was accepted once keeps answering
//! [`TriggerAcceptance::Duplicate`] forever, across restarts.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::MutexGuard;
use std::sync::PoisonError;

use codex_workflow_contracts::TriggerClass;
use codex_workflow_contracts::WorkflowDefinitionId;
use codex_workflow_triggers::AwaitRecord;
use codex_workflow_triggers::FireOutcome;
use codex_workflow_triggers::IncomingTrigger;
use codex_workflow_triggers::InstanceSettlement;
use codex_workflow_triggers::TriggerAcceptance;
use codex_workflow_triggers::TriggerEventKey;
use codex_workflow_triggers::TriggerLedger;
use codex_workflow_triggers::TriggerRecord;
use codex_workflow_triggers::WorkflowTriggerError;

use crate::DurableStoreError;
use crate::journal::Journal;
use crate::journal::load_journal;

/// The backing state of [`DurableTriggerLedger`] — identical in shape
/// and behavior to the in-memory double's.
#[derive(Clone, Debug, Default)]
struct LedgerState {
    next_event: u64,
    records: BTreeMap<(WorkflowDefinitionId, TriggerEventKey), TriggerRecord>,
    by_event_id: BTreeMap<String, (WorkflowDefinitionId, TriggerEventKey)>,
    awaiting: Vec<AwaitRecord>,
}

/// The journal payload of one first acceptance.
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct AcceptedRecord {
    record: TriggerRecord,
}

/// The journal payload of one settlement.
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SettledRecord {
    event_id: String,
    outcome: FireOutcome,
}

/// One journal event of the durable ledger.
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
enum LedgerJournalEvent {
    /// A first acceptance (allocates and records an event id).
    Accepted(AcceptedRecord),
    /// A settlement (audit plus await tracking).
    Settled(SettledRecord),
}

/// The durable trigger ledger: file-backed idempotency and audit.
///
/// A shared-state handle like the in-memory double: clones observe the
/// same backing journal.
#[derive(Clone, Debug)]
pub struct DurableTriggerLedger {
    inner: Arc<LedgerInner>,
}

/// The owned backing of [`DurableTriggerLedger`].
#[derive(Debug)]
struct LedgerInner {
    state: Mutex<LedgerState>,
    journal: Journal,
}

impl DurableTriggerLedger {
    /// Opens the ledger at journal file `path`, creating it as empty
    /// when absent.
    ///
    /// The journal is replayed in order; a torn tail is repaired by the
    /// documented journal policy, and a structurally corrupt committed
    /// line is a deterministic construction failure.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, DurableStoreError> {
        let path = path.as_ref();
        let events: Vec<LedgerJournalEvent> = load_journal(path)?;
        let mut state = LedgerState::default();
        for event in events {
            match event {
                LedgerJournalEvent::Accepted(AcceptedRecord { record }) => {
                    replay_acceptance(&mut state, path, record)?;
                }
                LedgerJournalEvent::Settled(SettledRecord { event_id, outcome }) => {
                    replay_settlement(&mut state, path, &event_id, outcome)?;
                }
            }
        }
        let journal = Journal::open(path)?;
        Ok(Self {
            inner: Arc::new(LedgerInner {
                state: Mutex::new(state),
                journal,
            }),
        })
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
        self.inner
            .state
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }
}

impl TriggerLedger for DurableTriggerLedger {
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
        state.records.insert(lookup.clone(), record.clone());
        let entry = LedgerJournalEvent::Accepted(AcceptedRecord { record });
        if let Err(error) = self
            .inner
            .journal
            .append(&entry)
            .map_err(DurableStoreError::into_trigger_error)
        {
            // Roll the in-memory state back so it keeps matching the
            // durable journal; the failed acceptance records nothing.
            state.records.remove(&lookup);
            state.by_event_id.remove(&event_id);
            state.next_event -= 1;
            return Err(error);
        }
        Ok(TriggerAcceptance::Accepted { event_id })
    }

    fn settle(&mut self, event_id: &str, outcome: FireOutcome) -> Result<(), WorkflowTriggerError> {
        let mut state = self.lock();
        let Some((recorded_workflow, key)) = state.by_event_id.get(event_id).cloned() else {
            return Err(WorkflowTriggerError::UnknownTriggerEvent {
                event_id: event_id.to_string(),
            });
        };
        let awaiting_before = state.awaiting.clone();
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
        if let Some(record) = state
            .records
            .get_mut(&(recorded_workflow.clone(), key.clone()))
        {
            record.settlements.push(outcome.clone());
        }
        let entry = LedgerJournalEvent::Settled(SettledRecord {
            event_id: event_id.to_string(),
            outcome,
        });
        if let Err(error) = self
            .inner
            .journal
            .append(&entry)
            .map_err(DurableStoreError::into_trigger_error)
        {
            // Roll the in-memory state back so it keeps matching the
            // durable journal; the failed settlement records nothing.
            state.awaiting = awaiting_before;
            if let Some(record) = state.records.get_mut(&(recorded_workflow, key)) {
                record.settlements.pop();
            }
            return Err(error);
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

/// Replays one acceptance into `state`, validating that the journal
/// only carries events this ledger minted.
fn replay_acceptance(
    state: &mut LedgerState,
    path: &Path,
    record: TriggerRecord,
) -> Result<(), DurableStoreError> {
    let number = record
        .event_id
        .strip_prefix("evt-")
        .and_then(|digits| digits.parse::<u64>().ok())
        .ok_or_else(|| {
            DurableStoreError::corrupt(
                path.display(),
                format!(
                    "event id `{}` was not minted by this ledger",
                    record.event_id
                ),
            )
        })?;
    let key = (record.workflow.clone(), record.key.clone());
    if state.records.contains_key(&key) {
        return Err(DurableStoreError::corrupt(
            path.display(),
            format!(
                "event `{}` was accepted twice in the journal",
                record.event_id
            ),
        ));
    }
    state.next_event = state.next_event.max(number);
    state
        .by_event_id
        .insert(record.event_id.clone(), key.clone());
    state.records.insert(key, record);
    Ok(())
}

/// Replays one settlement into `state`, applying exactly the in-memory
/// double's mutation.
fn replay_settlement(
    state: &mut LedgerState,
    path: &Path,
    event_id: &str,
    outcome: FireOutcome,
) -> Result<(), DurableStoreError> {
    let Some((recorded_workflow, key)) = state.by_event_id.get(event_id).cloned() else {
        return Err(DurableStoreError::corrupt(
            path.display(),
            format!("journal settles unknown event `{event_id}`"),
        ));
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

#[cfg(test)]
#[path = "trigger_ledger_tests.rs"]
mod tests;
