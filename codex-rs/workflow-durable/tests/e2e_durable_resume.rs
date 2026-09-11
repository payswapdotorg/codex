//! MWO-003 end-to-end: cross-plane restart — persisted run position,
//! drop-and-reload, startup reconciliation, resume continuation, and
//! trigger dedupe across the whole scenario.
//!
//! The scenario runs on the durable stores over one root directory:
//! the trigger plane fires a resumable workflow (a step, a wait for a
//! human event, a second step) to a pause, the whole application is
//! then dropped (every store closed), everything is reloaded from disk,
//! the startup sweep reconciles, the host resumes through the control
//! seam, and the lifecycle continues the run to completion. The proof
//! has four legs:
//!
//! 1. the pause checkpoint's persisted position (the pending re-entry
//!    point) survives the drop-and-reload;
//! 2. the post-crash signature — a `Running` instance with no live run
//!    (a resume that happened, then the application died before any
//!    continuation) — is reconciled to `Paused` with recovery evidence,
//!    and the sweep is idempotent;
//! 3. an explicit resume plus `WorkflowLifecycle::resume_run` continues
//!    the walk past the pause point to `Succeeded`, the terminal
//!    settlement discards the position, and the pinned version still
//!    verifies end to end;
//! 4. trigger dedupe still holds across the restarts.

#![allow(clippy::expect_used)]
#![allow(clippy::unwrap_used)]

mod common;

use pretty_assertions::assert_eq;

use codex_workflow_app::RunPositionStore;
use codex_workflow_app::RunTerminal;
use codex_workflow_app::WorkflowInstanceStore;
use codex_workflow_contracts::IrNodeId;
use codex_workflow_contracts::TriggerClass;
use codex_workflow_contracts::TriggerSource;
use codex_workflow_contracts::WorkflowInstanceStatus;
use codex_workflow_durable::DurableStores;
use codex_workflow_triggers::FireOutcome;
use codex_workflow_triggers::InstanceControl;
use codex_workflow_triggers::InstanceSettlement;
use codex_workflow_triggers::ResumeDirective;
use common::cleanup;
use common::human_event;
use common::install_request;
use common::sealed_resumable_version;
use common::temp_root;
use common::user_event;

fn node(value: &str) -> IrNodeId {
    IrNodeId::parse(value).expect("node id")
}

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

/// One resume directive for the paused `instance` at the wait node.
fn signoff_directive(instance: codex_workflow_contracts::WorkflowInstanceId) -> ResumeDirective {
    ResumeDirective {
        instance,
        node: node("wait-signoff"),
        trigger: TriggerSource {
            trigger: TriggerClass::HumanEvent,
            event_id: Some("evt-1".to_string()),
        },
    }
}

#[tokio::test]
async fn cross_plane_restart_persists_position_reconciles_and_resumes_to_completion() {
    let root = temp_root("resume-cross-plane");
    let version = sealed_resumable_version("triage-report", (1, 0, 0));
    // Phase 1: install and fire to a pause at the wait node.
    let paused = {
        let mut harness = common::durable_harness(&root);
        harness
            .plane
            .install(install_request(version.clone()))
            .expect("install");
        let first = harness
            .plane
            .ingest(&mut harness.lifecycle, user_event("triage-report", "u1"))
            .await
            .expect("fire u1");
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
        let paused = started_instance(&first);
        // The pause checkpoint persisted the pending re-entry point.
        let position = harness
            .stores
            .run_positions
            .load(&paused)
            .expect("load position")
            .expect("persisted at the pause checkpoint");
        assert_eq!(position.walk.current, Some(node("step-002")));
        assert_eq!(
            position.walk.path,
            vec![node("step-001"), node("wait-signoff")]
        );
        paused
    };
    // The whole application is dropped here: every store closed.

    // Phase 2: reload everything from disk. The control-plane state and
    // the persisted position survived; the cleanly-paused instance
    // needs no reconciliation.
    {
        let stores = DurableStores::open(&root).expect("reload stores");
        assert_eq!(
            stores
                .run_positions
                .load(&paused)
                .expect("load position")
                .expect("the position survived the drop-and-reload")
                .walk
                .current,
            Some(node("step-002"))
        );
        let mut harness = common::harness_over(stores);
        assert_eq!(
            harness
                .lifecycle
                .reconcile_startup(&harness.stores.instances.records())
                .expect("sweep the cleanly-paused store"),
            Vec::new(),
            "a clean pause reconciles nothing"
        );
        // The crash signature: the host resumes through the control
        // seam, then the application dies before any continuation.
        harness
            .stores
            .control
            .resume(&signoff_directive(paused))
            .expect("resume through the control seam");
        assert_eq!(
            harness
                .stores
                .instances
                .load(&paused)
                .expect("load record")
                .expect("resumed record")
                .status,
            WorkflowInstanceStatus::Running
        );
    }
    // Dropped again: a `Running` record with no live run anywhere.

    // Phase 3: reload, sweep, and verify the recovery state.
    {
        let stores = DurableStores::open(&root).expect("reload stores");
        let mut harness = common::harness_over(stores);
        let evidence_before = harness
            .stores
            .instances
            .load(&paused)
            .expect("load record")
            .expect("running record")
            .evidence
            .len();
        let reconciled = harness
            .lifecycle
            .reconcile_startup(&harness.stores.instances.records())
            .expect("sweep");
        assert_eq!(
            reconciled,
            vec![paused],
            "the post-crash signature is reconciled"
        );
        let recovered = harness
            .stores
            .instances
            .load(&paused)
            .expect("load record")
            .expect("reconciled record");
        assert_eq!(recovered.status, WorkflowInstanceStatus::Paused);
        assert_eq!(
            recovered.evidence.len(),
            evidence_before + 1,
            "the sweep appends exactly one recovery trace"
        );
        assert!(
            harness
                .lifecycle
                .reconcile_startup(&harness.stores.instances.records())
                .expect("second sweep")
                .is_empty(),
            "the second sweep is a no-op"
        );
        assert_eq!(
            harness.stores.control.directives().len(),
            1,
            "the earlier resume directive is journaled and survives"
        );

        // Phase 4: an explicit resume, then the continuation runs to
        // completion.
        harness
            .stores
            .control
            .resume(&signoff_directive(paused))
            .expect("resume through the control seam");
        let outcome = harness
            .lifecycle
            .resume_run(&paused)
            .await
            .expect("resume continuation");
        assert_eq!(outcome.terminal, RunTerminal::Completed);
        assert_eq!(outcome.instance.status, WorkflowInstanceStatus::Succeeded);
        assert_eq!(
            outcome.path,
            vec![node("step-001"), node("wait-signoff"), node("step-002")],
            "the continuation progresses past the pause point"
        );
        // The terminal settlement discarded the position.
        assert!(
            harness
                .stores
                .run_positions
                .load(&paused)
                .expect("load position")
                .is_none()
        );
        // The durable records still verify end to end (the pinned
        // version re-loads and re-verifies its integrity).
        let verified = harness.lifecycle.verify_run(&paused).expect("verify run");
        assert_eq!(verified.instance.status, WorkflowInstanceStatus::Succeeded);
        assert_eq!(verified.version, version.version_id);

        // Phase 5: trigger dedupe still holds across the restarts.
        let duplicate = harness
            .plane
            .ingest(&mut harness.lifecycle, user_event("triage-report", "u1"))
            .await
            .expect("re-ingest u1");
        assert_eq!(duplicate.len(), 1);
        assert_eq!(duplicate[0].outcome, FireOutcome::Duplicate);
        assert_eq!(duplicate[0].event_id, "evt-1");
    }
    cleanup(&root);
}

#[tokio::test]
async fn a_plane_resumed_instance_continues_through_the_lifecycle() {
    let root = temp_root("resume-plane-composition");
    let version = sealed_resumable_version("triage-report", (1, 0, 0));

    // The trigger plane owns the fire/resume correlation; the host
    // application owns the continuation (the composition MWO-003
    // enables): fire to a pause, fire the awaited human event (the
    // plane resumes `Paused -> Running` through the durable control
    // seam), then drive the lifecycle's resume continuation.
    let mut harness = common::durable_harness(&root);
    harness
        .plane
        .install(install_request(version.clone()))
        .expect("install");
    let first = harness
        .plane
        .ingest(&mut harness.lifecycle, user_event("triage-report", "u1"))
        .await
        .expect("fire u1");
    let paused = started_instance(&first);

    // The awaited human event fires: the plane resumes the paused
    // instance through the control seam (`Paused -> Running`) and
    // settles `Resumed`.
    let resumed = harness
        .plane
        .ingest(&mut harness.lifecycle, human_event("h1"))
        .await
        .expect("fire h1");
    assert_eq!(
        resumed[0].outcome,
        FireOutcome::Resumed {
            instances: vec![paused]
        }
    );

    // The host application continues the resumed run: the lifecycle
    // rehydrates the persisted position and completes the walk.
    let outcome = harness
        .lifecycle
        .resume_run(&paused)
        .await
        .expect("resume continuation");
    assert_eq!(outcome.terminal, RunTerminal::Completed);
    assert_eq!(outcome.instance.status, WorkflowInstanceStatus::Succeeded);
    assert_eq!(
        outcome.path,
        vec![node("step-001"), node("wait-signoff"), node("step-002")]
    );
    assert!(
        harness
            .stores
            .run_positions
            .load(&paused)
            .expect("load position")
            .is_none(),
        "the terminal settlement discarded the position"
    );
    let verified = harness.lifecycle.verify_run(&paused).expect("verify run");
    assert_eq!(verified.instance.status, WorkflowInstanceStatus::Succeeded);
    cleanup(&root);
}

#[tokio::test]
async fn ordinary_codex_behavior_is_untouched_without_a_workflow() {
    let root = temp_root("resume-ordinary");

    // No workflow is installed and none is active: the reconciliation
    // sweep is a no-op, the fire is not eligible, no run exists, and
    // nothing is persisted.
    let mut harness = common::durable_harness(&root);
    assert!(!harness.lifecycle.is_active());
    assert!(
        harness
            .lifecycle
            .reconcile_startup(&harness.stores.instances.records())
            .expect("sweep")
            .is_empty()
    );
    let reports = harness
        .plane
        .ingest(&mut harness.lifecycle, user_event("triage-report", "u1"))
        .await
        .expect("fire an uninstalled workflow");
    assert_eq!(reports.len(), 1);
    assert!(matches!(
        reports[0].outcome,
        FireOutcome::NotEligible { .. }
    ));
    assert!(
        harness.lifecycle.run().await.is_err_and(|error| matches!(
            error,
            codex_workflow_app::WorkflowAppError::NoActiveWorkflow
        )),
        "no active workflow: the ordinary-Codex no-op path"
    );
    assert!(harness.stores.run_positions.is_empty());
    assert!(harness.stores.instances.is_empty());
    cleanup(&root);
}
