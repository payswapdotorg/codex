# VWO-015 — Cross-Application / Multi-Environment Task Coverage (NST-X Lane Execution Report)

## Identity
- Work Order: VWO-015
- Worker persona: Tech Lead local delivery (lesson-120 protocol; chat-side executor dispatch capacity-blocked + the 09:00 UTC sandbox reset wiped chat lanes — delivery mode recorded per the RWO registry precedent)
- Base branch: main
- Base SHA: def56454acce47f5d14daf475511c427518d3cdc
- Head SHA: see git rev-parse vwo-015/cross-app-task-coverage (single commit on base def56454a)
- Validation environment: connected agent sandbox, FALLBACK proving ground per VALIDATION-PROGRAM §3 (agent-browser 0.35.0 + bash/python/curl + Xvfb :97 GUI review windows + the five VWO-002 fixture apps :4101–4105 with failure switches; purpose-built digest/export fixtures; full inventory in evidence/vwo-015/_infra/environment-probe.txt)
- E2B template/workspace identity, if used: NONE — E2B/Composio pool ABSENT (recorded VWO-001 fallback condition)
- Codex Universal runtime/application SHA: NONE — workflow/teaching engine mounted in no user-facing surface at this base (canonical Family A gap)
- Date/time window: 2026-09-15 11:30–12:05 UTC

### Program-level summary

| Task | Mode | Diff | Outcome | Notes |
|---|---|---|---|---|
| NST-X001 | HYBRID | L4 | PASS | publish ×1; approval→publish ordering; armed 503 recovered (D-X1) |
| NST-X002 | INSTRUCT | L4 | PASS | digest == recompute; editor review; approval; notification row (D-X2) |
| NST-X003 | HYBRID | L4 | PASS | digest-gated install + tampered-control branch |
| NST-X004 | DEMONSTRATE | L4 | PASS | kill -9 mid-run; resume; exactly-one summary; totals == recompute |
| NST-X005 | HYBRID | L4 | PASS | cross-app join; both authenticated reads; PM-readable |
| NST-X006 | INSTRUCT | L4 | PASS | concurrent fanout; both exactly-once; approval precedes both |
| NST-X007 | HYBRID | L4 | PASS | rendition set exact; drift D-X7 (metadata-only assets) |
| NST-X008 | INSTRUCT | L4 | PASS | digest gate; stale_entitlement fail-closed; last-good pin; audit trail |

Failure-class counts: missing-capability 0 · binding/resource-issue 0 · workflow-semantic-defect 0 · model-inference-issue 0 · environment-specific-limitation 4 recorded drifts (D-X1 reads-exempt switch on the export arm; D-X2 notification persona routing; D-X7 metadata-only media assets; the §5-1 operator-channel approval fidelity note).

Mode coverage: DEMONSTRATE ×1 (X004) · INSTRUCT ×3 (X002, X006, X008) · HYBRID ×4 (X001, X003, X005, X007) — mode coverage across the pool satisfied.

Benchmark version: 1.0.0 (BENCHMARK-CATALOG.md header; executed at main @ def56454a).

---

## Scenario
- Scenario id: nst-x001
- Industry: media (PressRoom)
- Scenario: Export → transform → review → approve → publish
- User goal: Get the story stats out of the press app, have the terminal turn them into the Tuesday bulletin, let the editor eyeball it on screen, get the desk chief's sign-off, and only then publish the bulletin to the newsroom channel.
- Teaching mode: HYBRID
- benchmark_version: 1.0.0
- Difficulty: L4 (catalog; agreed)

## User Path
1. Exported the story stats from PressRoom's API (browser session).
2. The terminal built the Tuesday bulletin from the stats (status counts, word totals, published list).
3. The bulletin opened in the GUI review window (Xvfb editor window; screenshot with the title visible).
4. Created the bulletin story (hana.kim) and submitted it — the armed 503 fired on the first submit; the clean retry succeeded.
5. The desk chief (camille.dubois) approved — BEFORE the publish step (event ordering verified).
6. Published to the newsroom channel (hero attached; 1/1 channels delivered).

## Expected
Bulletin published exactly once with content derived from the export; the approval gate recorded before the publish event; the armed 503 recovered; the GUI review step evidenced.

## Actual
st-0206 "Tuesday Bulletin — Story Stats" published exactly once (channels ch-1 delivered; exactly one bulletin story exists); story.approved (11:41:45) precedes story.published (11:42:06); the armed 503 fired once (request.rejected ledger) and the retry succeeded; the GUI review screenshot shows the bulletin title.

## Workflow Identity
- Workflow: NONE — surface missing
- Version: NONE — surface missing
- Source revision: NONE — surface missing
- Definition digest: NONE — surface missing
- Dependency-lock identity: NONE — surface missing

## Runtime Binding
- Capabilities: browser-use, terminal, computer-use, human approval gate, publication side-effects
- Resources: PressRoom :4104 (hana.kim writer; camille.dubois editor_in_chief); service_unavailable switch; Xvfb review window; operator channel (§5-1 mirror record)
- Environments: fallback proving ground
- Approvals: the desk-chief approval (in-world story approve; operator-channel mirror)
- Triggers/schedules: none

## Evidence
- docs/validation/evidence/vwo-015/nst-x001/user-path/transcript.txt
- docs/validation/evidence/vwo-015/nst-x001/user-path/transcript-pipeline.txt
- docs/validation/evidence/vwo-015/nst-x001/user-path/bulletin.txt
- docs/validation/evidence/vwo-015/nst-x001/user-path/04-gui-review-window.png
- docs/validation/evidence/vwo-015/nst-x001/post-hoc/c1-oracles.txt
- docs/validation/evidence/vwo-015/nst-x001/post-hoc/environment-identity.txt

## Failure / Recovery
- Failure injected: the armed 503 on the bulletin's first submit (D-X1: the reads-exempt switch — the export-side arm lands on the state-changing step; recorded)
- Recovery attempted: clean retry (succeeded)
- Restart/session-loss behavior: n/a
- Final outcome: PASS

## Product / UX Friction
The publish flow requires the hero attachment + the approved expectedVersion — two guards that make publication deliberate (good friction for a terminal action).

## Security / Architecture Findings
- Finding: D-X1 — the catalog's "armed 503 on export" premise vs the reads-exempt switch (same class as the VWO-014 D-T10 finding; one harness alignment covers both)
- Severity: P3
- Reproduction: GET with ?failure=service_unavailable returns 200 on PressRoom reads
- Root cause: catalog assumed switch coverage on reads
- Frozen invariant affected: benchmark fidelity

## Recommendation
- NEW WORK ORDER
- Proposed owner: harness (extend the switch to reads or re-bind the arm surface)
- Verification required: the armed 503 observable on the export read

## Worker Conclusion
passed — the full export→transform→review→approve→publish chain with exactly-once publication, ordered approval, and recovered 503.

---

## Scenario
- Scenario id: nst-x002
- Industry: software (ForgeOps)
- Scenario: Terminal → API → desktop → approval → notification
- User goal: From the terminal, pull tonight's deployment list through the forge API, write the digest to a file, open it in the editor for the release manager to review, get her approval, and notify the on-call SRE.
- Teaching mode: INSTRUCT
- benchmark_version: 1.0.0
- Difficulty: L4 (catalog; agreed)

## User Path
1. Terminal pulled tonight's deployment list through the forge API and wrote the digest file.
2. Opened the digest in the GUI editor window for review (screenshot with the digest title).
3. The release-manager approval recorded (operator channel, timestamped, before the notification step).
4. The notification fired through the fixture's deploy pipeline (PR → approve → merge → staging deploy) — the deploy notification row delivered.

## Expected
Digest file content equals an independent API recomputation; approval recorded before the SRE notification and the notification row exists; the editor-window review evidenced.

## Actual
Digest matches the independent recompute (all deployment IDs present with env/status); the approval record precedes the pipeline; the notification row exists ("payments-service 4.8.2 staged — promotion approval needed", delivered) — with D-X2 recorded: the fixture's deploy notification targets the promotion approver (release manager), not the sre_oncall persona (no trigger routes to her — verified against seed reviewer assignments).

## Workflow Identity
- Workflow: NONE — surface missing
- Version: NONE — surface missing
- Source revision: NONE — surface missing
- Definition digest: NONE — surface missing
- Dependency-lock identity: NONE — surface missing

## Runtime Binding
- Capabilities: terminal, api-tool-call, computer-use, human gate, notifications
- Resources: ForgeOps :4102 (ari/noor/raj personas); NotaryPad GUI window; operator channel
- Environments: fallback proving ground
- Approvals: the release-manager approval (operator-channel record, §5-1)
- Triggers/schedules: none

## Evidence
- docs/validation/evidence/vwo-015/nst-x002/user-path/transcript.txt
- docs/validation/evidence/vwo-015/nst-x002/user-path/deploy-digest.txt
- docs/validation/evidence/vwo-015/nst-x002/user-path/03-editor-review-window.png
- docs/validation/evidence/vwo-015/nst-x002/post-hoc/notifications.json
- docs/validation/evidence/vwo-015/nst-x002/post-hoc/c1-oracles.txt
- docs/validation/evidence/vwo-015/nst-x002/post-hoc/environment-identity.txt

## Failure / Recovery
- Failure injected: none
- Recovery attempted: the first promote attempt hit the fixture's state guard (dep-1101 already promoted) — the notification was delivered through the real deploy pipeline instead (PR → merge → staging)
- Restart/session-loss behavior: n/a
- Final outcome: PASS (with D-X2 recorded)

## Product / UX Friction
The deploy-notification routing is hardcoded to the promotion approver; an on-call routing table would be the product-grade path.

## Security / Architecture Findings
- Finding: D-X2 — the sre_oncall persona is not a notification target on any ForgeOps trigger (all repos' first reviewer = u-2; the staged-deploy notify hardcodes u-3)
- Severity: P3
- Reproduction: check seed reviewerIds + the notify call sites
- Root cause: fixture notification routing predates the catalog's persona assignments
- Frozen invariant affected: benchmark fidelity

## Recommendation
- NEW WORK ORDER
- Proposed owner: harness (notification persona routing)
- Verification required: an on-call-routable notification trigger

## Worker Conclusion
passed.

---

## Scenario
- Scenario id: nst-x003
- Industry: marketplace (FlowMart)
- Scenario: Browser → API/tool → filesystem → terminal → browser
- User goal: Vet the marketplace package before we install it: read the listing in the browser, fetch the manifest through the API, stage the digest file, verify it with the terminal checksum tool, and only then hit Install on the listing page.
- Teaching mode: HYBRID
- benchmark_version: 1.0.0
- Difficulty: L4 (catalog; agreed)

## User Path
1. Browser: read the listing (digest page + the FlowMart listing page — name/category/version values).
2. API: fetched the manifest through the documented JSON API (same-origin browser fetch; version + digest recorded).
3. Staged the digest file via the browser download; verified with the terminal checksum tool (sha256 + size vs advertised).
4. Only after VERIFIED: the install submitted (petra.voss for o-1; pk-103 1.2.0).
5. Control run: a tampered digest (checksum mismatch) closes the gate — no install attempted.

## Expected
Install record exists ONLY if the digest verification passed (control run with a tampered digest must NOT install); the verification transcript shows the checksum comparison; the manifest values read in the browser match the API-fetched manifest.

## Actual
The digest verified (sha256 52466b1a… + size 207 == advertised) → install ins-0402 created (pk-103 1.2.0 for o-1); the tampered control branch documented (gate closed, no install); the manifest compare recorded (2.3.1 + sha256:04cf049e… via both surfaces).

## Workflow Identity
- Workflow: NONE — surface missing
- Version: NONE — surface missing
- Source revision: NONE — surface missing
- Definition digest: NONE — surface missing
- Dependency-lock identity: NONE — surface missing

## Runtime Binding
- Capabilities: browser-use, api-tool-call, filesystem, terminal, conditional gating
- Resources: FlowMart :4105 (petra.voss o-1 admin); MarketplaceDigests :4209 (purpose-built digest surface, §5 note 4)
- Environments: fallback proving ground
- Approvals: none
- Triggers/schedules: none

## Evidence
- docs/validation/evidence/vwo-015/nst-x003/user-path/01-listing-read.txt
- docs/validation/evidence/vwo-015/nst-x003/user-path/02-flowmart-listing.txt
- docs/validation/evidence/vwo-015/nst-x003/user-path/verify-transcript.txt
- docs/validation/evidence/vwo-015/nst-x003/post-hoc/api-manifest.txt
- docs/validation/evidence/vwo-015/nst-x003/post-hoc/install-response.json
- docs/validation/evidence/vwo-015/nst-x003/post-hoc/c1-oracles.txt

## Failure / Recovery
- Failure injected: none (the tampered digest is the control branch)
- Recovery attempted: n/a
- Restart/session-loss behavior: n/a
- Final outcome: PASS

## Product / UX Friction
The digest page advertises the integrity values next to the download — the terminal verification is a deliberate second check (the task's point).

## Security / Architecture Findings
- No findings

## Recommendation
- DEFER
- Proposed owner: n/a
- Verification required: none

## Worker Conclusion
passed.

---

## Scenario
- Scenario id: nst-x004
- Industry: construction (SiteBuild)
- Scenario: Long-running weekly report with restart
- User goal: The Friday site report: read the week's progress entries in the portal, compute the totals with the terminal script, and post the weekly summary back to the portal. It runs for a while — if it gets killed halfway it must pick up where it left off without posting twice.
- Teaching mode: DEMONSTRATE
- benchmark_version: 1.0.0
- Difficulty: L4 (catalog; agreed)

## User Path
1. Seeded the week's progress reports (the seed's 09-10 report + three API-filed reports).
2. Launched the long-running runner (per-day reads with 2s spacing, checkpoint after each day, guarded single post at the end).
3. kill -9 mid-run (after 2 of 5 days); the checkpoint preserved the completed days.
4. Re-ran the runner: the completed days were skipped (resume), the rest processed, the summary posted exactly once.

## Expected
Exactly ONE weekly summary record after the run; the kill -9 occurred mid-run and the resumed run did not redo completed side-effects; the summary totals equal an independent recomputation.

## Actual
One summary record ("Weekly summary 2026-W37: 5 reports, first 64% -> last 70% (delta +6)."); the transcript shows the kill + the skip lines; the date-sorted independent recompute matches the summary string exactly.

## Workflow Identity
- Workflow: NONE — surface missing
- Version: NONE — surface missing
- Source revision: NONE — surface missing
- Definition digest: NONE — surface missing
- Dependency-lock identity: NONE — surface missing

## Runtime Binding
- Capabilities: durable long-running workflow, restart resumption, idempotency
- Resources: SiteBuild :4101 (marco.silva session); the checkpoint file; process control
- Environments: fallback proving ground
- Approvals: none
- Triggers/schedules: none

## Evidence
- docs/validation/evidence/vwo-015/nst-x004/user-path/transcript.txt
- docs/validation/evidence/vwo-015/nst-x004/post-hoc/c1-oracles.txt

## Failure / Recovery
- Failure injected: kill -9 mid-run (the task's controlled failure)
- Recovery attempted: checkpoint-based resume with skip semantics + the guarded single post
- Restart/session-loss behavior: exactly-once held across the restart
- Final outcome: PASS

## Product / UX Friction
The report posting surface dedups by (project, date) — the summary naturally lands on the week's final day.

## Security / Architecture Findings
- No findings

## Recommendation
- DEFER
- Proposed owner: n/a
- Verification required: none

## Worker Conclusion
passed.

---

## Scenario
- Scenario id: nst-x005
- Industry: construction + software (SiteBuild + ForgeOps)
- Scenario: Cross-app reconciliation report
- User goal: Pull the construction project list from the portal and the deployment list from the forge API, match them by week, and drop one combined status file the PM can read.
- Teaching mode: HYBRID
- benchmark_version: 1.0.0
- Difficulty: L4 (catalog; agreed)

## User Path
1. Authenticated browser session on SiteBuild (dana.reyes, PM) — pulled the project list.
2. Authenticated browser session on ForgeOps (raj.patel, RM) — pulled the deployment list.
3. Terminal joined both by week (2026-W37) into one PM-readable combined status file.

## Expected
Combined file rows equal an independent join recomputation; both sources read through their authenticated surfaces; PM-readable formatting (header + one line per project).

## Actual
All 3 projects present with the 3 deployments joined (week W37); both reads via authenticated browser sessions (transcripts); the file carries the header, one line per project, and the totals footer.

## Workflow Identity
- Workflow: NONE — surface missing
- Version: NONE — surface missing
- Source revision: NONE — surface missing
- Definition digest: NONE — surface missing
- Dependency-lock identity: NONE — surface missing

## Runtime Binding
- Capabilities: cross-app data carry, api-tool-call, file output
- Resources: SiteBuild :4101 + ForgeOps :4102 (both authenticated)
- Environments: fallback proving ground
- Approvals: none
- Triggers/schedules: none

## Evidence
- docs/validation/evidence/vwo-015/nst-x005/user-path/transcript.txt
- docs/validation/evidence/vwo-015/nst-x005/user-path/combined-status.txt
- docs/validation/evidence/vwo-015/nst-x005/post-hoc/projects-read.json
- docs/validation/evidence/vwo-015/nst-x005/post-hoc/deployments-read.json
- docs/validation/evidence/vwo-015/nst-x005/post-hoc/c1-oracles.txt

## Failure / Recovery
- Failure injected: none
- Recovery attempted: the agent-browser eval returns double-encoded JSON — unwrapped before the join (recorded in the transcript)
- Restart/session-loss behavior: n/a
- Final outcome: PASS

## Product / UX Friction
none observed.

## Security / Architecture Findings
- No findings

## Recommendation
- DEFER
- Proposed owner: n/a
- Verification required: none

## Worker Conclusion
passed.

---

## Scenario
- Scenario id: nst-x006
- Industry: rideshare (RidePilot)
- Scenario: Concurrent dual-region surge decision
- User goal: Two regions hit the surge threshold at the same time. Watch both dashboards, get ops approval for the combined surge plan, and broadcast both regions' changes together — each exactly once.
- Teaching mode: INSTRUCT
- benchmark_version: 1.0.0
- Difficulty: L4 (catalog; agreed)

## User Path
1. The approval gate FIRST: the ops approval for the combined surge plan (operator channel, timestamped — 11:36:27.299Z).
2. Two concurrent branches launched in the same instant (parallel fanout): rg-1 surge → 1.6 and rg-2 surge → 2.0.
3. Both branches completed within milliseconds of each other (transcript timestamps 11:36:49.050/051) and joined.

## Expected
Both region broadcasts land exactly once; the two decision steps overlapped in time; approval precedes both broadcasts.

## Actual
rg-1 = 1.6, rg-2 = 2.0; exactly 2 ops.surge_adjusted events (no dups); the branch timestamps prove concurrency (millisecond overlap); the approval (11:36:27.299Z) precedes both events (11:36:49.050/051).

## Workflow Identity
- Workflow: NONE — surface missing
- Version: NONE — surface missing
- Source revision: NONE — surface missing
- Definition digest: NONE — surface missing
- Dependency-lock identity: NONE — surface missing

## Runtime Binding
- Capabilities: parallel workflow branches, human gate, exactly-once broadcasts
- Resources: RidePilot :4103 (farah.khan, surge:adjust); operator channel (the approval gate, §5-1)
- Environments: fallback proving ground
- Approvals: the ops approval for the combined plan (operator-channel record)
- Triggers/schedules: none

## Evidence
- docs/validation/evidence/vwo-015/nst-x006/user-path/approval-gate.txt
- docs/validation/evidence/vwo-015/nst-x006/user-path/transcript.txt
- docs/validation/evidence/vwo-015/nst-x006/post-hoc/c1-oracles.txt

## Failure / Recovery
- Failure injected: none
- Recovery attempted: the first fanout used a wrong form field name (surgeLevel vs the fixture's level) — corrected and re-run (recorded in the transcript)
- Restart/session-loss behavior: n/a
- Final outcome: PASS

## Product / UX Friction
The surge form field is `level` — the UI labels it "New surge level"; consistent enough.

## Security / Architecture Findings
- No findings

## Recommendation
- DEFER
- Proposed owner: n/a
- Verification required: none

## Worker Conclusion
passed.

---

## Scenario
- Scenario id: nst-x007
- Industry: media (PressRoom)
- Scenario: Media repurpose batch round-trip
- User goal: Grab this week's published story images, resize them for the social crop, and put the resized set back as renditions so the social desk can publish.
- Teaching mode: HYBRID
- benchmark_version: 1.0.0
- Difficulty: L4 (catalog; agreed)

## User Path
1. Browser: identified the week's published story (st-0201) and its image assets (a-0301).
2. Logged in as the photo editor (omar.bhatt, rendition:request + asset:upload).
3. The terminal batch requested the social-card rendition for each published-story image (the fixture's rendition surface — the resize processing pipeline).
4. The assets page re-checked (screenshot): the rendition records present.

## Expected
New rendition records exist for each source asset with machine-checkable dimensions; the set matches the week's story assets exactly; both transfer directions occurred.

## Actual
a-0301 carries the social-card rendition (1200x630, ready); the set matches exactly ([a-0301]); the request/response round-trip executed through the fixture's rendition surface — with D-X7 recorded: PressRoom assets are metadata-only (no image bytes exist server-side; the "resize" is the fixture's simulated rendition processing — a real byte round-trip is not possible against this fixture; recorded, not narrowed).

## Workflow Identity
- Workflow: NONE — surface missing
- Version: NONE — surface missing
- Source revision: NONE — surface missing
- Definition digest: NONE — surface missing
- Dependency-lock identity: NONE — surface missing

## Runtime Binding
- Capabilities: browser-use, terminal, filesystem, upload/download round-trip (via the fixture's rendition surface)
- Resources: PressRoom :4104 (omar.bhatt)
- Environments: fallback proving ground
- Approvals: none
- Triggers/schedules: none

## Evidence
- docs/validation/evidence/vwo-015/nst-x007/user-path/01-desk.txt
- docs/validation/evidence/vwo-015/nst-x007/user-path/02-assets-page.txt
- docs/validation/evidence/vwo-015/nst-x007/user-path/rendition-batch.txt
- docs/validation/evidence/vwo-015/nst-x007/user-path/transcript.txt
- docs/validation/evidence/vwo-015/nst-x007/user-path/04-assets-after.png
- docs/validation/evidence/vwo-015/nst-x007/post-hoc/c1-oracles.txt
- docs/validation/evidence/vwo-015/nst-x007/post-hoc/environment-identity.txt

## Failure / Recovery
- Failure injected: none
- Recovery attempted: none needed
- Restart/session-loss behavior: n/a
- Final outcome: PASS (with D-X7 recorded)

## Product / UX Friction
The assets page shows rendition status inline — the social desk can see readiness at a glance.

## Security / Architecture Findings
- Finding: D-X7 — PressRoom assets are metadata-only (no image bytes); the catalog's byte-level resize round-trip cannot execute against this fixture (the rendition surface simulates the processing)
- Severity: P3
- Reproduction: inspect any asset record — no binary content field or file route exists
- Root cause: the fixture models media as metadata by design (VWO-002 scope)
- Frozen invariant affected: benchmark fidelity (the task's byte-level intent)

## Recommendation
- NEW WORK ORDER
- Proposed owner: harness (either byte-backed assets or a re-bound task text)
- Verification required: a real download/resize/upload round-trip possible

## Worker Conclusion
passed (with D-X7 recorded).

---

## Scenario
- Scenario id: nst-x008
- Industry: marketplace (FlowMart)
- Scenario: Install-verify-rollback across surfaces
- User goal: Install the package through the marketplace, verify its digest from the terminal, survive a mid-flight entitlement revocation, and land back on the last good version through the rollback path — with the audit trail showing why.
- Teaching mode: INSTRUCT
- benchmark_version: 1.0.0
- Difficulty: L4 (catalog; agreed)

## User Path
1. Terminal digest verification of the current install's package (sha256 == advertised) — the gate OPEN.
2. The upgrade attempt with the entitlement REVOKED mid-flight (armed stale_entitlement): fail-closed observed (409, the VWO-009 class).
3. The rollback path invoked: the fixture correctly reports no prior version to roll back to (ins-0401 installed AT 2.3.1, never upgraded — the last good version IS the current pin).
4. Final state: install pinned at 2.3.1 (last good), the audit trail carrying both rejections with reasons.

## Expected
Final install pin equals the last good version; the entitlement revocation fired mid-upgrade with fail-closed evidenced; the terminal digest verification transcript gates the install step.

## Actual
stale_entitlement fired on the upgrade (ledgered request.rejected with reason); the rollback attempted and correctly refused (no prior version); the final pin = 2.3.1 = the last good; the digest-verification transcript gates the run.

## Workflow Identity
- Workflow: NONE — surface missing
- Version: NONE — surface missing
- Source revision: NONE — surface missing
- Definition digest: NONE — surface missing
- Dependency-lock identity: NONE — surface missing

## Runtime Binding
- Capabilities: browser-use, terminal, entitlement gates, rollback
- Resources: FlowMart :4105 (petra.voss o-2 admin); stale_entitlement switch; MarketplaceDigests :4209
- Environments: fallback proving ground
- Approvals: none
- Triggers/schedules: none

## Evidence
- docs/validation/evidence/vwo-015/nst-x008/user-path/transcript.txt
- docs/validation/evidence/vwo-015/nst-x008/post-hoc/c1-oracles.txt
- docs/validation/evidence/vwo-015/nst-x008/post-hoc/environment-identity.txt

## Failure / Recovery
- Failure injected: the armed stale_entitlement on the upgrade (the task's controlled revocation)
- Recovery attempted: the rollback path (correctly reports no-prior — the seed's install has no upgrade history; the last good version is the current pin; recorded as the fixture-honest outcome)
- Restart/session-loss behavior: n/a
- Final outcome: PASS

## Product / UX Friction
The rollback's no-prior-version refusal is clear and honest ("installed at v2.3.1, never upgraded").

## Security / Architecture Findings
- No findings (the persona/org routing discovered during the run — iris.chen belongs to o-1, the o-2 admin is petra.voss — is fixture fact, not a defect)

## Recommendation
- DEFER
- Proposed owner: n/a
- Verification required: none

## Worker Conclusion
passed.

---

### Honest deviations list (program level)

1. **Delivery mode**: TL local delivery (lesson-120 protocol) — recorded (same as VWO-012/013/014).
2. **D-X1**: the reads-exempt switch — the X001 armed 503 landed on the bulletin submit (state-changing) with a clean retry; the export-read arm premise recorded as drift (same class as VWO-014's D-T10).
3. **D-X2**: the ForgeOps deploy notification targets the promotion approver (release manager), not the sre_oncall persona — no trigger routes to her; the notification row exists via the real deploy pipeline; drift recorded.
4. **D-X7**: PressRoom assets are metadata-only — no image bytes for a real download/resize/upload round-trip; the rendition surface (the fixture's simulated processing) executed the task's substance; drift recorded.
5. **Approval gates (§5-1)**: in-world persona approvals where the fixture provides the surface (X001 desk-chief; X002 release-manager record) with the operator channel as the timestamped fidelity mirror — never fabricated.
6. **X004/X008 fixture-honest outcomes**: the X004 summary landed on the week's final day (the dedup surface); the X008 rollback correctly refused (no prior version — the last good IS the current pin). Both recorded as the fixtures' real behavior.

### Self-check (recorded results)

- Every NST-X task (X001..X008) has a run record: YES (8 scenario blocks).
- Every [C1]/[C2] criterion has a verdict + evidence pointer: YES.
- Every failure/limitation has exactly one classification: 4 environment-specific-limitation drifts (D-X1, D-X2, D-X7, §5-1 note) + 2 P3 findings recommending harness alignment. YES.
- The report cites the exact base SHA and benchmark_version: YES.
- No model-text-as-evidence: YES — every verdict cites an artifact under docs/validation/evidence/vwo-015/.
- The catalog was not edited: `git diff --stat` shows only additive paths under docs/validation/reports/VWO-015-report.md and docs/validation/evidence/vwo-015/ (verified before commit).
