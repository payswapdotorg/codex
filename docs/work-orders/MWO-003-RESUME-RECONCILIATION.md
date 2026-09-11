# MWO-003 — Persisted Execution Position and Restart Reconciliation

**Status:** MERGED (PR #21, squash `6b60895ba`)
**Depends on:** MWO-001 (durable stores), MWO-002 (legal transitions + cancel)
**Program:** M4 durable-control-plane remediation
**Architecture:** 0.2.0

## Objective

Close the last two M4 gaps: **resume continuation** (a resumed instance actually continues executing from persisted position — today resume marks Paused→Running and nothing runs; walk continuation was explicitly deferred in the WO-011 report) and **startup reconciliation** (post-crash, `Running` instances with no live run are detected and transitioned per a documented policy — today crash leaves phantom Running records forever).

## Authorized change surface

- `codex-workflow-app`: a NEW additive port for persisted run position (e.g. `RunPositionStore`: save/load the active run's step position, decisions, and pending re-entry point; provide the in-memory double in `src/memory.rs`), `src/run.rs`/`src/walk.rs` persist position at the documented checkpoints, resume continuation rehydrates and continues the walk, a startup reconciliation sweep (e.g. `WorkflowLifecycle::reconcile_startup(...)` or equivalent) for orphaned Running instances, plus tests.
- `codex-workflow-durable` (MWO-001's crate): the durable `RunPositionStore` implementation + durability tests for it.
- `codex-workflow-contracts`: only if a record type is genuinely required; prefer composing from existing contract types. Any contract addition must be additive and justified in the report.

## Requirements

- **Persisted position:** the run persists its position at the same points the walk already records decisions; the record is control-plane state (never workflow semantics — the IR, version pin, digests are untouched).
- **Resume continuation:** after `InstanceControl::resume` (Paused→Running), the lifecycle rehydrates the run position and continues the walk from the persisted step — observable as the instance completing or progressing past the pause point in tests.
- **Startup reconciliation:** given the instance store, instances in `Running` with no live ActiveRun (the post-crash signature) transition through the MWO-002 seam to a documented recovery state (Paused with recovery evidence awaiting explicit resume, per the control-plane authority rules — never silently Running, never auto-executing), and the sweep is idempotent.
- **Cross-plane restart E2E:** a full scenario on the durable stores — create/start/pause, drop the whole application (stores closed), reload everything from disk, reconcile, resume, run to completion, assert integrity still verifies and trigger dedupe still holds. This is the architect's "cross-plane durability" item.

## Verification

`cargo test -p codex-workflow-app -p codex-workflow-durable`; clippy; fmt; family regression compile. Report at `docs/development-state/reports/MWO-003-report.md`.

## Forbidden

- New engines/runtimes; new external deps; contract reshaping; auto-resume without explicit control-plane transition; mutating workflow semantics during rehydration; edits outside the named crates.

## Completion report

Standard format plus: the checkpoint list, the reconciliation policy, the restart E2E evidence, and the honest statement of which M4 requirements remain open after this WO (if any).
