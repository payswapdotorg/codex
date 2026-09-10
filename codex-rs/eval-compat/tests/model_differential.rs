//! Model-plane differential tests (WO-013).
//!
//! Every scenario is static: scripted providers double for real model
//! providers behind the frozen WO-002 contract, selection and negotiation
//! run through the real `codex-model-provider` helpers, and streaming runs
//! through the real session/stream contract. The scenarios cover:
//!
//! - provider substitution preserving semantic output (equivalence);
//! - unknown providers failing loudly (no silent substitution);
//! - capability negotiation diverging explicitly (no silent degradation);
//! - injected output and failure divergences being detected (never
//!   silently dropped);
//! - negotiated capabilities surfacing context-window and feature data.

// The workspace clippy.toml allows `expect`/`unwrap` in test code; the
// same intent is spelled out at file level here (matching the WO-010
// end-to-end test precedent) for plain helper functions.
#![allow(clippy::expect_used)]
#![allow(clippy::unwrap_used)]

use std::sync::Arc;

use codex_eval_compat::EVAL_MODEL_SLUG;
use codex_eval_compat::ModelCompatCase;
use codex_eval_compat::ModelCompatHarness;
use codex_eval_compat::ModelErrorClass;
use codex_eval_compat::ScriptedModelProvider;
use codex_eval_compat::ScriptedTurn;
use codex_eval_compat::compare_model_runs;
use codex_eval_compat::equivalent_capabilities;
use codex_eval_compat::fixture_model_info;
use codex_model_contract::ModelDescriptor;
use codex_model_contract::ModelError;
use codex_model_contract::ModelFeature;
use codex_model_contract::ModelRequest;
use codex_protocol::models::ContentItem;
use codex_protocol::models::ResponseItem;
use codex_protocol::openai_models::ReasoningEffort;
use pretty_assertions::assert_eq;

const PROVIDER_A: &str = "provider-a";
const PROVIDER_B: &str = "provider-b";

fn user_message(text: &str) -> ResponseItem {
    ResponseItem::Message {
        id: None,
        role: "user".to_string(),
        content: vec![ContentItem::InputText {
            text: text.to_string(),
        }],
        phase: None,
        internal_chat_message_metadata_passthrough: None,
    }
}

fn case_for(provider_id: &str) -> ModelCompatCase {
    ModelCompatCase {
        descriptor: ModelDescriptor::new(provider_id, EVAL_MODEL_SLUG),
        model: fixture_model_info().expect("fixture model info"),
        required: vec![
            ModelFeature::ReasoningEffort(ReasoningEffort::Medium),
            ModelFeature::ImageInput,
        ],
        request: ModelRequest {
            model: ModelDescriptor::new(provider_id, EVAL_MODEL_SLUG),
            instructions: "Summarize the issue list.".to_string(),
            input: vec![user_message("the issue tracker is open")],
            ..ModelRequest::default()
        },
    }
}

fn equivalent_provider(provider_id: &str, answer: &str) -> Arc<ScriptedModelProvider> {
    Arc::new(
        ScriptedModelProvider::new(provider_id, vec![ScriptedTurn::message(answer)])
            .with_capability(EVAL_MODEL_SLUG, equivalent_capabilities(provider_id)),
    )
}

#[tokio::test]
async fn provider_substitution_preserves_semantic_output() {
    // Same scripted semantics, different provider identities: the
    // differential must report equivalence and the request digests must
    // match (the provider swap never changes semantic content).
    let harness = ModelCompatHarness::new(vec![
        equivalent_provider(PROVIDER_A, "the list shows triaged issues"),
        equivalent_provider(PROVIDER_B, "the list shows triaged issues"),
    ]);
    let baseline = harness.run(&case_for(PROVIDER_A)).await;
    let candidate = harness.run(&case_for(PROVIDER_B)).await;

    let differential = compare_model_runs(&baseline, &candidate);
    assert!(
        differential.equivalent,
        "equivalent providers must not diverge: {:?}",
        differential.divergences
    );
    assert_eq!(differential.baseline_provider, PROVIDER_A);
    assert_eq!(differential.candidate_provider, PROVIDER_B);
    assert_eq!(baseline.request_digest, candidate.request_digest);
    assert_eq!(
        baseline
            .response
            .as_ref()
            .expect("baseline response")
            .output,
        candidate
            .response
            .as_ref()
            .expect("candidate response")
            .output
    );
    assert_eq!(
        baseline
            .response
            .as_ref()
            .and_then(|response| response.end_turn),
        Some(true)
    );
}

#[tokio::test]
async fn unknown_provider_fails_loudly() {
    // A descriptor naming an unregistered provider must fail selection
    // explicitly; the harness never silently substitutes another provider.
    let harness = ModelCompatHarness::new(vec![equivalent_provider(PROVIDER_A, "answer")]);
    let record = harness.run(&case_for("absent-provider")).await;
    assert!(!record.selected);
    assert_eq!(
        record.selection_failure.expect("selection failure"),
        codex_model_provider::ProviderSelectionError::UnknownProvider(
            "absent-provider".to_string()
        )
    );
    assert!(record.negotiated.is_none());
    assert!(record.response.is_none());
}

#[tokio::test]
async fn capability_negotiation_divergence_is_detected_never_silent() {
    // Provider B drops support for the required reasoning effort: the
    // negotiation must fail explicitly with recovery alternatives, the
    // invocation must not happen, and the differential must report the
    // divergence.
    let mut restricted = equivalent_capabilities(PROVIDER_B);
    restricted.supported_reasoning_efforts = vec![ReasoningEffort::Low];
    let provider_b = Arc::new(
        ScriptedModelProvider::new(PROVIDER_B, vec![ScriptedTurn::message("answer")])
            .with_capability(EVAL_MODEL_SLUG, restricted),
    );
    let harness =
        ModelCompatHarness::new(vec![equivalent_provider(PROVIDER_A, "answer"), provider_b]);
    let baseline = harness.run(&case_for(PROVIDER_A)).await;
    let candidate = harness.run(&case_for(PROVIDER_B)).await;

    assert!(baseline.negotiated.is_some());
    let failure = candidate
        .negotiation_failure
        .as_ref()
        .expect("negotiation must fail explicitly");
    assert_eq!(
        failure.feature,
        ModelFeature::ReasoningEffort(ReasoningEffort::Medium)
    );
    assert_eq!(failure.supported_alternatives, vec!["low".to_string()]);
    // No silent degradation: the failing provider is never invoked.
    assert!(candidate.response.is_none());
    assert_eq!(candidate.invocation_failure, None);

    let differential = compare_model_runs(&baseline, &candidate);
    assert!(!differential.equivalent);
    assert!(differential.divergences.iter().any(|divergence| matches!(
        divergence,
        codex_eval_compat::ModelDivergence::NegotiationFailure { .. }
    )));
}

#[tokio::test]
async fn output_divergence_is_detected() {
    // Same capabilities, different scripted answers: the differential
    // must report the output divergence instead of passing silently.
    let harness = ModelCompatHarness::new(vec![
        equivalent_provider(PROVIDER_A, "the list shows triaged issues"),
        equivalent_provider(PROVIDER_B, "the tracker is empty"),
    ]);
    let baseline = harness.run(&case_for(PROVIDER_A)).await;
    let candidate = harness.run(&case_for(PROVIDER_B)).await;

    let differential = compare_model_runs(&baseline, &candidate);
    assert!(!differential.equivalent);
    assert!(differential.divergences.iter().any(|divergence| matches!(
        divergence,
        codex_eval_compat::ModelDivergence::Output { .. }
    )));
}

#[tokio::test]
async fn scripted_failure_is_normalized_not_silent() {
    // Provider B's stream fails rate-limited: the failure must surface as
    // the normalized taxonomy class (not a panic, not a silent skip), and
    // the differential must report it.
    let provider_b = Arc::new(
        ScriptedModelProvider::new(
            PROVIDER_B,
            vec![ScriptedTurn::fail(ModelError::RateLimited {
                message: "quota exhausted".to_string(),
                retry_after: None,
            })],
        )
        .with_capability(EVAL_MODEL_SLUG, equivalent_capabilities(PROVIDER_B)),
    );
    let harness =
        ModelCompatHarness::new(vec![equivalent_provider(PROVIDER_A, "answer"), provider_b]);
    let baseline = harness.run(&case_for(PROVIDER_A)).await;
    let candidate = harness.run(&case_for(PROVIDER_B)).await;

    assert_eq!(baseline.invocation_failure, None);
    assert_eq!(
        candidate.invocation_failure,
        Some(ModelErrorClass::RateLimited)
    );
    assert!(candidate.response.is_none());

    let differential = compare_model_runs(&baseline, &candidate);
    assert!(!differential.equivalent);
    assert!(differential.divergences.iter().any(|divergence| matches!(
        divergence,
        codex_eval_compat::ModelDivergence::Invocation { .. }
    )));
}

#[tokio::test]
async fn negotiated_capabilities_surface_context_and_features() {
    // The negotiated record must surface the catalog's context window and
    // supported features: the context-size observability the evaluation
    // records pin. Replaying the same case (scripted with a second,
    // identical turn) digests identically.
    let provider = || {
        Arc::new(
            ScriptedModelProvider::new(
                PROVIDER_A,
                vec![
                    ScriptedTurn::message("answer"),
                    ScriptedTurn::message("answer"),
                ],
            )
            .with_capability(EVAL_MODEL_SLUG, equivalent_capabilities(PROVIDER_A)),
        )
    };
    let harness = ModelCompatHarness::new(vec![provider()]);
    let record = harness.run(&case_for(PROVIDER_A)).await;
    let capabilities = record.negotiated.as_ref().expect("negotiated");
    assert_eq!(capabilities.model_id, EVAL_MODEL_SLUG);
    assert_eq!(capabilities.context_window, Some(272_000));
    assert_eq!(
        capabilities.supported_reasoning_efforts,
        vec![
            ReasoningEffort::Low,
            ReasoningEffort::Medium,
            ReasoningEffort::High
        ]
    );
    // The negotiated record digests reproducibly across replays.
    let first = record.fingerprint_digest().expect("fingerprint");
    let replay = harness.run(&case_for(PROVIDER_A)).await;
    assert_eq!(
        first,
        replay.fingerprint_digest().expect("replay fingerprint")
    );
}

#[tokio::test]
async fn exhausted_script_fails_loudly() {
    // Determinism rule: a request when the script is exhausted fails
    // loudly as a provider error (never a silent success), and the
    // normalized class lands in the record.
    let harness = ModelCompatHarness::new(vec![equivalent_provider(PROVIDER_A, "answer")]);
    let first = harness.run(&case_for(PROVIDER_A)).await;
    assert!(first.response.is_some());
    let replay = harness.run(&case_for(PROVIDER_A)).await;
    assert_eq!(replay.invocation_failure, Some(ModelErrorClass::Provider));
    assert!(replay.response.is_none());
}

#[tokio::test]
async fn fingerprint_is_provider_independent_for_equivalent_records() {
    // The semantic fingerprint excludes provider identity: equivalent
    // records from different providers digest identically, which is what
    // makes the fingerprint the differential equivalence class.
    let harness = ModelCompatHarness::new(vec![
        equivalent_provider(PROVIDER_A, "answer"),
        equivalent_provider(PROVIDER_B, "answer"),
    ]);
    let baseline = harness.run(&case_for(PROVIDER_A)).await;
    let candidate = harness.run(&case_for(PROVIDER_B)).await;
    assert_eq!(
        baseline.fingerprint_digest().expect("baseline digest"),
        candidate.fingerprint_digest().expect("candidate digest")
    );
}
