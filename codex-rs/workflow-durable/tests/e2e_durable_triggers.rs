//! MWO-001 end-to-end: a workflow-triggers plane scenario over the
//! durable stores, drop-and-reload, and parity with the in-memory
//! baseline.
//!
//! The scenario mirrors the WO-011 E2E suite's resume scenario: a
//! directly-sealed wait workflow (User start trigger, HumanEvent
//! resume trigger) is installed, fired to a pause, double-fired
//! (recorded no-op), resumed through the control seam, and fired again
//! (a second instance pinning the same immutable version). The durable
//! ledger, installation store, instance control, version store, and
//! instance store are boxed in place of the in-memory doubles.
//!
//! The proof has four legs:
//!
//! 1. the durable plane settles the same fire outcomes as the
//!    in-memory baseline;
//! 2. dropping every durable struct and re-opening the same root
//!    reconstructs the full control-plane state (ledger records with
//!    settlements, installation, instance statuses, await state,
//!    directives);
//! 3. **dedupe survives restart**: re-ingesting an already-accepted
//!    event after the reload is still a recorded no-op — the core M4
//!    idempotency property;
//! 4. the re-opened plane keeps working (a fresh event allocates the
//!    next event id and starts a new instance).

#![allow(clippy::expect_used)]
#![allow(clippy::unwrap_used)]

mod common;

use pretty_assertions::assert_eq;

use codex_workflow_app::WorkflowInstanceStore;
use codex_workflow_contracts::TriggerClass;
use codex_workflow_contracts::WorkflowInstanceStatus;
use codex_workflow_durable::DurableStores;
use codex_workflow_triggers::FireOutcome;
use codex_workflow_triggers::InstallationStore;
use codex_workflow_triggers::InstanceSettlement;
use codex_workflow_triggers::TriggerLedger;
use codex_workflow_triggers::TriggerRecord;
use common::cleanup;
use common::human_event;
use common::install_request;
use common::memory_harness;
use common::sealed_wait_version;
use common::temp_root;
use common::user_event;

/// Extracts the single started instance of a one-report fire.
fn started_instance(
    reports: &[codex_workflow_triggers::TriggerReport],
) -> codex_workflow_contracts::WorkflowInstanceId {
    assert_eq!(reports.len(), 1);
    match &reports[0].outcome {
        FireOutcome::Started { instance, .. } => *instance,
        other => panic!("expected a started instance, got {other:?}"),
    }
}

/// The identity-free shape of one settlement: instance ids are
/// allocated per run, so baseline/durable parity compares settlement
/// shapes, not the embedded identities.
fn settlement_shape(outcome: &FireOutcome) -> String {
    match outcome {
        FireOutcome::Started { status, .. } => match status {
            InstanceSettlement::Completed => "started-completed".to_string(),
            InstanceSettlement::Paused { awaiting, .. } => {
                format!("started-paused-{awaiting:?}")
            }
            InstanceSettlement::Failed { reason } => format!("started-failed-{reason}"),
        },
        FireOutcome::Duplicate => "duplicate".to_string(),
        FireOutcome::NotEligible { diagnostics } => format!("not-eligible-{}", diagnostics.len()),
        FireOutcome::InstantiationFailed { diagnostics } => {
            format!("instantiation-failed-{}", diagnostics.len())
        }
        FireOutcome::Resumed { instances } => format!("resumed-{}", instances.len()),
    }
}

/// The settlement-shape sequence of one ledger record.
fn settlement_shapes(record: &TriggerRecord) -> Vec<String> {
    record.settlements.iter().map(settlement_shape).collect()
}

#[tokio::test]
async fn durable_trigger_plane_matches_the_baseline_and_dedupe_survives_restart() {
    let root = temp_root("triggers");
    let workflow = codex_workflow_contracts::WorkflowDefinitionId::parse("triage-report")
        .expect("definition id");
    let version = sealed_wait_version("triage-report", (1, 0, 0));

    // 1. The in-memory baseline run of the full fire scenario.
    let baseline = {
        let mut harness = memory_harness();
        harness
            .plane
            .install(install_request(version.clone()))
            .expect("install baseline");
        let first = harness
            .plane
            .ingest(&mut harness.lifecycle, user_event("triage-report", "u1"))
            .await
            .expect("fire u1");
        let paused_instance = started_instance(&first);
        assert!(matches!(
            &first[0].outcome,
            FireOutcome::Started {
                status: InstanceSettlement::Paused {
                    awaiting: Some(TriggerClass::HumanEvent),
                    ..
                },
                ..
            }
        ));
        // The double fire is one recorded no-op.
        let duplicate = harness
            .plane
            .ingest(&mut harness.lifecycle, user_event("triage-report", "u1"))
            .await
            .expect("double fire");
        assert_eq!(duplicate.len(), 1);
        assert_eq!(duplicate[0].outcome, FireOutcome::Duplicate);
        // The human event resumes the paused instance.
        let resumed = harness
            .plane
            .ingest(&mut harness.lifecycle, human_event("h1"))
            .await
            .expect("fire h1");
        assert_eq!(
            resumed[0].outcome,
            FireOutcome::Resumed {
                instances: vec![paused_instance]
            }
        );
        // A retry starts a second instance pinning the same version.
        let retry = harness
            .plane
            .ingest(&mut harness.lifecycle, user_event("triage-report", "u2"))
            .await
            .expect("fire u2");
        let retry_instance = started_instance(&retry);
        (
            paused_instance,
            retry_instance,
            harness.ledger.records(),
            harness.instances.records(),
        )
    };
    let (baseline_paused, baseline_retry, baseline_records, baseline_instances) = baseline;
    assert_eq!(baseline_records.len(), 3);
    assert_eq!(baseline_instances.len(), 2);
    assert_eq!(
        baseline_instances
            .iter()
            .find(|record| record.instance_id == baseline_paused)
            .expect("resumed instance")
            .status,
        WorkflowInstanceStatus::Running
    );
    assert_eq!(
        baseline_instances
            .iter()
            .find(|record| record.instance_id == baseline_retry)
            .expect("retry instance")
            .status,
        WorkflowInstanceStatus::Paused
    );

    // 2. The durable run over the same scenario.
    let (durable_paused, durable_retry) = {
        let mut harness = common::durable_harness(&root);
        harness
            .plane
            .install(install_request(version.clone()))
            .expect("install durable");
        let first = harness
            .plane
            .ingest(&mut harness.lifecycle, user_event("triage-report", "u1"))
            .await
            .expect("fire u1 durable");
        let paused_instance = started_instance(&first);
        let duplicate = harness
            .plane
            .ingest(&mut harness.lifecycle, user_event("triage-report", "u1"))
            .await
            .expect("double fire durable");
        assert_eq!(duplicate[0].outcome, FireOutcome::Duplicate);
        let resumed = harness
            .plane
            .ingest(&mut harness.lifecycle, human_event("h1"))
            .await
            .expect("fire h1 durable");
        assert_eq!(
            resumed[0].outcome,
            FireOutcome::Resumed {
                instances: vec![paused_instance]
            }
        );
        let retry = harness
            .plane
            .ingest(&mut harness.lifecycle, user_event("triage-report", "u2"))
            .await
            .expect("fire u2 durable");
        let retry_instance = started_instance(&retry);

        // Parity: the same fire outcomes and settled statuses as the
        // baseline (modulo the allocated instance identities).
        assert_eq!(
            harness.stores.ledger.records().len(),
            baseline_records.len(),
            "the durable ledger settles the same audit trail"
        );
        for (durable, baseline_record) in harness
            .stores
            .ledger
            .records()
            .iter()
            .zip(baseline_records.iter())
        {
            assert_eq!(durable.workflow, baseline_record.workflow);
            assert_eq!(durable.trigger, baseline_record.trigger);
            assert_eq!(durable.key, baseline_record.key);
            assert_eq!(durable.source, baseline_record.source);
            assert_eq!(
                durable.first_seen_unix_ms,
                baseline_record.first_seen_unix_ms
            );
            assert_eq!(
                settlement_shapes(durable),
                settlement_shapes(baseline_record),
                "the durable ledger settles the same outcome shapes as the baseline"
            );
        }
        let durable_instances = harness.stores.instances.records();
        assert_eq!(durable_instances.len(), baseline_instances.len());
        for record in &durable_instances {
            let baseline_record = baseline_instances
                .iter()
                .find(|candidate| candidate.status == record.status)
                .expect("matching baseline status");
            assert_eq!(record.workflow, baseline_record.workflow);
            assert_eq!(record.version, baseline_record.version);
            assert_eq!(record.status, baseline_record.status);
        }
        (paused_instance, retry_instance)
    };

    // 3. Drop every durable struct and re-open from the same root.
    let stores = DurableStores::open(&root).expect("reopen durable stores");
    let records = stores.ledger.records();
    assert_eq!(records.len(), 3, "the full audit trail survives restart");
    for (durable, baseline_record) in records.iter().zip(baseline_records.iter()) {
        assert_eq!(durable.key, baseline_record.key);
        assert_eq!(durable.event_id, baseline_record.event_id);
        assert_eq!(
            settlement_shapes(durable),
            settlement_shapes(baseline_record)
        );
    }
    // The resumed instance kept Running; the retry instance kept
    // Paused; the installation is loadable.
    let resumed_record = stores
        .instances
        .load(&durable_paused)
        .expect("load resumed")
        .expect("resumed instance survived restart");
    assert_eq!(resumed_record.status, WorkflowInstanceStatus::Running);
    let retry_record = stores
        .instances
        .load(&durable_retry)
        .expect("load retry")
        .expect("retry instance survived restart");
    assert_eq!(retry_record.status, WorkflowInstanceStatus::Paused);
    let configuration = stores
        .installations
        .load(&workflow)
        .expect("load installation")
        .expect("installation survived restart");
    assert_eq!(configuration.version_id, version.version_id);
    assert!(
        stores
            .ledger
            .awaiting(&workflow, TriggerClass::HumanEvent)
            .expect("awaiting")
            .iter()
            .any(|record| record.instance == durable_retry),
        "the retry instance's await registration survives restart"
    );
    assert_eq!(stores.control.directives().len(), 1);
    assert_eq!(stores.control.directives()[0].instance, durable_paused);

    // 4. Dedupe survives restart: re-wiring a plane over the re-opened
    //    stores and re-ingesting accepted events is a recorded no-op,
    //    while a fresh event allocates the next event id.
    let mut harness = common::harness_over(stores);
    // The forge install registry is in-process host state; after a
    // restart the host re-hydrates it from the durable control plane.
    // Re-installing over the re-opened durable stores re-records the
    // same pin, configuration, and version record.
    let rehydrated = harness
        .plane
        .install(install_request(version.clone()))
        .expect("re-hydrate the forge registry");
    assert_eq!(rehydrated.version_id, version.version_id);
    assert_eq!(rehydrated.workflow, workflow);
    let duplicate = harness
        .plane
        .ingest(&mut harness.lifecycle, user_event("triage-report", "u1"))
        .await
        .expect("re-ingest u1");
    assert_eq!(duplicate.len(), 1);
    assert_eq!(duplicate[0].outcome, FireOutcome::Duplicate);
    assert_eq!(duplicate[0].event_id, "evt-1");
    let duplicate = harness
        .plane
        .ingest(&mut harness.lifecycle, user_event("triage-report", "u2"))
        .await
        .expect("re-ingest u2");
    assert_eq!(duplicate[0].outcome, FireOutcome::Duplicate);
    assert_eq!(duplicate[0].event_id, "evt-3");
    let fresh = harness
        .plane
        .ingest(&mut harness.lifecycle, user_event("triage-report", "u3"))
        .await
        .expect("fire u3");
    assert_eq!(fresh[0].event_id, "evt-4", "event id allocation resumes");
    let third = started_instance(&fresh);
    let third_record = harness
        .stores
        .instances
        .load(&third)
        .expect("load third")
        .expect("third instance stored");
    assert_eq!(third_record.status, WorkflowInstanceStatus::Paused);
    let final_records: Vec<TriggerRecord> = harness.stores.ledger.records();
    assert_eq!(final_records.len(), 4);
    cleanup(&root);
}
