# MWO-002 — Legal Transitions and Cancellation

**Status:** MERGED (PR #19, squash `039273223`)
**Depends on:** none (parallel with MWO-001)
**Program:** M4 durable-control-plane remediation
**Architecture:** 0.2.0

## Objective

Give `WorkflowInstanceStatus` a **declared, enforced legal-transition table** (mirroring the `ReadinessState::legal_from` precedent in `execution-contracts/src/readiness.rs`), and implement the missing **cancel** operation. The M4 audit found: statuses are direct field assignments with no legality guard, and `Cancelled` is never constructed anywhere.

## Authorized change surface (bounded edits to two existing crates — additive-first)

- `codex-workflow-contracts` (`src/instance.rs` + `src/instance_tests.rs`): the transition table + helpers, e.g. `legal_transitions()`, `can_transition_to()`, and an `IllegalStatusTransition`-shaped error or a re-exported contract error — follow the crate's existing conventions.
- `codex-workflow-app` (`src/lifecycle.rs`, `src/event.rs`, `src/run.rs`, `src/error.rs` + tests): a single transition seam (all status mutations route through one helper that validates against the table), a `WorkflowLifecycle::cancel(instance, reason)` operation, and a `WorkflowEvent` variant for cancellation.
- No other crates. No contract reshaping beyond the additive table/error/event. `workflow-triggers`' `InstanceControl::resume` keeps its Paused→Running-only rule.

## The legal table (derive + verify from live source, then encode)

Observed transitions today: create → `Pending`; start → `Running` (Pending→Running); Wait/HumanGate → `Paused` (Running→Paused); completion → `Succeeded`; failure → `Failed`; recovery escalation → `Failed`; resume → `Running` (Paused→Running, triggers seam).

Encode, at minimum:
- `Pending → {Running, Cancelled}`
- `Running → {Paused, Succeeded, Failed, Cancelled}`
- `Paused → {Running, Failed, Cancelled}`
- `Failed`, `Succeeded`, `Cancelled` terminal.

Audit the actual code and tests first; if the live tree performs a transition this table forbids (or the docs permit one the table omits), reconcile to the documented contract and REPORT the discrepancy — never silently widen the table to bless an undocumented transition. Existing tests must stay green (or be updated with justification in the report).

## Cancellation

- `Running` and `Paused` instances cancel to `Cancelled` through the seam: one `WorkflowEvent` (e.g. `InstanceCancelled { instance, reason }`), evidence recorded per the existing evidence conventions, store save through the normal path.
- Double-cancel and cancel-of-terminal are explicit illegal-transition errors, never silent no-ops.
- Cancellation does NOT mutate workflow semantics: version pin, integrity digests, dependency identities untouched; evidence records the operator-visible reason.

## Verification

`cargo test -p codex-workflow-contracts -p codex-workflow-app`; clippy `-D warnings`; fmt; regressions for the whole workflow family compile (dependents of the two crates). E2E: cancel from Running, cancel from Paused, cancel-before-start, illegal-transition rejection for every forbidden pair, terminal immutability. Report at `docs/development-state/reports/MWO-002-report.md`.

## Forbidden

- New crates, new engines, new external deps.
- Changing any frozen invariant: version immutability, trigger idempotency, authorization, evidence kinds beyond the additive cancellation event.
- Edits outside the two named crates.

## Completion report

Standard format plus: the final transition table, the enforcement-point list (every mutation site that now routes through the seam), the discrepancy audit (table vs live behavior), and cancel acceptance evidence.
