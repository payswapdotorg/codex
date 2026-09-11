//! Durable trigger ledger tests: parity with the in-memory reference
//! ledger, drop-and-reload dedupe, and torn-tail tolerance.

use pretty_assertions::assert_eq;

use super::DurableTriggerLedger;
use crate::testutil::cleanup;
use crate::testutil::temp_root;
use crate::testutil::workflow_id;
use codex_workflow_contracts::IrNodeId;
use codex_workflow_contracts::TriggerClass;
use codex_workflow_contracts::WorkflowDefinitionId;
use codex_workflow_contracts::WorkflowInstanceId;
use codex_workflow_triggers::AwaitRecord;
use codex_workflow_triggers::FireOutcome;
use codex_workflow_triggers::InMemoryTriggerLedger;
use codex_workflow_triggers::IncomingTrigger;
use codex_workflow_triggers::InstanceSettlement;
use codex_workflow_triggers::TriggerAcceptance;
use codex_workflow_triggers::TriggerEventKey;
use codex_workflow_triggers::TriggerLedger;
use codex_workflow_triggers::TriggerRecord;
use codex_workflow_triggers::WorkflowTriggerError;

/// A direct user trigger envelope for `workflow` with `key`.
fn event(workflow: &str, key: &str) -> IncomingTrigger {
    IncomingTrigger {
        trigger: TriggerClass::User,
        key: TriggerEventKey::parse(key).expect("event key"),
        occurred_at_unix_ms: 0,
        source: "user:test".to_string(),
        routing: None,
        target: Some(workflow_id(workflow)),
        payload_digest: None,
    }
}

/// A settlement that paused at a wait node awaiting `class`.
fn paused_at(instance: WorkflowInstanceId, class: TriggerClass) -> FireOutcome {
    FireOutcome::Started {
        instance,
        status: InstanceSettlement::Paused {
            node: IrNodeId::parse("wait-signoff").expect("node id"),
            awaiting: Some(class),
        },
    }
}

/// The await record a paused settlement of `instance` registers.
fn await_record(workflow: &WorkflowDefinitionId, instance: WorkflowInstanceId) -> AwaitRecord {
    AwaitRecord {
        workflow: workflow.clone(),
        instance,
        node: IrNodeId::parse("wait-signoff").expect("node id"),
        class: TriggerClass::HumanEvent,
    }
}

#[test]
fn acceptance_dedupe_and_settlement_match_the_in_memory_ledger() {
    let root = temp_root("ledger-parity");
    let mut durable = DurableTriggerLedger::open(root.join("triggers.jsonl")).expect("open");
    let mut memory = InMemoryTriggerLedger::new();
    let workflow = workflow_id("triage-report");

    // First sight allocates evt-1; a re-sight is a recorded duplicate.
    let first = durable
        .accept(&workflow, &event("triage-report", "k-1"), 100)
        .expect("accept");
    let first_memory = memory
        .accept(&workflow, &event("triage-report", "k-1"), 100)
        .expect("accept");
    assert_eq!(first, first_memory);
    assert_eq!(
        first,
        TriggerAcceptance::Accepted {
            event_id: "evt-1".to_string()
        }
    );
    let duplicate = durable
        .accept(&workflow, &event("triage-report", "k-1"), 200)
        .expect("accept");
    let duplicate_memory = memory
        .accept(&workflow, &event("triage-report", "k-1"), 200)
        .expect("accept");
    assert_eq!(duplicate, duplicate_memory);
    assert_eq!(
        duplicate,
        TriggerAcceptance::Duplicate {
            event_id: "evt-1".to_string()
        }
    );

    // A different workflow accepting the same event key allocates its
    // own acceptance (evt-2), exactly like the reference ledger.
    let other = workflow_id("other-report");
    let second = durable
        .accept(&other, &event("other-report", "k-1"), 300)
        .expect("accept");
    let second_memory = memory
        .accept(&other, &event("other-report", "k-1"), 300)
        .expect("accept");
    assert_eq!(second, second_memory);
    assert_eq!(
        second,
        TriggerAcceptance::Accepted {
            event_id: "evt-2".to_string()
        }
    );

    // Settling an unknown event id is the same error on both ledgers.
    assert!(matches!(
        (
            durable.settle("evt-404", FireOutcome::Duplicate),
            memory.settle("evt-404", FireOutcome::Duplicate)
        ),
        (
            Err(WorkflowTriggerError::UnknownTriggerEvent { .. }),
            Err(WorkflowTriggerError::UnknownTriggerEvent { .. })
        )
    ));

    // Records (including duplicate no-op settlements) match.
    durable
        .settle("evt-1", FireOutcome::Duplicate)
        .expect("settle durable");
    memory
        .settle("evt-1", FireOutcome::Duplicate)
        .expect("settle memory");
    assert_eq!(durable.records(), memory.records());
    let key = TriggerEventKey::parse("k-1").expect("key");
    assert_eq!(
        durable.record(&workflow, &key),
        memory.record(&workflow, &key)
    );
    assert_eq!(durable.awaits(), memory.awaits());
    cleanup(&root);
}

#[test]
fn await_registration_and_resolution_match_the_in_memory_ledger() {
    let root = temp_root("ledger-awaits");
    let mut durable = DurableTriggerLedger::open(root.join("triggers.jsonl")).expect("open");
    let mut memory = InMemoryTriggerLedger::new();
    let workflow = workflow_id("triage-report");
    let instance = WorkflowInstanceId::generate();

    let acceptance = durable
        .accept(&workflow, &event("triage-report", "k-1"), 100)
        .expect("accept");
    memory
        .accept(&workflow, &event("triage-report", "k-1"), 100)
        .expect("accept");
    let TriggerAcceptance::Accepted { event_id } = acceptance else {
        panic!("first sight must be accepted");
    };

    // A paused settlement registers the await correlation.
    durable
        .settle(&event_id, paused_at(instance, TriggerClass::HumanEvent))
        .expect("settle durable");
    memory
        .settle(&event_id, paused_at(instance, TriggerClass::HumanEvent))
        .expect("settle memory");
    assert_eq!(
        durable
            .awaiting(&workflow, TriggerClass::HumanEvent)
            .expect("awaiting durable"),
        vec![await_record(&workflow, instance)]
    );
    assert_eq!(
        durable
            .awaiting(&workflow, TriggerClass::HumanEvent)
            .expect("awaiting durable"),
        memory
            .awaiting(&workflow, TriggerClass::HumanEvent)
            .expect("awaiting memory")
    );

    // A resume settlement clears the registrations of the resumed
    // instances and is recorded as audit.
    let resumed = FireOutcome::Resumed {
        instances: vec![instance],
    };
    durable
        .settle(&event_id, resumed.clone())
        .expect("settle durable");
    memory.settle(&event_id, resumed).expect("settle memory");
    assert_eq!(
        durable
            .awaiting(&workflow, TriggerClass::HumanEvent)
            .expect("awaiting durable"),
        memory
            .awaiting(&workflow, TriggerClass::HumanEvent)
            .expect("awaiting memory")
    );
    assert_eq!(durable.records(), memory.records());
    cleanup(&root);
}

#[test]
fn dedupe_and_settlements_survive_drop_and_reload() {
    let root = temp_root("ledger-reload");
    let path = root.join("triggers.jsonl");
    let workflow = workflow_id("triage-report");
    let instance = WorkflowInstanceId::generate();
    let records;
    {
        let mut ledger = DurableTriggerLedger::open(&path).expect("open");
        let first = ledger
            .accept(&workflow, &event("triage-report", "k-1"), 100)
            .expect("accept");
        assert_eq!(
            first,
            TriggerAcceptance::Accepted {
                event_id: "evt-1".to_string()
            }
        );
        ledger
            .settle("evt-1", paused_at(instance, TriggerClass::HumanEvent))
            .expect("settle paused");
        let second = ledger
            .accept(&workflow, &event("triage-report", "k-2"), 200)
            .expect("accept");
        assert_eq!(
            second,
            TriggerAcceptance::Accepted {
                event_id: "evt-2".to_string()
            }
        );
        ledger
            .settle("evt-2", FireOutcome::Duplicate)
            .expect("settle duplicate");
        records = ledger.records();
    }

    // Re-open from the same root: the full audit trail re-observes.
    let mut ledger = DurableTriggerLedger::open(&path).expect("reopen");
    assert_eq!(
        ledger.records(),
        records,
        "records and settlements survive restart"
    );
    assert_eq!(
        ledger
            .awaiting(&workflow, TriggerClass::HumanEvent)
            .expect("awaiting"),
        vec![await_record(&workflow, instance)],
        "await registrations survive restart"
    );

    // The core M4 property: an accepted key never accepts twice, even
    // across restarts.
    assert_eq!(
        ledger
            .accept(&workflow, &event("triage-report", "k-1"), 999)
            .expect("dedupe"),
        TriggerAcceptance::Duplicate {
            event_id: "evt-1".to_string()
        }
    );
    assert_eq!(
        ledger
            .accept(&workflow, &event("triage-report", "k-2"), 999)
            .expect("dedupe"),
        TriggerAcceptance::Duplicate {
            event_id: "evt-2".to_string()
        }
    );

    // Event id allocation resumes without reusing a committed id.
    let third = ledger
        .accept(&workflow, &event("triage-report", "k-3"), 300)
        .expect("accept");
    assert_eq!(
        third,
        TriggerAcceptance::Accepted {
            event_id: "evt-3".to_string()
        }
    );
    assert_eq!(ledger.records().len(), 3);
    cleanup(&root);
}

#[test]
fn truncated_tail_settlement_is_skipped_deterministically() {
    let root = temp_root("ledger-torn");
    let path = root.join("triggers.jsonl");
    let workflow = workflow_id("triage-report");
    let instance = WorkflowInstanceId::generate();
    {
        let mut ledger = DurableTriggerLedger::open(&path).expect("open");
        ledger
            .accept(&workflow, &event("triage-report", "k-1"), 100)
            .expect("accept one");
        ledger
            .accept(&workflow, &event("triage-report", "k-2"), 200)
            .expect("accept two");
        ledger
            .settle("evt-1", paused_at(instance, TriggerClass::HumanEvent))
            .expect("settle");
    }

    // Truncate the last journal line mid-record: the settlement of
    // evt-1 was never committed.
    let bytes = std::fs::read(&path).expect("read journal");
    std::fs::write(&path, &bytes[..bytes.len() - 2]).expect("truncate");

    let ledger = DurableTriggerLedger::open(&path).expect("reload with torn tail");
    let records = ledger.records();
    assert_eq!(records.len(), 2, "both acceptances were committed");
    let settled = records
        .iter()
        .find(|record| record.key.as_ref() == "k-1")
        .expect("k-1 accepted")
        .settlements
        .len();
    assert_eq!(
        settled, 0,
        "the torn settlement is skipped, never half-applied"
    );
    assert!(
        ledger
            .awaiting(&workflow, TriggerClass::HumanEvent)
            .expect("awaiting")
            .is_empty(),
        "await state derives only from committed settlements"
    );
    // The dedupe guarantee still holds for both committed acceptances.
    let mut ledger = ledger;
    assert_eq!(
        ledger
            .accept(&workflow, &event("triage-report", "k-1"), 999)
            .expect("dedupe"),
        TriggerAcceptance::Duplicate {
            event_id: "evt-1".to_string()
        }
    );
    assert_eq!(
        ledger
            .accept(&workflow, &event("triage-report", "k-2"), 999)
            .expect("dedupe"),
        TriggerAcceptance::Duplicate {
            event_id: "evt-2".to_string()
        }
    );
    cleanup(&root);
}

#[test]
fn committed_ledger_corruption_is_a_deterministic_error() {
    let root = temp_root("ledger-corrupt");
    let path = root.join("triggers.jsonl");
    let workflow = workflow_id("triage-report");
    {
        let mut ledger = DurableTriggerLedger::open(&path).expect("open");
        ledger
            .accept(&workflow, &event("triage-report", "k-1"), 100)
            .expect("accept");
        ledger
            .accept(&workflow, &event("triage-report", "k-2"), 200)
            .expect("accept");
    }
    // Corrupt the FIRST (committed) line: never a silent skip.
    let text = std::fs::read_to_string(&path).expect("read journal");
    let rest = text.lines().skip(1).collect::<Vec<&str>>().join("\n");
    std::fs::write(&path, format!("@@garbage@@\n{rest}\n")).expect("corrupt");

    let error = DurableTriggerLedger::open(&path).expect_err("corruption must fail");
    assert!(
        matches!(error, crate::DurableStoreError::Corrupt { .. }),
        "committed ledger corruption is deterministic, got {error:?}"
    );
    // The file was NOT silently repaired.
    assert!(
        std::fs::read_to_string(&path)
            .expect("reread")
            .starts_with("@@garbage@@")
    );
    cleanup(&root);
}

#[test]
fn duplicate_acceptance_in_the_journal_is_a_deterministic_error() {
    let root = temp_root("ledger-double-accept");
    let path = root.join("triggers.jsonl");
    let workflow = workflow_id("triage-report");
    {
        let mut ledger = DurableTriggerLedger::open(&path).expect("open");
        ledger
            .accept(&workflow, &event("triage-report", "k-1"), 100)
            .expect("accept");
    }
    // Replay a crafted journal that accepts the same key twice: the
    // ledger never writes this, so reading it back is corruption.
    let text = std::fs::read_to_string(&path).expect("read journal");
    std::fs::write(&path, format!("{text}{text}")).expect("duplicate the acceptance");

    let error = DurableTriggerLedger::open(&path).expect_err("double acceptance must fail");
    assert!(
        matches!(error, crate::DurableStoreError::Corrupt { .. }),
        "a replayed duplicate acceptance is deterministic, got {error:?}"
    );
    cleanup(&root);
}

#[test]
fn record_snapshots_expose_the_full_audit_shape() {
    let root = temp_root("ledger-shape");
    let mut ledger = DurableTriggerLedger::open(root.join("triggers.jsonl")).expect("open");
    let workflow = workflow_id("triage-report");
    let acceptance = ledger
        .accept(&workflow, &event("triage-report", "k-1"), 1_234)
        .expect("accept");
    let TriggerAcceptance::Accepted { event_id } = acceptance else {
        panic!("first sight must be accepted");
    };
    ledger
        .settle(&event_id, FireOutcome::Duplicate)
        .expect("settle");

    let records = ledger.records();
    assert_eq!(records.len(), 1);
    let expected = TriggerRecord {
        workflow: workflow.clone(),
        event_id: event_id.clone(),
        trigger: TriggerClass::User,
        key: TriggerEventKey::parse("k-1").expect("key"),
        source: "user:test".to_string(),
        payload_digest: None,
        first_seen_unix_ms: 1_234,
        settlements: vec![FireOutcome::Duplicate],
    };
    assert_eq!(records[0], expected);
    let key = TriggerEventKey::parse("k-1").expect("key");
    assert_eq!(ledger.record(&workflow, &key), Some(expected));
    assert!(ledger.awaits().is_empty());
    cleanup(&root);
}
