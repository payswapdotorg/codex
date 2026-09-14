# RWO-008 Completion Report (filed by Tech Lead)

**Work Order:** RWO-008 — FlowMart entitlement resolution: renewal must restore authority (VWO-010 Family H P1 — entitlement shadowing)
**Session:** chat.z.ai agents-tab, GLM-5.3 + Full-Stack, chat dad8e4a7-8dc9-4a22-8830-61a8856a6baf (completed 2026-09-13 19:38 UTC; pod slept ~7h; harvested 2026-09-14 02:5x UTC after a status-check nudge re-bound the workspace as ws-939a0132)
**Delivery:** git bundle RWO-008-delivery.bundle (13,677 bytes; head e3bbf653278ddd3fb69b7c465cbd70933d1405ec, exactly 1 commit over dispatch base 2d37cc0ef; file tree intact after the sleep — 77 entries incl. the bundle + worklog)

## Worker's Completion Report (verbatim)

=== RWO-008 COMPLETION REPORT ===

Work Order ID: RWO-008

base branch + base SHA: main @ 2d37cc0ef7d075e877f370790f82ac9c70df26a4 (verified reachable via git rev-parse 2d37cc0ef7d075e877f370790f82ac9c70df26a4^{commit} → exact SHA; it is main's head as of dispatch, VWO-010 synthesis merged. Note: origin/main had advanced to d304e2043 at clone time — the RWO-003/RWO-004 merges landed after dispatch; per the work order's precedence rule I based on the verified dispatch SHA, which is reachable.) [evidence: docs/validation/evidence/rwo-008/_infra/post-hoc/20260913T184323Z-environment-and-base-identity.txt]

head SHA (your branch, after your commit): e3bbf653278ddd3fb69b7c465cbd70933d1405ec on rwo-008/entitlement-shadowing (single commit, parent = base). Delivery bundle created and verified: /home/z/my-project/RWO-008-delivery.bundle (2d37cc0ef..rwo-008/entitlement-shadowing, bundle verify → "okay"). NOT pushed (no credentials), per the work order.

changed files/surfaces:
docs/validation/fixtures/marketplace/ops-install.js — entitlementFor first-match → newest-ACTIVE resolution; grantEntitlement documentation of the newest-active selection choice
docs/validation/fixtures/marketplace/FAILURES.md — stale_entitlement natural cases reworded; new "Entitlement resolution — renewal restores authority (RWO-008)" section
docs/validation/fixtures/README.md — FlowMart row: "revocation is terminal per record, recovery is a new grant (RWO-008)"
docs/validation/evidence/rwo-008/ — 4 new evidence artifacts (post-hoc ×3, _infra ×1)
Frozen engine crates (codex-workflow-*, codex-execution-contracts): untouched. RWO doc, the six validation reports, and docs/validation/reports/REPORT-SCHEMA.md: byte-identical to base.

implementation summary:
entitlementFor (was lines 11–13, first-match find()): now filters records for org+package to ACTIVE ones (status === 'active' && !isPast(validUntil)) and resolves the latest grantedAt (tie-break: highest id). Reuses the canonical ops.js isPast()/today() via a minimal { state } clock so the fixture clock (state.meta.today) stays authoritative — the exported signature (state, orgId, packageId) is unchanged. If none is active but revoked/expired records exist, it resolves the MOST RECENT such record (same ordering), so the fail-closed 409 names the newest relevant entitlement. checkEntitlement was not modified — the resolved record flows through the existing revoked/expired error paths.
grantEntitlement: took the RWO's "smallest" alternative — newest-active selection alone, no superseded status/event; documented in a block comment (renewals are new grants; the fresh grant has grantedAt = today + highest id so it immediately becomes operative; revocation stays terminal per record).
FAILURES.md/README (Fix scope item 3): revocation terminal per record; recovery is a new grant; newest-ACTIVE resolution documented.

verification commands and exact results (all real runs, node v24.19.0; cargo absent but not needed — fix scope is JS-fixture-only):
bash docs/validation/fixtures/run-all.sh --reset → all five apps healthy on 127.0.0.1:4101–4105
bash docs/validation/fixtures/verify-sweep.sh → RESULT: 59 passed, 0 failed (also 59/59 at base pre-fix — no sweep-visible regression)
VWO-010 C-restart battery re-run, identical requests at base and on this branch: revoke ent-0501 → ok; petra upgrade → 409 stale_entitlement naming ent-0501 revoked (identical base/after — fail-closed preserved); grant fresh ent-0504 → ok; petra retries the SAME upgrade → BEFORE: 409 naming ent-0501 (the defect) / AFTER: 200 "upgraded 2.3.1 → 2.4.0"; configure after renewal also recovers; expired-only → 409 naming ent-0502; auto-trial install unchanged; extra selection probes: expired-old + active-new → newest ACTIVE used (base: 409); two same-day actives → tie-break highest id; revoked-old + expired-newest → 409 names the newest relevant record.
node --check → OK; determinism: two post-fix battery runs identical modulo timestamps + provenance commit.

acceptance-criteria evidence: all four criteria PASS, mapped to post-hoc/20260913T184323Z-acceptance-criteria.txt (revoke-blocks naming the revoked record; grant-new-active → the SAME blocked op succeeds; expired-only blocks naming the expired record; auto-trial path unchanged), with full transcripts in 20260913T184323Z-familyH-entitlement-shadowing-before-after.txt and the sweep in 20260913T184323Z-verify-sweep.txt.

rollback note: Single-commit fixture + docs change, no data migration (newest-active selection is read-only over the existing list). Rollback = revert e3bbf6532 / discard the branch.

known limitations / honest deviations: isPast reuse via { state } clock (identical semantics, exported signature unchanged); "most-recent" ordering for the revoked/expired fallback = latest grantedAt + highest id (satisfies the stated intent in all probed states); no development-state artifacts created (outside the RWO contract — Tech Lead files them at merge time, parity confirmed); based on the dispatch SHA per precedence; cargo absent (JS-only scope); asciicheck em-dash style consistent with base; battery script kept in the sandbox workspace (all requests embedded in committed transcripts).

=== END RWO-008 COMPLETION REPORT ===

## Tech-Lead Independent Verification (2026-09-14 02:4x–03:0x UTC)

**Wake + harvest chain:** the pod slept ~7h post-completion with every files/status/git API returning 429 "sandbox is inactive"; a status-check-only nudge turn (02:42 UTC) hit the "Limit Sandbox Concurrency" modal — released the three merged rwo-002 leftover sandboxes (0e47bfd4 first; remaining buttons stale-UI) — the turn then re-bound the workspace as ws-939a0132 with the file tree INTACT (77 entries incl. RWO-008-delivery.bundle + worklog.md); harvested both immediately (bundle 13,677 bytes).

**Bundle integrity:** `git bundle verify` → "is okay", requires 2d37cc0ef, contains exactly refs/heads/rwo-008/entitlement-shadowing @ e3bbf6532. Fetched as rwo-008-incoming; one commit, 7 files +1426/−3; zero frozen-path and zero engine changes.

**Sweep (real run, node v24.19.0):** branch worktree → run-all --reset (5/5 healthy) → verify-sweep → 59 passed, 0 failed.

**Acceptance probes (independent, marketplace :4105):**
- P1 baseline: petra upgrade ins-0401 with active ent-0501 → 200 (2.3.1→2.4.0).
- P2 fail-closed: dan revokes ent-0501 → the SAME upgrade → 409 stale_entitlement, entitlementId ent-0501, status revoked.
- P3 THE FIX: dan grants fresh active ent-0504 (valid 2027-05-01) → the SAME blocked op chain succeeds: rollback 200 (2.4.0→2.3.1) then upgrade 200 (2.3.1→2.4.0). (An initial retry 409 duplicate_event "already at v2.4.0" was a probe artifact — the entitlement gate had already passed.)
- P4 expired-only: iris install pk-101 → 409 stale_entitlement naming ent-0502 expired (validUntil 2026-08-01), the newest relevant record.

**Conflict resolution + merged-main identity:** RWO-008 and RWO-002 both edited ops-install.js (adjacent insertions — configPatch() vs entitlementFor(), disjoint functions). Trial merge off main @ 6a68f0218 → conflict resolved as union; sweep on the merged tree → 59/59; tree 0690d4e42. PR #37 head was a cherry-pick of e3bbf6532 onto main with the identical resolution — tree 0690d4e42 byte-identical to the verified trial tree; squash-merged as 9d32eff77 (same tree). Branch ref deleted.

**Disposition of flagged questions:** (1) isPast via { state } clock — accepted (semantics identical, signature stable). (2) fallback recency = grantedAt+id — accepted (satisfies the stated intent in every probed state). (3) development-state parity — confirmed: Tech Lead files the report + state bump at merge time (this document + execution-state.json update).

**Result:** RWO-008 MERGED via PR #37 (squash 9d32eff77). 5/12 RWOs closed (001, 002, 003, 004, 008). Remaining: rwo-005/006/010/011/012 undispatched; rwo-007 parked; rwo-009 queued (failed-fetch state, retry pending); vwo-011 completion harvested-pending (pod still sleeping; same nudge path proven).
