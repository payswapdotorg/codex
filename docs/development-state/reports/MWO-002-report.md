# MWO-002 Report — Legal Transitions and Cancellation

**Work Order:** MWO-002 (docs/work-orders/MWO-002-LEGAL-TRANSITIONS-CANCEL.md)
**Status:** MERGED via PR #19 (squash `03927322311a0299b9560487ae0ba653f6993836`); independently verified by the Tech Lead on toolchain 1.95.0 (contracts 90, app 5+15, triggers 23, dependents compile, clippy/fmt clean; two integration fixes applied: test-local harness shadowing rename, reason.clone -> &reason)
**Program:** M4 durable-control-plane remediation (architect mandate TECH_LEAD_START_HERE.md §16)
**Architecture:** 0.2.0

## Delivery

- **Worker session:** MWO-002 Worker (full-stack agents-tab session; this report's author).
- **Base:** main @ `7b7dfa0cb2499a68d145fe15193ec0e786dd4284`, verified equal to origin/main at dispatch (`git rev-parse 7b7dfa0cb^{commit}` reachable and identical to `origin/main`).
- **Branch:** `mwo-002/workflow-lifecycle`, one commit: `feat(workflow-lifecycle): MWO-002 legal transitions and cancellation` (exact head SHA recorded in the completion message and verifiable via the delivery bundle).
- **Bundle:** `mwo-002-delivery.bundle` at the sandbox file-API-visible root, range `7b7dfa0cb..mwo-002/workflow-lifecycle`.
- **Change surface (bounded exactly as authorized):** two crates, 8 files, +579/−25 lines, zero new crates, zero new external deps, zero `Cargo.toml`/`Cargo.lock`/`MODULE.bazel.lock` changes, zero edits outside the two named crates:
  - `codex-workflow-contracts`: `src/instance.rs` (the declared table + helpers), `src/instance_tests.rs` (table tests).
  - `codex-workflow-app`: `src/error.rs` (`IllegalStatusTransition` variant), `src/event.rs` (`InstanceCancelled` variant), `src/lifecycle.rs` (the one seam, `cancel`, routed mutation sites, test module declaration), `src/run.rs` (routed settlement sites), `src/lifecycle_tests.rs` (NEW sibling test module), `tests/e2e_workflow_lifecycle.rs` (cancel E2E suite + extracted `await_signoff_artifact` helper).

## Verification statement (read first)

**This sandbox has no Rust toolchain** (`which cargo`/`rustc`/`rustup` all absent; no `~/.cargo`, no `~/.rustup`). Consequently:

- `cargo test -p codex-workflow-contracts -p codex-workflow-app`, `cargo clippy ... -- -D warnings`, and `cargo fmt ... -- --check` were **NOT executed**. No test counts below are execution results.
- Instead, **strongest-possible static verification** was performed: every added name was cross-checked against the live tree's actual signatures (`from_digest`, `ContentDigest::of`, `InMemoryApprovalSource::approving`, `CapabilityRegistry::new`, port trait methods, `ActiveRun` field visibility); borrow/reborrow patterns mirror in-file precedents (`fail_instance(&mut instance, ...)` call chains); exhaustiveness breakage from the new `WorkflowEvent`/`WorkflowAppError` variants was ruled out by grepping every `match` site in the workflow family (all wildcard-armed or non-exhaustive `matches!`); the enum itself gained no variants (only an `impl` block), so `eval-compat`'s 6-arm `status_projection` and all dependents compile unchanged; every line was length-checked against rustfmt's `max_width=100`, chain breaks follow the repo's proven `chain_width` vertical style, and `matches!` interiors (which rustfmt preserves verbatim — proven by a pre-existing 101-char line at `e2e_workflow_lifecycle.rs:1226`) mirror file-local forms; clippy deny-list rules were checked item-by-item (`expect_used`/`unwrap_used` only under `clippy.toml`'s `allow-expect-in-tests`, `uninlined_format_args` inlined, `redundant_clone` avoided via a needed `reason.clone()`, no small single-use helper methods added on `WorkflowLifecycle` beyond the seam itself).
- **The Tech Lead compiles and runs everything independently** (the dispatch says so explicitly). Given the WO-016 precedent where the Tech Lead's real toolchain caught worker static-verification misses, the risk that a real compile surfaces something remains nonzero and is acknowledged here rather than hidden.

## Implementation surface

### Contracts (`codex-workflow-contracts/src/instance.rs`, additive only)

`WorkflowInstanceStatus` gains an `impl` block mirroring the `ReadinessState` precedent shapes (`ALL` const, `matches!`-based const predicate, slice-returning lookup — `legal_from` is non-const in the precedent, so `legal_transitions`/`can_transition_to` are non-const too; `is_terminal` is a `const fn` like `is_diagnostic`):

- `pub const ALL: [Self; 6]` — every status, for exhaustive table tests.
- `pub fn legal_transitions(self) -> &'static [Self]` — the declared table (below).
- `pub fn can_transition_to(self, target: Self) -> bool` — table lookup (`legal_transitions().contains(&target)`, the same containment idiom `ReadinessTracker::advance` uses).
- `pub const fn is_terminal(self) -> bool` — `Succeeded | Failed | Cancelled`.

The enum's variants, serde shape, and every other contract record are untouched. `lib.rs` needed no change (the helpers live on the already-exported type). The module/enum doc comments were updated because the old text ("transition rules ... are intentionally not encoded here") became false the moment the M4 remediation moved the declared table here.

### Application (`codex-workflow-app`)

- **The one status-mutation seam** — `WorkflowLifecycle::transition_status(&mut self, instance: &mut WorkflowInstance, target) -> Result<(), WorkflowAppError>` (pub(crate), same-crate reusable by MWO-003's reconciliation sweep per its packet): validates `instance.status.can_transition_to(target)`, mutates the in-memory record only on success, and returns `WorkflowAppError::IllegalStatusTransition { instance, current, target }` on refusal. Each caller persists through the normal instance-store save path and emits its own event, preserving today's per-site ordering.
- **`WorkflowLifecycle::cancel(&mut self, instance: &WorkflowInstanceId, reason: impl Into<String>) -> Result<WorkflowInstance, WorkflowAppError>`** — synchronous (the ports are deliberately synchronous record seams). Flow: load the durable record (`InstanceUnavailable` when missing) → **fail fast** through the seam (`Pending/Running/Paused -> Cancelled`; double-cancel and cancel-of-terminal refused before any side effect) → drop the in-flight `ActiveRun` **when it belongs to this instance** (`is_some_and` match on instance id) so no later `run()` can settle the terminal record → record the operator-visible reason as a `Trace` evidence payload `{ "instance", "operation": "cancel", "reason" }` through the existing `store_evidence` seam (which appends the reference and persists the record through the normal save path) → emit `WorkflowEvent::InstanceCancelled { instance, reason }` → return the settled record.
- **`WorkflowEvent::InstanceCancelled { instance, reason }`** — appended at the end of the enum (additive; externally-tagged serde, camelCase `instanceCancelled`; no `deny_unknown_fields` concern for an additive variant).
- **`WorkflowAppError::IllegalStatusTransition { instance, current, target }`** — placed with the instance errors; typed statuses for programmatic matching (`{current:?}`/`{target:?}` thiserror formatting, the same Debug-interpolation idiom the file already uses for `{code:?}`).

## The final transition table

```text
Pending   -> Running | Failed | Cancelled
Running   -> Paused | Succeeded | Failed | Cancelled
Paused    -> Running | Failed | Cancelled
Succeeded, Failed, Cancelled   terminal: no transitions
```

26 of the 36 status pairs are illegal, including every self-transition, every transition out of a terminal status, and every settlement of a non-terminal status into `Pending`.

## Enforcement-point list (every workflow-app status mutation, before vs after)

| # | Site | Transition | Before | After |
|---|------|-----------|--------|-------|
| 1 | `lifecycle.rs` `instantiate` (record creation) | allocation of `Pending` via `WorkflowInstance::new` | constructor call | unchanged — creation is contract-level record allocation, not a transition; the constructor's public contract (used by MWO-001 durable stores and tests) is intentionally untouched ("no contract reshaping") |
| 2 | `lifecycle.rs` `instantiate` start | `Pending -> Running` | direct field assignment | `transition_status` seam, then the existing save + `InstanceStarted` event |
| 3 | `lifecycle.rs` `fail_instance` (validate/approve/bind gate failures) | `Pending -> Failed` (documented instantiation-failure settlement) | direct field assignment | seam; a refused settlement (already-terminal record) is returned as the `IllegalStatusTransition` error instead of being silently dropped, and no event/save happens in that case |
| 4 | `run.rs` `run` settlement, `RunTerminal::Completed` | `Running -> Succeeded` | direct field assignment | seam + existing `RunCompleted` event ordering |
| 5 | `run.rs` `run` settlement, `RunTerminal::Paused` | `Running -> Paused` | direct field assignment | seam + existing `RunPaused` event ordering |
| 6 | `run.rs` `run` settlement, `RunTerminal::Failed` | `Running -> Failed` | direct field assignment | seam + existing `RunFailed` event ordering |
| 7 | `lifecycle.rs` `cancel` (NEW) | `Pending/Running/Paused -> Cancelled` | (did not exist) | seam, evidence, event, save |

Not routed, deliberately: `workflow-triggers`' `InMemoryInstanceControl::resume` (`Paused -> Running`, `workflow-triggers/src/memory.rs:484`) keeps its **own** explicit guard (`IllegalResume` unless `Paused`) — it is a separate crate outside this WO's authorized surface, its rule is a strict subset of the contract table, and the app-level seam does not demand touching it. Verified, not edited. (The real host's `InstanceControl` implementation will validate against the same contract table through MWO-003's seam composition.)

## Discrepancy audit (table vs live behavior)

Audited every `instance.status =` / `WorkflowInstanceStatus::` construction site in the whole `codex-rs` tree at the base SHA (production and tests), plus the docs' contract statements:

1. **`Pending -> Failed` — live tree performs it; the packet's "at minimum" table omits it.** Performed by `fail_instance` when a validate/approve/bind gate refuses during instantiation, **documented in the live code** (`lifecycle.rs` doc: "On any phase failure the durable instance record is settled as `Failed` with the structured reason before the original error is returned") and **asserted by two existing E2E tests** (`e2e_approval_denial_blocks_execution_before_any_action`, `e2e_policy_scope_denies_the_browser_environment`, both `records[0].status == Failed` before any execution). The packet's own observed-transition list includes "failure → Failed" without restricting the source status, and the table is explicitly "at minimum". **Reconciliation: the encoded table includes `Pending -> Failed` — a widening beyond the packet's minimum list, reported here explicitly, never silently blessed.** The alternative (forbidding it) would break the documented fail-fast settlement, break two existing-green tests, and narrow live documented behavior. This is the only deviation from the packet's minimum table.
2. **`Paused -> Failed` — the packet's minimum table declares it; no live code exercises it today.** Kept as declared (the packet is the documented contract; the natural exerciser is MWO-003's resume continuation, where a resumed run's failure would settle from `Running`, and a reconciliation failure could settle from `Paused`). Reported for completeness.
3. **`Cancelled` never constructed in production at base** (the M4 audit's finding) — confirmed by the same grep: only serde/tests referenced it. This WO adds the one legal constructor path (`cancel`).
4. **All other live transitions** (`Pending -> Running`, `Running -> {Paused, Succeeded, Failed}`, `Paused -> Running` in triggers) are inside the packet's minimum table. **No transition the table forbids is performed anywhere in the live tree.**

## Cancellation

- Legal sources: `Pending`, `Running`, `Paused` (all non-terminal), enforced by the seam — so cancel-before-start, cancel-from-Running, and cancel-from-Paused are the three legal operator entry points.
- One `WorkflowEvent` per cancellation: `InstanceCancelled { instance, reason }`, emitted after the record has been persisted through `store_evidence`'s normal save path.
- Evidence per the existing conventions: a `Trace`-kind payload (the dispatch traces in `run.rs` are the precedent for structured control-plane-operation traces: `{ node, binding, operation, outcome, ... }`; the cancel trace is `{ instance, operation: "cancel", reason }`). No new `EvidenceKind` was added (the WO forbids that); `Trace` was chosen over `Recovery` because Recovery-kind payloads are `codex_execution_contracts::Recovery` records (binding + failure + strategy) by established convention, which a cancellation is not. The payload digests through the normal evidence-store path and is appended to the instance's evidence list.
- Double-cancel and cancel-of-terminal: explicit `IllegalStatusTransition` errors returned **before** any evidence record, event, or store write (fail-fast ordering in `cancel`), proven side-effect-free by the E2E double-cancel test (event count, evidence count, and stored status all unchanged across the refused call).
- **No semantic mutation:** the cancel path never touches `instance.version`, digests, or the version record; `verify_run` after cancellation re-loads the pinned version and re-runs `verify_integrity` end to end (asserted in the E2E tests). The in-flight `ActiveRun` (walk position, decisions — control-plane run state, not workflow semantics) is dropped when it belongs to the cancelled instance.

## Acceptance-criteria evidence (packet bullet → code + test)

1. *"Declared, enforced legal-transition table mirroring the ReadinessState::legal_from precedent"* — `instance.rs` `impl WorkflowInstanceStatus` (shapes mirror `ALL`/`legal_from`/`is_diagnostic`); exhaustive pair test `every_status_pair_matches_the_declared_legal_table` (all 36 pairs against an independently written `LEGAL_PAIRS` expectation) + `legal_transitions_list_exactly_the_declared_targets` (content and order) + `terminal_statuses_admit_no_transitions` + `observed_live_transitions_stay_legal`.
2. *"Statuses are direct field assignments with no legality guard" (the audit finding) — fixed* — the enforcement-point table above; grep-clean: `rg '\.status\s*=\s*WorkflowInstanceStatus'` in `codex-workflow-app/src` now returns only the seam's single assignment inside `transition_status` (plus the contract-level constructor call in `instantiate`).
3. *"Implement the missing cancel operation; Cancelled is never constructed"* — `WorkflowLifecycle::cancel`; `Cancelled` is now constructed exactly once, through the seam.
4. *"Single transition seam ... returns an explicit IllegalStatusTransition-shaped error"* — `transition_status` + `WorkflowAppError::IllegalStatusTransition`; unit test `the_seam_enforces_the_declared_table_for_every_status_pair` drives the **real seam** for all 36 pairs (legal pairs mutate, every illegal pair is refused with `current`/`target` exactly matching the pair and the record left untouched).
5. *Table at minimum* — encoded with the one reported reconciliation (`Pending -> Failed`); see the discrepancy audit.
6. *"Existing tests must stay green"* — behavior on every legal path is byte-for-byte the previous ordering (validate-then-mutate-then-save-then-event); no existing test's observable outcome changes; the only pre-existing code touched is the assignment sites and two doc comments. The wait-node test was mechanically refactored (its artifact construction extracted into `await_signoff_artifact`, identical logic) to share it with the paused-cancel test — a pure extraction, justified here.
7. *"Running and Paused instances cancel to Cancelled through the seam; one WorkflowEvent; evidence per existing conventions; store save through the normal path"* — `cancel` implementation + E2E `cancel_from_running_settles_the_instance_as_cancelled` and `cancel_from_paused_settles_the_instance_and_refuses_double_cancel`.
8. *"Double-cancel and cancel-of-terminal are explicit illegal-transition errors, never silent no-ops"* — E2E double-cancel (typed `IllegalStatusTransition { current: Cancelled, target: Cancelled }`, zero new events/evidence, store record unchanged) and `settled_instances_are_immutable_under_cancel` (Succeeded→Cancelled and Failed→Cancelled both refused with typed current/target; stored statuses unchanged).
9. *"Cancellation does NOT mutate workflow semantics: version pin, integrity digests, dependency identities untouched"* — E2E: `verify_run` post-cancel re-verifies the pinned version end to end (`assert_eq!(verified.version, artifact.version.version_id)` + `verify_evidence` digest checks) in the running, paused, and before-start tests.
10. *"E2E: cancel from Running, cancel from Paused, cancel-before-start, illegal-transition rejection for every forbidden pair, terminal immutability"* — respectively: `cancel_from_running_...` (also proves the run takeover: later `run()` is `NoActiveWorkflow` and zero actions dispatched), `cancel_from_paused_...`, `cancel_before_start_...` (seeded Pending record; exactly one event, no active workflow), the 36-pair seam unit test + the exhaustive contracts table tests, and `settled_instances_are_immutable_under_cancel` (all three terminal statuses covered across the two tests, `Cancelled` via double-cancel).
11. *"workflow-triggers' InstanceControl::resume keeps its Paused→Running-only rule"* — verified unchanged (`IllegalResume` guard intact); no edit made or needed; justification in the enforcement-point section.
12. *"Report at docs/development-state/reports/MWO-002-report.md"* — this file.

## Known limitations / deferrals

- **Static verification only** (see the verification statement): the Tech Lead must run `cargo test -p codex-workflow-contracts -p codex-workflow-app`, clippy `-D warnings`, and fmt for real; regressions for the dependents (`codex-workflow-triggers`, `codex-workflow-evolution`, `codex-eval-compat`, `codex-env-adapters`, `codex-resource-providers`) are compile-only concerns here — all usages were statically confirmed compatible (comparisons and already-exhaustive 6-arm matches; no enum variant was added).
- **Cancel does not interrupt a run that is mid-`await`**: `cancel` and `run` both take `&mut self` on the same lifecycle, so they serialize on a single owner; cancel between run invocations (including from another lifecycle handle bound to the same durable stores) is the modeled control-plane path. A concurrent run that settles between cancel's load and its save would surface as a store-level last-writer-wins on the shared record seam — the cross-plane race window is inherent to the record-seam design and is explicitly MWO-003's restart/reconciliation territory (its packet owns "cancellation races" under the validation program).
- **Evidence/instance persistence is not cross-plane atomic** (evidence store write then instance save, mirroring the existing `store_evidence` behavior); a failure between them leaves evidence without the appended reference — the same characteristic the run path already has; MWO-001/MWO-003 own durability and reconciliation.
- **The cancel reason string is unbounded**, matching every existing `reason: String` in the event/error surface (no clamping precedent exists for operator-supplied reasons in this crate; failure messages are bounded by `ExecutionFailure`, but operator reasons are host-owned data).
- `Paused -> Failed` is declared but not yet exercised by live code (see the discrepancy audit); it awaits MWO-003.
- `lifecycle.rs` is 514 LoC (was 411), slightly over the AGENTS.md soft target of 500 but well under the 800 hard line and below the same crate's `run.rs` at 556; the WO names `lifecycle.rs` as the authorized home for the seam and `cancel`.

## Forbidden-change compliance

- No new crates, engines, or external deps (no manifest changes at all).
- No frozen invariant touched: version immutability (integrity re-verified post-cancel in tests), trigger idempotency (triggers crate untouched), authorization gates untouched, evidence kinds untouched (only the additive cancellation **event**, as authorized).
- No edits outside the two named crates.
