=== WO-013 COMPLETION REPORT ===

Work Order: WO-013 — Evaluation and Differential Compatibility

Base SHA: 67c0460ae16836cac23e13e38f3bea3fcb4682e0 (verified reachable via git rev-parse 67c0460ae16836cac23e13e38f3bea3fcb4682e0^{commit}; exact WO-mandated base. Note: origin/main HEAD has since moved to bf82abd4f019e42fe636bc2b14c99d5885b0b483 — WO-014/WO-015 activity — so rebasing onto current main is a trivial reconcile since this is a pure leaf crate.)

Head SHA: 9f61c7a0ec71757a4e0ef5c75cfa445c5d1cb78b (branch wo-013/eval-compat, commit message: feat(eval-compat): WO-013 evaluation and differential compatibility)

Session recovery note: The sandbox pod was reset between sessions — the prior /home/z/codex clone and its toolchain were gone. The entire Work Order was re-executed from scratch in this session (clone → branch → audit → implement → verify → commit → deliver). Nothing was salvaged from the lost session.

Changed surfaces (15 files, +3,421 lines, no existing crate modified):

codex-rs/Cargo.toml — exactly one members line added: "eval-compat", (immediately after "workflow-app",)
codex-rs/Cargo.lock — auto-updated with the single new package entry codex-eval-compat (+19 lines, same property as the WO-010 merge)
codex-rs/eval-compat/** — new leaf crate (package codex-eval-compat, lib codex_eval_compat): Cargo.toml, 9 src modules (lib.rs, model_fixture.rs, env_adapter.rs, action_source.rs, workflow_fixture.rs, record.rs, harness.rs, differential.rs, compat.rs), 3 integration test files

Implementation summary:

Reproducible model/provider compatibility tests — ScriptedModelProvider/ScriptedModelSession implement the frozen WO-002 contract (ModelProvider, ModelSession, real ModelStream channel semantics); ModelCompatHarness drives the real WO-002 selection helpers (select_provider_for_descriptor, negotiate_model_capabilities) and captures every failure in the record — selection, negotiation, and invocation failures are never silently swallowed; script exhaustion fails loudly as a provider error.
Workflow execution benchmarks — WorkflowEvalHarness drives the real WO-010 lifecycle (select → instantiate → run → verify) over in-memory control-plane seams, with action proposals served by the scripted model through the real session/stream path (ModelBackedActionSource, a sync-bridge over the frozen StepActionSource port) and environment execution through a scripted EnvironmentAdapter. Covers: mixed-environment golden path, transient-retry recovery with evidence, permanent escalation, walk budgets (long-session bound), context-window observability, and deterministic replay with byte-identical content-addressed fingerprints (ContentDigest over canonical semantic projections).
Differential testing — same immutable WorkflowVersion + same scripted environment, only the model provider varies: compare_workflow_runs/compare_model_runs enumerate ALL divergences (terminal, status, path, per-action, evidence/recovery histograms, escalations, model requests, negotiation, invocation class, output, end-turn, request digest) — never early-return, never silent. Divergence injection (different action target, restricted capabilities, scripted failure) is detected and reported.
Upstream compatibility — UpstreamCompatSnapshot pins the ordinary-Codex no-workflow contract observably: no lifecycle state, NoActiveWorkflow refusals, zero events/evidence/instances/adapter-executions/model-invocations. The snapshot is content-addressed and the digest is pinned in the test, so future changes that break ordinary Codex behavior fail loudly.
Read-only guarantee — evaluation runs never mutate installed versions (asserted byte-identical before/after runs, including a recovery run); tampered version records (same id, mutated identity-covered content) fail verify_integrity and never execute.

Test commands and exact results (run from codex-rs/, recorded before target cleanup; builds used CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_DEV_STRIP=symbols CARGO_INCREMENTAL=0 to fit the 9.9G sandbox disk — these affect debuginfo/strip only, not test semantics):

cargo test -p codex-eval-compat → 18 passed, 0 failed (lib 0; model_differential 8/8; upstream_compat 2/2; workflow_differential 8/8)
cargo clippy -p codex-eval-compat --all-targets -- -D warnings → clean, no warnings (finished without error)
cargo fmt -p codex-eval-compat -- --check → exit 0, no diffs (stable rustfmt 1.9.0; only the usual nightly-only imports_granularity config warnings, same as the rest of the workspace)

Acceptance evidence (per WO-013 required outcomes):

Reproducible model/provider compatibility tests — provider_substitution_preserves_semantic_output, fingerprint_is_provider_independent_for_equivalent_records, negotiated_capabilities_surface_context_and_features (all pass; provider swap changes only descriptor identity, semantic digests equal).
Workflow execution benchmarks covering environments, permissions, context, long sessions, failures, recovery, mixed-environment runs — transient recovery (transient_failure_recovers_with_evidence), permanent escalation, walk-budget bound, capability gating (binding-plan authorization via the approval seam), and the two-step mixed-path run through the registry, all deterministic and fingerprinted. Permissions are covered through the approve-gate (approval evidence recorded per binding; denials flow as ApprovalDenied per the frozen WO-010 path — the InMemory approval source approves, matching the WO-010 E2E scope).
Compare intentionally unchanged behavior against the upstream Codex baseline — no_active_workflow_executes_nothing + compatibility_snapshot_digest_is_stable_and_pinned: pinned digest sha256:ba95e86010d1dd88101ab866303eba5be8c6c9fc7f2ec4e9b96b7c33b5e95655 at fixture version 1.
Record outcome, safety, efficiency, recovery, latency/cost, and compatibility metrics — WorkflowRunRecord/ModelCompatRecord carry terminal/status/path/actions/evidence histograms/recovery histograms/escalations/model-request counters plus scripted logical durations; digests are content-addressed.
Evaluation fixtures deterministic and versioned — fixed identities, fixed scripts, pinned fixture constants (EVAL_*), deterministic version ids; deterministic_replay_reproduces_identical_records asserts whole-record equality and identical fingerprints.

Forbidden (respected): No synthetic-benchmark optimization at the expense of invariants (doubles only script, semantics delegate to frozen crates); no runtime semantics changed (zero modifications to existing crates); benchmark output treated as evidence, never authority (divergences reported, never auto-resolved).

Compatibility impact: Ordinary Codex behavior is untouched by construction: codex-eval-compat is a leaf crate adding no code to codex-core/CLI/app-server; the no-workflow contract is asserted with zero side effects (pinned digest). With no active workflow, nothing in the eval surfaces executes. The single-line members addition plus lockfile entry match the WO-010 merge precedent exactly.

Delivery artifacts (all under the file-API-visible root /home/z/my-project):

/home/z/my-project/wo-013-delivery.bundle — git bundle 67c0460ae..wo-013/eval-compat, git bundle verify → okay, contains ref 9f61c7a0ec71757a4e0ef5c75cfa445c5d1cb78b refs/heads/wo-013/eval-compat
/home/z/my-project/wo-013-delivery/Cargo.toml.reference — the clone's codex-rs/Cargo.toml for members-line reconciliation
/home/z/my-project/wo-013-delivery/codex-rs/eval-compat/ — full crate contents (verified diff -r identical to the clone):
Cargo.toml, src/{lib,action_source,compat,differential,env_adapter,harness,model_fixture,record,workflow_fixture}.rs, tests/{model_differential,workflow_differential,upstream_compat}.rs
Full working clone preserved at /home/z/codex (branch wo-013/eval-compat checked out); not pushed (no credentials — Tech Lead applies, verifies, and merges). Cargo target dirs removed after recording results; sandbox disk back to 44% used.

Known limitations:

Latency/cost metrics are logical counters (model invocations, dispatched actions, evidence records, scripted durations) — no live providers, no wall-clock timing; live provider benchmarking needs an explicit external-effects basis (future WO).
The upstream-baseline comparison pins the ordinary-Codex no-workflow observable contract at this layer (CLI/app-server surfaces are untouched by construction); it is not a byte-diff of an upstream binary.
MODULE.bazel.lock was not refreshed (no Bazel in this sandbox); CI lockfile drift checks may flag it — the WO-010 merge shipped with the same property, and the AGENTS.md remediation is just bazel-lock-update on a capable host.
Dependency-graph.json in-tree still lists WO-013 as "blocked" (stale at base); state reconciliation belongs to the Tech Lead's merge commit per repo convention.

Deferred items:

Live/external-effects provider benchmarking and wall-clock latency/cost capture → needs a Work Order with an external-effects basis (candidate: extend codex-eval-compat harnesses with a real-adapter seam; belongs with WO-014 evaluation tooling).
Broader upstream behavioral diffing (CLI-level golden outputs) → defer until an ordinary-Codex surface actually changes behavior (owner: Tech Lead via a new WO).
Resume/paused-run differential coverage (long-session continuation) → deferred to the scheduling/resume Work Order that owns instance resume semantics.

Risks:

The pinned compat digest changes if the no-workflow contract is deliberately extended (re-pin and bump COMPAT_FIXTURE_VERSION — the failure message documents this).
ModelBackedActionSource drives the scripted session through futures::executor::block_on; this is documented and safe only because the scripted future is immediate (no timers/IO). A future live-adapter variant must restructure to an async seam.
Base is behind current main (WO-014/WO-015 merged since): rebase risk is minimal (leaf crate, one members line), but execution-contracts gained new environments in WO-015 — re-verification after rebase is the Tech Lead's acceptance gate.

