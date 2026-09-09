# WO-008 Audit Report — Workflow Teaching and Compiler

**Session:** /c/373224cc-bd5b-4f04-9203-0513470e9163 (agents tab, GLM-5.3, Full-Stack skill; tab B59ECBA1; dispatched 2026-09-09 ~06:31 UTC via the dispatcher's assault protocol after the GLM-5.3 capacity peak cleared).
**Auditor's clone:** `https://github.com/payswapdotorg/codex` @ 87107464a00ddf17d7066d2a8b53783de2b988fe (`--depth 1`, anonymous read-only).
**Method:** static-reasoning audit (no Rust toolchain in the worker sandbox) of every src module, the integration test, the dependency crate surface (`codex-workflow-contracts`), workspace wiring, lint gates, and forbidden-pattern scan.

## Auditor's final answer (verbatim prose)

The implementation in `codex-rs/teaching-compiler` is compile-sound and forbidden-pattern-clean. The gaps found were all test-coverage gaps against the Work Order's acceptance/evidence requirements (untested compile error paths, unasserted capability/binding inference outputs, unasserted publication-immutability/digest pinning, and an unreachable-in-tests optimization evidence gate). Four files are corrected below; all other files are sound and are not re-emitted.

*(The auditor then emitted four corrected files verbatim in the session transcript — `src/blueprint.rs`, `src/compiler.rs`, `src/optimization.rs`, `tests/teaching_pipeline.rs` — and staged copies in its sandbox at `/home/z/my-project/wo-008-audit/teaching-compiler/`. The full session transcript is the authoritative emission record; the corrections were applied to the repo as recorded in the Tech Lead addendum below.)*

Files corrected (4, test-coverage gaps only; corrected copies also staged at /home/z/my-project/wo-008-audit/teaching-compiler/):

- `codex-rs/teaching-compiler/src/blueprint.rs` — added 3 tests covering the previously untested compile error paths (EmptySession, SessionNotClosed, NoCompilableSteps, result-before-action InvalidEvent); inlined the annotation format arg (style only, byte-identical output).
- `codex-rs/teaching-compiler/src/compiler.rs` — added `emits_neutral_inferences_and_binding_proposals_per_step` (capability hints, binding proposals, environment-neutrality of proposed references).
- `codex-rs/teaching-compiler/src/optimization.rs` — added `observed_steps_without_evidence_block_collapse`, exercising the observed-step evidence gate through the public serde surface (the compiler's own graphs are linear step chains, so the gate was previously unreachable in tests), plus its StepOrigin test import.
- `codex-rs/teaching-compiler/tests/teaching_pipeline.rs` — added `approved_output_is_digest_pinned_and_stable` (digest equality across identical finalizations, sha256:+64-hex pin format, publication-allowing simulation, and post-finalize lifecycle lock rejecting validate/approve/finalize).

Test count: 41 → 48. No implementation (non-test) code was defective; `src/approval.rs`, `src/candidate.rs`, `src/error.rs`, `src/evidence.rs`, `src/graph.rs`, `src/lib.rs`, `src/mode.rs`, `src/proposal.rs`, `src/session.rs`, `src/simulation.rs`, `src/trajectory.rs`, `src/validation.rs`, `Cargo.toml`, and BUILD wiring are sound and not re-emitted.

Known limitations:

- No Rust toolchain in the sandbox: compile-correctness and test outcomes were verified by static reasoning only, not by `cargo build`/`cargo test`/`cargo clippy`. The corrected files should be run through CI in the repo.
- `teaching-compiler` has no `BUILD.bazel`; this matches the repo's convention for newly added crates (`workflow-forge`, `browser-use-adapter`, `computer-use-adapter` are likewise unwired, and no Bazel target references them), with the Cargo workspace as the build path — flagged for the Tech Lead, not treated as a WO-008 defect.
- The optimization evidence gate is unreachable through the compiler's own output by construction (taught graphs are step chains with no sequences); the added test reaches it through the public serde deserialization surface, which is the only public path that can produce that candidate shape.
- The clone is `--depth 1`; only the single HEAD commit (8710746) is visible, and the corrected files are delivered here rather than pushed (anonymous read-only access).

## Tech Lead verification addendum (2026-09-09, ~10:40 UTC)

Independent verification and correction application in the Tech Lead sandbox **with a real Rust toolchain** (cargo 1.95.0), which the worker sandbox lacked.

**Ground truth at 8710746** (measured by `cargo clippy -p codex-teaching-compiler --all-targets` in a pristine worktree of main):

- The integration test `tests/teaching_pipeline.rs` failed the workspace lint gate: **4 × `clippy::expect_used` errors** (the file lacked the file-level `#![allow(clippy::expect_used)]` that sibling integration files carry).
- `src/simulation.rs` test blocks: **2 × `redundant closure` + 1 × `redundant clone` errors** under the workspace deny set.
- 2 × `needless_borrows_for_generic_args` warnings (`approval.rs:124`, `evidence.rs:107`) — CI compiles tests with `-D warnings`.
- The four test-coverage gaps the auditor identified were real: `src/blueprint.rs` had **0** tests, `src/compiler.rs` 3, `src/optimization.rs` 6, `tests/teaching_pipeline.rs` 5 (the file's `approved_output_is_digest_pinned_and_stable` was absent).

**Corrections applied (this commit).** The auditor's four emitted corrected files were applied — `src/blueprint.rs` (3 tests + the documented `{annotation}` format-arg inlining, byte-identical output), `src/compiler.rs` (`emits_neutral_inferences_and_binding_proposals_per_step`), `src/optimization.rs` (`observed_steps_without_evidence_block_collapse` + its `StepOrigin` import), and `tests/teaching_pipeline.rs` (`approved_output_is_digest_pinned_and_stable`) — verified verbatim against the session's staged workspace copy (downloaded from the session UI). The lint fixes complete the set: the file-level `#![allow(clippy::expect_used)]` doc header for the integration test, `next.map(node_id)` / `condition` (not `.clone()`) in `simulation.rs`, and the two needless-borrow removals. `cargo fmt` canonicalized the touched files.

**Final verification (all green):** `cargo test -p codex-teaching-compiler` — **47 passed, 0 failed** (41 unit + 6 integration; was 40 at 8710746); `cargo clippy --all-targets` — 0 errors, 0 warnings; `cargo fmt -- --check` — clean.

**Notes on the audit process (for future audits):** (1) the auditor had no Rust toolchain, so its clippy-conformance claim missed the genuine lint errors above — real-toolchain verification by the Tech Lead remains mandatory; (2) the auditor's gap analysis mixed the prompt's embedded SOURCE BUNDLE with the fresh clone (its "41 → 48" count is bundle-relative; the repo base was 40) — future audit prompts must direct the worker to reconcile every claim against the live clone and treat the bundle as fallback only; (3) the worker's sandbox staging and full workspace tar (workspace-373224cc….tar via the workspace Download button) preserved the corrected files exactly — a reliable recovery channel for agents-tab sessions.

Status: **WO-008 audit COMPLETE — implementation sound; all four coverage corrections + lint fixes applied and verified green; WO-010 unblocked.** This agents-tab session (/c/373224cc) plus this report are the authoritative work record for WO-008.
