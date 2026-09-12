# VWO-004 — Enterprise Operations User Validation Report

Class A scenario run report (REPORT-SCHEMA.md). Worker operated strictly as a
real enterprise-operations user (non-implementer): no changes to `codex-rs/**`
or any semantic contract; fixtures, scripts and docs under
`docs/validation/**` were used as shipped by VWO-002/VWO-003.

## Identity
- Work Order: VWO-004
- Worker persona: enterprise operations user (real-user validator — non-implementer)
- Base branch: main
- Base SHA: 0d314fd09cec70d000f33c81286edcd8cbf72501
- Head SHA: see git rev-parse vwo-004/enterprise-ops-validation (single commit on base 0d314fd)
- Validation environment: sandbox clone of github.com/payswapdotorg/codex on branch vwo-004/enterprise-ops-validation; all five VWO-002 synthetic enterprise fixtures live on 127.0.0.1:4101-4105 (SiteBuild Field Suite / ForgeOps / RidePilot / PressRoom / FlowMart), started with fixtures/run-all.sh and verified with verify-sweep.sh (all 5 families PASS — see _infra evidence); browser surface driven headless via agent-browser 0.35.0 (Playwright chromium); terminal surface via `node deploy-console.js` piped sessions; capability_check.sh recorded exit 1 with 1 required failure (Rust toolchain absent — a VWO-001-owned surface, not required for these user paths; deviation recorded honestly) and 2 fidelity failures (E2B/Composio integrations absent — the recorded VWO-001 fallback condition governs; xdotool absent — desktop capability level virtual-display+capture); single-invocation sandbox constraint honored (one observe/capture pipeline per invocation per SCENARIO-CATALOG.md §2 rule 3)
- E2B template/workspace identity, if used: not used — E2B/Composio integrations absent in this environment (VWO-001 fallback record governs)
- Codex Universal runtime/application SHA: NONE — no user-facing Codex Universal runtime/application surface exists at base 0d314fd (the workflow family ships as library crates only; see docs/validation/evidence/VWO-004/_infra/post-hoc/20260912T003813Z-product-surface-absence-check.txt)
- Date/time window: 2026-09-11T23:56Z → 2026-09-12T07:10Z (UTC)

Teaching-mode coverage delivered (VALIDATION-PROGRAM.md §6): DEMONSTRATE ×4
(construction-daily-progress, software-engineering-onboarding,
rideshare-driver-onboarding, media-editorial-publishing), INSTRUCT ×4
(construction-safety-incident, software-pr-triage,
rideshare-support-escalation, media-breaking-news), HYBRID ×4
(construction-procurement-approval, software-incident-response,
rideshare-surge-ops, media-content-repurposing). Every scenario's business
path was executed by a human-observable surface (browser, plus the terminal
console where the catalog binds one); every catalog-listed failure switch was
exercised (injected or natural); every scenario ended with reset
verification. Every scenario's Codex Universal teaching step is blocked by
the same absent product surface (finding F1, P0) — recorded, never fabricated
(SCENARIO-CATALOG.md binding rule 2).

Modality coverage exercised across the worker's scenarios: Browser (all 12),
Terminal (software-incident-response promote via deploy-console.js;
software-engineering-onboarding blocked-promote via the same console), API
(read-only post-hoc event/state pulls and natural-twin probes — never in
place of a user-path step), Human gate (the validator's own judgement
performed the approval/decision steps the catalog assigns to humans). No
MCP/Tool surfaces are bound by these 12 catalog entries (none exist in the
fixtures to exercise — recorded, not worked around).

Finding index used across scenario blocks (root causes deduplicated):
- F1 (P0, all 12 scenarios): Codex Universal teaching surface absent at base 0d314fd.
- F2 (P2, construction-procurement-approval): catalog step 4 seed/catalog mismatch — no second OPEN purchase request with a valid vendor exists in the shipped seed.
- F3 (P2, rideshare-support-escalation): catalog step 4 expects the support agent to resolve; the fixture grants ticket:resolve to ops only.
- F4 (P2, media-content-repurposing): catalog lists missing_asset on rendition; the fixture implements data_conflict on rendition instead (missing_asset is bound to attach/publish only).
- F5 (P3, construction-safety-incident): follow-up-action note surface is realized as the close-with-resolution-note; "investigating" status visible but minor surface mismatch.
- F6 (P2, media-breaking-news): catalog's fast channels are web+push; the fixture has no push channel (fast-path intent preserved via web+rss, skipping the licensed partner-app).

## Scenario
- Scenario id: construction-daily-progress
- Industry: construction
- Scenario: daily site progress reporting (SiteBuild Field Suite, port 4101)
- Teaching mode: DEMONSTRATE
- User goal: submit the daily site progress report (percent complete, tasks, photos, notes) so the PM sees it without a phone call — and teach Codex Universal that routine by doing it while the system observes.

## User Path
1. Signed in at http://127.0.0.1:4101/login as marco.silva (site foreman; persona table is on the login page).
2. Opened project page /projects/p-101 (Harborview Tower, 62% at seed) and filled the daily report form: 68% complete, task t-101c, photo a-101, notes.
3. Submitted the report — flash "Progress report pr-4002 saved for Harborview Tower (68%)." plus a new report row; project percent updated 62 → 68.
4. Signed in as dana.reyes and opened /notifications — the PM notification for pr-4002 is visible and delivered.
5. Re-submitted the same report for the same date — red duplicate-rejection flash naming the existing report ([duplicate_event], HTTP 409 semantics).
6. Armed the notification_failure failure switch (X-Failure-Switch request header on the p-102 submit) — report pr-4003 saved with warning flash "Saved, but the notification to the project manager could not be delivered (check /notifications and the event feed for notification.failed)." and the failed notification row visible under /notifications.
7. Catalog step 6 (Codex Universal DEMONSTRATE replay + compiled-workflow review) not performed — teaching surface missing (see Actual).

## Expected
Report saved and visible; project percent updated; PM notification delivered; same-day re-submission rejected with a clear [duplicate_event] flash; with notification_failure armed the save still succeeds, the notification row shows failed, a notification.failed event appears, and the human sees a warning (not a silent drop); the taught workflow captures one-report-per-project-per-day.

## Actual
All business-path expectations met exactly: pr-4002 saved with flash and row, project percent 62 → 68 visible on the project page; PM notification delivered (visible as dana.reyes); duplicate re-submission rejected with a red flash naming the existing report ([duplicate_event]); with notification_failure armed, pr-4003 (Eastside Interchange, 40%) saved and the warning flash plus the failed notification row were both visible — the failure is observable and actionable, not silent. Post-hoc pulls show the progress.submitted and notification.failed events and the state deltas. The teaching step could not be performed: at base 0d314fd the Codex Universal workflow family (codex-workflow-app, codex-teaching-compiler, codex-workflow-forge/-durable/-triggers/-evolution/-distribution) exists only as library crates with no product consumer (no [[bin]], no CLI subcommand, no app-server protocol mention) — there is no surface on which a normal person could replay the routine or review a compiled workflow (F1, P0). Recorded in the step-06 note; not worked around.

## Workflow Identity
- Workflow: NONE — surface missing (no Codex Universal teaching surface exists at base 0d314fd; F1)
- Version: NONE — surface missing (F1)
- Source revision: NONE — surface missing (F1)
- Definition digest: NONE — surface missing (F1)
- Dependency-lock identity: NONE — surface missing (F1)

## Runtime Binding
- Capabilities: progress:create exercised by marco.silva (role site_foreman) in the fixture; Codex Universal capability binding: NONE — surface missing (F1)
- Resources: SiteBuild Field Suite fixture at 127.0.0.1:4101; project p-101 (Harborview Tower); PM notification channel; photo asset a-101; task t-101c
- Environments: local sandbox fixture process (node server.js --port 4101); browser surface (headless chromium via agent-browser 0.35.0)
- Approvals: none required for the foreman (creation only); PM visibility is the human gate — notification delivered
- Triggers/schedules: one-report-per-project-per-day duplicate gate exercised; the taught workflow would carry a daily schedule trigger and a notification side-effect — no scheduler surface exists to verify against (F1)

## Evidence
- docs/validation/evidence/VWO-004/construction-daily-progress/user-path/20260912T000238Z-step01-login-page-with-persona-table.txt
- docs/validation/evidence/VWO-004/construction-daily-progress/user-path/20260912T000238Z-step02-project-page-with-report-form.txt
- docs/validation/evidence/VWO-004/construction-daily-progress/user-path/20260912T000238Z-step03-daily-report-form-filled.txt
- docs/validation/evidence/VWO-004/construction-daily-progress/user-path/20260912T000238Z-step04-report-saved-flash-and-row.txt
- docs/validation/evidence/VWO-004/construction-daily-progress/user-path/20260912T000238Z-step04-report-saved-screenshot.png
- docs/validation/evidence/VWO-004/construction-daily-progress/user-path/20260912T000238Z-step05-pm-notification-delivered.txt
- docs/validation/evidence/VWO-004/construction-daily-progress/user-path/20260912T000238Z-step07-duplicate-rejection-flash.txt
- docs/validation/evidence/VWO-004/construction-daily-progress/user-path/20260912T000238Z-step07-duplicate-rejection-screenshot.png
- docs/validation/evidence/VWO-004/construction-daily-progress/user-path/20260912T000227Z-switch-notification-failure-warning-flas.txt
- docs/validation/evidence/VWO-004/construction-daily-progress/user-path/20260912T000227Z-switch-notification-failed-row-visible.txt
- docs/validation/evidence/VWO-004/construction-daily-progress/user-path/20260912T000253Z-step06-teaching-surface-missing-note.md
- docs/validation/evidence/VWO-004/construction-daily-progress/user-path/20260912T000253Z-step06-no-teaching-surface-in-app-nav.txt
- docs/validation/evidence/VWO-004/construction-daily-progress/post-hoc/20260912T000259Z-events-and-state-pulls.txt
- docs/validation/evidence/VWO-004/construction-daily-progress/post-hoc/20260912T000305Z-reset-verification.txt

## Failure / Recovery
- Failure injected: notification_failure on the p-102 daily-report submit (X-Failure-Switch header); natural duplicate_event on the same-day re-submission
- Recovery attempted: followed the warning's own directions — read /notifications and the event feed; the failed notification row and notification.failed event are observable; re-read state (p-102 percent 40 correct, pr-4003 intact — no corruption); manual notify or next-day resubmission is the documented recovery; the duplicate submission was correctly rejected with the record intact
- Restart/session-loss behavior: not attempted — not on this scenario's catalog path; fixture uptime was continuous through the run (healthz) and browser sessions were recreated between steps without losing server state
- Final outcome: passed for the business path — save decoupled from notification (pr-4003 saved, warning shown, failed row visible), duplicate gate verified, reset verified (p-101 percent restored to 62, p-102 to 35, world.reset recorded); teaching step blocked (F1)

## Product / UX Friction
The business path is clean and self-explanatory: persona table on the login page, one form, clear flashes that name the record ids. Friction observed: the failure-switch surface is operator-grade — flipping a switch uses the in-app switches page or a request header (the validator used the header); a normal person would not know how to inject a notification failure, but also never needs to. Positively, the notification-failure warning points the human at /notifications and the event feed (actionable, not silent).

## Security / Architecture Findings
- Finding: F1 (deduplicated across all 12 scenario blocks) — Codex Universal teaching surface absent: the catalog's step-6 DEMONSTRATE replay/compile could not be performed; a normal person cannot teach this routine to Codex Universal at all because no product surface exists (workflow family is library crates only at base 0d314fd).
- Severity: P0
- Reproduction: repo-level consumer scan docs/validation/evidence/VWO-004/_infra/post-hoc/20260912T003813Z-product-surface-absence-check.txt plus the per-scenario note docs/validation/evidence/VWO-004/construction-daily-progress/user-path/20260912T000253Z-step06-teaching-surface-missing-note.md
- Root cause: codex-workflow-app / codex-teaching-compiler (and forge/durable/triggers/evolution/distribution) ship as libraries with no [[bin]], CLI subcommand, or app-server consumer at this SHA
- Frozen invariant affected: the north-star premise that a non-implementer human can teach workflows through a product surface — unmet (surface absent), not a regression of a previously-working invariant

## Recommendation
- NEW WORK ORDER — F1 (P0): build and ship the user-facing Codex Universal teaching surface (a product consumer for the workflow family).
  - Proposed owner: Tech Lead to route to the workflow-app / product-surface owner
  - Verification required: re-run the 12 VWO-004 teaching steps against the new surface and re-validate this report with validate-report.py

## Worker Conclusion
blocked — the human business path passed end-to-end (report saved, percent updated, PM notified, duplicate and notification-failure behaviors verified with recovery), but the catalog's teaching step could not be performed because the Codex Universal teaching surface does not exist at this SHA (F1, P0); recorded, not worked around.

## Scenario
- Scenario id: construction-procurement-approval
- Industry: construction
- Scenario: procurement approval with vendor-contract gate (SiteBuild Field Suite, port 4101)
- Teaching mode: HYBRID
- User goal: run the procurement approval workflow — review purchase request prq-5001 (High-tensile rebar, SteelCo Fabrication), approve or reject it, see the PO issued — and teach it with initial instructions plus a demonstrated exception (the expired-contract rejection).

## User Path
1. Signed in as dana.reyes (project manager) and opened /procurement — the request list shows vendor, amount and the contract-validity display (prq-5001 → SteelCo Fabrication, contract until 2026-08-01, i.e. expired).
2. Opened prq-5001 and attempted to approve — the natural stale entitlement fired: red flash "Vendor contract for SteelCo Fabrication expired on 2026-08-01 — a new vendor agreement must be in place before this request can be approved. [stale_entitlement]" (fail-closed).
3. Catalog step 4 (decide the other open request with a valid vendor — approve and observe the PO issued) not performable as written: the shipped seed contains no second OPEN purchase request with a valid vendor; the PO surface was instead verified on the seeded static PO po-2041 (mismatch recorded as F2).
4. HYBRID instruct phase captured: the approval policy instruction text ("approve PRs up to $50k when the vendor contract is valid; route to finance above") is recorded verbatim in the instruct-phase evidence file.
5. Demonstrated the exception by rejecting prq-5001 — the request moves to rejected with a confirmation flash (demo-phase evidence).
6. Armed data_conflict on the decide — simulated conflict flash "Purchase request prq-5001 was already decisioned by another approver while you were reviewing it (simulated conflict). [data_conflict]"; the record stayed intact.
7. Second browser session (agent-browser --session second) with a stale form double-decided the already-rejected prq-5001 — natural fail-closed conflict ("already in status rejected").
8. Catalog step 5 teaching/reconciliation not performed — teaching surface missing (see Actual).

## Expected
Approvals only with a valid vendor contract; the rejection message names the expired contract and expiry date; PO issue observable; double-decide fails with [data_conflict]; instruction and demonstration reconcile with the expired contract remaining a hard gate.

## Actual
The vendor-contract gate behaved exactly as catalogued: the prq-5001 approve attempt failed closed naming SteelCo Fabrication and the 2026-08-01 expiry ([stale_entitlement]); the demo-phase rejection of prq-5001 landed; the injected data_conflict decide failed closed with the record intact; the stale-form double-decide in a second session was naturally rejected ("already in status rejected"). Post-hoc event pull shows procurement.rejected and exactly one decision for prq-5001 — no double PO. Deviations: (a) catalog step 4 is impossible on the shipped seed — there is no second OPEN request with a valid vendor, and po-2041 is a static seeded record, so the approve→PO-issued live path could not be walked (F2, harness-level); (b) the teaching/reconciliation step could not be performed — no Codex Universal surface exists (F1, P0). Both recorded with evidence notes, not worked around.

## Workflow Identity
- Workflow: NONE — surface missing (no Codex Universal teaching surface exists at base 0d314fd; F1)
- Version: NONE — surface missing (F1)
- Source revision: NONE — surface missing (F1)
- Definition digest: NONE — surface missing (F1)
- Dependency-lock identity: NONE — surface missing (F1)

## Runtime Binding
- Capabilities: po:approve exercised by dana.reyes (project manager) in the fixture; Codex Universal capability binding: NONE — surface missing (F1)
- Resources: SiteBuild Field Suite fixture at 127.0.0.1:4101; purchase request prq-5001 (SteelCo Fabrication); seeded PO po-2041; vendor-contract entitlement records
- Environments: local sandbox fixture process (node server.js --port 4101); browser surface including a second concurrent browser session for the double-decide
- Approvals: PM approval gate (po:approve) exercised — reject path live, approve path blocked by the natural vendor-contract entitlement gate; finance routing above threshold described in the instruct text (surface to execute it missing, F1)
- Triggers/schedules: request-decision conflict gate (data_conflict) exercised both injected and natural; the taught workflow would carry an approval-required step and an amount-threshold branch — no compile surface to verify against (F1)

## Evidence
- docs/validation/evidence/VWO-004/construction-procurement-approval/user-path/20260912T000449Z-instruct-phase-instruction-text.md
- docs/validation/evidence/VWO-004/construction-procurement-approval/user-path/20260912T000628Z-step01-procurement-list-with-contract-validity.txt
- docs/validation/evidence/VWO-004/construction-procurement-approval/user-path/20260912T000628Z-step03-approve-attempt-expired-contract-flash.txt
- docs/validation/evidence/VWO-004/construction-procurement-approval/user-path/20260912T000628Z-step04-catalog-seed-mismatch-note.md
- docs/validation/evidence/VWO-004/construction-procurement-approval/user-path/20260912T000628Z-step05-demo-phase-prq-5001-rejected.txt
- docs/validation/evidence/VWO-004/construction-procurement-approval/user-path/20260912T000628Z-step06-second-session-double-decide-conflict.txt
- docs/validation/evidence/VWO-004/construction-procurement-approval/user-path/20260912T000628Z-switch-data-conflict-decide-flash.txt
- docs/validation/evidence/VWO-004/construction-procurement-approval/user-path/20260912T000639Z-demo-phase-reconciliation-surface-missin.md
- docs/validation/evidence/VWO-004/construction-procurement-approval/post-hoc/20260912T000648Z-events-and-state-pulls.txt
- docs/validation/evidence/VWO-004/construction-procurement-approval/post-hoc/20260912T000648Z-reset-verification.txt
- docs/validation/evidence/VWO-004/construction-procurement-approval/post-hoc/20260912T000739Z-events-pull-corrected.txt
- docs/validation/evidence/VWO-004/construction-procurement-approval/post-hoc/20260912T000739Z-final-reset-verification.txt

## Failure / Recovery
- Failure injected: data_conflict on the prq-5001 decide (X-Failure-Switch header); natural stale_entitlement on the approve attempt; natural data_conflict on the second-session stale-form double-decide
- Recovery attempted: re-read the request state after each conflict (the flash names the deciding approver / current status); the record was intact after every failure — recovery is re-read and re-decide on current state; no double PO appeared in events or state
- Restart/session-loss behavior: exercised in the analogous form the catalog offers — a second concurrent browser session with a stale form was rejected fail-closed (session-loss mid-decision leaves the record consistent); server restart not on the catalog path
- Final outcome: passed for the gates exercised — entitlement gate, single-decision conflict gate, and seed reset all verified; the approve→PO live path was impossible on the shipped seed (F2) and the teaching/reconciliation step is blocked (F1)

## Product / UX Friction
The entitlement failure is excellent for a normal person: it names the vendor, the expiry date, and the remediation ("a new vendor agreement must be in place"). The conflict flashes likewise name the current state. Friction: a user who wanted the PO-issued experience would be confused that only one open request exists and it is the expired-contract one — the catalog promises a second decidable request that the seed does not ship (F2); this is harness-level, not product.

## Security / Architecture Findings
- Finding: F1 (deduplicated; root cause identical to scenario block 1) — the catalog's HYBRID instruct+reconcile step could not be performed: no Codex Universal teaching surface exists at base 0d314fd, so instruction/demonstration reconciliation could not be verified.
- Severity: P0
- Reproduction: docs/validation/evidence/VWO-004/_infra/post-hoc/20260912T003813Z-product-surface-absence-check.txt and docs/validation/evidence/VWO-004/construction-procurement-approval/user-path/20260912T000639Z-demo-phase-reconciliation-surface-missin.md
- Root cause: workflow family ships as library crates only at this SHA (no product consumer)
- Frozen invariant affected: HYBRID reconciliation guarantee (VALIDATION-PROGRAM.md §6 — "instruction and demonstration are reconciled rather than one silently overriding the other") is unverifiable — the surface that would reconcile them does not exist
- Finding: F2 — catalog/seed mismatch: SCENARIO-CATALOG entry step 4 requires deciding a second OPEN request with a valid vendor (approve → PO issued); the shipped VWO-002 seed has no such request, so that live sub-path is impossible and the PO surface is only inspectable via the static seeded po-2041. A normal person following the catalog would hit a dead end not caused by the product.
- Severity: P2
- Reproduction: docs/validation/evidence/VWO-004/construction-procurement-approval/user-path/20260912T000628Z-step04-catalog-seed-mismatch-note.md
- Root cause: the catalog entry was written against a richer seed than VWO-002 shipped
- Frozen invariant affected: none — harness-level mismatch (fixture/catalog contract), product gates behaved correctly

## Recommendation
- NEW WORK ORDER — F1 (P0): as in scenario block 1 (build the user-facing Codex Universal teaching surface; owner Tech Lead → workflow-app owner; verification = re-run teaching steps + re-validate report).
- FIX NOW — F2 (P2): align the catalog entry or the seed — add a second OPEN purchase request with a valid vendor to the construction seed (preferred, keeps the catalog semantics) or rebind catalog step 4 to the natural prq-5001 gate.
  - Proposed owner: validation harness maintainer (VWO-002/VWO-003 owner)
  - Verification required: verify-sweep.sh PASS + re-run construction-procurement-approval with the approve→PO live path

## Worker Conclusion
blocked — the entitlement gate, conflict gates and reset all passed (fail-closed semantics verified injected and natural), but the approve→PO live sub-path was impossible on the shipped seed (F2, P2) and the catalog's HYBRID teaching/reconciliation step could not be performed because the Codex Universal teaching surface does not exist at this SHA (F1, P0); both recorded, not worked around.

## Scenario
- Scenario id: construction-safety-incident
- Industry: construction
- Scenario: safety incident reporting and closure (SiteBuild Field Suite, port 4101)
- Teaching mode: INSTRUCT
- User goal: report a new safety incident from the field, have it land in the safety register (si-*), add follow-up actions, and close it after sign-off — described to Codex Universal in words (INSTRUCT), letting the system infer the intake→investigate→close semantics.

## User Path
1. Signed in as marco.silva (site foreman); opened /safety — the safety register and the incident report form are on one page.
2. Filled the incident report form (near-miss, moderate severity, description) and submitted — incident si-2003 created with a flash and register row.
3. Armed service_unavailable on the report submit — fail-safe flash "The \"site-photo-store\" service is unavailable (simulated). State-changing requests are rejected; read requests still work. [service_unavailable]" — the register kept reading, no partial entry was created; a clean retry succeeded (si-2004).
4. Signed in as sam.oconnell (safety officer); opened si-2003 — officer view shows the close form; closed the incident with a resolution note — closure observed in the register.
5. Armed permission_denied on close — 403 flash "Permission denied (simulated): \"safety:close\" is required and the check failed while the failure switch is on. [permission_denied]" naming the required permission; clean retry closed the incident.
6. Signed in as marco.silva (foreman) and opened si-2003 — no close form is rendered (permission-gated UI); the natural foreman close attempt was additionally probed post-hoc via the API twin and failed 403 [permission_denied] naming safety:close.
7. Captured the INSTRUCT instruction text (the canonical closure policy description) as evidence; catalog step 5 teaching/inference not performed — teaching surface missing (see Actual).

## Expected
Incident created and visible; closure restricted to the safety officer; inferred workflow matches the described process including the role split and the 48h timer; foreman close attempt fails [permission_denied]; service_unavailable on report fails safe with no partial register entry and reads alive, retry after clear succeeds.

## Actual
The intake→close path behaved as catalogued: si-2003 (near-miss, moderate) registered with flash and row; the service_unavailable injection failed safe (503 semantics, reads alive, no partial entry, retry succeeded as si-2004); the safety officer closed si-2003 with a resolution note; the injected permission_denied on close named safety:close and the clean retry succeeded; the foreman's UI renders no close form and the natural API-twin close attempt fails 403 naming safety:close. Post-hoc pulls show safety.incident_reported and safety.incident_closed events and the register state. Deviations: (a) the catalog's "add a follow-up action note" step is realized in the fixture as the close-with-resolution-note (and the investigating status is visible pre-close) — a minor surface mismatch (F5, P3), intent preserved; (b) the INSTRUCT inference step could not be performed — no Codex Universal surface exists (F1, P0). The instruction text itself was captured verbatim as evidence of what a normal person would have said.

## Workflow Identity
- Workflow: NONE — surface missing (no Codex Universal teaching surface exists at base 0d314fd; F1)
- Version: NONE — surface missing (F1)
- Source revision: NONE — surface missing (F1)
- Definition digest: NONE — surface missing (F1)
- Dependency-lock identity: NONE — surface missing (F1)

## Runtime Binding
- Capabilities: safety:report exercised by marco.silva (site_foreman); safety:read + safety:close exercised by sam.oconnell (safety_officer); foreman correctly denied safety:close; Codex Universal capability binding: NONE — surface missing (F1)
- Resources: SiteBuild Field Suite fixture at 127.0.0.1:4101; safety register; incidents si-2003/si-2004 (run) and si-2001 (seed); site-photo-store dependency (named in the 503)
- Environments: local sandbox fixture process (node server.js --port 4101); browser surface across two personas
- Approvals: safety-officer closure gate (safety:close) exercised — authorized closure succeeded, unauthorized (foreman) blocked both in UI (form absent) and at the API boundary
- Triggers/schedules: incident-created trigger semantics exercised; the described 48h closure timer has no fixture surface to exercise (would be inferred by the teaching step — blocked, F1)

## Evidence
- docs/validation/evidence/VWO-004/construction-safety-incident/user-path/20260912T000834Z-instruct-phase-instruction-text.md
- docs/validation/evidence/VWO-004/construction-safety-incident/user-path/20260912T001012Z-step01-safety-register-and-report-form.txt
- docs/validation/evidence/VWO-004/construction-safety-incident/user-path/20260912T001012Z-step02-incident-created-flash-and-register.txt
- docs/validation/evidence/VWO-004/construction-safety-incident/user-path/20260912T001012Z-step03-officer-view-close-forms.txt
- docs/validation/evidence/VWO-004/construction-safety-incident/user-path/20260912T001012Z-step04-incident-closed-with-resolution.txt
- docs/validation/evidence/VWO-004/construction-safety-incident/user-path/20260912T001012Z-step06-foreman-view-no-close-form.txt
- docs/validation/evidence/VWO-004/construction-safety-incident/user-path/20260912T001012Z-switch-service-unavailable-failsafe.txt
- docs/validation/evidence/VWO-004/construction-safety-incident/user-path/20260912T001012Z-switch-permission-denied-close.txt
- docs/validation/evidence/VWO-004/construction-safety-incident/user-path/20260912T001012Z-switch-retry-succeeds-recovery.txt
- docs/validation/evidence/VWO-004/construction-safety-incident/post-hoc/20260912T001002Z-events-state-permission-pulls.txt
- docs/validation/evidence/VWO-004/construction-safety-incident/post-hoc/20260912T001023Z-natural-permission-denied-foreman-close.txt
- docs/validation/evidence/VWO-004/construction-safety-incident/post-hoc/20260912T001012Z-reset-verification.txt

## Failure / Recovery
- Failure injected: service_unavailable on incident report (X-Failure-Switch header — fail-safe verified: reads alive, no partial entry); permission_denied on safety-officer close (403 naming safety:close)
- Recovery attempted: clean retry after each switch cleared — report retry succeeded (si-2004), close retry succeeded; state re-read between attempts showed no partial records
- Restart/session-loss behavior: not attempted — not on this scenario's catalog path; both personas' browser sessions were recreated between steps with server state continuous (healthz uptime)
- Final outcome: passed for the business path — intake, fail-safe, closure gate, and reset all verified; the INSTRUCT inference step is blocked (F1)

## Product / UX Friction
Very low friction on the business path: one page holds the register and the form; flashes name the incident ids and the exact permission or dependency. Positive: the 503 message explicitly separates "state-changing requests are rejected; read requests still work", which is exactly what a non-technical foreman needs to know during an outage. Minor: the catalog's "follow-up action note" is folded into the close-with-resolution-note — a user looking for a separate follow-up surface will not find one (F5, cosmetic).

## Security / Architecture Findings
- Finding: F1 (deduplicated; root cause identical to scenario block 1) — the INSTRUCT inference step could not be performed: the captured instruction text has no Codex Universal surface to be submitted to, so the inferred workflow (role split, 48h timer, toolbox-talk branch) could not be evaluated.
- Severity: P0
- Reproduction: docs/validation/evidence/VWO-004/_infra/post-hoc/20260912T003813Z-product-surface-absence-check.txt and docs/validation/evidence/VWO-004/construction-safety-incident/user-path/20260912T000834Z-instruct-phase-instruction-text.md (instruction captured, submission surface absent)
- Root cause: workflow family ships as library crates only at this SHA (no product consumer)
- Frozen invariant affected: INSTRUCT-mode guarantee (VALIDATION-PROGRAM.md §6 — the system infers semantics from a description) is unverifiable — no inference surface exists
- Finding: F5 — follow-up-note surface mismatch: the catalog expects an explicit "add a follow-up action note" step and an investigating→closed transition with separate note surface; the fixture realizes notes at closure (resolution note) and shows investigating status — intent preserved, surface narrower than catalogued.
- Severity: P3
- Reproduction: docs/validation/evidence/VWO-004/construction-safety-incident/user-path/20260912T001012Z-step03-officer-view-close-forms.txt (officer surface) vs catalog step 3
- Root cause: fixture surface narrower than the catalog entry's wording
- Frozen invariant affected: none — cosmetic/documentation-level mismatch

## Recommendation
- NEW WORK ORDER — F1 (P0): as in scenario block 1 (build the user-facing Codex Universal teaching surface; owner Tech Lead → workflow-app owner; verification = re-run teaching steps + re-validate report).
- DEFER — F5 (P3): widen the safety surface or adjust catalog wording when the safety family is next touched; no user path is blocked.

## Worker Conclusion
blocked — the intake→close business path passed end-to-end with fail-safe, permission gate and reset verified, but the catalog's INSTRUCT teaching step could not be performed because the Codex Universal teaching surface does not exist at this SHA (F1, P0); recorded, not worked around.

## Scenario
- Scenario id: software-pr-triage
- Industry: software
- Scenario: pull-request triage with CI and code-owner gates (ForgeOps, port 4102)
- Teaching mode: INSTRUCT
- User goal: triage an open PR — check CI, review, approve, and merge — described to Codex Universal in words; the system must infer the CI gate, the code-owner approval, the self-approval prohibition, and the merge→staging deploy side-effect.

## User Path
1. Signed in as ari.klein (engineer, PR author); opened /repos/r-1 (payments-service) — the repo page lists the open PRs.
2. Opened the pending PR (Refund idempotency keys, pr-0301) in the author view — its CI run status is visible (passed).
3. Attempted to approve own PR as the author — the UI hides the approve form for the author; the natural self-approval rejection ([permission_denied], 403) was probed and recorded post-hoc via the API twin.
4. Re-ran CI with service_unavailable armed — fail-safe 503 (reads still work); after the switch cleared, a clean retry returned run-0073 passed.
5. Signed in as noor.haddad (code owner); approved pr-0301 — the approval is recorded on the PR; a second approval from the same reviewer was naturally rejected: "Noor Haddad already approved PR #302. [duplicate_event]" (409).
6. Merged with data_conflict armed — 409 flash "Merge rejected (simulated): the base branch \"main\" moved under the merge window; rebase required. [data_conflict]"; the clean merge then produced the merged state, the staging deployment dep-1104 with artifact, and the service.version_bumped event.
7. Catalog step 6 (INSTRUCT description → inferred gates) not performed — teaching surface missing (see Actual).

## Expected
Self-approval blocked; approval and merge recorded; staging deployment observable; duplicate second approval rejected [duplicate_event]; data_conflict merge fails closed with no double deploy; service_unavailable on CI re-run leaves reads alive and a clean retry passes.

## Actual
All gates behaved as catalogued: the author view exposes no approve form and the API twin confirms the 403 self-approval block; the injected service_unavailable on the CI re-run failed safe with reads alive and the clean retry (run-0073) passed; noor.haddad's approval is recorded; the same-reviewer second approval was rejected 409 [duplicate_event] ("Noor Haddad already approved PR #302."); the injected data_conflict merge failed closed with the rebase-required flash; the clean merge produced the merged PR, staging deployment dep-1104, and the service.version_bumped event in the post-hoc pull. Deviations: the INSTRUCT inference step could not be performed — no Codex Universal surface exists (F1, P0). One evidence-hygiene note: the first post-hoc events pull used a filtered query that returned a partial list; it was superseded by a corrected unfiltered pull (both retained, the corrected one is authoritative).

## Workflow Identity
- Workflow: NONE — surface missing (no Codex Universal teaching surface exists at base 0d314fd; F1)
- Version: NONE — surface missing (F1)
- Source revision: NONE — surface missing (F1)
- Definition digest: NONE — surface missing (F1)
- Dependency-lock identity: NONE — surface missing (F1)

## Runtime Binding
- Capabilities: pr:review + pr:merge exercised by noor.haddad (code owner); author self-approval correctly denied; Codex Universal capability binding: NONE — surface missing (F1)
- Resources: ForgeOps fixture at 127.0.0.1:4102; repo r-1 (payments-service); PR pr-0301 (Refund idempotency keys); CI runs (run-0073); artifact art-0902; staging deployment dep-1104
- Environments: local sandbox fixture process (node server.js --port 4102); browser surface across two personas
- Approvals: code-owner approval gate (non-self) exercised — author blocked, owner approval recorded; merge gate exercised with conflict fail-closed
- Triggers/schedules: CI-completed gating and merge→staging-deploy side-effect exercised; the taught workflow would carry PR-opened + CI-completed event triggers — no compile surface to verify against (F1)

## Evidence
- docs/validation/evidence/VWO-004/software-pr-triage/user-path/20260912T001123Z-instruct-phase-instruction-text.md
- docs/validation/evidence/VWO-004/software-pr-triage/user-path/20260912T001252Z-step01-repo-page-with-open-prs.txt
- docs/validation/evidence/VWO-004/software-pr-triage/user-path/20260912T001252Z-step02-pr-page-ci-passed-author-view.txt
- docs/validation/evidence/VWO-004/software-pr-triage/user-path/20260912T001252Z-step03-step03-author-view-no-approve-form.txt
- docs/validation/evidence/VWO-004/software-pr-triage/user-path/20260912T001252Z-switch-service-unavailable-ci-rerun.txt
- docs/validation/evidence/VWO-004/software-pr-triage/user-path/20260912T001252Z-switch-ci-retry-succeeds.txt
- docs/validation/evidence/VWO-004/software-pr-triage/user-path/20260912T001252Z-step04-approval-recorded.txt
- docs/validation/evidence/VWO-004/software-pr-triage/user-path/20260912T001252Z-natural-duplicate-approval-rejected.txt
- docs/validation/evidence/VWO-004/software-pr-triage/user-path/20260912T001252Z-switch-merge-data-conflict.txt
- docs/validation/evidence/VWO-004/software-pr-triage/user-path/20260912T001252Z-step05-merged-staging-deployed.txt
- docs/validation/evidence/VWO-004/software-pr-triage/user-path/20260912T001252Z-step05-merged-staging-screenshot.png
- docs/validation/evidence/VWO-004/software-pr-triage/user-path/20260912T001252Z-step06-teaching-surface-missing-note.md
- docs/validation/evidence/VWO-004/software-pr-triage/post-hoc/20260912T001237Z-self-approval-and-events-pulls.txt
- docs/validation/evidence/VWO-004/software-pr-triage/post-hoc/20260912T001443Z-events-pull-corrected.txt
- docs/validation/evidence/VWO-004/software-pr-triage/post-hoc/20260912T001252Z-reset-verification.txt

## Failure / Recovery
- Failure injected: service_unavailable on CI re-run (fail-safe 503, reads alive; clean retry passed); data_conflict on merge (409 rebase-required, no double deploy)
- Recovery attempted: retried the CI re-run after the switch cleared (run-0073 passed) and re-ran the merge cleanly (merged + staging deployed) — both recoveries succeeded; the duplicate second approval needed no recovery (record intact)
- Restart/session-loss behavior: not attempted — not on this scenario's catalog path; persona browser sessions were recreated between steps with server state continuous
- Final outcome: passed for the business path — CI gate, code-owner gate, self-approval block, duplicate gate, merge-conflict gate and staging side-effect all verified; reset verified; the INSTRUCT inference step is blocked (F1)

## Product / UX Friction
The PR page communicates state well for a non-implementer reviewer: CI status, approvals and merge result are all visible with named ids. Friction: the author's own PR page simply hides the approve form rather than explaining why — a normal person might wonder whether the feature is missing rather than deliberately withheld (the API twin makes the intent explicit; the UI could state "you cannot approve your own PR"). The duplicate-approval and merge-conflict flashes are exemplary (they name the actor and the remediation).

## Security / Architecture Findings
- Finding: F1 (deduplicated; root cause identical to scenario block 1) — the INSTRUCT inference step could not be performed: no Codex Universal surface exists to submit the triage-policy description to, so the inferred gates (CI + code-owner + non-self + staging side-effect) could not be evaluated.
- Severity: P0
- Reproduction: docs/validation/evidence/VWO-004/_infra/post-hoc/20260912T003813Z-product-surface-absence-check.txt and docs/validation/evidence/VWO-004/software-pr-triage/user-path/20260912T001252Z-step06-teaching-surface-missing-note.md
- Root cause: workflow family ships as library crates only at this SHA (no product consumer)
- Frozen invariant affected: INSTRUCT-mode guarantee (VALIDATION-PROGRAM.md §6) is unverifiable — no inference surface exists

## Recommendation
- NEW WORK ORDER — F1 (P0): as in scenario block 1 (build the user-facing Codex Universal teaching surface; owner Tech Lead → workflow-app owner; verification = re-run teaching steps + re-validate report).

## Worker Conclusion
blocked — the triage business path passed end-to-end (CI re-run with fail-safe, owner approval, duplicate rejection, merge with conflict gate, staging deployment), but the catalog's INSTRUCT teaching step could not be performed because the Codex Universal teaching surface does not exist at this SHA (F1, P0); recorded, not worked around.

## Scenario
- Scenario id: software-incident-response
- Industry: software
- Scenario: production incident response across browser + terminal console (ForgeOps, port 4102, deploy-console.js)
- Teaching mode: HYBRID
- User goal: take a production incident from open → mitigate → promote the fix to production → resolve — taught with initial instructions plus demonstrated critical steps, and executed across BOTH the browser surface and the family's desktop-capable terminal console.

## User Path
1. Signed in as mei.chen (SRE on-call); opened /incidents — the mitigated incident inc-0902 (Checkout latency spike) is listed.
2. Opened a new incident for payments-service (sev2, payments-api marked down) — inc-0903 appears open with the service health link.
3. Signed in as noor.haddad; approved and merged the fix PR — producing the staging deployment dep-1104 (with artifact art-0902).
4. Opened a terminal, ran `cd docs/validation/fixtures/software && node deploy-console.js`; logged in as raj.patel (release manager); ran `deployments r-1` — the deployment list shows dep-1104 staging, dep-1102/1101 production.
5. In the console, promoted dep-1104 with service_unavailable armed — "FAILED 503: service_unavailable — The \"ci-runner-pool\" service is unavailable (simulated). State-changing requests are rejected; read requests still work."; promoted with missing_asset armed — "FAILED 404: missing_asset — Release artifact \"art-0902\" could not be resolved in the artifact registry (simulated dangling reference)."; clean retry — "OK: payments-service 4.8.2 promoted to production (deployment dep-1105)."
6. Back in the browser as mei.chen — /deployments shows the production deployment after the promote; resolved inc-0903 — resolution observed.
7. Catalog step 6 (HYBRID instruct + mixed-modality reconciliation) not performed — teaching surface missing (see Actual).

## Expected
Incident lifecycle visible; production promotion requires the release manager; mixed-modality workflow compiles with both step types; service version bump observable; promote fails safe under service_unavailable (no partial deployment, console shows the 503 with the named dependency) and under missing_asset (artifact id named); retry after clear succeeds.

## Actual
The full lifecycle behaved as catalogued: inc-0903 opened (sev2, payments-api down); the fix merged to staging (dep-1104); the terminal console exercised the release-manager promotion gate — the service_unavailable promote failed safe naming ci-runner-pool, the missing_asset promote failed naming artifact art-0902, and the clean retry promoted payments-service 4.8.2 to production as dep-1105; mei.chen resolved inc-0903 in the browser. The post-hoc event pull shows exactly ONE deployment.promoted event — the fail-safe left no partial or double deployment. Deviation: the HYBRID mixed-modality reconciliation step could not be performed — no Codex Universal surface exists to compile browser+terminal steps into one workflow (F1, P0); both modalities were still exercised end-to-end by the human, so the surfaces themselves are proven.

## Workflow Identity
- Workflow: NONE — surface missing (no Codex Universal teaching surface exists at base 0d314fd; F1)
- Version: NONE — surface missing (F1)
- Source revision: NONE — surface missing (F1)
- Definition digest: NONE — surface missing (F1)
- Dependency-lock identity: NONE — surface missing (F1)

## Runtime Binding
- Capabilities: incident:open + incident:resolve exercised by mei.chen (SRE); deploy:promote exercised by raj.patel (release manager) via the terminal console; pr:review/pr:merge by noor.haddad; Codex Universal capability binding: NONE — surface missing (F1)
- Resources: ForgeOps fixture at 127.0.0.1:4102; incidents inc-0902 (seed) and inc-0903 (run); PR fix; artifact art-0902; deployments dep-1104 (staging) and dep-1105 (production); ci-runner-pool dependency (named in the 503)
- Environments: local sandbox fixture process (node server.js --port 4102); browser surface (two personas) AND the desktop-target terminal console (node deploy-console.js piped sessions, four sessions A–D)
- Approvals: release-manager promotion approval exercised at the terminal boundary — the promote command is gated on the logged-in release manager; SRE resolution gated on incident:resolve
- Triggers/schedules: incident-opened trigger semantics exercised; the taught workflow would carry the trigger plus a promote step bound to the terminal surface — no compile surface to verify against (F1)

## Evidence
- docs/validation/evidence/VWO-004/software-incident-response/user-path/20260912T001509Z-instruct-phase-instruction-text.md
- docs/validation/evidence/VWO-004/software-incident-response/user-path/20260912T001914Z-step01-incidents-list-mitigated-inc0902.txt
- docs/validation/evidence/VWO-004/software-incident-response/user-path/20260912T001914Z-step02-incident-inc0903-opened-sev2.txt
- docs/validation/evidence/VWO-004/software-incident-response/user-path/20260912T001914Z-step03-terminal-console-promote-transcript.txt
- docs/validation/evidence/VWO-004/software-incident-response/user-path/20260912T001914Z-step04-deployments-production-after-promote.txt
- docs/validation/evidence/VWO-004/software-incident-response/user-path/20260912T001914Z-step05-incident-resolved-healthy.txt
- docs/validation/evidence/VWO-004/software-incident-response/user-path/20260912T001914Z-demo-phase-reconciliation-surface-missin.md
- docs/validation/evidence/VWO-004/software-incident-response/post-hoc/20260912T001914Z-events-and-service-state.txt
- docs/validation/evidence/VWO-004/software-incident-response/post-hoc/20260912T002010Z-events-pull-corrected-unfiltered.txt
- docs/validation/evidence/VWO-004/software-incident-response/post-hoc/20260912T001915Z-reset-verification.txt

## Failure / Recovery
- Failure injected: service_unavailable on promote (ci-runner-pool named — 503 fail-safe); missing_asset on promote (artifact art-0902 named — 404)
- Recovery attempted: re-ran the promote after each failure cleared — the clean retry succeeded ("OK: payments-service 4.8.2 promoted to production (deployment dep-1105)") and the post-hoc event pull shows exactly ONE deployment.promoted (no partial/double deploy from the failed attempts)
- Restart/session-loss behavior: exercised across console sessions — each deploy-console.js session is an independent login (session loss analog) and none left partial state; server state continuous through the run
- Final outcome: passed for the business path — incident open→resolve, mixed browser+terminal execution, fail-safe promote with named dependencies, single production promotion, and reset all verified; the HYBRID reconciliation step is blocked (F1)

## Product / UX Friction
The terminal console is genuinely usable by a release manager: plain commands (login, deployments, promote), plain answers, failures that name the dependency and the artifact id, and a clean success line that names the service, version and deployment id. The browser side keeps the incident and deployment state visible so the two modalities corroborate each other. Friction: the console prints "token stored for this session" — accurate but slightly jargon-adjacent; harmless. No other friction observed.

## Security / Architecture Findings
- Finding: F1 (deduplicated; root cause identical to scenario block 1) — the HYBRID mixed-modality reconciliation step could not be performed: there is no Codex Universal surface to compile the browser steps and the terminal promote into one workflow, so "neither silently overriding the other" could not be verified.
- Severity: P0
- Reproduction: docs/validation/evidence/VWO-004/_infra/post-hoc/20260912T003813Z-product-surface-absence-check.txt and docs/validation/evidence/VWO-004/software-incident-response/user-path/20260912T001914Z-demo-phase-reconciliation-surface-missin.md
- Root cause: workflow family ships as library crates only at this SHA (no product consumer)
- Frozen invariant affected: HYBRID reconciliation guarantee (VALIDATION-PROGRAM.md §6) is unverifiable — the compile surface does not exist

## Recommendation
- NEW WORK ORDER — F1 (P0): as in scenario block 1 (build the user-facing Codex Universal teaching surface; owner Tech Lead → workflow-app owner; verification = re-run teaching steps + re-validate report).

## Worker Conclusion
blocked — the incident→promote→resolve business path passed end-to-end across browser AND terminal surfaces with fail-safe, named-dependency failures and exactly one production promotion verified, but the catalog's HYBRID teaching/reconciliation step could not be performed because the Codex Universal teaching surface does not exist at this SHA (F1, P0); recorded, not worked around.

## Scenario
- Scenario id: software-engineering-onboarding
- Industry: software
- Scenario: new-engineer onboarding with permission boundaries (ForgeOps, port 4102)
- Teaching mode: DEMONSTRATE
- User goal: a new engineer's first week — find the ownership directory, read internal docs, file a first issue, attempt a first doc edit — demonstrated end-to-end so Codex Universal can observe and later automate the onboarding checklist.

## User Path
1. Signed in as tobi.oyelaran (intern — lowest-privilege onboarding subject); opened /teams — the ownership directory shows payments-core owns payments-service.
2. Opened /docs — the onboarding/ownership runbook (v3) is listed with versioning; read the runbook.
3. Opened /issues; created a first issue ("typo in payments runbook", P3) — issue i-0504 is listed.
4. Attempted to promote a deployment as the intern via the terminal console (deploy-console.js) — natural permission rejection: 403 permission_denied naming deploy:promote.
5. Armed permission_denied on issue create (the intern's one allowed power) — 403 flash naming issue:create; clean retry created the issue.
6. Signed in as noor.haddad (code owner / buddy); edited the runbook with data_conflict armed — 409 optimistic-concurrency failure telling the editor to re-read; clean save produced runbook v4.
7. Catalog step 6 (DEMONSTRATE replay of the checklist) not performed — teaching surface missing (see Actual).

## Expected
Ownership discoverable; issue creation works for the intern; privileged actions fail [permission_denied] with the required permission named; doc edit by the owner bumps version; data_conflict on the doc edit fails with re-read-and-retry guidance; the inferred workflow encodes the read/create-only boundary.

## Actual
All boundaries behaved as catalogued: the /teams directory answered the ownership question in one page; the intern created issue i-0504; the intern's console promote attempt was rejected 403 naming deploy:promote; the injected permission_denied on issue create named issue:create and the clean retry succeeded; the owner's data_conflict edit failed closed with re-read guidance and the clean save bumped the runbook v3 → v4. Deviation: the DEMONSTRATE replay step could not be performed — no Codex Universal surface exists (F1, P0).

## Workflow Identity
- Workflow: NONE — surface missing (no Codex Universal teaching surface exists at base 0d314fd; F1)
- Version: NONE — surface missing (F1)
- Source revision: NONE — surface missing (F1)
- Definition digest: NONE — surface missing (F1)
- Dependency-lock identity: NONE — surface missing (F1)

## Runtime Binding
- Capabilities: issue:create exercised by tobi.oyelaran (intern) — and correctly nothing else; deploy:promote correctly denied at the console boundary; doc:write exercised by noor.haddad; Codex Universal capability binding: NONE — surface missing (F1)
- Resources: ForgeOps fixture at 127.0.0.1:4102; /teams ownership directory; runbook doc (v3→v4); issue i-0504; deploy console
- Environments: local sandbox fixture process (node server.js --port 4102); browser surface (two personas) plus the terminal console for the blocked-promote attempt
- Approvals: permission boundaries as approval-like gates — intern read/create-only boundary enforced at both UI and API/console boundaries; owner edit gate exercised with optimistic-versioning conflict
- Triggers/schedules: doc-updated trigger semantics exercised (version bump); the taught workflow would carry the trigger and role-restricted steps — no compile surface to verify against (F1)

## Evidence
- docs/validation/evidence/VWO-004/software-engineering-onboarding/user-path/20260912T002204Z-step01-teams-ownership-directory.txt
- docs/validation/evidence/VWO-004/software-engineering-onboarding/user-path/20260912T002204Z-step02-docs-list-with-versioning.txt
- docs/validation/evidence/VWO-004/software-engineering-onboarding/user-path/20260912T002204Z-step02-runbook-doc-v3.txt
- docs/validation/evidence/VWO-004/software-engineering-onboarding/user-path/20260912T002204Z-step03-first-issue-created.txt
- docs/validation/evidence/VWO-004/software-engineering-onboarding/user-path/20260912T002204Z-step04-intern-promote-blocked-console.txt
- docs/validation/evidence/VWO-004/software-engineering-onboarding/user-path/20260912T002204Z-switch-permission-denied-issue-create.txt
- docs/validation/evidence/VWO-004/software-engineering-onboarding/user-path/20260912T002204Z-switch-data-conflict-doc-edit.txt
- docs/validation/evidence/VWO-004/software-engineering-onboarding/user-path/20260912T002204Z-step05-owner-doc-edit-v4.txt
- docs/validation/evidence/VWO-004/software-engineering-onboarding/user-path/20260912T002204Z-step06-teaching-surface-missing-note.md
- docs/validation/evidence/VWO-004/software-engineering-onboarding/post-hoc/20260912T002204Z-events-and-doc-state.txt
- docs/validation/evidence/VWO-004/software-engineering-onboarding/post-hoc/20260912T002204Z-reset-verification.txt

## Failure / Recovery
- Failure injected: permission_denied on the intern's issue create (403 naming issue:create — the one allowed power blocked); data_conflict on the owner's doc edit (409 optimistic-concurrency with re-read guidance)
- Recovery attempted: clean retry after each switch cleared — the issue was created and the doc edit saved as v4; state re-read between attempts showed no partial records
- Restart/session-loss behavior: the intern's console session is an independent login (session-loss analog) and left no partial state; browser sessions recreated between personas with server state continuous
- Final outcome: passed for the business path — ownership discovery, first issue, both permission boundaries, optimistic-versioning conflict and reset all verified; the DEMONSTRATE replay step is blocked (F1)

## Product / UX Friction
An exemplary onboarding surface for a normal person: the ownership question is answered by a directory page, the runbook is versioned in place, and every rejection names the missing permission so the intern learns the boundary from the error itself. Friction: none of substance observed on the business path.

## Security / Architecture Findings
- Finding: F1 (deduplicated; root cause identical to scenario block 1) — the DEMONSTRATE replay step could not be performed: no Codex Universal surface exists to observe the checklist and infer the onboarding workflow with its permission boundaries.
- Severity: P0
- Reproduction: docs/validation/evidence/VWO-004/_infra/post-hoc/20260912T003813Z-product-surface-absence-check.txt and docs/validation/evidence/VWO-004/software-engineering-onboarding/user-path/20260912T002204Z-step06-teaching-surface-missing-note.md
- Root cause: workflow family ships as library crates only at this SHA (no product consumer)
- Frozen invariant affected: DEMONSTRATE-mode guarantee (VALIDATION-PROGRAM.md §6 — the system observes a performed process) is unverifiable — no observation surface exists

## Recommendation
- NEW WORK ORDER — F1 (P0): as in scenario block 1 (build the user-facing Codex Universal teaching surface; owner Tech Lead → workflow-app owner; verification = re-run teaching steps + re-validate report).

## Worker Conclusion
blocked — the onboarding business path passed end-to-end (ownership found, runbook read, issue filed, both permission boundaries enforced with named permissions, owner edit with conflict gate), but the catalog's DEMONSTRATE teaching step could not be performed because the Codex Universal teaching surface does not exist at this SHA (F1, P0); recorded, not worked around.

## Scenario
- Scenario id: rideshare-driver-onboarding
- Industry: rideshare
- Scenario: driver onboarding from public intake to live map (RidePilot, port 4103)
- Teaching mode: DEMONSTRATE
- User goal: take a driver from public application through document screening to activation on the map — demonstrated for Codex Universal to observe the full intake→screen→activate pipeline and its gates.

## User Path
1. With NO login, opened http://127.0.0.1:4103/intake; filled name/phone/plate/region; submitted — the application confirmation flash shows the new application id (da-0304).
2. Signed in as jules.moreau (regional ops manager); opened /onboarding — the queue shows da-0304 in submitted state.
3. Screened da-0304 with documents verified — approved; the state change is visible in the queue.
4. Tried screening seed application da-0302 (Elena Petrova) — natural rejection: insurance document missing ([missing_asset]); the application stays in screening.
5. Activated the screened applicant da-0304 — the driver (dr-0504) appears in the directory and on /map, and the welcome broadcast was sent.
6. Tried activating seed application da-0303 (Tomás Rivera) — natural rejection: expired permit ([stale_entitlement]); no partial driver record was created.
7. Catalog step 7 (DEMONSTRATE replay of steps 1–5) not performed — teaching surface missing (see Actual).

## Expected
Public intake works without auth; screening requires an ops role; missing insurance and expired permit each fail closed with named causes; activation is visible on the map and directory; no partial driver record on failed activation.

## Actual
The full pipeline behaved as catalogued: the public intake needed no login and returned the da-0304 confirmation; the ops queue showed it submitted; screening with verified documents approved it; the da-0302 screening failed closed naming the missing insurance document; activation of da-0304 produced driver dr-0504 on the directory AND the live map with the welcome broadcast sent; the da-0303 activation failed closed naming the expired permit with no partial driver record. Post-hoc pulls show driver.activated and application.* events. Deviation: the DEMONSTRATE replay step could not be performed — no Codex Universal surface exists (F1, P0).

## Workflow Identity
- Workflow: NONE — surface missing (no Codex Universal teaching surface exists at base 0d314fd; F1)
- Version: NONE — surface missing (F1)
- Source revision: NONE — surface missing (F1)
- Definition digest: NONE — surface missing (F1)
- Dependency-lock identity: NONE — surface missing (F1)

## Runtime Binding
- Capabilities: driver screen/activate exercised by jules.moreau (ops); public intake requires no auth (by design); Codex Universal capability binding: NONE — surface missing (F1)
- Resources: RidePilot fixture at 127.0.0.1:4103; applications da-0304 (run), da-0302/da-0303 (seed gates); driver dr-0504; the live map; the broadcast channel
- Environments: local sandbox fixture process (node server.js --port 4103); browser surface including an unauthenticated public page
- Approvals: ops screening + activation gates exercised — both natural document/permit gates fail closed with named causes
- Triggers/schedules: application-submitted trigger semantics exercised (queue population); the taught workflow would carry the trigger and document/permit validity conditions — no compile surface to verify against (F1)

## Evidence
- docs/validation/evidence/VWO-004/rideshare-driver-onboarding/user-path/20260912T002430Z-step01-public-intake-confirmation.txt
- docs/validation/evidence/VWO-004/rideshare-driver-onboarding/user-path/20260912T002430Z-step02-onboarding-queue-submitted.txt
- docs/validation/evidence/VWO-004/rideshare-driver-onboarding/user-path/20260912T002430Z-step03-screening-approved.txt
- docs/validation/evidence/VWO-004/rideshare-driver-onboarding/user-path/20260912T002430Z-step04-da0302-insurance-missing-block.txt
- docs/validation/evidence/VWO-004/rideshare-driver-onboarding/user-path/20260912T002430Z-step05-driver-activated-dr0504.txt
- docs/validation/evidence/VWO-004/rideshare-driver-onboarding/user-path/20260912T002430Z-step05-live-map-with-driver.txt
- docs/validation/evidence/VWO-004/rideshare-driver-onboarding/user-path/20260912T002430Z-step05-live-map-screenshot.png
- docs/validation/evidence/VWO-004/rideshare-driver-onboarding/user-path/20260912T002430Z-step05-welcome-broadcast-sent.txt
- docs/validation/evidence/VWO-004/rideshare-driver-onboarding/user-path/20260912T002430Z-step06-da0303-permit-expired-block.txt
- docs/validation/evidence/VWO-004/rideshare-driver-onboarding/user-path/20260912T002430Z-step07-teaching-surface-missing-note.md
- docs/validation/evidence/VWO-004/rideshare-driver-onboarding/post-hoc/20260912T002430Z-events-and-driver-state.txt
- docs/validation/evidence/VWO-004/rideshare-driver-onboarding/post-hoc/20260912T002430Z-reset-verification.txt

## Failure / Recovery
- Failure injected: none via switch — this scenario's catalogued failure modes occur naturally on the seed applications (da-0302 missing insurance [missing_asset]; da-0303 expired permit [stale_entitlement]) and were exercised as natural gates
- Recovery attempted: verified the documented recovery postures — the blocked applications stay in their pre-gate state (no partial driver record), and the valid applicant proceeded through the full pipeline after document verification
- Restart/session-loss behavior: not attempted — not on this scenario's catalog path; the unauthenticated intake page and the ops session were exercised as separate browser contexts with server state continuous
- Final outcome: passed for the business path — public intake, screening gate, activation on map + directory, both natural fail-closed gates and reset all verified; the DEMONSTRATE replay step is blocked (F1)

## Product / UX Friction
The public intake form is exactly what a non-technical applicant expects (no login, plain confirmation with an application id). The ops queue makes the state machine legible. Friction: none of substance observed on the business path; the two rejection causes name the exact document/permit problem so an ops user knows what to ask the applicant for.

## Security / Architecture Findings
- Finding: F1 (deduplicated; root cause identical to scenario block 1) — the DEMONSTRATE replay step could not be performed: no Codex Universal surface exists to observe the intake→screen→activate pipeline and infer its document/permit gates.
- Severity: P0
- Reproduction: docs/validation/evidence/VWO-004/_infra/post-hoc/20260912T003813Z-product-surface-absence-check.txt and docs/validation/evidence/VWO-004/rideshare-driver-onboarding/user-path/20260912T002430Z-step07-teaching-surface-missing-note.md
- Root cause: workflow family ships as library crates only at this SHA (no product consumer)
- Frozen invariant affected: DEMONSTRATE-mode guarantee (VALIDATION-PROGRAM.md §6) is unverifiable — no observation surface exists

## Recommendation
- NEW WORK ORDER — F1 (P0): as in scenario block 1 (build the user-facing Codex Universal teaching surface; owner Tech Lead → workflow-app owner; verification = re-run teaching steps + re-validate report).

## Worker Conclusion
blocked — the onboarding business path passed end-to-end (public intake → screening → activation on the live map with broadcast, plus both natural fail-closed document/permit gates), but the catalog's DEMONSTRATE teaching step could not be performed because the Codex Universal teaching surface does not exist at this SHA (F1, P0); recorded, not worked around.

## Scenario
- Scenario id: rideshare-support-escalation
- Industry: rideshare
- Scenario: support ticket response and escalation (RidePilot, port 4103)
- Teaching mode: INSTRUCT
- User goal: handle a rider/driver complaint end-to-end — respond, escalate to engineering when reproducible, resolve — described to Codex Universal in words; the system must infer respond→escalate→resolve, the billing-refund branch, and the notification to the ops director.

## User Path
1. Signed in as ana.silva (support agent); opened /support; opened ticket tk-0701 (Charged twice for a cancelled trip — high priority, open).
2. Responded to the customer — the reply is recorded in the message thread.
3. Escalated the ticket with notification_failure armed — flash "Ticket tk-0701 escalated to regional ops." plus warning "Escalation recorded, but the ops notification could not be delivered."; the failed notification row is visible in /notifications.
4. Catalog step 4 expects the agent to resolve the ticket — MISMATCH: the fixture grants ticket:resolve to ops only, so jules.moreau resolved tk-0701 instead (the intent — closure by a higher authority after the refund — is preserved; recorded as F3).
5. Armed data_conflict on the resolve — 409 fail-closed with the record intact.
6. Opened tk-0702 (already escalated) and attempted to escalate it again — natural 409 [data_conflict] via the API twin (the UI hides the escalate form for non-open tickets).
7. Attempted to screen a driver as the support agent — natural 403 [permission_denied] naming driver:screen.
8. Catalog step 6 (INSTRUCT description → inferred escalation policy) not performed — teaching surface missing (see Actual).

## Expected
Thread and state transitions visible; re-escalation blocked [data_conflict]; agent cannot screen drivers [permission_denied]; with notification_failure armed the escalation is recorded but the ops notification shows failed with a warning (observable, not silent); resolve of an already-decided ticket fails closed.

## Actual
The escalation path behaved as catalogued: the reply landed in the thread; the escalation was recorded with the observable warning and the failed notification row; the data_conflict resolve failed closed; the tk-0702 re-escalation was naturally rejected; the agent's driver-screen attempt was rejected 403 naming driver:screen. Deviations: (a) catalog step 4 assigns resolution to the agent, but the fixture's permission model is ops-only for ticket:resolve — jules.moreau resolved instead (F3, P2, harness-level; the closure-by-higher-authority intent is preserved); (b) the INSTRUCT inference step could not be performed — no Codex Universal surface exists (F1, P0).

## Workflow Identity
- Workflow: NONE — surface missing (no Codex Universal teaching surface exists at base 0d314fd; F1)
- Version: NONE — surface missing (F1)
- Source revision: NONE — surface missing (F1)
- Definition digest: NONE — surface missing (F1)
- Dependency-lock identity: NONE — surface missing (F1)

## Runtime Binding
- Capabilities: ticket respond/escalate exercised by ana.silva (support agent); ticket:resolve exercised by jules.moreau (ops — the fixture's resolver role); driver:screen correctly denied to the agent; Codex Universal capability binding: NONE — surface missing (F1)
- Resources: RidePilot fixture at 127.0.0.1:4103; tickets tk-0701 (run) and tk-0702 (seed conflict gate); the ops notification channel
- Environments: local sandbox fixture process (node server.js --port 4103); browser surface across two personas
- Approvals: escalation-to-engineering handoff exercised; closure gate exercised by the ops role (per fixture permission model — see F3)
- Triggers/schedules: ticket-created trigger semantics exercised; the taught workflow would carry the trigger and a priority-based branch — no compile surface to verify against (F1)

## Evidence
- docs/validation/evidence/VWO-004/rideshare-support-escalation/user-path/20260912T002453Z-instruct-phase-instruction-text.md
- docs/validation/evidence/VWO-004/rideshare-support-escalation/user-path/20260912T002636Z-step01-ticket-thread-open.txt
- docs/validation/evidence/VWO-004/rideshare-support-escalation/user-path/20260912T002636Z-step02-reply-recorded-in-thread.txt
- docs/validation/evidence/VWO-004/rideshare-support-escalation/user-path/20260912T002636Z-step03-escalation-recorded-notification-warning.txt
- docs/validation/evidence/VWO-004/rideshare-support-escalation/user-path/20260912T002636Z-step03-escalation-notification-row.txt
- docs/validation/evidence/VWO-004/rideshare-support-escalation/user-path/20260912T002604Z-step04-step04-agent-resolve-permission-mismatch.md
- docs/validation/evidence/VWO-004/rideshare-support-escalation/user-path/20260912T002637Z-step04-ops-resolved-ticket.txt
- docs/validation/evidence/VWO-004/rideshare-support-escalation/user-path/20260912T002637Z-switch-data-conflict-resolve.txt
- docs/validation/evidence/VWO-004/rideshare-support-escalation/user-path/20260912T002637Z-step06-teaching-surface-missing-note.md
- docs/validation/evidence/VWO-004/rideshare-support-escalation/post-hoc/20260912T002626Z-conflicts-permissions-events.txt
- docs/validation/evidence/VWO-004/rideshare-support-escalation/post-hoc/20260912T002637Z-reset-verification.txt

## Failure / Recovery
- Failure injected: notification_failure on escalate (escalation recorded + observable warning + failed row); data_conflict on resolve (409 fail-closed, record intact)
- Recovery attempted: read /notifications as the warning directs (failed row observable — not silent); re-read ticket state after the conflict (status unchanged); natural re-escalation of tk-0702 and the agent screen attempt probed via the API twin where the UI hides the form — both fail closed as catalogued
- Restart/session-loss behavior: not attempted — not on this scenario's catalog path; persona sessions recreated with server state continuous
- Final outcome: passed for the gates exercised — escalation with observable notification failure, conflict gates, permission boundary and reset all verified; resolution executed by ops instead of the agent (F3) and the INSTRUCT inference step is blocked (F1)

## Product / UX Friction
The ticket thread makes the state machine legible and the notification-failure warning is actionable (points at the notification surface). Friction: a support agent who believes they own the ticket end-to-end will discover mid-flow that resolution is not theirs to perform — the UI simply lacks the resolve affordance for the agent role. That is a defensible permission design, but the catalog and the fixture disagree on who resolves (F3), and the UI gives no explanatory text for the missing affordance.

## Security / Architecture Findings
- Finding: F1 (deduplicated; root cause identical to scenario block 1) — the INSTRUCT inference step could not be performed: no Codex Universal surface exists to submit the escalation-policy description to, so the inferred respond→escalate→resolve workflow with the billing branch could not be evaluated.
- Severity: P0
- Reproduction: docs/validation/evidence/VWO-004/_infra/post-hoc/20260912T003813Z-product-surface-absence-check.txt and docs/validation/evidence/VWO-004/rideshare-support-escalation/user-path/20260912T002637Z-step06-teaching-surface-missing-note.md
- Root cause: workflow family ships as library crates only at this SHA (no product consumer)
- Frozen invariant affected: INSTRUCT-mode guarantee (VALIDATION-PROGRAM.md §6) is unverifiable — no inference surface exists
- Finding: F3 — catalog/fixture permission mismatch: catalog step 4 has the support agent resolve the ticket after the refund; the fixture grants ticket:resolve to ops only, so the ops manager resolved it. The business intent (closure by higher authority) is preserved; the catalog's role assignment is not executable as written.
- Severity: P2
- Reproduction: docs/validation/evidence/VWO-004/rideshare-support-escalation/user-path/20260912T002604Z-step04-step04-agent-resolve-permission-mismatch.md and docs/validation/evidence/VWO-004/rideshare-support-escalation/user-path/20260912T002637Z-step04-ops-resolved-ticket.txt
- Root cause: catalog entry's role model richer than the shipped fixture permission model
- Frozen invariant affected: none — harness-level mismatch; the fixture's own permission model behaved consistently and fail-closed

## Recommendation
- NEW WORK ORDER — F1 (P0): as in scenario block 1 (build the user-facing Codex Universal teaching surface; owner Tech Lead → workflow-app owner; verification = re-run teaching steps + re-validate report).
- FIX NOW — F3 (P2): align catalog step 4 with the fixture's permission model (preferred: grant the support agent ticket:resolve in the seed, or rebind the step to the ops resolver) so future wave-1/2 workers do not record the same mismatch.
  - Proposed owner: validation harness maintainer (VWO-002/VWO-003 owner)
  - Verification required: verify-sweep.sh PASS + re-run rideshare-support-escalation

## Worker Conclusion
blocked — the respond→escalate path passed with observable notification failure and conflict/permission gates verified (resolution completed by the ops role per the fixture's model — F3 recorded), but the catalog's INSTRUCT teaching step could not be performed because the Codex Universal teaching surface does not exist at this SHA (F1, P0); recorded, not worked around.

## Scenario
- Scenario id: rideshare-surge-ops
- Industry: rideshare
- Scenario: surge operations with concurrent-change and permission contrast (RidePilot, port 4103)
- Teaching mode: HYBRID
- User goal: watch region demand, decide surge, broadcast the change to drivers — instructed first ("raise surge when demand exceeds supply by 2×"), then the exception demonstrated (concurrent conflicting surge change), so Codex Universal infers the decision loop.

## User Path
1. Signed in as farah.khan (ops director); opened /ops — the four regions show demand/supply and current surge (Airport Corridor at 1.8×).
2. Opened /map — the hot regions are visually correlated (before screenshot captured).
3. Set the Airport Corridor surge 1.8 → 2.2 — the broadcast was sent; /ops and /map both show the change (after screenshot captured).
4. In a second browser session (jules.moreau, concurrent), attempted to change surge on the same region with data_conflict armed — fail-closed flash "Another ops manager adjusted Airport Corridor surge while you were saving (simulated). [data_conflict]" naming the current state (2.2×).
5. Signed in as kwame.mensah (finance analyst) and attempted to set surge — the UI renders no surge form for the finance role; the natural API-twin attempt was rejected 403 [permission_denied] naming surge:adjust.
6. Armed notification_failure on a surge set — the surge was set and the broadcast row shows failed with a warning (observable, not silent).
7. Recorded the reconcile note; catalog step 6 (HYBRID instruct + reconciliation) not performed — teaching surface missing (see Actual).

## Expected
Surge change visible in ops + map; drivers notified; concurrent change fails [data_conflict] (fail-closed with current state); finance analyst blocked [permission_denied]; notification_failure leaves the surge set with a failed broadcast row plus warning.

## Actual
The decision loop behaved as catalogued: the surge change (1.8 → 2.2) was visible on both /ops and /map with the broadcast sent; the concurrent second-session change failed closed naming the current 2.2× state; the finance role saw no form and the API-twin attempt failed 403 naming surge:adjust; the notification_failure injection left the surge set with the failed broadcast row and a visible warning. Post-hoc pulls show region.surge_set and broadcast.* events. Deviation: the HYBRID reconciliation step could not be performed — no Codex Universal surface exists (F1, P0).

## Workflow Identity
- Workflow: NONE — surface missing (no Codex Universal teaching surface exists at base 0d314fd; F1)
- Version: NONE — surface missing (F1)
- Source revision: NONE — surface missing (F1)
- Definition digest: NONE — surface missing (F1)
- Dependency-lock identity: NONE — surface missing (F1)

## Runtime Binding
- Capabilities: surge exercised by farah.khan (ops director) and blocked for kwame.mensah (finance — payout:approve only); Codex Universal capability binding: NONE — surface missing (F1)
- Resources: RidePilot fixture at 127.0.0.1:4103; the four regions (Airport Corridor et al.); the driver broadcast channel; the live map
- Environments: local sandbox fixture process (node server.js --port 4103); browser surface including TWO concurrent sessions for the conflict exercise
- Approvals: ops-director decision gate exercised; finance contrast verified at both UI (no form) and API boundary
- Triggers/schedules: demand-threshold decision semantics exercised; the taught workflow would carry a demand-threshold event trigger and a broadcast side-effect — no compile surface to verify against (F1)

## Evidence
- docs/validation/evidence/VWO-004/rideshare-surge-ops/user-path/20260912T002658Z-instruct-phase-instruction-text.md
- docs/validation/evidence/VWO-004/rideshare-surge-ops/user-path/20260912T002914Z-step01-regions-demand-surge-table.txt
- docs/validation/evidence/VWO-004/rideshare-surge-ops/user-path/20260912T002914Z-step01-ops-dashboard-surge-form.txt
- docs/validation/evidence/VWO-004/rideshare-surge-ops/user-path/20260912T002914Z-step02-live-map-before.txt
- docs/validation/evidence/VWO-004/rideshare-surge-ops/user-path/20260912T002914Z-step02-live-map-before-screenshot.png
- docs/validation/evidence/VWO-004/rideshare-surge-ops/user-path/20260912T002914Z-step03-surge-set-broadcast-sent.txt
- docs/validation/evidence/VWO-004/rideshare-surge-ops/user-path/20260912T002914Z-step03-live-map-after-surge.txt
- docs/validation/evidence/VWO-004/rideshare-surge-ops/user-path/20260912T002914Z-step03-live-map-after-screenshot.png
- docs/validation/evidence/VWO-004/rideshare-surge-ops/user-path/20260912T002914Z-step04-concurrent-change-conflict-failsclosed.txt
- docs/validation/evidence/VWO-004/rideshare-surge-ops/user-path/20260912T002914Z-step05-finance-view-no-surge-form.txt
- docs/validation/evidence/VWO-004/rideshare-surge-ops/user-path/20260912T002914Z-switch-notification-failure-broadcast.txt
- docs/validation/evidence/VWO-004/rideshare-surge-ops/user-path/20260912T002914Z-demo-phase-reconciliation-surface-missin.md
- docs/validation/evidence/VWO-004/rideshare-surge-ops/post-hoc/20260912T002915Z-events-and-region-state.txt
- docs/validation/evidence/VWO-004/rideshare-surge-ops/post-hoc/20260912T002915Z-natural-permission-denied-finance-surge.txt
- docs/validation/evidence/VWO-004/rideshare-surge-ops/post-hoc/20260912T002915Z-reset-verification.txt

## Failure / Recovery
- Failure injected: data_conflict on the concurrent surge change (fail-closed naming the current 2.2× state); notification_failure on the surge broadcast (surge set, broadcast row failed + warning)
- Recovery attempted: the losing session's documented recovery — re-read the current region state (which shows 2.2×) and re-decide; the notification failure's warning points at the notification surface where the failed row is observable
- Restart/session-loss behavior: exercised via the second concurrent browser session (stale-view analog) — the conflict was rejected fail-closed and both sessions re-read consistent state; server state continuous
- Final outcome: passed for the business path — surge decision visible on ops+map, broadcast semantics, concurrent fail-closed conflict, permission contrast and reset all verified; the HYBRID reconciliation step is blocked (F1)

## Product / UX Friction
The ops dashboard answers the demand/supply question in one table and the map corroborates it visually. The conflict flash is exemplary for a normal person: it names the region, the competing actor and the current value. Friction: the map is presentational (no drill-through from a hot region to its surge form) — the user must go back to /ops to act; minor.

## Security / Architecture Findings
- Finding: F1 (deduplicated; root cause identical to scenario block 1) — the HYBRID instruct+reconcile step could not be performed: no Codex Universal surface exists to compile the surge policy and the demonstrated concurrent-change exception, so last-writer-wins/conflict semantics could not be verified as explicitly reconciled by the system.
- Severity: P0
- Reproduction: docs/validation/evidence/VWO-004/_infra/post-hoc/20260912T003813Z-product-surface-absence-check.txt and docs/validation/evidence/VWO-004/rideshare-surge-ops/user-path/20260912T002914Z-demo-phase-reconciliation-surface-missin.md
- Root cause: workflow family ships as library crates only at this SHA (no product consumer)
- Frozen invariant affected: HYBRID reconciliation guarantee (VALIDATION-PROGRAM.md §6) is unverifiable — the compile surface does not exist

## Recommendation
- NEW WORK ORDER — F1 (P0): as in scenario block 1 (build the user-facing Codex Universal teaching surface; owner Tech Lead → workflow-app owner; verification = re-run teaching steps + re-validate report).

## Worker Conclusion
blocked — the surge decision loop passed end-to-end (ops+map visibility, broadcast, concurrent fail-closed conflict, finance permission contrast, observable notification failure), but the catalog's HYBRID teaching/reconciliation step could not be performed because the Codex Universal teaching surface does not exist at this SHA (F1, P0); recorded, not worked around.

## Scenario
- Scenario id: media-editorial-publishing
- Industry: media
- Scenario: editorial publishing with role gates and channel fan-out (PressRoom, port 4104)
- Teaching mode: DEMONSTRATE
- User goal: publish a story end-to-end — draft, hero image, review, approval, multi-channel publication — demonstrated so Codex Universal observes the editorial gates.

## User Path
1. Signed in as hana.kim (staff writer); opened /desk; created a draft (title, dek, body) — the story page shows the new draft st-0206.
2. Attached the hero image (asset a-0301) — the hero is visible on the story page.
3. Submitted for review — the story shows in_review.
4. Attempted to approve own story as the writer — the UI hides the approve form for the writer; the natural self-approval rejection (403 [permission_denied] story:approve) was probed and recorded via the API twin.
5. Signed in as camille.dubois (editor-in-chief); approved; published to the channels — flash "Story st-0206 published (3/4 channels delivered)." plus warning "Published, but distribution to these channels failed (stale license): partner-app." — a natural partial success where the partner-app channel job failed [stale_entitlement] naming the license expiry while the valid channels received the story.
6. Published seed story st-0205 (approved, NO hero) — natural block [missing_asset]; the story stays approved (publish gate intact).
7. Catalog step 7 (DEMONSTRATE replay of steps 1–5) not performed — teaching surface missing (see Actual).

## Expected
Lifecycle states visible at each step; self-approval blocked; no-hero publish blocked; publication fans out to channels with per-channel status; the partner-app channel fails with the license cause while valid channels still receive the story (partial success with warning, failed job observable).

## Actual
The editorial path behaved as catalogued: the draft→hero→in_review transitions are visible on the story page; the writer's approve attempt is absent from the UI and rejected 403 story:approve at the API boundary; the editor's approve+publish produced the 3/4 partial success with the stale-license warning naming partner-app; the no-hero publish of st-0205 was blocked [missing_asset] with the story remaining approved. Post-hoc pulls show story.*, distribution.* and the failed channel job. Deviation: the DEMONSTRATE replay step could not be performed — no Codex Universal surface exists (F1, P0).

## Workflow Identity
- Workflow: NONE — surface missing (no Codex Universal teaching surface exists at base 0d314fd; F1)
- Version: NONE — surface missing (F1)
- Source revision: NONE — surface missing (F1)
- Definition digest: NONE — surface missing (F1)
- Dependency-lock identity: NONE — surface missing (F1)

## Runtime Binding
- Capabilities: story:create/edit exercised by hana.kim (staff writer); approve/publish exercised by camille.dubois (editor-in-chief); writer self-approval correctly denied; Codex Universal capability binding: NONE — surface missing (F1)
- Resources: PressRoom fixture at 127.0.0.1:4104; story st-0206 (run) and st-0205 (seed hero gate); hero asset a-0301; the four distribution channels (partner-app license expired 2026-06-01)
- Environments: local sandbox fixture process (node server.js --port 4104); browser surface across two personas
- Approvals: editor approval gate (non-self) exercised; per-channel license gates exercised (partial success with named cause)
- Triggers/schedules: publication trigger semantics exercised; the taught workflow would carry publish/approve steps and the trigger — no compile surface to verify against (F1)

## Evidence
- docs/validation/evidence/VWO-004/media-editorial-publishing/user-path/20260912T003057Z-step01-draft-created.txt
- docs/validation/evidence/VWO-004/media-editorial-publishing/user-path/20260912T003057Z-step02-hero-attached.txt
- docs/validation/evidence/VWO-004/media-editorial-publishing/user-path/20260912T003057Z-step03-submitted-in-review.txt
- docs/validation/evidence/VWO-004/media-editorial-publishing/user-path/20260912T003018Z-step04-step04-writer-self-approval-blocked.txt
- docs/validation/evidence/VWO-004/media-editorial-publishing/user-path/20260912T003057Z-step05-editor-approved.txt
- docs/validation/evidence/VWO-004/media-editorial-publishing/user-path/20260912T003057Z-step05-published-partial-channel-failure.txt
- docs/validation/evidence/VWO-004/media-editorial-publishing/user-path/20260912T003057Z-step05-published-story-screenshot.png
- docs/validation/evidence/VWO-004/media-editorial-publishing/user-path/20260912T003057Z-step06-st0205-no-hero-publish-blocked.txt
- docs/validation/evidence/VWO-004/media-editorial-publishing/user-path/20260912T003057Z-step07-teaching-surface-missing-note.md
- docs/validation/evidence/VWO-004/media-editorial-publishing/post-hoc/20260912T003057Z-events-and-story-state.txt
- docs/validation/evidence/VWO-004/media-editorial-publishing/post-hoc/20260912T003057Z-reset-verification.txt

## Failure / Recovery
- Failure injected: none via switch — this scenario's catalogued failure modes occur naturally (partner-app stale license on publish; st-0205 no-hero block) and both were exercised as natural gates
- Recovery attempted: verified the documented recovery postures — the failed channel job is observable per-channel (renew license → republish/correct is the documented path); the no-hero publish leaves the story approved so attaching a hero and re-publishing is possible; no state corruption from the partial success
- Restart/session-loss behavior: not attempted — not on this scenario's catalog path; persona sessions recreated with server state continuous
- Final outcome: passed for the business path — full lifecycle, self-approval block, hero gate, partial channel success with named cause and reset all verified; the DEMONSTRATE replay step is blocked (F1)

## Product / UX Friction
The story page narrates its own lifecycle (draft → in_review → published with per-channel status). The partial-success warning is exactly right for a normal person: it separates what shipped (3/4) from what failed and why (stale license on partner-app). Friction: none of substance observed on the business path.

## Security / Architecture Findings
- Finding: F1 (deduplicated; root cause identical to scenario block 1) — the DEMONSTRATE replay step could not be performed: no Codex Universal surface exists to observe the editorial gates (role split, hero gate, per-channel licenses) and infer the publishing workflow.
- Severity: P0
- Reproduction: docs/validation/evidence/VWO-004/_infra/post-hoc/20260912T003813Z-product-surface-absence-check.txt and docs/validation/evidence/VWO-004/media-editorial-publishing/user-path/20260912T003057Z-step07-teaching-surface-missing-note.md
- Root cause: workflow family ships as library crates only at this SHA (no product consumer)
- Frozen invariant affected: DEMONSTRATE-mode guarantee (VALIDATION-PROGRAM.md §6) is unverifiable — no observation surface exists

## Recommendation
- NEW WORK ORDER — F1 (P0): as in scenario block 1 (build the user-facing Codex Universal teaching surface; owner Tech Lead → workflow-app owner; verification = re-run teaching steps + re-validate report).

## Worker Conclusion
blocked — the publishing business path passed end-to-end (draft→hero→review→approval→multi-channel publication with a natural partial success and named causes, plus the no-hero gate), but the catalog's DEMONSTRATE teaching step could not be performed because the Codex Universal teaching surface does not exist at this SHA (F1, P0); recorded, not worked around.

## Scenario
- Scenario id: media-content-repurposing
- Industry: media
- Scenario: social repurposing with renditions, duplicate gate and corrections (PressRoom, port 4104)
- Teaching mode: HYBRID
- User goal: repurpose a published story for social — request a social-card rendition, schedule per-network posts, apply a correction and re-distribute — instructed first, with the duplicate-post exception demonstrated.

## User Path
1. Signed in as omar.bhatt (photo editor); opened /assets; requested a social-card rendition of the harbor-bridge image (a-0301) — the rendition became ready. (First form interaction landed on the wrong table row and rendered a thumbnail rendition instead; redone correctly — see Product / UX Friction.)
2. Signed in as nia.roberts (social distribution manager); opened /social; scheduled a linkup post for the published story st-0201 — the scheduled post sp-0103 is visible.
3. Published the post — published with engagement placeholders.
4. Scheduled a second post for the SAME story + network — natural duplicate rejections [duplicate_event]: feedbook and chirp are seeded duplicates and the live linkup re-schedule was rejected too; the first posts remain intact.
5. camille.dubois applied a correction to st-0201 with data_conflict armed — 409 flash; the clean retry applied the correction and re-distributed to 3 channels (story v4).
6. Switch-completion run (catalog-vs-fixture check): with missing_asset armed on a rendition request, the rendition for a-0302 SUCCEEDED — the fixture implements no missing_asset branch on renditions; the implemented rendition switch is data_conflict, which was then exercised — 409 "Rendition request conflicts with an in-flight job for a-0301 (simulated). [data_conflict]" → clean retry → social-card ready; a natural unresolvable-asset rendition was probed via the API twin (asset a-9999) → 404 asset_not_found naming the id (the UI combobox only offers existing assets). Mismatch recorded as F4.
7. Catalog step 6 (HYBRID instruct + reconciliation) not performed — teaching surface missing (see Actual).

## Expected
Renditions ready; posts scheduled and published; same story+network twice → 409 [duplicate_event] with the first post intact; correction bumps the story version and re-distributes; data_conflict on correction with stale expectedVersion fails closed with re-read-and-retry; missing_asset on rendition fails with the asset id.

## Actual
The repurposing path behaved as catalogued for every implemented behavior: the social-card rendition became ready; the linkup post was scheduled and published (engagement placeholders); the duplicate gate rejected same story+network re-posts naturally (seeded feedbook/chirp and the live linkup attempt) with the original posts intact; the correction's data_conflict failed closed and the clean retry produced story v4 with re-distribution to 3 channels. Deviations: (a) the catalog's missing_asset-on-rendition switch is NOT implemented by the fixture — with the switch armed the rendition succeeded, and the implemented rendition switch is data_conflict (in-flight job conflict) instead; the missing-asset intent is only reachable naturally via an unresolvable asset reference (API twin, 404 asset_not_found naming the id) (F4, P2, harness-level); (b) the HYBRID reconciliation step could not be performed — no Codex Universal surface exists (F1, P0).

## Workflow Identity
- Workflow: NONE — surface missing (no Codex Universal teaching surface exists at base 0d314fd; F1)
- Version: NONE — surface missing (F1)
- Source revision: NONE — surface missing (F1)
- Definition digest: NONE — surface missing (F1)
- Dependency-lock identity: NONE — surface missing (F1)

## Runtime Binding
- Capabilities: asset rendition:request exercised by omar.bhatt (photo editor); social:schedule + social:post exercised by nia.roberts (social distribution manager); correction exercised by camille.dubois (editor-in-chief); Codex Universal capability binding: NONE — surface missing (F1)
- Resources: PressRoom fixture at 127.0.0.1:4104; assets a-0301/a-0302 and renditions (thumbnail, social-card, hero); story st-0201 (published); social posts (feedbook/chirp seeded, linkup sp-0103 live); the distribution channels
- Environments: local sandbox fixture process (node server.js --port 4104); browser surface across three personas
- Approvals: distribution-manager posting gate exercised; editor correction gate exercised with optimistic-versioning conflict
- Triggers/schedules: per-network social-schedule semantics exercised; correction event trigger with re-distribution exercised; the taught workflow would carry both — no compile surface to verify against (F1)

## Evidence
- docs/validation/evidence/VWO-004/media-content-repurposing/user-path/20260912T003118Z-instruct-phase-instruction-text.md
- docs/validation/evidence/VWO-004/media-content-repurposing/user-path/20260912T003402Z-step01-socialcard-rendition-ready.txt
- docs/validation/evidence/VWO-004/media-content-repurposing/user-path/20260912T003402Z-step02-linkup-post-scheduled.txt
- docs/validation/evidence/VWO-004/media-content-repurposing/user-path/20260912T003402Z-step03-post-published-engagement.txt
- docs/validation/evidence/VWO-004/media-content-repurposing/user-path/20260912T003402Z-step04-natural-duplicate-chirp-rejected.txt
- docs/validation/evidence/VWO-004/media-content-repurposing/user-path/20260912T003402Z-step04-duplicate-linkup-post-rejected.txt
- docs/validation/evidence/VWO-004/media-content-repurposing/user-path/20260912T003402Z-switch-data-conflict-correction.txt
- docs/validation/evidence/VWO-004/media-content-repurposing/user-path/20260912T003402Z-step05-correction-v4-redistributed.txt
- docs/validation/evidence/VWO-004/media-content-repurposing/user-path/20260912T003402Z-demo-phase-reconciliation-surface-missin.md
- docs/validation/evidence/VWO-004/media-content-repurposing/user-path/20260912T065502Z-switch-missing-asset-not-applied-to-rend.txt
- docs/validation/evidence/VWO-004/media-content-repurposing/user-path/20260912T065509Z-switch-data-conflict-rendition-flash.txt
- docs/validation/evidence/VWO-004/media-content-repurposing/user-path/20260912T065516Z-switch-data-conflict-rendition-recovery.txt
- docs/validation/evidence/VWO-004/media-content-repurposing/user-path/20260912T065528Z-rendition-switch-mismatch-note.md
- docs/validation/evidence/VWO-004/media-content-repurposing/post-hoc/20260912T003402Z-events-and-social-state.txt
- docs/validation/evidence/VWO-004/media-content-repurposing/post-hoc/20260912T003402Z-reset-verification.txt
- docs/validation/evidence/VWO-004/media-content-repurposing/post-hoc/20260912T065544Z-switch-completion-events-and-state.txt
- docs/validation/evidence/VWO-004/media-content-repurposing/post-hoc/20260912T065603Z-switch-completion-reset-verification.txt

## Failure / Recovery
- Failure injected: data_conflict on the correction (409 fail-closed, clean retry → v4 + re-distribution to 3 channels); data_conflict on the rendition request (409 in-flight-job conflict naming a-0301, clean retry → social-card ready)
- Recovery attempted: re-read and retry after each conflict — both recoveries succeeded; duplicate re-posts needed no recovery (original posts intact); the missing_asset-on-rendition probe documented the fixture's actual behavior (switch not applied — F4) and the natural unresolvable-asset path was probed via the API twin (404 asset_not_found naming a-9999)
- Restart/session-loss behavior: not attempted — not on this scenario's catalog path; three persona sessions were recreated with server state continuous (media fixture restarted cleanly between the primary and switch-completion runs with reset verification on both)
- Final outcome: passed for every implemented behavior — rendition, scheduling, publication, duplicate gate, correction with re-distribution, rendition conflict gate and reset all verified; the catalog's rendition missing_asset switch is unimplemented (F4) and the HYBRID reconciliation step is blocked (F1)

## Product / UX Friction
The duplicate-post and correction flashes name the story, the network and the remediation. Real friction observed: the rendition request form lives inside the assets table and its per-row control is easy to mis-target — the validator's own first interaction landed on the wrong row and rendered a thumbnail rendition instead of the social card (recoverable: re-request on the correct row; the wrong rendition is itself a legitimate asset). A keyboard/assistive-technology user would hit the same row-scoping hazard. Minor: engagement placeholders show as 0/0/0 immediately after publish (cosmetic).

## Security / Architecture Findings
- Finding: F1 (deduplicated; root cause identical to scenario block 1) — the HYBRID instruct+reconcile step could not be performed: no Codex Universal surface exists to compile the repurposing policy and the demonstrated duplicate exception.
- Severity: P0
- Reproduction: docs/validation/evidence/VWO-004/_infra/post-hoc/20260912T003813Z-product-surface-absence-check.txt and docs/validation/evidence/VWO-004/media-content-repurposing/user-path/20260912T003402Z-demo-phase-reconciliation-surface-missin.md
- Root cause: workflow family ships as library crates only at this SHA (no product consumer)
- Frozen invariant affected: HYBRID reconciliation guarantee (VALIDATION-PROGRAM.md §6) is unverifiable — the compile surface does not exist
- Finding: F4 — catalog/fixture switch mismatch: SCENARIO-CATALOG lists missing_asset on rendition (referenced asset unresolvable); PressRoom's rendition request implements no missing_asset branch — with the switch armed the rendition succeeds. The implemented rendition switch is data_conflict (in-flight job conflict, 409 naming the asset id), and FAILURES.md binds missing_asset to attach/publish only. The missing-asset intent is only reachable naturally (unresolvable asset reference → 404 asset_not_found).
- Severity: P2
- Reproduction: docs/validation/evidence/VWO-004/media-content-repurposing/user-path/20260912T065502Z-switch-missing-asset-not-applied-to-rend.txt and docs/validation/evidence/VWO-004/media-content-repurposing/user-path/20260912T065528Z-rendition-switch-mismatch-note.md
- Root cause: catalog entry written against a switch binding the fixture does not implement on that operation
- Frozen invariant affected: none — harness-level mismatch; the fixture's own behaviors (data_conflict rendition gate, natural asset_not_found) are consistent and fail-closed

## Recommendation
- NEW WORK ORDER — F1 (P0): as in scenario block 1 (build the user-facing Codex Universal teaching surface; owner Tech Lead → workflow-app owner; verification = re-run teaching steps + re-validate report).
- FIX NOW — F4 (P2): align the catalog entry with the fixture — either implement the missing_asset branch on rendition requests in the media fixture (preferred: it is the catalogued user-meaningful failure) or rebind the catalog's rendition switch to data_conflict.
  - Proposed owner: validation harness maintainer (VWO-002/VWO-003 owner)
  - Verification required: verify-sweep.sh PASS + re-run media-content-repurposing with the rendition switch

## Worker Conclusion
blocked — the repurposing business path passed for every implemented behavior (rendition ready, post scheduled/published, duplicate gate, correction with re-distribution and conflict recovery, reset), but the catalog's rendition missing_asset switch is unimplemented by the fixture (F4, P2) and the HYBRID teaching/reconciliation step could not be performed because the Codex Universal teaching surface does not exist at this SHA (F1, P0); recorded, not worked around.

## Scenario
- Scenario id: media-breaking-news
- Industry: media
- Scenario: breaking-news fast path with correction loop (PressRoom, port 4104)
- Teaching mode: INSTRUCT
- User goal: the breaking-news fast path — a metro story goes from blank page to published in minutes with an expedited (but still real) approval, then a correction loop when facts change — described to Codex Universal in words; the system must infer the fast path WITHOUT weakening the gates.

## User Path
1. Signed in as hana.kim (writer); created a breaking draft ("Council votes 8–1 …" st-0206), attached the hero image, submitted for review.
2. Signed in as camille.dubois (editor-in-chief); observed the editor queue with the breaking story; approved via the single-editor expedited path; published to the fast channels — MISMATCH vs catalog: the fixture has no "push" channel, so the fast-path intent was preserved by publishing web-front-page + rss-feed and skipping the licensed partner-app and slow channels (F6) — the flash shows 2/2 channels delivered.
3. Observed published state and distribution jobs (screenshot captured).
4. Facts changed: applied a correction — story version bumped to v3 with re-distribution to 2 channels and a visible correction note.
5. Attempted the fast path WITHOUT a hero (st-0207) — publish blocked [missing_asset]; the approval gate was NOT weakened (the story still required hero + approval).
6. Switch-completion run: with notification_failure armed on publish — flash "Story st-0206 published (2/2 channels delivered)." plus warning "Published, but the author notification could not be delivered."; the author's /notifications shows the "Your story … is live" row with status failed (observable, not silent) and a notification.failed event appears. With data_conflict armed on the correction — 409 flash "Story st-0206 changed while the correction was being applied (simulated). [data_conflict]"; the clean retry applied the correction ("Correction c-800802 applied; story re-distributed to 2 channel(s)").
7. Catalog step 5 (INSTRUCT description → inferred fast path) not performed — teaching surface missing (see Actual).

## Expected
Expedited path visible; correction re-distributes with a visible correction note; hero gate still enforced on the fast path (breaking news is not exempt); data_conflict on correction fails closed with re-read + retry; notification_failure on publish leaves the story live with a failed notification row and a warning; the inferred workflow keeps the approval step (no silent gate removal).

## Actual
The fast path behaved as catalogued for every implemented behavior: the breaking draft went draft→in_review→approved (single editor)→published with 2/2 fast channels delivered; the correction bumped the story to v3 and re-distributed with a visible correction note; the no-hero fast path was blocked [missing_asset] with the approval gate intact; the notification_failure publish left the story live with the author's failed notification row observable plus a warning; the data_conflict correction failed closed and the clean retry completed ("Correction c-800802 applied; story re-distributed to 2 channel(s)"). Deviations: (a) the catalog's "push" fast channel does not exist in the fixture — the fast-path intent (web-first, skip licensed partner-app) was preserved with web-front-page + rss-feed (F6, P2, harness-level); (b) the INSTRUCT inference step could not be performed — no Codex Universal surface exists (F1, P0).

## Workflow Identity
- Workflow: NONE — surface missing (no Codex Universal teaching surface exists at base 0d314fd; F1)
- Version: NONE — surface missing (F1)
- Source revision: NONE — surface missing (F1)
- Definition digest: NONE — surface missing (F1)
- Dependency-lock identity: NONE — surface missing (F1)

## Runtime Binding
- Capabilities: story:create/edit exercised by hana.kim (writer); expedited single-editor approve + publish + correct exercised by camille.dubois (editor-in-chief); Codex Universal capability binding: NONE — surface missing (F1)
- Resources: PressRoom fixture at 127.0.0.1:4104; stories st-0206 (run), st-0207 (no-hero gate) and st-0201 (seed); hero asset a-0301; fast channels web-front-page + rss-feed; the author notification channel
- Environments: local sandbox fixture process (node server.js --port 4104); browser surface across two personas (primary run) and a replayed switch-completion run after reset
- Approvals: single-editor expedited approval exercised — real gate, one step; hero gate exercised on the fast path (not weakened)
- Triggers/schedules: breaking intake + correction-within-minutes semantics exercised; the taught workflow would carry the breaking trigger and the correction timer — no compile surface to verify against (F1)

## Evidence
- docs/validation/evidence/VWO-004/media-breaking-news/user-path/20260912T003420Z-instruct-phase-instruction-text.md
- docs/validation/evidence/VWO-004/media-breaking-news/user-path/20260912T003601Z-step01-breaking-draft-hero-attached.txt
- docs/validation/evidence/VWO-004/media-breaking-news/user-path/20260912T003601Z-step01-breaking-submitted-in-review.txt
- docs/validation/evidence/VWO-004/media-breaking-news/user-path/20260912T003601Z-step02-editor-queue-breaking-story.txt
- docs/validation/evidence/VWO-004/media-breaking-news/user-path/20260912T003501Z-step02-step02-fast-channel-selection-note.md
- docs/validation/evidence/VWO-004/media-breaking-news/user-path/20260912T003601Z-step02-single-editor-approval.txt
- docs/validation/evidence/VWO-004/media-breaking-news/user-path/20260912T003601Z-step03-published-fast-channels.txt
- docs/validation/evidence/VWO-004/media-breaking-news/user-path/20260912T003601Z-step03-published-screenshot.png
- docs/validation/evidence/VWO-004/media-breaking-news/user-path/20260912T003601Z-step04-correction-v3-redistributed.txt
- docs/validation/evidence/VWO-004/media-breaking-news/user-path/20260912T003601Z-step06-no-hero-fastpath-blocked.txt
- docs/validation/evidence/VWO-004/media-breaking-news/user-path/20260912T003601Z-step05-teaching-surface-missing-note.md
- docs/validation/evidence/VWO-004/media-breaking-news/user-path/20260912T065139Z-switch-notification-failure-publish-warn.txt
- docs/validation/evidence/VWO-004/media-breaking-news/user-path/20260912T065156Z-switch-notification-failed-row-author-vi.txt
- docs/validation/evidence/VWO-004/media-breaking-news/user-path/20260912T065210Z-switch-data-conflict-correction-flash.txt
- docs/validation/evidence/VWO-004/media-breaking-news/user-path/20260912T065227Z-switch-data-conflict-recovery-clean-retr.txt
- docs/validation/evidence/VWO-004/media-breaking-news/post-hoc/20260912T003601Z-events-and-story-state.txt
- docs/validation/evidence/VWO-004/media-breaking-news/post-hoc/20260912T003601Z-reset-verification.txt
- docs/validation/evidence/VWO-004/media-breaking-news/post-hoc/20260912T065340Z-switch-completion-events-and-state.txt
- docs/validation/evidence/VWO-004/media-breaking-news/post-hoc/20260912T065348Z-switch-completion-reset-verification.txt

## Failure / Recovery
- Failure injected: notification_failure on publish (story live, author notification row failed + warning — observable, not silent); data_conflict on correction (409 fail-closed; clean retry applied the correction and re-distributed)
- Recovery attempted: the author read /notifications where the failed row is visible (the warning directs there); the correction was re-applied after re-reading the story state — "Correction c-800802 applied; story re-distributed to 2 channel(s)"
- Restart/session-loss behavior: not attempted — not on this scenario's catalog path; the switch-completion run was a clean reset-and-replay of the full fast path (draft → approve → publish → correct) after which reset was verified again
- Final outcome: passed for every implemented behavior — expedited approval with intact gates, fast-channel publication, correction loop with re-distribution, no-hero block, observable notification failure, conflict recovery and reset (×2) all verified; the push channel does not exist (F6) and the INSTRUCT inference step is blocked (F1)

## Product / UX Friction
The expedited path is exactly as fast as catalogued — blank page to published in one editor action, with the correction note visible on the published story. The no-hero block message makes clear that breaking news is not exempt from the hero gate. Friction: a normal person told to "publish to web and push" will not find any push-like channel in the channel list — the fixture's channels are web-front-page, rss-feed, partner-app and the social networks; the user must improvise the fast-path channel selection themselves (the mismatch is recorded as F6; the validator preserved the intent by skipping the licensed and slow channels).

## Security / Architecture Findings
- Finding: F1 (deduplicated; root cause identical to scenario block 1) — the INSTRUCT inference step could not be performed: no Codex Universal surface exists to submit the breaking-policy description to, so the inferred fast path (expedited-but-real approval, correction timer) could not be evaluated.
- Severity: P0
- Reproduction: docs/validation/evidence/VWO-004/_infra/post-hoc/20260912T003813Z-product-surface-absence-check.txt and docs/validation/evidence/VWO-004/media-breaking-news/user-path/20260912T003601Z-step05-teaching-surface-missing-note.md
- Root cause: workflow family ships as library crates only at this SHA (no product consumer)
- Frozen invariant affected: INSTRUCT-mode guarantee (VALIDATION-PROGRAM.md §6) is unverifiable — no inference surface exists
- Finding: F6 — catalog/fixture channel mismatch: the catalog's fast channels are "web + push"; the PressRoom fixture has no push channel. The fast-path intent was preserved (web-front-page + rss-feed, skipping the licensed partner-app), and the approval/hero gates were demonstrably NOT weakened, but the catalogued channel set is not executable as written.
- Severity: P2
- Reproduction: docs/validation/evidence/VWO-004/media-breaking-news/user-path/20260912T003501Z-step02-step02-fast-channel-selection-note.md
- Root cause: catalog entry written against a channel set the fixture does not ship
- Frozen invariant affected: none — harness-level mismatch; the fixture's own gates behaved correctly and fail-closed

## Recommendation
- NEW WORK ORDER — F1 (P0): as in scenario block 1 (build the user-facing Codex Universal teaching surface; owner Tech Lead → workflow-app owner; verification = re-run teaching steps + re-validate report).
- FIX NOW — F6 (P2): align the catalog or the fixture — add a push-like fast channel to the media fixture (preferred) or rebind the catalog's fast-channel wording to the shipped web-front-page + rss-feed pair.
  - Proposed owner: validation harness maintainer (VWO-002/VWO-003 owner)
  - Verification required: verify-sweep.sh PASS + re-run media-breaking-news with the push channel

## Worker Conclusion
blocked — the breaking-news business path passed for every implemented behavior (expedited single-editor approval with intact hero gate, fast-channel publication, correction loop with re-distribution, observable notification failure and conflict recovery), but the catalog's push channel does not exist in the fixture (F6, P2) and the INSTRUCT teaching step could not be performed because the Codex Universal teaching surface does not exist at this SHA (F1, P0); recorded, not worked around.
