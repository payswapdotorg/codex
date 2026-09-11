//! MWO-001 end-to-end: a workflow-app lifecycle scenario over the
//! durable stores, drop-and-reload, and parity with the in-memory
//! baseline.
//!
//! The scenario is static and mirrors the WO-010 E2E suite's shapes: a
//! directly-sealed single-step workflow (no capabilities, so the run
//! completes without adapters) is published, selected, instantiated,
//! run to `Succeeded`, and reconciled through
//! [`WorkflowLifecycle::verify_run`], with the durable version,
//! instance, and evidence stores boxed in place of the in-memory
//! doubles. Evidence payloads are then driven through the port exactly
//! as the lifecycle's own `store_evidence` path does.
//!
//! The proof has three legs:
//!
//! 1. the durable run settles the same control-plane state as an
//!    identical in-memory run (statuses, evidence references —
//!    locators and digests included);
//! 2. dropping every durable struct and re-opening the same root
//!    reconstructs that state identically;
//! 3. evidence references still verify and the store keeps minting
//!    fresh locators after the restart.

#![allow(clippy::expect_used)]
#![allow(clippy::unwrap_used)]

mod common;

use pretty_assertions::assert_eq;

use codex_workflow_app::EvidenceStore;
use codex_workflow_app::InstantiateRequest;
use codex_workflow_app::WorkflowInstanceStore;
use codex_workflow_app::WorkflowVersionStore;
use codex_workflow_contracts::EvidenceKind;
use codex_workflow_contracts::EvidenceReference;
use codex_workflow_contracts::WorkflowInstanceStatus;
use codex_workflow_durable::DurableStores;
use common::cleanup;
use common::memory_harness;
use common::sealed_step_version;
use common::temp_root;

/// The evidence payloads the scenario stores, in order.
fn payloads() -> Vec<serde_json::Value> {
    vec![
        serde_json::json!({ "kind": "observation", "text": "the tracker is open" }),
        serde_json::json!({ "kind": "testResult", "suite": "smoke", "passed": true }),
    ]
}

/// Drives the evidence plane and instance record the way the
/// lifecycle's own evidence path does: store each payload, append the
/// reference to the instance, save the instance.
async fn drive_lifecycle_and_evidence(
    lifecycle: &mut codex_workflow_app::WorkflowLifecycle,
    evidence: &mut dyn EvidenceStore,
    instances: &mut dyn WorkflowInstanceStore,
    version_id: &codex_workflow_contracts::WorkflowVersionId,
) -> codex_workflow_app::VerifiedRun {
    lifecycle
        .select_version(version_id)
        .expect("select version");
    let instance = lifecycle
        .instantiate(InstantiateRequest::default())
        .await
        .expect("instantiate");
    let outcome = lifecycle.run().await.expect("run");
    assert_eq!(outcome.instance.status, WorkflowInstanceStatus::Succeeded);

    let mut record = instances
        .load(&instance.instance_id)
        .expect("load instance")
        .expect("instance stored");
    for payload in payloads() {
        let reference = evidence
            .store(EvidenceKind::Observation, &payload)
            .expect("store evidence");
        record.record_evidence(reference);
    }
    let promoted = evidence
        .promote(
            EvidenceKind::Trace,
            "codex-env-adapters/browser/7",
            &"ab".repeat(32),
        )
        .expect("promote");
    record.record_evidence(promoted);
    instances.save(record).expect("save instance");
    lifecycle
        .verify_run(&instance.instance_id)
        .expect("verify run")
}

#[tokio::test]
async fn durable_lifecycle_matches_the_in_memory_baseline_and_survives_restart() {
    let root = temp_root("lifecycle");
    let version = sealed_step_version("release-notes", (1, 4, 2));
    let version_id = version.version_id.clone();

    // 1. The in-memory baseline run.
    let mut memory = memory_harness();
    let baseline = {
        memory.versions.publish(version.clone()).expect("publish");
        let mut evidence = memory.evidence.clone();
        let mut instances = memory.instances.clone();
        drive_lifecycle_and_evidence(
            &mut memory.lifecycle,
            &mut evidence,
            &mut instances,
            &version_id,
        )
        .await
    };

    // 2. The durable run over the same scenario, with the durable
    //    stores boxed in place of the in-memory doubles.
    let durable_run = {
        let mut harness = common::durable_harness(&root);
        harness
            .stores
            .versions
            .publish(version.clone())
            .expect("publish durable");
        let mut evidence = harness.stores.evidence.clone();
        let mut instances = harness.stores.instances.clone();
        let verified = drive_lifecycle_and_evidence(
            &mut harness.lifecycle,
            &mut evidence,
            &mut instances,
            &version_id,
        )
        .await;
        // Parity: the settled instance state matches the baseline
        // except for the allocated instance identity.
        assert_eq!(verified.instance.status, baseline.instance.status);
        assert_eq!(verified.instance.workflow, baseline.instance.workflow);
        assert_eq!(verified.instance.version, baseline.instance.version);
        assert_eq!(verified.instance.evidence, baseline.instance.evidence);
        assert_eq!(verified.version, baseline.version);
        verified
    };

    // 3. Drop every durable struct and re-open from the same root.
    let mut stores = DurableStores::open(&root).expect("reopen durable stores");
    let reloaded = stores
        .instances
        .load(&durable_run.instance.instance_id)
        .expect("load instance")
        .expect("instance survived restart");
    assert_eq!(
        reloaded.status,
        WorkflowInstanceStatus::Succeeded,
        "instance statuses are preserved across restart"
    );
    assert_eq!(reloaded.evidence, durable_run.instance.evidence);
    assert_eq!(
        stores
            .versions
            .load(&version_id)
            .expect("load version")
            .expect("version survived restart"),
        version,
        "the sealed version record re-observes identically"
    );

    // Every STORED evidence reference still verifies against its
    // payload; the promoted adapter reference carries no payload by
    // contract (its bytes stay in the adapter's own journal), exactly
    // like the in-memory double.
    let mut verified_references = 0;
    let mut promoted_seen = false;
    for reference in &reloaded.evidence {
        if stores.evidence.payload(&reference.locator).is_some() {
            assert!(
                stores.evidence.verify(reference).expect("verify"),
                "{reference:?} must verify after restart"
            );
            verified_references += 1;
        } else {
            promoted_seen = true;
        }
    }
    assert_eq!(verified_references, payloads().len());
    assert!(
        promoted_seen,
        "the promoted adapter reference is part of the durable instance record"
    );
    assert!(
        reloaded
            .evidence
            .iter()
            .any(|reference: &EvidenceReference| reference.kind == EvidenceKind::Trace),
        "the promoted adapter reference is recorded on the instance"
    );
    assert_eq!(stores.evidence.len(), payloads().len());

    // The evidence sequence resumes after the restart: the next store
    // mints the next locator, never reusing one.
    let next = stores
        .evidence
        .store(
            EvidenceKind::Artifact,
            &serde_json::json!({ "kind": "artifact" }),
        )
        .expect("store after restart");
    assert_eq!(
        next.locator,
        format!("workflow-app/artifact/{}", payloads().len() + 1)
    );
    cleanup(&root);
}
