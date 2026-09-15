# RWO-011 — Rejection/Conflict Audit Events in the Shared Fixture Runtime: Completion Report

**Status:** COMPLETE — branch `rwo-011/rejection-audit-events`
**Base:** main @ f501d65fc (post-RWO-010)
**Work order:** `docs/validation/work-orders/RWO-011.md` (Family J; closes VWO-007 F2, VWO-008 F6, VWO-009 F3)
**Scope:** shared fixture runtime + README vocabulary — no Rust, no protocol, no zst, no engine, no per-app fixture changes

## What was built

One emit site in the shared request pipeline makes every repelled operation
durably auditable across all five fixture apps:

1. **`_lib/runtime.js` — `handle()`**. The catch block (the single error path
   every route shares) now records a typed `request.rejected` event for every
   `AppError` with status **409, 403, or 503** before the error propagates:
   `{op, code, status, actor}` in `data`, `subject: op:<opKey>`,
   `summary: "Rejected <op> — <code> (HTTP <status>)"`, plus
   `simulated: true` when the switch drove the rejection. The op key reuses
   the idempotency ledger's convention (`route.opKey || METHOD pattern`), so
   rejection events and dedup signatures name operations identically. To make
   the matched route and authenticated actor visible to the catch block,
   their declarations were hoisted above the `try` — behavior-neutral for the
   success path.
2. **Additive by design.** The idempotency-dedup ledger, success-path events,
   natural record state, and every HTTP response are unchanged; the audit
   emit sits in its own `try/catch` so it can never mask the original error.
   Validation 400s and 401/404 stay silent — they are not repelled
   attacks/races (a probe against a domain 404 `region_not_found` confirmed
   silence).
3. **README event vocabulary** — new contract bullet: repelled operations
   are *auditable*, covering 409/403/503 with the event shape.

**Scope decision (documented deviation):** the WO's fix-scope parenthetical
names "status 409/403", but its own AC3 requires the *503-refused upgrade*
to appear in the feed. The emit therefore covers 409/403/503 — all
repelled-operation statuses. 503s only fire on state-changing requests
(reads still work), so the AC3 semantics and the "NOT validation 400s"
exclusion both hold.

## Acceptance criteria → evidence (all real runs; `docs/validation/evidence/rwo-011/`)

1. **Storm probe (VWO-007 F2):** 8 parallel identical driver-intake POSTs →
   1× HTTP 200 + 7× HTTP 409, and the feed shows **exactly 7
   `request.rejected` events** naming `POST /api/intake/apply` /
   `duplicate_event` (natural plate-collision duplicates; no simulated flag).
2. **Approval-race context (VWO-008 F6):** the losing race submission
   (parallel decides on prq-5003: 200 + 409) and the stale re-submission
   both record `data_conflict` events with the actor (`dana.reyes`); the
   permission-denied probe records `permission_denied` (403) with actor
   `marco.silva`.
3. **Distribution context (VWO-009 F3):** the 503-refused upgrade records
   `service_unavailable` (HTTP 503, `simulated: true`); the stale_entitlement
   block records `stale_entitlement` (`simulated: true`); the natural
   stale-`expectedCurrentVersion` upgrade records `data_conflict` with no
   simulated flag.
4. **Regression:** `verify-sweep.sh` → **59/59 PASS** after the change. The
   sweep contains no exact event-count assertions (checked before merging);
   its own 409 switch probes now additionally produce rejection events, which
   is the intended behavior.

## Lineage

Tech-Lead direct execution (single-file bounded scope; rwo-007 precedent).
Implementation audited live: the hoist is behavior-neutral, the emit is
exception-guarded, the dedup ledger path was re-exercised by the storm probe,
and the domain-404 silence case was probed explicitly.
