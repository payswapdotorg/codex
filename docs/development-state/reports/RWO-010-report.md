# RWO-010 — Harness Alignment Batch: Catalog/Seed/Fixture Drift: Completion Report

**Status:** COMPLETE — branch `rwo-010/harness-alignment-batch`
**Base:** main @ dfcb74171 (post-RWO-007)
**Work order:** `docs/validation/work-orders/RWO-010.md` (VWO-010 synthesis; VWO-004 F2–F6 + VWO-005 F7)
**Scope:** fixture/seed/catalog-only — no Rust, no protocol, no zst, no engine changes

## What was built (six bounded alignment fixes, all harness-internal drift)

1. **VWO-004 F2 — second OPEN purchase request (construction).** The seed's
   `prq-5002` is a `po_issued` record (ID genuinely taken), so the catalogued
   approve→PO sub-path gained `prq-5003`: OPEN (`submitted`), valid vendor
   `c-1` ConcreteWorks (active, contract 2027-06-30), distinct amount ($87,000
   vs $1,104,000 / $144,000), requester u-2 (same as prq-5001), `prqSeq`
   bumped 5002→5003 so future requests cannot collide.
2. **VWO-004 F3 — support-agent resolution (rideshare).** `ana.silva`
   (support_agent) granted `ticket:resolve` (the WO's preferred fix).
3. **VWO-004 F4 — rendition `missing_asset` (media).** `requestRendition`
   gained the `missing_asset` branch after the natural `asset_not_found`
   check: 404 `missing_asset` with `simulated: true` on a resolvable asset —
   same switch semantics as attach/publish. Both switch tables (server.js
   failures registry + FAILURES.md) now document the rendition binding.
4. **VWO-004 F6 — push fast channel (media).** Seed gained `ch-6`
   news-push-alerts (kind `push`, license 2027-03-31 — valid, licensed
   differently from the expired partner-app). `publishStory` accepts any
   channel id; the license check passes → job `sent`.
5. **VWO-004 F5 — follow-up-note step name (catalog wording).** Construction
   safety scenario step 3 renamed to the realized surface: the follow-up is
   the close-with-resolution note (no separate follow-up form exists).
6. **VWO-005 F7 — hero attach on approved story (media).** The attach form's
   status list gained `approved` (the WO's smallest fix: allow attach; the
   publish gate re-validates the hero). The server-side `attachAsset` never
   had a status gate (existence + asset resolution + role only), so the UI
   form was the sole blocker; `published`/`corrected` remain excluded
   (correction-gate discipline untouched).

## Acceptance criteria → evidence (all real runs; `docs/validation/evidence/rwo-010/`)

1. `verify-sweep.sh` after `run-all.sh --reset`: **59/59 PASS** — no
   regression across all five fixture apps, golden paths and all switches.
2. Each aligned scenario path runs as catalogued (live AC battery,
   `post-hoc/verification.txt`, captured on a fresh seed reset):
   - procurement approve→PO on the second OPEN PRQ: **prq-5003 approved,
     po-2042 issued** (HTTP 200, vendor contract validated);
   - support-agent resolution: **ana.silva resolved tk-0702**
     (escalated → resolved, HTTP 200);
   - rendition `missing_asset` switch arms and fires: **HTTP 404
     missing_asset (simulated: true)** on a valid asset; control without the
     switch renders normally (HTTP 200);
   - breaking-news publishes to a push-like fast channel: **st-0205 published
     with jobs (ch-1 web: sent, ch-6 push: sent)** — "2/2 channels delivered";
   - follow-up-note step name matches the surface: catalog step renamed
     (commit diff);
   - hero attach on approved story works: **a-0301 attached as hero to
     st-0205 while `approved` (status stays approved)**, then the publish
     gate re-validated the hero and published — the formerly dead-end path is
     closed end-to-end.

## Lineage

Worker flight 1 (Task-tool) applied all six fixes (00:25-style bounded scope,
8 files +16/−7) and died at the session boundary immediately after starting
`run-all.sh` (fixture servers came up orphaned with zero verification
traffic). Tech-Lead audit-first completion per the lesson-120 protocol: every
fix audited against the work order (status vocabulary, vendor/user/project
existence, switch-placement semantics, server-side gate absence for F7,
channel license validity for F6) — no defects found; the full verification
battery was then re-run live by the TL on a fresh seed reset.
