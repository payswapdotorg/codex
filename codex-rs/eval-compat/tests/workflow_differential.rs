//! Workflow-plane differential and reproducibility tests (WO-013).
//!
//! Every scenario is static and end to end: the fixture workflow is
//! published through the real teaching → compile → approve → publish →
//! install pipeline, runs execute through the real WO-010 lifecycle
//! (select → instantiate → run → verify) with action proposals served by
//! scripted model providers, and environment execution is served by the
//! scripted adapter. The scenarios cover:
//!
//! - provider substitution equivalence on the same immutable workflow
//!   version (the differential golden path);
//! - deterministic replay (byte-identical fingerprints, equal records);
//! - injected action divergence being detected, never silent;
//! - transient failure recovering through the frozen pipeline with
//!   recovery evidence;
//! - permanent failure escalating with a settled, persisted record;
//! - tampered versions never executing (integrity gate);
//! - installed versions staying read-only across runs;
//! - walk budgets bounding long runs.

// The workspace clippy.toml allows `expect`/`unwrap` in test code; the
// same intent is spelled out at file level here (matching the WO-010
// end-to-end test precedent) for plain helper functions.
#![allow(clippy::expect_used)]
#![allow(clippy::unwrap_used)]

use std::sync::Arc;

use codex_eval_compat::ScriptedEnvTurn;
use codex_eval_compat::ScriptedModelProvider;
use codex_eval_compat::ScriptedTurn;
use codex_eval_compat::WorkflowEvalHarness;
use codex_eval_compat::compare_workflow_runs;
use codex_eval_compat::equivalent_capabilities;
use codex_eval_compat::published_fixture;
use codex_execution_contracts::FailureKind;
use codex_workflow_app::RunTerminal;
use codex_workflow_app::WalkConfig;
use codex_workflow_contracts::WorkflowInstanceStatus;
use codex_workflow_contracts::WorkflowIrNode;
use pretty_assertions::assert_eq;

const PROVIDER_A: &str = "provider-a";
const PROVIDER_B: &str = "provider-b";

/// The action proposal the fixture model answers for step 001.
fn inspect_action_text() -> String {
    r#"{"operation":"inspect","target":"issues.example.com","inputs":{"depth":"full"}}"#.to_string()
}

/// The action proposal the fixture model answers for step 002.
fn record_action_text() -> String {
    r#"{"operation":"record","target":"summary","inputs":{"format":"markdown"}}"#.to_string()
}

/// A provider script producing the two fixture action proposals.
fn action_script() -> Vec<ScriptedTurn> {
    vec![
        ScriptedTurn::message(inspect_action_text()),
        ScriptedTurn::message(record_action_text()),
    ]
}

/// A divergent provider script: step 001 proposes a different target.
fn divergent_action_script() -> Vec<ScriptedTurn> {
    vec![
        ScriptedTurn::message(
            r#"{"operation":"inspect","target":"tracker.internal","inputs":{"depth":"full"}}"#,
        ),
        ScriptedTurn::message(record_action_text()),
    ]
}

/// A provider equipped with the equivalent capability fixture.
fn provider(provider_id: &str, script: Vec<ScriptedTurn>) -> Arc<ScriptedModelProvider> {
    Arc::new(
        ScriptedModelProvider::new(provider_id, script).with_capability(
            codex_eval_compat::EVAL_MODEL_SLUG,
            equivalent_capabilities(provider_id),
        ),
    )
}

/// Two succeeding environment turns.
fn env_script() -> Vec<ScriptedEnvTurn> {
    vec![
        ScriptedEnvTurn::succeed(serde_json::json!({"state": "inspected"})),
        ScriptedEnvTurn::succeed(serde_json::json!({"state": "recorded"})),
    ]
}

#[tokio::test]
async fn same_workflow_two_providers_are_equivalent() {
    // The differential golden path: one immutable workflow version, the
    // same scripted environment, two different model providers serving
    // the action-proposal seam. The runs must be observably equivalent
    // and their semantic fingerprints must digest identically.
    let (artifact, _installs) = published_fixture().expect("fixture");

    let mut baseline_harness =
        WorkflowEvalHarness::new(provider(PROVIDER_A, action_script()), env_script())
            .expect("baseline harness");
    let baseline_version = baseline_harness.install(&artifact).expect("install");
    let baseline = baseline_harness
        .run(&baseline_version, WalkConfig::default())
        .await
        .expect("baseline run");

    let mut candidate_harness =
        WorkflowEvalHarness::new(provider(PROVIDER_B, action_script()), env_script())
            .expect("candidate harness");
    let candidate_version = candidate_harness.install(&artifact).expect("install");
    let candidate = candidate_harness
        .run(&candidate_version, WalkConfig::default())
        .await
        .expect("candidate run");

    // Both harnesses pinned the same immutable version identity.
    assert_eq!(baseline.version, candidate.version);
    assert_eq!(baseline.version, artifact.version.version_id);
    assert_eq!(baseline.terminal, RunTerminal::Completed);
    assert_eq!(candidate.terminal, RunTerminal::Completed);
    assert_eq!(baseline.status, WorkflowInstanceStatus::Succeeded);
    assert_eq!(
        baseline.path,
        vec![
            codex_eval_compat::node_id("step-001").expect("node"),
            codex_eval_compat::node_id("step-002").expect("node"),
        ]
    );
    assert_eq!(baseline.actions.len(), 2);
    assert_eq!(baseline.model_requests, 2);

    let differential = compare_workflow_runs(&baseline, &candidate);
    assert!(
        differential.equivalent,
        "provider substitution must stay equivalent: {:?}",
        differential.divergences
    );
    assert_eq!(
        baseline.fingerprint_digest().expect("baseline fingerprint"),
        candidate
            .fingerprint_digest()
            .expect("candidate fingerprint")
    );
}

#[tokio::test]
async fn deterministic_replay_reproduces_identical_records() {
    // Reproducibility: the same provider, same scripts, fresh harness —
    // the records compare equal as whole objects and the fingerprints are
    // byte-identical.
    let (artifact, _installs) = published_fixture().expect("fixture");
    let first = {
        let mut harness =
            WorkflowEvalHarness::new(provider(PROVIDER_A, action_script()), env_script())
                .expect("harness");
        let version = harness.install(&artifact).expect("install");
        harness
            .run(&version, WalkConfig::default())
            .await
            .expect("first run")
    };
    let second = {
        let mut harness =
            WorkflowEvalHarness::new(provider(PROVIDER_A, action_script()), env_script())
                .expect("harness");
        let version = harness.install(&artifact).expect("install");
        harness
            .run(&version, WalkConfig::default())
            .await
            .expect("second run")
    };
    assert_eq!(first, second);
    assert_eq!(
        first.fingerprint_digest().expect("first digest"),
        second.fingerprint_digest().expect("second digest")
    );
}

#[tokio::test]
async fn injected_action_divergence_is_detected() {
    // Provider B proposes a different action for step 001: the
    // differential must enumerate the action divergence; the comparison
    // never silently passes.
    let (artifact, _installs) = published_fixture().expect("fixture");
    let mut baseline_harness =
        WorkflowEvalHarness::new(provider(PROVIDER_A, action_script()), env_script())
            .expect("baseline harness");
    let baseline_version = baseline_harness.install(&artifact).expect("install");
    let baseline = baseline_harness
        .run(&baseline_version, WalkConfig::default())
        .await
        .expect("baseline run");

    let mut candidate_harness = WorkflowEvalHarness::new(
        provider(PROVIDER_B, divergent_action_script()),
        env_script(),
    )
    .expect("candidate harness");
    let candidate_version = candidate_harness.install(&artifact).expect("install");
    let candidate = candidate_harness
        .run(&candidate_version, WalkConfig::default())
        .await
        .expect("candidate run");

    let differential = compare_workflow_runs(&baseline, &candidate);
    assert!(!differential.equivalent);
    assert_eq!(
        differential.divergences,
        vec![codex_eval_compat::WorkflowDivergence::Action {
            index: 0,
            baseline: baseline.actions[0].clone(),
            candidate: candidate.actions[0].clone(),
        }]
    );
    assert_ne!(baseline.actions[0].target, candidate.actions[0].target);
}

#[tokio::test]
async fn transient_failure_recovers_with_evidence() {
    // A transient environment failure flows through the frozen recovery
    // pipeline (retry on the same binding), completes the run, and is
    // recorded as recovery evidence — all reflected in the record.
    let (artifact, _installs) = published_fixture().expect("fixture");
    let scripted_env = vec![
        ScriptedEnvTurn::fail(FailureKind::Transient, "connection reset").expect("scripted"),
        ScriptedEnvTurn::succeed(serde_json::json!({"state": "inspected"})),
        ScriptedEnvTurn::succeed(serde_json::json!({"state": "recorded"})),
    ];
    let mut harness = WorkflowEvalHarness::new(provider(PROVIDER_A, action_script()), scripted_env)
        .expect("harness");
    let version = harness.install(&artifact).expect("install");
    let record = harness
        .run(&version, WalkConfig::default())
        .await
        .expect("recovered run");

    assert_eq!(record.terminal, RunTerminal::Completed);
    assert_eq!(record.status, WorkflowInstanceStatus::Succeeded);
    // One proposed action per step; the failed attempt retried once, so
    // the environment executed three times.
    assert_eq!(record.model_requests, 2);
    assert_eq!(record.actions.len(), 3);
    assert_eq!(record.recovery_histogram.get("retry"), Some(&1));
    assert_eq!(record.escalations, 0);
    // Approvals: one per distinct binding at instantiation, one fresh
    // re-authorization for the retry.
    let expected_evidence: std::collections::BTreeMap<String, usize> = ([
        ("approval", 2),
        ("observation", 2),
        ("recovery", 1),
        ("trace", 2),
    ])
    .into_iter()
    .map(|(key, count)| (key.to_string(), count))
    .collect();
    assert_eq!(record.evidence_histogram, expected_evidence);
    // Every recorded evidence reference still digests to its payload.
    let instance_id = only_instance_id(&harness);
    let verified = harness.verify(&instance_id).expect("verify");
    for reference in &verified.evidence {
        assert!(
            harness.verify_evidence(reference).expect("evidence verify"),
            "evidence {} must digest to its recorded value",
            reference.locator
        );
    }
}

#[tokio::test]
async fn permanent_failure_escalates_and_settles() {
    // A permanent environment failure escalates immediately (no retry),
    // the run settles as failed, and the escalation is recorded.
    let (artifact, _installs) = published_fixture().expect("fixture");
    let scripted_env = vec![
        ScriptedEnvTurn::fail(FailureKind::Permanent, "tracker rejected the action")
            .expect("scripted"),
    ];
    let mut harness = WorkflowEvalHarness::new(provider(PROVIDER_A, action_script()), scripted_env)
        .expect("harness");
    let version = harness.install(&artifact).expect("install");
    let record = harness
        .run(&version, WalkConfig::default())
        .await
        .expect("escalated run settles");

    assert_eq!(record.status, WorkflowInstanceStatus::Failed);
    assert!(matches!(record.terminal, RunTerminal::Failed { .. }));
    assert_eq!(record.escalations, 1);
    assert_eq!(record.recovery_histogram.get("retry"), None);
    assert_eq!(record.model_requests, 1);
    assert_eq!(record.actions.len(), 1);
    // The escalation is persisted as recovery evidence.
    assert_eq!(record.evidence_histogram.get("recovery"), Some(&1));
}

#[tokio::test]
async fn tampered_version_never_executes() {
    // Integrity gate: a store record claiming the published version id
    // with mutated identity-covered content fails selection and never
    // reaches instantiation.
    let (artifact, _installs) = published_fixture().expect("fixture");
    let mut harness = WorkflowEvalHarness::new(provider(PROVIDER_A, action_script()), env_script())
        .expect("harness");
    let version_id = harness.install(&artifact).expect("install");

    let mut tampered = harness
        .version_snapshot(&version_id)
        .expect("snapshot")
        .expect("stored version");
    if let Some(WorkflowIrNode::Step(step)) = tampered
        .definition
        .ir
        .nodes
        .get_mut(&codex_eval_compat::node_id("step-001").expect("node"))
    {
        step.description = Some("tampered by the integrity probe".to_string());
    }
    harness
        .publish_version_record(tampered)
        .expect("publish tampered record");

    let refusal = harness
        .run(&version_id, WalkConfig::default())
        .await
        .expect_err("tampered versions never execute");
    assert!(matches!(
        refusal,
        codex_workflow_app::WorkflowAppError::VersionIntegrity { .. }
    ));
    // Nothing executed: no instances, no evidence, no proposals.
    assert_eq!(harness.instance_count(), 0);
    assert_eq!(harness.evidence_count(), 0);
    assert_eq!(harness.model_request_count(), 0);
}

#[tokio::test]
async fn published_version_is_read_only_across_runs() {
    // Read-only guarantee: evaluation runs never mutate the installed
    // (immutable) version record — before and after a full run (including
    // a recovery), the store returns the byte-identical record.
    let (artifact, _installs) = published_fixture().expect("fixture");
    let scripted_env = vec![
        ScriptedEnvTurn::fail(FailureKind::Transient, "connection reset").expect("scripted"),
        ScriptedEnvTurn::succeed(serde_json::json!({"state": "inspected"})),
        ScriptedEnvTurn::succeed(serde_json::json!({"state": "recorded"})),
    ];
    let mut harness = WorkflowEvalHarness::new(provider(PROVIDER_A, action_script()), scripted_env)
        .expect("harness");
    let version_id = harness.install(&artifact).expect("install");
    let before = harness
        .version_snapshot(&version_id)
        .expect("snapshot")
        .expect("stored version");

    let record = harness
        .run(&version_id, WalkConfig::default())
        .await
        .expect("run");
    assert_eq!(record.terminal, RunTerminal::Completed);

    let after = harness
        .version_snapshot(&version_id)
        .expect("snapshot")
        .expect("stored version");
    assert_eq!(before, after);
}

#[tokio::test]
async fn walk_budget_bounds_the_run() {
    // Long-session bound: a walk budget smaller than the graph settles the
    // run as failed without a phantom Running record.
    let (artifact, _installs) = published_fixture().expect("fixture");
    let mut harness = WorkflowEvalHarness::new(provider(PROVIDER_A, action_script()), env_script())
        .expect("harness");
    let version_id = harness.install(&artifact).expect("install");
    let record = harness
        .run(&version_id, WalkConfig { max_steps: 1 })
        .await
        .expect("budgeted run settles");

    assert_eq!(record.status, WorkflowInstanceStatus::Failed);
    assert!(matches!(record.terminal, RunTerminal::Failed { .. }));
    // Only the first step executed before the budget aborted the walk.
    assert_eq!(record.model_requests, 1);
    assert_eq!(record.actions.len(), 1);
}

/// Extracts the single instance id a run created, from the observed
/// lifecycle events.
///
/// Records deliberately exclude instance identities (they are
/// run-specific), so verification helpers read the id from the event
/// stream instead.
fn only_instance_id(harness: &WorkflowEvalHarness) -> codex_workflow_contracts::WorkflowInstanceId {
    harness
        .observed_events()
        .iter()
        .find_map(|event| match event {
            codex_workflow_app::WorkflowEvent::InstanceCreated { instance, .. } => Some(*instance),
            _ => None,
        })
        .expect("instance created event")
}
