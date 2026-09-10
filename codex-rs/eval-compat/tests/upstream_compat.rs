//! Upstream-Codex compatibility tests (WO-013).
//!
//! The compatibility contract under evaluation: **with no workflow
//! selected, ordinary Codex behavior is untouched** — the lifecycle holds
//! no state, every run-path operation refuses with `NoActiveWorkflow`,
//! and no events, evidence, instances, actions, or model invocations
//! happen. The snapshot digest is pinned so any future change that
//! breaks this contract fails loudly.

// The workspace clippy.toml allows `expect`/`unwrap` in test code; the
// same intent is spelled out at file level here (matching the WO-010
// end-to-end test precedent) for plain helper functions.
#![allow(clippy::expect_used)]
#![allow(clippy::unwrap_used)]

use std::sync::Arc;

use codex_eval_compat::ScriptedEnvTurn;
use codex_eval_compat::ScriptedModelProvider;
use codex_eval_compat::ScriptedTurn;
use codex_eval_compat::UpstreamCompatSnapshot;
use codex_eval_compat::equivalent_capabilities;
use pretty_assertions::assert_eq;

/// A provider that would record any invocation (none may happen).
fn idle_provider() -> Arc<ScriptedModelProvider> {
    Arc::new(
        ScriptedModelProvider::new(
            "provider-a",
            vec![ScriptedTurn::message("must never be served")],
        )
        .with_capability(
            codex_eval_compat::EVAL_MODEL_SLUG,
            equivalent_capabilities("provider-a"),
        ),
    )
}

/// An environment script that would record any execution (none may
/// happen).
fn idle_env_script() -> Vec<ScriptedEnvTurn> {
    vec![ScriptedEnvTurn::succeed(serde_json::json!({
        "state": "must never execute"
    }))]
}

#[tokio::test]
async fn no_active_workflow_executes_nothing() {
    let snapshot = UpstreamCompatSnapshot::capture(idle_provider(), idle_env_script())
        .await
        .expect("snapshot");

    // The lifecycle holds no state and both run-path operations refuse.
    assert!(snapshot.lifecycle_inactive);
    let refusal = codex_workflow_app::WorkflowAppError::NoActiveWorkflow.to_string();
    assert_eq!(
        snapshot.instantiate_refusal.as_deref(),
        Some(refusal.as_str())
    );
    assert_eq!(snapshot.run_refusal.as_deref(), Some(refusal.as_str()));

    // Nothing executed and nothing was recorded.
    assert_eq!(snapshot.events_recorded, 0);
    assert_eq!(snapshot.evidence_payloads, 0);
    assert_eq!(snapshot.instances_created, 0);
    assert_eq!(snapshot.versions_stored, 0);
    assert_eq!(snapshot.adapter_executions, 0);
    assert_eq!(snapshot.model_requests, 0);
    // The registry still holds the registered adapter binding (the host
    // owns construction); the refused operations did not touch it.
    assert_eq!(snapshot.registered_bindings, 1);
}

#[tokio::test]
async fn compatibility_snapshot_digest_is_stable_and_pinned() {
    // Capturing twice yields the same content-addressed digest...
    let first = UpstreamCompatSnapshot::capture(idle_provider(), idle_env_script())
        .await
        .expect("first snapshot");
    let second = UpstreamCompatSnapshot::capture(idle_provider(), idle_env_script())
        .await
        .expect("second snapshot");
    assert_eq!(first, second);
    assert_eq!(
        first.digest().expect("first digest"),
        second.digest().expect("second digest")
    );

    // ...and the digest is pinned: any change to the no-workflow
    // observable behavior — the ordinary-Codex compatibility surface —
    // changes the digest and fails this test loudly.
    let digest = first.digest().expect("digest").as_str().to_string();
    assert_eq!(
        digest, PINNED_NO_WORKFLOW_DIGEST,
        "the no-workflow compatibility snapshot changed; if the change is \
         intentional, re-pin the digest and bump COMPAT_FIXTURE_VERSION"
    );
}

/// The pinned digest of the no-workflow compatibility snapshot at
/// fixture version 1.
///
/// Regenerate with the snapshot digest output when the contract changes
/// deliberately, and bump
/// [`COMPAT_FIXTURE_VERSION`](codex_eval_compat::COMPAT_FIXTURE_VERSION).
const PINNED_NO_WORKFLOW_DIGEST: &str =
    "sha256:ba95e86010d1dd88101ab866303eba5be8c6c9fc7f2ec4e9b96b7c33b5e95655";
