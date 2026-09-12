# VWO-007 — Persistence, Restart, Recovery, and Cancellation Adversary Report

## Identity
- Work Order: VWO-007 (wave-2 adversarial validator)
- Worker persona: persistence/restart/recovery/cancellation adversary — attacks durable workflow authority via crashes, restarts, duplicate events, trigger storms, provider failures, and cancellation races; reports truthfully
- Base branch: main
- Base SHA: 0d314fd09cec70d000f33c81286edcd8cbf72501
- Head SHA: see git rev-parse vwo-007/persistence-recovery-adversary (single commit on base 0d314fd)
- Validation environment: sandbox fallback proving ground per VALIDATION-PROGRAM.md — VWO-002 five-app fixture harness (SiteBuild 4101 construction, ForgeOps 4102 software, RidePilot 4103 rideshare, PressRoom 4104 media, FlowMart 4105 marketplace) driven via agent-browser 0.35.0, curl, and the node deploy-console.js terminal surface; codex workflow-family crates attacked as libraries via cargo test (rustc/cargo 1.95.0, pinned); no user-facing Codex runtime build was executed — the engine is library-only (Finding F1)
- Codex Universal runtime/application SHA: 0d314fd09cec70d000f33c81286edcd8cbf72501
- Date/time window: 2026-09-12T00:07:40Z to 2026-09-12T01:11:28Z (UTC)

## Scenario
- Scenario id: adv-session-loss-restart
- Industry: adversarial (proving ground: construction / SiteBuild Field Suite, port 4101)
- Scenario: kill the runtime mid-workflow, lose the session, restart without reseed — prove durable state, execution position, idempotency, and version pinning survive
- Teaching mode: DEMONSTRATE
- User goal: Marco Silva (foreman) runs the taught daily-progress workflow `construction-daily-progress`; the adversary interrupts it with SIGKILL after submission and with session loss, restarts the process without reseeding, and attempts duplicate re-fires and idempotency-key replays across the restart

## User Path
1. Signed in as marco.silva (foreman) on the SiteBuild login page (persona table visible), landed on the signed-in dashboard.
2. Opened /projects/p-101, filled the daily progress report form (task t-101c, 68% complete, photo a-101, notes "Formwork on level 13 complete; crane B certified."), and submitted it — the page confirmed "Progress report pr-4002 saved for Harborview Tower (68%)" with t-101c now done and the project at 68%.
3. Captured the pre-kill visible state (report saved flash, 68% progress, PM notification delivered to dana.reyes).
4. Killed the construction process with kill -9 (pid 13962, mid-session) and inspected durable state on disk after the SIGKILL (reports, events, notification rows).
5. Restarted the process WITHOUT reseed (restart transcript: pid 15052, port 4101, healthz ok, events 4, resets 1 — no state rebuilt).
6. Observed the session-lost anonymous view: the public project page still shows 68% and both report rows (pr-4001, pr-4002) — authority persists independent of the session.
7. Signed back in as marco.silva (fresh session) and observed the recovery surface: pr-4002 present, 68% intact, t-101c still done — no blank slate.
8. Re-fired the same daily trigger (same project + same date) — the page rejected it: "A progress report for p-101 on 2026-09-12 already exists — daily reports are one per project per day. [duplicate_event]".

## Expected
The submitted report survives the SIGKILL; restart without reseed preserves all durable state; session loss does not lose authority (state visible without a session, full surface after re-login); re-firing the same daily trigger does not duplicate (one report per project per day held, [duplicate_event] on the replay); an idempotency-key replay is rejected durably across restart with no second report; the event feed shows continuity across kill+restart; version pinning survives restart; the human sees recovery state, not a blank slate.

## Actual
Every attack was repelled. After kill -9, disk inspection showed both reports (pr-4001 60%, pr-4002 68%), p-101 percentComplete 68, 4 events (world.reset, auth.login, progress.submitted, notification.delivered), and both notification rows — nothing lost. The restart transcript confirms restart without reseed (healthz: events 4, resets 1). The anonymous session-lost view and the re-signed-in recovery surface both show 68% + pr-4002 + t-101c done. The same-day re-fire was rejected with the duplicate_event flash. The idempotency-key replay attack (API twin, POST /api/progress with idempotencyKey VWO007-KEY-1): first submit created pr-4003 (71%); an immediate replay returned HTTP 409 duplicate_event ("signature \"POST /api/progress:VWO007-KEY-1\" was first seen 2026-09-12T00:36:52.667Z"); after killing and restarting the process again, the SAME key replay still returned HTTP 409 with the IDENTICAL firstSeenAt, and reports on disk were exactly three (pr-4001, pr-4002, pr-4003) with p-101 at 71 — the dedup ledger itself is durable. Event continuity held: ev-0001 through ev-0009 unbroken across kill+restart+session-loss. Version pinning across restart held: the FlowMart install ins-0401 remained pinned to 2.3.1 through kill -9 + restart without reseed, and re-publishing version 2.3.1 was rejected as duplicate_event ("published versions are immutable") with digests unchanged. Engine-level parity for the same guarantees is proven by the library suites: codex-workflow-durable 46 tests, codex-workflow-app 36 tests, codex-workflow-triggers 23 tests, all passing at the base tree — including "the full control plane survives drop and reload", "positions survive drop and reload", "dedupe and settlements survive drop and reload", "reconcile startup transitions orphaned running records to paused" (idempotent sweep), and "tampered snapshots fail integrity on load". Reset verification at scenario end: seed state restored.

## Workflow Identity
- Workflow: construction-daily-progress (taught DEMONSTRATE workflow on the SiteBuild fixture); engine-side counterpart: codex-workflow-durable control plane — library-only, no product mount (Finding F1)
- Version: NONE — surface missing (no product surface exposes the workflow version; see Finding F1)
- Source revision: NONE — surface missing (engine crates tested at base SHA 0d314fd09cec70d000f33c81286edcd8cbf72501 as libraries; not user-addressable)
- Definition digest: NONE — surface missing (digests exist in the evidence store internally — evidence_store tests prove digest minting and reload stability — but no surface renders them)
- Dependency-lock identity: NONE — surface missing (Cargo.lock at the base tree is the only lock identity; not user-addressable)

## Runtime Binding
- Capabilities: browser (agent-browser 0.35.0), terminal (bash process control), file API (fixture state.json inspection)
- Resources: SiteBuild fixture app (port 4101), personas marco.silva (foreman, submitter) and dana.reyes (PM, notification recipient), project p-101, tasks t-101a..t-101d, photos a-101/a-102
- Environments: local sandbox fallback proving ground; fixture process lifecycle controlled by the adversary (kill -9 / restart, no reseed)
- Approvals: foreman-submitted daily report triggers PM notification (n-0001 delivered to dana.reyes) — the notification gate survived the SIGKILL and restart
- Triggers/schedules: daily one-report-per-project-per-day trigger idempotency (natural duplicate oracle) plus explicit idempotencyKey replay semantics across restart

## Evidence
- docs/validation/evidence/VWO-007/adv-session-loss-restart/user-path/20260912T003050Z-step01-login-page-persona-table.txt
- docs/validation/evidence/VWO-007/adv-session-loss-restart/user-path/20260912T003059Z-step02-signed-in-dashboard.txt
- docs/validation/evidence/VWO-007/adv-session-loss-restart/user-path/20260912T003225Z-step03-pre-kill-report-saved-68pct.txt
- docs/validation/evidence/VWO-007/adv-session-loss-restart/user-path/20260912T003240Z-step04-foreman-notifications-pre-kill.txt
- docs/validation/evidence/VWO-007/adv-session-loss-restart/user-path/20260912T003604Z-step05-session-lost-anonymous-view.txt
- docs/validation/evidence/VWO-007/adv-session-loss-restart/user-path/20260912T003618Z-step06-post-restart-recovery-surface-rep.txt
- docs/validation/evidence/VWO-007/adv-session-loss-restart/user-path/20260912T003636Z-step07-duplicate-replay-rejected-after-r.txt
- docs/validation/evidence/VWO-007/adv-session-loss-restart/post-hoc/20260912T003307Z-durable-state-after-sigkill.txt
- docs/validation/evidence/VWO-007/adv-session-loss-restart/post-hoc/20260912T003516Z-restart-transcript-no-reseed.txt
- docs/validation/evidence/VWO-007/adv-session-loss-restart/post-hoc/20260912T003652Z-idempotency-key-replay-attack-part1.txt
- docs/validation/evidence/VWO-007/adv-session-loss-restart/post-hoc/20260912T003708Z-idempotency-key-replay-attack-part2.txt
- docs/validation/evidence/VWO-007/adv-session-loss-restart/post-hoc/20260912T003722Z-event-continuity-and-reset-verification.txt
- docs/validation/evidence/VWO-007/_infra/post-hoc/20260912T005002Z-flowmart-pin-state-before-restart.txt
- docs/validation/evidence/VWO-007/_infra/post-hoc/20260912T005033Z-flowmart-pin-state-after-restart.txt
- docs/validation/evidence/VWO-007/_infra/post-hoc/20260912T002217Z-durable-library-test-results.txt
- docs/validation/evidence/VWO-007/_infra/post-hoc/20260912T002900Z-app-library-test-results.txt
- docs/validation/evidence/VWO-007/_infra/post-hoc/20260912T002913Z-triggers-library-test-results.txt
- docs/validation/evidence/VWO-007/_infra/post-hoc/20260912T000820Z-product-surface-dependency-probe.txt
- docs/validation/evidence/VWO-007/_infra/post-hoc/20260912T000831Z-product-surface-absence-probe.txt

## Failure / Recovery
- Failure injected: SIGKILL (kill -9) of the fixture process mid-session, twice (once after report submission, once during the idempotency-key replay attack); session loss (cookie invalidated by restart)
- Recovery attempted: process restart WITHOUT reseed; re-sign-in as the same user; duplicate trigger re-fire; idempotency-key replay before and after the second restart
- Restart/session-loss behavior: all durable state survived both SIGKILLs (reports, events, notifications, dedup ledger with identical firstSeenAt); session loss did not affect authority — anonymous view showed the persisted state and re-login restored the full surface
- Final outcome: passed — no lost transition, no duplicate report, no blank slate; recovery state fully human-observable

## Product / UX Friction
None observed on the fixture surface beyond Finding F1: the recovery surface shows intact data (68%, report rows, completed task) rather than a blank slate, and the duplicate re-fire returns an explicit, human-readable rejection. One observation (not a finding): there is no explicit "instance recovered/resumed" indicator — the human infers recovery from the intact data, which the catalog accepts ("resumes or explicitly reports a resumable state").

## Security / Architecture Findings
- Finding: F1 — the durable workflow authority (codex-workflow-durable, -app, -triggers, -contracts, -forge, -evolution, -distribution crates) is mounted in NO user-facing product surface: the CLI has no workflow subcommands, app-server-protocol exposes zero workflow methods, and the TUI has no workflow surface (reverse-dependency probe: each crate's only dependent is its own Cargo.toml). A normal person using Codex today cannot obtain or observe any of the persistence guarantees this WO verified — they exist only in fixture simulations and library code. The acceptance question ("can a normal person use Codex Universal end-to-end without understanding internals?") fails for durable workflows: the surface itself is absent, and every Workflow Identity field a person would need (version, definition digest, dependency lock) is unexposed.
- Severity: P1
- Reproduction: see _infra/post-hoc/product-surface-dependency-probe.txt and product-surface-absence-probe.txt — "codex-workflow-app is depended on by: ./workflow-app/Cargo.toml" (self only); "app-server-protocol workflow methods:" (empty result); TUI matches are snapshot/test-internal strings only.
- Root cause: the engine was built library-first; no mount/integration has landed in any user surface.
- Frozen invariant affected: durable workflow authority — persistence, recovery, and idempotency guarantees must be user-obtainable through a product surface, not library-internal.

## Recommendation
- NEW WORK ORDER — mount the workflow control plane behind one user surface (CLI subcommand or app-server method set) exposing at minimum: instance list with status/position, resume, cancel, and the Workflow Identity fields (version, definition digest, dependency-lock identity). Proposed owner: Tech Lead to assign the codex-rs workflow-family owner. Verification required: re-run VWO-007 scenarios against the mounted surface; confirm a person can observe instance recovery after a kill/restart without internals knowledge.

## Worker Conclusion
passed — every injected interruption (SIGKILL, session loss, restart without reseed, duplicate re-fire, idempotency-key replay across restart) was repelled by the durable authority with human-observable recovery state and durable dedup; the P1 gap is the missing product-surface mount of the engine (F1), not a durability defect.

## Scenario
- Scenario id: adv-duplicate-triggers
- Industry: adversarial (proving ground: rideshare / RidePilot, port 4103)
- Scenario: concurrent duplicate trigger storm on the public driver-onboarding intake — exactly one application may be accepted, the duplicate rejected as a replay
- Teaching mode: HYBRID
- User goal: public applicants fire the driver-onboarding workflow's intake trigger twice concurrently with the same payload (same plate ZZVWO-7701), then an 8-way parallel storm with an identical plate and an 8-way storm with distinct plates, then replays after kill+restart; Jules Moreau (ops) must see exactly one new application in the queue with no double screening work

## User Path
1. Opened the public /intake form (no auth required) and filled identical driver data (same name, phone, plate ZZVWO-7701, region Downtown Core) in two browser tabs.
2. Submitted both tabs as close to simultaneously as a human can — tab 1 confirmed "Application da-0304 submitted for Downtown Core. Screening usually completes within 2 business days."
3. Observed tab 2's rejection flash: "An active application for plate \"ZZVWO-7701\" already exists — duplicate intake is rejected. [duplicate_event]".
4. Signed in as jules.moreau and opened /onboarding — the queue shows exactly ONE new application (da-0301..da-0304; no double screening task).
5. Checked /notifications — a single application notification, no duplicates.

## Expected
Exactly one accepted application; one observable 409-style rejection naming the plate; the ops queue has a single new entry with no double screening work; the event feed shows the accept (and, per the catalog, the rejected replay); the dedup survives process restart (a post-restart replay of the same plate is still rejected); distinct-plate concurrent submissions are all accepted (no false-positive dedup).

## Actual
The human path held exactly: one acceptance (da-0304), one rejection flash, single queue entry, single notification. STORM A (8 parallel submissions, identical payload, plate ZZVWO-STORM-A): exactly 1×HTTP 200 (da-0305) and 7×HTTP 409 duplicate_event — no double accept. STORM B (8 parallel submissions, distinct plates): 8×HTTP 200 (da-0306..da-0313) — no false-positive dedup. Durable dedup across restart: after kill+restart of the fixture process, replaying storm plate ZZVWO-STORM-A and the human-path plate ZZVWO-7701 both returned HTTP 409 duplicate_event; the applications table held exactly 13 rows (da-0301..da-0313) — no extras, no losses. The event feed recorded 10 application_submitted events (the accepts only). One delta vs the catalog expectation: the REJECTED replays left no event in the audit feed — the rejection is observable only in the actor's HTTP 409 response at the moment of the attempt (finding F2). Reset verification at scenario end: seed state restored. Adversary probe iteration, recorded for honesty: two storm attempts before STORM A errored on my own payload mistakes (404 region_not_found using region code "DT"; 400 validation_error with a mis-named field) — product behaved correctly by rejecting malformed input; not product findings.

## Workflow Identity
- Workflow: rideshare-driver-onboarding (taught HYBRID workflow on the RidePilot fixture); engine-side counterpart: codex-workflow-triggers + codex-workflow-durable trigger_ledger — library-only, no product mount (Finding F1)
- Version: NONE — surface missing (see Finding F1)
- Source revision: NONE — surface missing (engine crates tested at base SHA 0d314fd09cec70d000f33c81286edcd8cbf72501 as libraries)
- Definition digest: NONE — surface missing
- Dependency-lock identity: NONE — surface missing

## Runtime Binding
- Capabilities: browser (two-tab concurrent submission), terminal (parallel curl storm via xargs/background jobs), process control (kill + restart)
- Resources: RidePilot fixture app (port 4103), public intake form (unauthenticated trigger), ops persona jules.moreau, regions rg-1/Downtown Core
- Environments: local sandbox fallback proving ground; fixture process lifecycle controlled by the adversary
- Approvals: not applicable on the public intake (no approval gate before screening); ops queue review is the human checkpoint
- Triggers/schedules: concurrent duplicate trigger dedup on the intake event (same plate = natural idempotency key); trigger acceptance + settlement semantics across restart (engine parity: trigger_ledger acceptance-dedupe and settlement survive drop and reload; duplicate acceptance in the journal is a deterministic error)

## Evidence
- docs/validation/evidence/VWO-007/adv-duplicate-triggers/user-path/20260912T003754Z-step01-intake-form-public.txt
- docs/validation/evidence/VWO-007/adv-duplicate-triggers/user-path/20260912T003817Z-step02-tab1-accepted-da-0304.txt
- docs/validation/evidence/VWO-007/adv-duplicate-triggers/user-path/20260912T003826Z-step03-tab2-duplicate-rejected.txt
- docs/validation/evidence/VWO-007/adv-duplicate-triggers/user-path/20260912T003841Z-step04-ops-queue-single-new-application.txt
- docs/validation/evidence/VWO-007/adv-duplicate-triggers/user-path/20260912T003905Z-step05-ops-notifications-single-applicat.txt
- docs/validation/evidence/VWO-007/adv-duplicate-triggers/post-hoc/20260912T003952Z-concurrent-storm-same-plate.txt
- docs/validation/evidence/VWO-007/adv-duplicate-triggers/post-hoc/20260912T004008Z-concurrent-storm-distinct-plates.txt
- docs/validation/evidence/VWO-007/adv-duplicate-triggers/post-hoc/20260912T004027Z-durable-dedup-across-restart.txt
- docs/validation/evidence/VWO-007/adv-duplicate-triggers/post-hoc/20260912T004039Z-event-feed-and-reset-verification.txt
- docs/validation/evidence/VWO-007/_infra/post-hoc/20260912T002217Z-durable-library-test-results.txt
- docs/validation/evidence/VWO-007/_infra/post-hoc/20260912T000820Z-product-surface-dependency-probe.txt

## Failure / Recovery
- Failure injected: concurrent duplicate triggers (two-tab same-payload submission; 8-way identical-plate storm), process kill + restart mid-scenario, post-restart replays of already-accepted plates
- Recovery attempted: post-restart replay of storm plate and human-path plate; state reconciliation check (applications must be exactly 13)
- Restart/session-loss behavior: the dedup ledger survived kill+restart — post-restart replays rejected with HTTP 409; no application lost or duplicated
- Final outcome: passed — exactly one accept per distinct payload across all concurrency levels and the restart; no double screening work created

## Product / UX Friction
The rejection flash is explicit and human-readable (names the plate and the duplicate_event reason). Minor friction: a rejected replay leaves no trace in the event feed, so an ops person auditing later cannot see that a duplicate was attempted (finding F2); the actor sees the 409 only at the moment of the attempt.

## Security / Architecture Findings
- Finding: F1 (same as scenario 1) — the trigger/dedup authority proven here (fixture-level natural dedup + engine trigger_ledger with 23 passing tests) is mounted in no user-facing product surface; a normal person cannot obtain these guarantees from Codex itself. Reproduction and root cause as recorded in scenario 1 (dependency/absence probes).
- Severity: P1
- Reproduction: docs/validation/evidence/VWO-007/_infra/post-hoc/20260912T000820Z-product-surface-dependency-probe.txt — workflow-triggers has no dependents beyond its own Cargo.toml.
- Root cause: engine built library-first; no mount has landed.
- Frozen invariant affected: trigger idempotency/authority must be user-obtainable, not library-internal.
- Finding: F2 — dedup rejections are invisible to the event audit trail: the catalog's expected observable outcome "events show both the accept and the rejected replay" is only half-delivered. The accept is recorded (10 application_submitted events); the rejected replays (two-tab duplicate, 7 storm rejections, post-restart replays) recorded nothing. A person auditing the workflow later cannot see that duplicates were attempted and repelled — they must trust the actor's transient 409 flash.
- Severity: P2
- Reproduction: post-hoc/20260912T004039Z-event-feed-and-reset-verification.txt — the feed lists only the accepts; zero rejection records. Same pattern in the construction event-continuity evidence (no rejection events for the idempotency-key replays).
- Root cause: the shared fixture runtime emits events only on successful mutations; the dedup path throws before any emit.
- Frozen invariant affected: audit observability of the dedup guarantee — the authority held, but the evidence of it being held is not durably recorded.
- Recommendation for F2: NEW WORK ORDER — emit a rejection audit event (for example application.duplicate_rejected) in the shared fixture runtime's dedup/conflict path. Proposed owner: validation harness owner (VWO-003 lineage). Verification required: re-run adv-duplicate-triggers and confirm the rejected replay appears in /api/events.

## Recommendation
- NEW WORK ORDER for F1 (mount the workflow control plane behind a user surface; owner: Tech Lead to assign the codex-rs workflow-family owner; verification: re-run VWO-007 scenarios against the mounted surface)
- NEW WORK ORDER for F2 (emit dedup-rejection audit events in the fixture runtime; owner: validation harness owner; verification: re-run adv-duplicate-triggers and observe the rejection in the event feed)

## Worker Conclusion
passed — concurrent duplicate triggers, trigger storms, and post-restart replays were all repelled with exactly one accept per distinct payload and no lost applications; the P1 engine-mount gap (F1) and the P2 audit-trail gap (F2) are recorded as findings, not durability defects.

## Scenario
- Scenario id: adv-provider-failure
- Industry: adversarial (proving ground: software / ForgeOps, port 4102)
- Scenario: provider death under the promote operation — CI runner pool 503 and release artifact 404 must fail safe with zero partial production state and support retry after provider recovery
- Teaching mode: INSTRUCT
- User goal: Raj Patel (release manager) executes the incident→promote workflow `software-incident-response` through the deploy-console.js terminal surface; the adversary kills the synthetic ci-runner-pool (503) and the artifact registry (404) under the promote step, verifies no partial production state appears, and retries after provider recovery — including one retry after killing and restarting the fixture process itself

## User Path
1. Signed in on the terminal console (login raj.patel) and listed deployments for repo r-1 (deployments r-1) — dep-1102 production 4.8.1, dep-1101 staging 4.8.1.
2. Armed the failure switch (switch service_unavailable) and ran promote dep-1101 — the console printed "FAILED 503: service_unavailable — The \"ci-runner-pool\" service is unavailable (simulated). State-changing requests are rejected; read requests still work."
3. Opened /deployments in the browser — no partial promotion appeared; r-1 production still serves dep-1102 4.8.1.
4. Retried the promote WITHOUT the switch — success: r-1 promoted, production version bumped.
5. Approved and merged a codeowner PR to mint a fresh staging deployment (dep-1104, payments-service 4.8.2), then armed service_unavailable and ran promote dep-1104 — FAILED 503 naming ci-runner-pool; the browser showed zero partial promotion (r-1 production still 4.8.1).
6. Retried without the switch — OK: payments-service 4.8.2 promoted to production (dep-1105); version bump observable.
7. Minted another staging deployment (dep-1106, web-checkout 14.3.1, artifact art-0903), armed missing_asset, and ran promote dep-1106 — FAILED 404: "Release artifact \"art-0903\" could not be resolved in the artifact registry (simulated dangling reference)"; the browser confirmed fail-safe (r-2 production still dep-1103 14.3.0).
8. Killed and restarted the fixture process (provider + process both recovered), then retried promote dep-1106 — OK: web-checkout 14.3.1 promoted to production (deployment dep-1107).

## Expected
Provider failures produce named, actionable errors (dependency named: ci-runner-pool for 503, artifact id art-0903 for 404); production state is untouched on failure (no partial promotion, old version still served); retry after provider recovery succeeds and bumps the version; the workflow records failure and recovery (events); the human can tell WHAT failed and WHAT to do; the same fail-safe holds when the process itself is killed and restarted between failure and retry.

## Actual
All fail-safe semantics held. ATTACK A (503 under promote dep-1101): named ci-runner-pool failure, reads still worked (deployments listing succeeded), zero partial state in the browser, plain retry succeeded. ATTACK B (503 under promote dep-1104 for payments-service 4.8.2): FAILED 503, browser showed r-1 production still 4.8.1 (no partial), retry succeeded — dep-1105 production 4.8.2, version bump 4.8.1→4.8.2. ATTACK C (404 under promote dep-1106 for web-checkout 14.3.1): FAILED 404 naming artifact art-0903, browser showed r-2 production still dep-1103 14.3.0, then kill+restart of the fixture process followed by a successful retry — dep-1107 production 14.3.1. Every failure named the failed dependency and the retry path; no half-promoted state appeared at any point. The event feed recorded the deployment lifecycle (deployment events for the successful promotes) and the reset verification restored seed state.

## Workflow Identity
- Workflow: software-incident-response (taught INSTRUCT workflow on the ForgeOps fixture, terminal surface deploy-console.js); engine-side counterpart: codex-workflow-app run/lifecycle fail-safe seams — library-only, no product mount (Finding F1)
- Version: NONE — surface missing (see Finding F1)
- Source revision: NONE — surface missing (engine crates tested at base SHA 0d314fd09cec70d000f33c81286edcd8cbf72501 as libraries)
- Definition digest: NONE — surface missing
- Dependency-lock identity: NONE — surface missing

## Runtime Binding
- Capabilities: terminal (node deploy-console.js, the desktop-target console), browser (deployment console page verification), process control (kill + restart of port 4102)
- Resources: ForgeOps fixture app (port 4102), persona raj.patel (release_manager) plus codeowner approvers, repos r-1 (payments-service) and r-2 (web-checkout), deployments dep-1101..dep-1107, artifacts art-0901..art-0903
- Environments: local sandbox fallback proving ground; synthetic providers ci-runner-pool and the artifact registry, killed via the documented failure switches
- Approvals: promote is a human approval gate (release manager); the approval survived provider failure — the gate is re-armed by the plain retry, and the codeowner PR approval gate (approve + merge) minted the fresh staging deployments
- Triggers/schedules: promote dispatch under provider failure; retry-after-recovery semantics; no double promotion after the process restart (the retry created exactly one production deployment)

## Evidence
- docs/validation/evidence/VWO-007/adv-provider-failure/user-path/20260912T004104Z-step02-console-login-deployments.txt
- docs/validation/evidence/VWO-007/adv-provider-failure/user-path/20260912T004123Z-step03-browser-deployments-pre-attack.txt
- docs/validation/evidence/VWO-007/adv-provider-failure/user-path/20260912T004129Z-step04-console-promote-service-unavailab.txt
- docs/validation/evidence/VWO-007/adv-provider-failure/user-path/20260912T004140Z-step05-browser-no-partial-state-after-50.txt
- docs/validation/evidence/VWO-007/adv-provider-failure/user-path/20260912T004146Z-step06-console-retry-promote-success.txt
- docs/validation/evidence/VWO-007/adv-provider-failure/user-path/20260912T004237Z-step07-codeowner-approves-pr.txt
- docs/validation/evidence/VWO-007/adv-provider-failure/user-path/20260912T004251Z-step08-release-manager-merges-staging-de.txt
- docs/validation/evidence/VWO-007/adv-provider-failure/user-path/20260912T004309Z-step09-promote-dep1104-service-unavailab.txt
- docs/validation/evidence/VWO-007/adv-provider-failure/user-path/20260912T004315Z-step10-browser-fail-safe-no-partial-prom.txt
- docs/validation/evidence/VWO-007/adv-provider-failure/user-path/20260912T004321Z-step11-retry-promote-success-version-bum.txt
- docs/validation/evidence/VWO-007/adv-provider-failure/user-path/20260912T004402Z-step14-promote-missing-asset-404.txt
- docs/validation/evidence/VWO-007/adv-provider-failure/user-path/20260912T004407Z-step15-browser-fail-safe-after-404.txt
- docs/validation/evidence/VWO-007/adv-provider-failure/user-path/20260912T004421Z-step16-retry-after-kill-restart-success.txt
- docs/validation/evidence/VWO-007/adv-provider-failure/post-hoc/20260912T004427Z-events-and-reset-verification.txt
- docs/validation/evidence/VWO-007/_infra/post-hoc/20260912T002900Z-app-library-test-results.txt
- docs/validation/evidence/VWO-007/_infra/post-hoc/20260912T000820Z-product-surface-dependency-probe.txt

## Failure / Recovery
- Failure injected: synthetic provider deaths under the promote operation — ci-runner-pool 503 (twice) and artifact registry 404 (once); plus a process kill + restart between the 404 failure and its retry
- Recovery attempted: plain retry after provider restoration (the switch is per-request, so the retry IS the recovery); retry after kill+restart of the fixture process
- Restart/session-loss behavior: the process restart lost no deployment state — the staging deployment dep-1106 and the production state dep-1103 were intact, and the retry promoted exactly once (dep-1107)
- Final outcome: passed — zero partial production state across all three attacks; every failure named its dependency; every retry succeeded with an observable version bump

## Product / UX Friction
None observed: the failure output names the dependency (ci-runner-pool / art-0903), states that reads still work, and the successful retries print the promoted version and deployment id. The console's per-request switch arming is explained in-app. The only gap is the engine mount (Finding F1).

## Security / Architecture Findings
- Finding: F1 (same as scenario 1) — the fail-safe promote semantics proven here (and the engine parity: codex-workflow-app lifecycle/run seams enforce declared status-pair transitions; cancelling an unknown instance is an availability error, not a silent success) are mounted in no user-facing product surface.
- Severity: P1
- Reproduction: docs/validation/evidence/VWO-007/_infra/post-hoc/20260912T000820Z-product-surface-dependency-probe.txt — workflow-app has no dependents beyond its own Cargo.toml.
- Root cause: engine built library-first; no mount has landed.
- Frozen invariant affected: fail-safe provider semantics must be user-obtainable through a product surface.

## Recommendation
- NEW WORK ORDER — mount the workflow control plane behind one user surface (as detailed in scenario 1's recommendation). Proposed owner: Tech Lead to assign the codex-rs workflow-family owner. Verification required: re-run the provider-failure attack against the mounted surface and confirm a person sees named provider failures, zero partial state, and successful retry without internals knowledge.

## Worker Conclusion
passed — all three provider-death attacks failed safe with named errors, zero partial production state, and successful retries (including after a process kill+restart); the P1 engine-mount gap (F1) is the only finding.

## Scenario
- Scenario id: adv-cancellation-race
- Industry: adversarial (proving ground: media / PressRoom, port 4104)
- Scenario: cancel/alter a story at the same moment an editor publishes it — exactly one terminal transition wins, the loser fails closed, no half-published state, published version immutable afterwards
- Teaching mode: DEMONSTRATE
- User goal: Hana Kim (writer) holds a stale edit form (rendered at v2 while the story was in_review) while Camille Dubois (editor-in-chief) approves and publishes the same story; the writer submits her edit AFTER publication (expect [data_conflict]); a second publish attempt is rejected ([duplicate_event]); the editor then applies a correction — the legitimate post-publication path — with a version bump and re-distribution

## User Path
1. Signed in as hana.kim (writer), opened /stories/st-0202 (in_review, v2), attached the hero image (a-0302, budget-hearing-chamber.jpg — required to publish), and filled her edit into the form (adding "Writer urgent addendum: council sources now dispute the police overtime figure.") WITHOUT submitting — the held-stale-form setup.
2. Signed out, signed in as camille.dubois (editor-in-chief), and approved the story — "Story st-0202 approved for publication." (still v2).
3. Published to web-front-page — "Story st-0202 published (1/1 channels delivered)." — the story is now v3 published with distribution job dj-0604 sent.
4. Signed back in as hana.kim and opened the published story — the Edit form NO LONGER RENDERS (fail-closed UI: editing is only offered for draft/changes_requested/in_review); her held stale form (expectedVersion 2) was then replayed through the form's documented API twin (POST /api/stories/edit) — rejected: HTTP 409 data_conflict, "Version mismatch on story st-0202: you worked from v2 but the current version is v3 — reload and reapply."
5. Attempted to publish the same story a second time (as camille.dubois, via the API twin since the Publish form no longer renders for a published story) — rejected: HTTP 409 duplicate_event, "Story st-0202 is already published — use a correction instead."; story state unchanged (published, v3, addendum absent).
6. Applied a correction as the editor through the browser form (summary "Corrects the police-overtime line after the fiscal office reissuance.", replacing "a contested police overtime line" with "a police overtime line revised after reissuance") — "Correction c-800802 applied; story re-distributed to 1 channel(s)." — the story is now v4 corrected with a new distribution job dj-0605 and the correction recorded in the Corrections table.

## Expected
One winner per transition (the editor's publish wins; the writer's stale edit loses); the loser gets an explicit conflict naming the version movement; the published body is unchanged by the losing edit; a second publish fails closed with [duplicate_event]; no mixed or half-published state; the published content is immutable except via the governed correction path, which bumps the version and re-distributes; events record story.approved, story.published, correction.applied, and the distributions.

## Actual
Exactly one winner per transition. The stale edit was rejected with data_conflict naming both versions ("you worked from v2 but the current version is v3 — reload and reapply") and the published body was untouched (integrity check: "writer held-edit landed: False"). The duplicate publish was rejected with duplicate_event pointing to the governed alternative ("use a correction instead") and the story remained published v3 with no re-distribution. The correction path worked as designed: c-800802 recorded, version bumped to v4, status corrected, re-distribution job dj-0605 sent, and only the beforeText→afterText replacement changed the body ("correction landed: True"). The UI itself fails closed — neither the Edit form nor the Publish form renders once the story is published, so a person cannot even attempt the race through the visible surface; the API twin (which the form help text documents) still enforces the same guards. Event feed continuity: ev-0001..ev-0017 unbroken — world.reset, logins, story.asset_attached, story.approved, notification, distribution.sent (dj-0604), story.published, notification, more logins, distribution.sent (dj-0605, the re-distribution), correction.applied, notification — exactly one of each terminal transition. Reset verification at scenario end: seed state restored. Note on evidence: this scenario was executed twice (a first complete pass, then a definitive re-run after an environment interruption); the pointers below reference the definitive chain, which includes every catalog step.

## Workflow Identity
- Workflow: media-editorial-publishing (taught DEMONSTRATE workflow on the PressRoom fixture); engine-side counterpart: codex-workflow-app resume/cancel seams (cancel discards the persisted position; resume refuses while an active run is held) — library-only, no product mount (Finding F1)
- Version: NONE — surface missing (see Finding F1)
- Source revision: NONE — surface missing (engine crates tested at base SHA 0d314fd09cec70d000f33c81286edcd8cbf72501 as libraries)
- Definition digest: NONE — surface missing
- Dependency-lock identity: NONE — surface missing

## Runtime Binding
- Capabilities: browser (agent-browser — two-persona session switching, form holds), API twin (curl, replaying the held stale form and the duplicate publish)
- Resources: PressRoom fixture app (port 4104), personas hana.kim (staff_writer, author of st-0202) and camille.dubois (editor_in_chief), story st-0202, hero asset a-0302, channel web-front-page
- Environments: local sandbox fallback proving ground; the "race" is staged by holding the writer's form across the editor's terminal transitions
- Approvals: editor-in-chief approval gate (story.approved) before the publish gate (story:publish permission); the correction gate (correction:apply, editors only) is the governed post-publication path
- Triggers/schedules: publication terminality (duplicate publish = duplicate_event); cancellation-vs-action race semantics via optimistic concurrency (expectedVersion); correction as the governed evolution path with re-distribution

## Evidence
- docs/validation/evidence/VWO-007/adv-cancellation-race/user-path/20260912T010832Z-step01-writer-hana-holds-edit-v2-in-revi.txt
- docs/validation/evidence/VWO-007/adv-cancellation-race/user-path/20260912T010902Z-step02-editor-camille-approves-v2.txt
- docs/validation/evidence/VWO-007/adv-cancellation-race/user-path/20260912T010910Z-step03-editor-publishes-v3-distribution.txt
- docs/validation/evidence/VWO-007/adv-cancellation-race/user-path/20260912T010953Z-step04a-writer-view-no-edit-form-publish.txt
- docs/validation/evidence/VWO-007/adv-cancellation-race/user-path/20260912T011038Z-step04b-writer-stale-edit-rejected-data-.txt
- docs/validation/evidence/VWO-007/adv-cancellation-race/user-path/20260912T011049Z-step05-duplicate-publish-rejected.txt
- docs/validation/evidence/VWO-007/adv-cancellation-race/user-path/20260912T011111Z-step06-editor-applies-correction-v4.txt
- docs/validation/evidence/VWO-007/adv-cancellation-race/post-hoc/20260912T011123Z-body-integrity-and-correction-record.txt
- docs/validation/evidence/VWO-007/adv-cancellation-race/post-hoc/20260912T011128Z-events-and-reset-verification.txt
- docs/validation/evidence/VWO-007/_infra/post-hoc/20260912T002900Z-app-library-test-results.txt
- docs/validation/evidence/VWO-007/_infra/post-hoc/20260912T000820Z-product-surface-dependency-probe.txt

## Failure / Recovery
- Failure injected: cancellation/alteration race — the writer's held stale edit submitted AFTER the editor's publish (optimistic-concurrency conflict); duplicate publish on the terminal state
- Recovery attempted: the writer's documented recovery is "reload and reapply" (the rejection message says so); the editor's governed evolution path (correction) was exercised successfully
- Restart/session-loss behavior: not applicable in this scenario (no process kill — the race is a logical cancellation race, not a crash); session switching between personas was performed via sign-out/sign-in with no state loss
- Final outcome: passed — exactly one terminal transition won; the loser failed closed with an explicit version-movement conflict; the published body was immutable except via the correction path; no mixed state at any point

## Product / UX Friction
None observed: the conflict message tells the loser exactly what to do ("reload and reapply"), the duplicate-publish message points to the governed alternative ("use a correction instead"), and the correction form re-distributes automatically. The fail-closed UI (edit/publish forms disappear once published) prevents a person from even attempting the race through the visible surface — the API twin still enforces the guards for programmatic actors.

## Security / Architecture Findings
- Finding: F1 (same as scenario 1) — the terminality/cancellation semantics proven here (and the engine parity: cancel discards the persisted position; resume refuses while an active run is held; the seam enforces the declared status-pair table so no undeclared transition exists) are mounted in no user-facing product surface.
- Severity: P1
- Reproduction: docs/validation/evidence/VWO-007/_infra/post-hoc/20260912T000820Z-product-surface-dependency-probe.txt — workflow-app/workflow-durable have no dependents beyond their own Cargo.toml.
- Root cause: engine built library-first; no mount has landed.
- Frozen invariant affected: cancellation/terminality authority must be user-obtainable through a product surface.

## Recommendation
- NEW WORK ORDER — mount the workflow control plane behind one user surface (as detailed in scenario 1's recommendation). Proposed owner: Tech Lead to assign the codex-rs workflow-family owner. Verification required: re-run the cancellation-race attack against the mounted surface; confirm a person sees exactly one winner, an explicit conflict for the loser, and the governed correction path.

## Worker Conclusion
passed — the cancellation race produced exactly one winner per transition with an explicit data_conflict for the loser, an immutable published body (held edit never landed), a duplicate_event on the second publish, and a working governed correction path (v4, re-distribution); the P1 engine-mount gap (F1) is the only finding.
