# RWO-003 Completion Report (filed by Tech Lead)

**Work Order:** RWO-003 — FlowMart cross-tenant install write guard (VWO-010 Family E)
**Session:** chat.z.ai agents-tab, GLM-5.3 + Full-Stack, chat e940d86e-4de2-4410-a1f4-f69ae906df48
**Dispatched:** 2026-09-12 18:25 UTC · **Completed:** 2026-09-12 ~23:17 UTC (incl. peak-hours stall-recovery cycles)
**Delivery:** git bundle RWO-003-delivery.bundle (head 486120ffd6d2a6fef710398576da6f231438ee0f, exactly 1 commit over base)

## Worker's Completion Report (verbatim)

=== RWO-003 COMPLETION REPORT ===

Work Order ID: RWO-003
base branch + base SHA: main @ 2d37cc0ef7d075e877f370790f82ac9c70df26a4 (verified via git rev-parse 2d37cc0ef7d075e877f370790f82ac9c70df26a4^{commit} — clone HEAD was exactly the dispatch base; evidence: _infra/post-hoc/20260912T230441Z-environment-and-base-identity.txt)
head SHA (your branch, after your commit): 486120ffd6d2a6fef710398576da6f231438ee0f on rwo-003/flowmart-org-guard (exactly 1 commit over base; bundle at /home/z/my-project/RWO-003-delivery.bundle, git bundle verify OK)
changed files/surfaces: docs/validation/fixtures/marketplace/ops-install.js (+21 lines, the Fix scope — the ONLY code change) and 4 new evidence files under docs/validation/evidence/rwo-003/. No engine crates (codex-workflow-*, codex-execution-contracts untouched), no contract files mutated (RWO-003.md, six VWO reports, REPORT-SCHEMA.md byte-identical to base — empty git diff).
implementation summary: In configureInstall (~line 100), upgradeInstall (~124) and rollbackInstall (~170), immediately after resolving the install and before any entitlement check or mutation, added exactly the RWO sketch: const actorOrg = orgForUser(ctx.state, ctx.user.id); if (actorOrg !== inst.orgId) throw new R.AppError(403, 'permission_denied', ...). Guard is absolute (no marketplace_admin override) per the RWO's smallest-fix note. All three throw before any state/event/history write, so failed attacks leave zero trace.
verification commands and exact results:
- bash docs/validation/fixtures/run-all.sh --reset → all five apps started + healthy on 4101–4105, all reset; then bash docs/validation/fixtures/verify-sweep.sh → RESULT: 59 passed, 0 failed (exit 0). Transcript: post-hoc/20260912T230511Z-verify-sweep.txt.
- VWO-010 Family E reproduction re-run (same requests, petra.voss/Acme o-2 → Northwind ins-0402): BEFORE (base, clean tree) configure/upgrade(pin 1.2.0→1.3.0)/rollback(pin back) all ok:true HTTP 200 with petra (u-6) in history + 3 events; AFTER (fix) all three → HTTP 403 permission_denied with {"installId":"ins-0402","installOrg":"o-1"} and message naming Northwind Logistics + the denied action; zero state change (version 1.2.0, config {zones:A}, history 1 entry by u-3), zero install.* events. Transcript: post-hoc/20260912T230511Z-familyE-cross-tenant-before-after.txt.
- Same-org (iris.chen) configure/upgrade/rollback → all HTTP 200 ok:true; reads org-scoped both directions; acceptance assertions 17/17 PASS. Transcript: post-hoc/20260912T230511Z-acceptance-criteria.txt.
acceptance-criteria evidence:
- AC1 (cross-tenant configure/upgrade/rollback → 403 permission_denied naming owning org; no state change; no event; no history entry) → acceptance-criteria transcript §AC1 + before/after transcript.
- AC2 (same-org operations unchanged; VWO-005/006/009 user paths still pass) → acceptance-criteria §AC2 + verify-sweep marketplace section (59/59 overall).
- AC3 (read path already org-scoped — unchanged) → before/after transcript probe 0 + acceptance-criteria §AC3.
rollback note: single-file fixture change; revert commit 486120ffd. No state migration, no schema change, no API contract change beyond the intended 403s.
known limitations / honest deviations:
- cargo/rustc ABSENT in the worker sandbox → no Rust build/test ran; RWO-003 touches zero Rust code (pure JS fixture), all JS verification ran for real under node v24.19.0.
- The sandbox reaps background processes between shell invocations, so reproduction phases ran as self-contained scripts; the RWO's two named verification commands were run verbatim.
- The first AFTER attempt was silently served by a stale pre-fix server and showed ok:true; it was detected, discarded, and re-run cleanly — the shipped AFTER transcript is from the clean run.
- Question for Tech Lead (recorded, NOT changed — out of scope): marketplace_admin exemption note; revokeEntitlement/grantEntitlement remain non-org-guarded (different actor population).
- Evidence tokens defanged; credential scan clean. No push performed (no credentials); delivery is the working tree + the git bundle.
=== END RWO-003 COMPLETION REPORT ===

## Tech-Lead Independent Verification (2026-09-12 23:5x UTC)

- Bundle harvested from the worker workspace via the in-page workspaces API (8,860 bytes; `git bundle verify` OK against base 2d37cc0ef).
- Gates: merge-base == main HEAD (2d37cc0ef); exactly 1 commit; head SHA 486120ffd matches report; diff scope = ops-install.js (+21) + 4 evidence files only (+754 lines).
- Code review: guard placed in all three functions after install resolution, before entitlement/mutation; absolute (no admin override); error carries org name + installId/installOrg. APPROVED.
- Re-verification on this machine (node v24.19.0): run-all.sh --reset → 5/5 apps healthy; verify-sweep.sh → **59 passed, 0 failed**; live probes — cross-tenant configure/upgrade/rollback as petra.voss (Acme o-2) against Northwind ins-0402 → **HTTP 403 permission_denied** (all three, org named in message); same-org control as iris.chen → HTTP 200 ok.
- Verdict: Family E (P0 cross-tenant install writes) CLOSED on the FlowMart fixture; legitimate same-org flows unharmed.
