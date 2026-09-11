# MWO-003 — Persisted Execution Position and Restart Reconciliation: Worker Completion Report

**Work Order:** MWO-003 (docs/work-orders/MWO-003-RESUME-RECONCILIATION.md)
**Program:** M4 durable-control-plane remediation (architect mandate `TECH_LEAD_START_HERE.md` §16)
**Architecture:** 0.2.0 (frozen; no divergence)
**Status:** implemented on branch `mwo-003/resume-reconciliation` — awaiting the Tech Lead's independent verification

## Verification statement (read first)

All verification below ran for real on this sandbox's freshly installed toolchain **cargo/rustc 1.95.0** (the repo's `rust-toolchain.toml` pin; no toolchain was present, so a minimal profile was installed via rustup, the same path the MWO-001 worker took). Commands ran from `codex-rs/` with `CARGO_PROFILE_DEV_DEBUG=0` for disk hygiene.

## 1. Base SHA & Head SHA

- **Base:** `2c01fcd898b8089ecd4d5926cf66f2879a49bc4c` — verified reachable via `git rev-parse 2c01fcd89^{commit}`; it is the clone's `origin/main` HEAD at dispatch (MWO-001's merge-state commit on top of PR #20 `ac3f9ced5` and PR #19 `039273223`). No fallback was needed: **the clone wins**, and every bundled signature was reconciled against the live tree (the bundle matched the base verbatim in every file consulted).
- **Branch:** `mwo-003/resume-reconciliation`, created from that base.
- **Implementation commit:** `feat(workflow-resume): MWO-003 persisted execution position and restart reconciliation` (exact SHA in the completion message and the delivery bundle; this report is a second commit on top).

## 2. Changed Files (20 files, +2,488 / −58)

| File | Δ | Purpose |
| --- | --- | --- |
| `workflow-app/src/port.rs` | +61 | the `RunPosition` record (control-plane state only) + the `RunPositionStore` port (`save`/`load`/`discard`) |
| `workflow-app/src/walk.rs` | +62 | `WalkPosition` record, `Walk::position()`/`Walk::resume()`, and the pause arms now record the wait/human-gate node's successor as the pending re-entry point |
| `workflow-app/src/resume.rs` | +355 | NEW module: `WorkflowLifecycle::with_positions`, `resume_run`, `reconcile_startup`, the checkpoint helpers (`persist_position`/`discard_position`), rehydration (`load_position`, `reestablish_execution`), `distinct_decision_bindings` |
| `workflow-app/src/lifecycle.rs` | +25/−6 | the `positions` seam field (`None` in `new`, the pre-MWO-003 default), the instantiation start checkpoint (a checkpoint-seam failure settles `Failed` through `fail_instance`, never a phantom `Running`), `cancel` discards the persisted position |
| `workflow-app/src/run.rs` | +21/−4 | `ActiveRun.resources` (the opaque resource identities for re-binding), step-completion and pause checkpoints in `execute_run`, terminal settlement discards the position (a pause keeps it) |
| `workflow-app/src/error.rs` | +36 | additive variants: `InstanceNotRunning`, `RunPositionUnavailable`, `ActiveRunHeld` (every existing `match` on the enum is wildcard-armed or `matches!` — re-audited by grep; MWO-002's precedent) |
| `workflow-app/src/event.rs` | +18 | additive variants: `RunResumed { instance, node }`, `InstanceReconciled { instance, reason }` |
| `workflow-app/src/memory.rs` | +56 | `InMemoryRunPositionStore` (the double; shared-state handle like the others) |
| `workflow-app/src/lib.rs` | +11/−4 | `pub mod resume` + re-exports (`RunPosition`, `RunPositionStore`, `InMemoryRunPositionStore`, `WalkPosition`) |
| `workflow-app/src/resume_tests.rs` | +553 | NEW sibling test module: 11 unit tests |
| `workflow-app/src/walk_tests.rs` | +116 | 3 walk tests: pause re-entry, terminal-less pause resume, position round-trip (incl. budget carry-over) |
| `workflow-app/tests/e2e_workflow_lifecycle.rs` | +320/−3 | the harness attaches the position double; `resumable_signoff_artifact` (browser step → wait → desktop step, published through the teaching compiler); 2 E2E tests |
| `workflow-durable/src/run_position_store.rs` | +134 | NEW: `DurableRunPositionStore` over one atomic snapshot (`run-positions.json`), mirroring the instance store's write contract incl. rollback-on-flush-failure; `discard` of an absent position performs no i/o |
| `workflow-durable/src/run_position_store_tests.rs` | +204 | NEW: 5 unit tests (parity with the double, checkpoint replacement, drop-and-reload, discard-survival, unparseable-snapshot determinism) |
| `workflow-durable/src/facade.rs` | +10/−5 | `DurableStores.run_positions` field + `RUN_POSITIONS_FILE` const + open wiring + layout doc line (all seven stores) |
| `workflow-durable/src/facade_tests.rs` | +33 | extended: empty-root, full-control-plane drop-and-reload, canonical layout, and the credential-scan test now cover `run-positions.json` |
| `workflow-durable/src/lib.rs` | +43/−30 | docs updated to the seven-store facade; module wiring + re-export |
| `workflow-durable/src/testutil.rs` | +27 | `run_position` fixture (control-plane state only) |
| `workflow-durable/tests/common/mod.rs` | +102/−28 | both harnesses attach their run-position store (`InMemoryRunPositionStore` / the durable store); `resumable_ir` + `sealed_resumable_version` fixtures |
| `workflow-durable/tests/e2e_durable_resume.rs` | +359 | NEW: the cross-plane restart E2E suite (3 tests) |

Zero `Cargo.toml`/`Cargo.lock`/`BUILD.bazel`/`MODULE.bazel.lock` changes; zero new external dependencies; zero edits outside `codex-workflow-app`, `codex-workflow-durable`, and their own test directories. `codex-workflow-contracts` was **not** touched: the `RunPosition` record composes existing serde-serializable types (`WorkflowInstanceId`, `WorkflowVersionId`, `IrNodeId`, `BindingDecision`, `BindingPolicy`, `ResourceBinding`) — no contract addition was needed.

## 3. Implementation Summary

**Persisted position (the port).** `RunPositionStore` is a new additive port in `workflow-app/src/port.rs`: `save` (keyed by instance identity, the latest checkpoint replaces the earlier), `load`, and `discard` (absent ⇒ ordinary no-op). The `RunPosition` record is control-plane state only — the walk's position (`WalkPosition`: pending re-entry node, continuation frames, budget consumed, visited path), the binding decisions resolved at instantiation, the policy, the walk budget, and the opaque credential-free resource identities. The IR, the version pin, and the digests are never part of it: rehydration re-loads the pinned version from the version store and re-runs `verify_integrity` end to end before anything executes.

**The checkpoint list** (persisted at the same boundaries the walk already records progress):

1. **Instantiation activation** — the run is resumable from the moment it starts, so a crash between instantiation and the first step completion still reconciles and resumes (a checkpoint-seam failure there settles the record `Failed` through the MWO-002 seam, mirroring the run path's internal-seam-failure contract).
2. **Every step completion** (`StepCompleted`) — the position advances past the settled step, so a crash resumes at its successor instead of re-executing it.
3. **Every pause** (`RunPaused`, wait/human-gate) — the position holds the wait node's pending re-entry point (its declared successor), which is exactly what a resume continues from. Enabling this required the walk's pause arms to record `self.current = node.next` before returning `Paused` — previously the successor was simply lost, which is precisely why the WO-011 report could defer walk continuation.
4. **Terminal settlement** (`Succeeded`/`Failed`) — the position is discarded: a settled record never looks resumable.
5. **Cancellation** — the position is discarded together with the in-flight-run takeover.

**Resume continuation** — `WorkflowLifecycle::resume_run(&instance)`:

- refuses while the lifecycle still holds an active run (`ActiveRunHeld` — dropping it silently would strand a `Running` record exactly like a crash);
- loads the instance record and requires it to be **`Running` already** — the explicit control-plane `Paused → Running` transition the host's `InstanceControl` seam performs. `resume_run` never performs that transition itself (auto-resume without an explicit control-plane transition is forbidden; a `Paused` record is refused with `InstanceNotRunning`, proven by test);
- loads the persisted position (present + version-pin cross-checked, `RunPositionUnavailable` otherwise) and the pinned version, re-verified end to end;
- re-establishes execution state exactly like the original run: `refresh_readiness` (adapters probed; bindings already `BOUND` in this process are skipped by the registry's own selectable-skip), the persisted resource identities re-attached, and every distinct binding the persisted decisions route through re-crosses the approval gate with a fresh grant (approvals stay fresh — the retry path's `restore_binding` precedent);
- restores the walk through `Walk::resume` (budget consumed carries over — a resumed run does not get a fresh budget) and **continues through the ordinary `run()` path** — the same checkpoints, recovery pipeline, evidence, and MWO-002 seam settlements apply, so the continuation is observable as the instance progressing past the pause point and completing.

**Startup reconciliation** — `WorkflowLifecycle::reconcile_startup(&records)`. The host enumerates the instance store (both the in-memory double and the durable store expose a `records()` audit handle) and hands the sweep the records; the sweep re-loads each `Running` candidate through the port (the store is authoritative), skips any instance whose live run this lifecycle still holds, and transitions every orphan through the one MWO-002 status-mutation seam (`Running → Paused`), recording a `Trace`-kind evidence payload `{ instance, operation: "reconcile-startup", reason }` and an `InstanceReconciled` event. **The reconciliation policy:** never silently `Running`, never auto-executing — the documented recovery state is `Paused` with recovery evidence awaiting an explicit resume. Idempotent by construction: reconciled records are `Paused`, so a second sweep finds nothing `Running` and returns empty (proven by test).

**Durable store.** `DurableRunPositionStore` mirrors the instance store's atomic-snapshot pattern exactly (temp → fsync → rename; in-memory rollback on flush failure; unparseable snapshot is a deterministic construction failure). The position of one instance is current-state (the latest checkpoint replaces the earlier one), so a snapshot — not a journal — is the established fit; the facade gains the seventh store (`run-positions.json`), and the canonical-layout plus no-credentials tests cover it.

## 4. Verification Results (real toolchain, pinned 1.95.0)

- `cargo test -p codex-workflow-app` → **36/36 green** (19 lib: 5 pre-existing + 11 `resume_tests` + 3 new `walk_tests`; 17 E2E: 15 pre-existing + 2 new).
- `cargo test -p codex-workflow-durable` → **46/46 green** (41 lib: 36 pre-existing + 5 `run_position_store_tests`; 5 E2E: the 2 pre-existing baselines — still green with the position store wired into the harnesses — plus 3 new `e2e_durable_resume` tests).
- `cargo test -p codex-workflow-triggers` → **23/23 green** (9 lib + 14 E2E — unchanged code, regression only).
- `cargo clippy -p codex-workflow-app -p codex-workflow-durable -p codex-workflow-triggers --all-targets -- -D warnings` → **clean** (`Finished`, zero warnings).
- `cargo fmt -p codex-workflow-app -p codex-workflow-durable -p codex-workflow-triggers -- --check` → **exit 0** (only the pre-existing repo-wide notice that `imports_granularity` in rustfmt.toml is nightly-only on stable — the same non-failure MWO-001 recorded).
- Family regression compile: `cargo check -p codex-workflow-distribution -p codex-workflow-evolution -p codex-eval-compat -p codex-resource-providers -p codex-env-adapters -p codex-workflow-triggers --all-targets` → **Finished clean** (`LifecycleDeps` was deliberately left untouched so every out-of-surface construction site — `eval-compat/src/harness.rs`, the `env-adapters`/`resource-providers`/`triggers` E2E harnesses — compiles unchanged).
- `codex-rs/target/` is removed after these results were recorded; the work tree is the commits described above.

## 5. The Restart E2E Evidence (the architect's cross-plane durability item)

`workflow-durable/tests/e2e_durable_resume.rs::cross_plane_restart_persists_position_reconciles_and_resumes_to_completion` runs the full scenario on the durable stores:

1. **create/start/pause** — install the resumable version (step → wait-for-HumanEvent → step), fire `u1`; the instance settles `Paused` awaiting `HumanEvent`, and the pause checkpoint's persisted position (current = `step-002`, path = `[step-001, wait-signoff]`) is asserted in `run-positions.json`.
2. **drop the whole application** — the harness (lifecycle, plane, stores, journals) goes out of scope; every store is closed.
3. **reload everything from disk** — `DurableStores::open(root)`; the persisted position re-observes identically (pending re-entry `step-002`); a sweep over the cleanly-paused store is a no-op.
4. **the crash signature** — the host resumes through the **real** `DurableInstanceControl` seam (`Paused → Running`, directive journaled), then the application is dropped again before any continuation: a `Running` record with no live run anywhere.
5. **reconcile** — reload, sweep: the orphan transitions to `Paused` with exactly one appended recovery trace, the `InstanceReconciled` event fires, and the second sweep is a no-op; the earlier resume directive is still journaled.
6. **resume + run to completion** — explicit resume through the control seam, then `resume_run`: terminal `Completed`, status `Succeeded`, path `[step-001, wait-signoff, step-002]` (progress past the pause point), the position discarded at settlement.
7. **integrity still verifies** — `verify_run` re-loads the pinned version and re-verifies it end to end (`VersionIntegrity` clean).
8. **trigger dedupe still holds** — re-ingesting `u1` through the re-wired plane is a recorded `Duplicate` (event id `evt-1`).

The companion test `a_plane_resumed_instance_continues_through_the_lifecycle` proves the trigger-plane composition named by the Work Order: fire to a pause, fire the awaited human event (the plane resumes `Paused → Running` through the control seam and settles `Resumed`), then the host application drives `resume_run` to `Succeeded`. `ordinary_codex_behavior_is_untouched_without_a_workflow` is the no-op E2E: no installation ⇒ the sweep is empty, the fire is `NotEligible`, `run()` is `NoActiveWorkflow`, and nothing is persisted.

At the application layer, `startup_reconciliation_recovers_an_orphaned_running_record` runs the same crash-recovery loop against the **real browser and desktop adapters**: a fresh lifecycle over the same stores (fresh registry, fresh approval source) sweeps an orphaned `Running` record to `Paused`, and two resume cycles walk it through the browser step, the wait, and the desktop step to `Succeeded` — proving rehydration re-probes adapters, re-attaches resources, and re-crosses the approval gate through a registry that starts empty.

## 6. Acceptance Criteria — Evidence per Packet Bullet

| Requirement | Evidence |
| --- | --- |
| Persisted position at the points the walk already records decisions; control-plane state only (IR/version pin/digests untouched) | `port.rs` (`RunPosition` composes contract/execution records, no semantics), `resume.rs::persist_position`; checkpoints 1–5 above; tests: `instantiate_persists_the_starting_position`, `a_resumed_run_continues_from_the_persisted_position_and_completes` (asserts the pause checkpoint's re-entry point), E2E phase 1; version re-verification on every rehydration (`resume_run` + `verify_run` assertions) |
| NEW additive port + in-memory double | `port.rs::RunPositionStore`, `memory.rs::InMemoryRunPositionStore`; parity test `save_load_discard_match_the_in_memory_double` |
| `run.rs`/`walk.rs` persist position at the documented checkpoints | `run.rs` `execute_run` (step completion, pause), `run()` settlement discard, `lifecycle.rs` instantiate checkpoint + cancel discard; `walk.rs` `position()`/`resume()` + pause re-entry points |
| Resume continuation after `InstanceControl::resume` (`Paused → Running`), rehydrates and continues from the persisted step — observable as completing/progressing past the pause | `resume.rs::resume_run`; tests: `resume_continuation_requires_the_control_plane_transition` (the anti-auto-resume guard), `a_resumed_run_continues_from_the_persisted_position_and_completes`, E2E `resumed_run_continues_past_the_wait_node_and_completes` (browser step does not re-execute, desktop step dispatches exactly once), `a_plane_resumed_instance_continues_through_the_lifecycle` (the real control seam + real plane), the cross-plane E2E phases 5–6 |
| Startup reconciliation: `Running` with no live `ActiveRun` → the documented recovery state through the MWO-002 seam, `Paused` with recovery evidence awaiting explicit resume; never silently `Running`, never auto-executing; idempotent | `resume.rs::reconcile_startup`; tests: `reconcile_startup_transitions_orphaned_running_records_to_paused`, `the_reconciliation_sweep_is_idempotent`, `reconciliation_leaves_non_running_records_untouched` (all six statuses), E2E `startup_reconciliation_recovers_an_orphaned_running_record` + the cross-plane E2E phases 4–5 |
| Durable `RunPositionStore` + durability tests | `run_position_store.rs` + `run_position_store_tests.rs` (5 tests); facade integration tests |
| Cross-plane restart E2E on the durable stores (drop the application, reload, reconcile, resume, complete, integrity verifies, dedupe holds) | `e2e_durable_resume.rs` (section 5 above) |
| Ordinary Codex compatibility: no workflow active → nothing changes (a no-op E2E) | `e2e_durable_resume.rs::ordinary_codex_behavior_is_untouched_without_a_workflow`; the pre-existing app E2E `ordinary_codex_behavior_is_untouched_without_a_workflow` stays green (a lifecycle constructed via `new` keeps the pre-MWO-003 behavior exactly — `persist_position`/`discard_position` are no-ops without the seam, verified by `resume_run_without_a_position_store_is_refused` and every pre-existing test) |
| Verification: `cargo test -p codex-workflow-app -p codex-workflow-durable`; clippy; fmt; family regression compile | section 4 — all green/clean on 1.95.0 |
| Forbidden: new engines/runtimes; new external deps; contract reshaping; auto-resume without an explicit control-plane transition; mutating workflow semantics during rehydration; edits outside the named crates | the walk continuation reuses the existing `run()` path (no second engine); zero manifest changes; `codex-workflow-contracts` untouched; the status precondition + `InstanceNotRunning` refusal; rehydration re-verifies the pinned version and never adopts semantics from the position record; the diff touches only the two authorized crates (their own tests included) |

## 7. M4 Completion Statement

The five declared M4 durable-control-plane requirements are closed at their owning planes: **stores durable** (MWO-001), **transitions enforced** (MWO-002), **cancel works** (MWO-002), **resume continues** (MWO-003), **restart reconciles** (MWO-003). What remains open is integration-level follow-up, none of it inside MWO-003's authorized change surface:

1. **The trigger plane does not itself drive the continuation.** `WorkflowTriggerPlane::start_or_resume` performs the `Paused → Running` resume through the control seam and settles `Resumed`; the host application then calls `WorkflowLifecycle::resume_run` (the composition the E2E demonstrates). Wiring the call inside `codex-workflow-triggers` was outside this WO's authorized surface — it is a small, well-bounded follow-up WO on that crate.
2. **Await re-registration after reconciliation.** The sweep lives in `codex-workflow-app`; the trigger ledger's await registrations are derived from the trigger plane's settlements. In the resume-then-crash case the await was already cleared by the resuming fire's `Resumed` settlement, and the sweep cannot re-register it (no port reaches the ledger from the app crate). The documented host path (direct control-seam resume, as the E2E drives) is unaffected; re-deriving awaits on reconcile belongs to the same trigger-plane follow-up.
3. **Cancellation races** across separate lifecycle owners sharing stores (the MWO-002 report's note) remain a validation-program concern (VWO scenario), not an MWO remediation item.

## 8. Compatibility Impact

Ordinary Codex behavior is untouched. `LifecycleDeps` and `WorkflowLifecycle::new` are unchanged, so every existing construction site (including the out-of-surface `eval-compat` harness and the `env-adapters`/`resource-providers`/`triggers` E2E harnesses) compiles and behaves identically — proven by the family regression compile and the 23 unchanged trigger-plane tests. A lifecycle without an attached position store performs no persistence and no rehydration (the pre-MWO-003 behavior exactly); all 15 pre-existing app E2E tests and both durable baselines stay green with the seam attached, because the checkpoints write only run-position records. No new external dependencies, no manifest changes, no threads, no network.

## 9. Known Limitations

1. **Checkpoint granularity is step completion.** A crash mid-step (after `StepStarted`, before `StepCompleted`) resumes by re-executing that step — at-least-once step semantics. The partial attempt's events and evidence remain (append-only by design); side effects of the incomplete step are the environment adapters' domain (the same characteristic any checkpoint-based resumable executor has).
2. **The sweep needs enumerated records.** The frozen `WorkflowInstanceStore` port has no listing method, and adding a defaulted one was rejected (a defaulted "unsupported" answer would make reconciliation silently impossible on non-listing hosts). Both the in-memory double and the durable store expose `records()`; the sweep's doc states the contract.
3. **Snapshot write amplification.** `run-positions.json` rewrites its whole ordered map per checkpoint — O(state) per step completion, the same characteristic MWO-001 documented for the other snapshot stores. Correct and simple for control-plane volumes; a hybrid journal is a future optimization.
4. **Single-active lifecycle.** `resume_run` refuses (`ActiveRunHeld`) while any active run is held rather than silently replacing it; a host must settle or drop the held run first.
5. **Rehydration appends fresh approval evidence** for every distinct re-established binding — deliberate (approvals stay fresh), and observable as evidence growth across restart cycles.
6. `Paused -> Failed` remains declared-but-unexercised by live code (carried over from MWO-002's report; this WO's failure settlements occur from `Running`).

## 10. Deferred Items + Owning WO

- **Trigger-plane wiring of `resume_run` after its control resume, and await re-registration after reconciliation** → a bounded follow-up Work Order on `codex-workflow-triggers` (owner: the Tech Lead's next dispatch; the composition is already proven by `a_plane_resumed_instance_continues_through_the_lifecycle`).
- **Journal/snapshot hybrids or compaction for the snapshot stores** (incl. `run-positions.json`) → future optimization WO if control-plane volumes demand it (MWO-001's standing deferral, unchanged).
- **Cancellation-race hardening across shared-store lifecycles** → the validation program (VWO scenarios) per the M4 gate's authority rules.

## 11. Risks

1. **The pause-arm change in `walk.rs`** (recording the pending re-entry point before returning `Paused`) alters the walk's internal state at a terminal. Verified compatible: the visited path and terminal outcomes are byte-identical (the teaching-compiler equivalence test stays green), no pre-existing caller inspected `walk.current` after a pause (audited: `run()` consumes and drops the walk), and the change is exactly what makes resumption possible at all.
2. **Tech Lead compiles independently:** all results here are real (rustup 1.95.0, exact commands in section 4), but toolchain/lock resolution may differ cosmetically. Zero manifest changes means zero lock drift.
3. **Bazel lockfile:** per repo AGENTS.md, Cargo changes normally require `just bazel-lock-update` — this WO makes no manifest changes at all (nothing to lock), and the workflow leaf crates carry no BUILD.bazel entries (the MWO-001 precedent).
4. **The at-least-once step semantics** (limitation 1) surface only under mid-step crashes; hosts needing exactly-once steps need idempotent actions (the execution-contracts domain, unchanged by this WO).

## 12. Artifacts

- Branch `mwo-003/resume-reconciliation` at `/home/z/codex` (base `2c01fcd89`, implementation commit + this report commit on top).
- Git bundle: `/home/z/my-project/mwo-003-delivery.bundle` (range `2c01fcd89..mwo-003/resume-reconciliation`).
- Sandbox working tree retained for harvest via the file API; `codex-rs/target/` removed after the results were recorded.
