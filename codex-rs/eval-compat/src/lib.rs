//! Evidence-driven evaluation and differential compatibility (WO-013).
//!
//! This crate is the reproducible evaluation harness of the Codex
//! universal workflow platform: it builds **model/provider compatibility
//! tests**, **workflow execution benchmarks**, and **differential
//! comparisons** on top of the already-merged frozen surfaces — the WO-002
//! universal model contract (and `codex-model-provider`'s selection
//! helpers) and the WO-010 workflow application runtime — without
//! duplicating any of them.
//!
//! # What this crate is
//!
//! - Deterministic, versioned **fixtures**: scripted model providers
//!   ([`ScriptedModelProvider`]) implementing the frozen model contract, a
//!   scripted environment adapter ([`ScriptedEnvironmentAdapter`])
//!   implementing the frozen WO-005 adapter boundary, and a fully
//!   deterministic published workflow version
//!   ([`published_fixture`]) built through the real teaching → compile →
//!   approve → publish → install pipeline.
//! - **Harnesses**: [`ModelCompatHarness`] (select → negotiate → stream →
//!   record) and [`WorkflowEvalHarness`] (select → instantiate → run →
//!   verify) driving the real runtimes over in-memory control-plane seams.
//! - **Records and fingerprints**: every run produces a
//!   [`WorkflowRunRecord`] or [`ModelCompatRecord`] whose semantic
//!   equivalence class is content-addressed, so runs replay and compare
//!   exactly.
//! - **Differential comparison** ([`compare_workflow_runs`],
//!   [`compare_model_runs`]): same workflow + same scripted environment,
//!   different model provider — divergences are enumerated explicitly,
//!   never silently dropped.
//! - **Upstream compatibility snapshots** ([`UpstreamCompatSnapshot`]):
//!   the ordinary-Codex no-workflow contract (nothing executes without a
//!   selected version) pinned observably and content-addressed, so future
//!   changes that break it fail loudly.
//!
//! # What this crate deliberately is not
//!
//! - **No second runtime, engine, or contract.** Every semantic decision
//!   delegates to the frozen crates; this crate only scripts doubles,
//!   drives runs, and records observations.
//! - **No live benchmarks.** No network, no credentials, no wall-clock
//!   timing: latency/cost metrics are logical counters (model
//!   invocations, dispatched actions, evidence records, scripted
//!   durations). Live provider benchmarking belongs to a later Work
//!   Order with an explicit external-effects basis.
//! - **No benchmark output as authority.** Records and digests are
//!   evidence for comparison, not semantic truth; divergences are
//!   reported, never auto-resolved.
//! - **No mutation of installed artifacts.** Evaluation runs consume
//!   immutable published versions read-only (asserted by tests).
//!
//! # Determinism rules
//!
//! - Fixtures are fully deterministic: fixed identities, fixed scripts,
//!   fixed digests. Two runs of the same fixture produce byte-identical
//!   fingerprints.
//! - Script exhaustion fails loudly (provider error / permanent execution
//!   failure), never silently succeeds.
//! - The provider-swap primitive (`for_provider`) changes only provider
//!   identity, never semantic content — the same property the frozen
//!   model contract pins.
//!
//! # Example shape
//!
//! ```text
//! publish fixture -> install into harness A (provider-a) -> run -> record A
//!                 -> install into harness B (provider-b) -> run -> record B
//! compare_workflow_runs(record A, record B) -> equivalent (or divergences)
//! ```
#![deny(missing_docs)]

mod action_source;
mod compat;
mod differential;
mod env_adapter;
mod harness;
mod model_fixture;
mod record;
mod workflow_fixture;

pub use action_source::ModelBackedActionSource;
pub use compat::COMPAT_FIXTURE_VERSION;
pub use compat::UpstreamCompatSnapshot;
pub use differential::ModelDifferential;
pub use differential::ModelDivergence;
pub use differential::WorkflowDifferential;
pub use differential::WorkflowDivergence;
pub use differential::compare_model_runs;
pub use differential::compare_workflow_runs;
pub use env_adapter::EVAL_ADAPTER_ID;
pub use env_adapter::EVAL_CAPABILITY;
pub use env_adapter::EVAL_ENVIRONMENT;
pub use env_adapter::ScriptedEnvTurn;
pub use env_adapter::ScriptedEnvironmentAdapter;
pub use harness::EVAL_APPROVER;
pub use harness::ModelCompatHarness;
pub use harness::WorkflowEvalHarness;
pub use model_fixture::EVAL_MODEL_SLUG;
pub use model_fixture::ScriptedModelProvider;
pub use model_fixture::ScriptedTurn;
pub use model_fixture::equivalent_capabilities;
pub use model_fixture::fixture_model_info;
pub use record::ActionFootprint;
pub use record::ModelCompatCase;
pub use record::ModelCompatRecord;
pub use record::ModelErrorClass;
pub use record::NEUTRAL_PROVIDER_KEY;
pub use record::NormalizedNegotiationFailure;
pub use record::NormalizedResponse;
pub use record::WorkflowRunRecord;
pub use record::evidence_histogram;
pub use workflow_fixture::EVAL_RELEASE_TAG;
pub use workflow_fixture::EVAL_REPOSITORY;
pub use workflow_fixture::EVAL_SEMANTIC_VERSION;
pub use workflow_fixture::EVAL_WORKFLOW;
pub use workflow_fixture::definition_id;
pub use workflow_fixture::fixture_bindings;
pub use workflow_fixture::node_id;
pub use workflow_fixture::published_fixture;
pub use workflow_fixture::repository_id;
pub use workflow_fixture::taught_fixture;
