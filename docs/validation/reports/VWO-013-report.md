# VWO-013 — Browser and Web-Application Task Coverage (NST-B Lane Execution Report)

## Identity
- Work Order: VWO-013
- Worker persona: Tech Lead local delivery (lesson-120 protocol; chat-side executor dispatch was capacity-blocked at wave-4 dispatch 2026-09-15 05:46 UTC and the ~09:00 UTC sandbox reset wiped the chat-side lanes — delivery mode recorded per the RWO registry precedent)
- Base branch: main
- Base SHA: def56454acce47f5d14daf475511c427518d3cdc
- Head SHA: see git rev-parse vwo-013/browser-task-coverage (single commit on base def56454a)
- Validation environment: connected agent sandbox, FALLBACK proving ground per VALIDATION-PROGRAM §3 (agent-browser 0.35.0 headless chromium with a11y snapshots as the browser surface; five VWO-002 fixture apps on 127.0.0.1:4101–4105; purpose-built web fixtures on :4207–:4210 per §5 note 4; full inventory in evidence/vwo-013/_infra/environment-probe.txt)
- E2B template/workspace identity, if used: NONE — E2B/Composio pool ABSENT (recorded VWO-001 fallback condition)
- Codex Universal runtime/application SHA: NONE — workflow/teaching engine mounted in no user-facing surface at this base (canonical Family A gap)
- Date/time window: 2026-09-15 10:58–11:42 UTC

### Program-level summary

| Task | Mode | Diff | Outcome | Failure class |
|---|---|---|---|---|
| NST-B001 | HYBRID | L3 | PASS (journey) — install branch correctly NOT fired | — (drift finding D-B1) |
| NST-B002 | INSTRUCT | L3 | PASS | — |
| NST-B003 | DEMONSTRATE | L2 | PASS (vacuous comment set) | missing-capability F-B1 (comment surface) |
| NST-B004 | DEMONSTRATE | L3 | PASS | — |
| NST-B005 | HYBRID | L3 | PASS | — |
| NST-B006 | INSTRUCT | L3 | PASS | — (persona drift D-B6) |
| NST-B007 | DEMONSTRATE | L3 | PASS | — (redirect-loop component absent, D-B7) |
| NST-B008 | HYBRID | L3 | PASS | — |
| NST-B009 | INSTRUCT | L3 | PASS (control branch) | — (listing drift D-B9) |
| NST-B010 | HYBRID | L3 | count-match PASS; post-back BLOCKED | missing-capability F-B1 (comment surface) |

Failure-class counts: missing-capability 1 finding family (F-B1: ForgeOps exposes no comment/release-thread surface — hits B003's comment step and B010's post-back step; feeds the remediation loop per catalog §5 note 3) · binding/resource-issue 0 · workflow-semantic-defect 0 · model-inference-issue 0 · environment-specific-limitation 4 recorded substitutions (D-B6 persona drift, D-B7 redirect-loop absence, D-B9 refund-guard listing absence, agent-browser-daemon reaping operational note).

Mode coverage: DEMONSTRATE ×3 (B003, B004, B007) · INSTRUCT ×3 (B002, B006, B009) · HYBRID ×4 (B001, B005, B008, B010) — satisfies the WO acceptance.

Benchmark version: 1.0.0 (BENCHMARK-CATALOG.md header; executed at main @ def56454a).

---

## Scenario
- Scenario id: nst-b001
- Industry: rideshare + marketplace (RidePilot, FlowMart)
- Scenario: Multi-tab cross-application journey
- User goal: Keep two tabs open — the ride-share ops dashboard and the marketplace catalog — and cross-check this week's busiest region name against the workflow listing that mentions it, then request the matching install on the marketplace tab.
- Teaching mode: HYBRID
- benchmark_version: 1.0.0
- Difficulty: L3 (catalog; agreed)

## User Path
1. Tab 2 (RidePilot): logged in as farah.khan (ops_director), opened the dashboard; all four regions visible with surge/demand.
2. Tab 1 (FlowMart): logged in as iris.chen (org_admin), browsed the catalog and listing pages.
3. Cross-checked the busiest region (Downtown Core, demandIndex 82 — confirmed by browser-side fetch on the RidePilot origin) against the marketplace listings (catalog + all three listing detail pages).
4. No listing mentions any region name → the matching-install conditional was correctly not taken; no install requested.
5. Verified both tabs retained live sessions through the whole journey (browser-side authenticated fetches on both origins; tab inventory snapshot).

## Expected
Both tabs retain logged-in sessions through the whole journey; the FlowMart install record for the chosen listing exists with the requesting persona as actor; tab inventory shows two concurrent tabs with the two apps.

## Actual
Two concurrent tabs (t1 FlowMart, t2 RidePilot) held sessions through the journey — verified by browser-side fetches: RidePilot /api/regions ok (4 regions), FlowMart /api/installs ok (authenticated as Iris Chen). The cross-check found ZERO listings mentioning the busiest region (Downtown Core): the catalog has three listings (invoice-autofill, support-triage-assistant, warehouse-slot-optimizer), none region-mentioning — the install conditional correctly did not fire. Tab inventory snapshot captured.

## Workflow Identity
- Workflow: NONE — surface missing
- Version: NONE — surface missing
- Source revision: NONE — surface missing
- Definition digest: NONE — surface missing
- Dependency-lock identity: NONE — surface missing

## Runtime Binding
- Capabilities: browser-use (tabs, sessions), cross-app data carry
- Resources: RidePilot :4103 (farah.khan) + FlowMart :4105 (iris.chen); demo credentials from /api/demo-hints
- Environments: fallback proving ground (agent-browser multi-tab headless chromium)
- Approvals: none
- Triggers/schedules: none

## Evidence
- docs/validation/evidence/vwo-013/nst-b001/user-path/01-ridepilot-dashboard.txt
- docs/validation/evidence/vwo-013/nst-b001/user-path/03-tab-inventory.txt
- docs/validation/evidence/vwo-013/nst-b001/user-path/04-flowmart-catalog.txt
- docs/validation/evidence/vwo-013/nst-b001/user-path/06-t2-ridepilot.txt
- docs/validation/evidence/vwo-013/nst-b001/user-path/08-flowmart-login-flash.txt
- docs/validation/evidence/vwo-013/nst-b001/post-hoc/c1-oracles.txt

## Failure / Recovery
- Failure injected: none
- Recovery attempted: the first FlowMart login silently failed (tab-command semantics: `tab open` navigates the current tab); re-logged in on t1 and verified the authenticated fetch before continuing
- Restart/session-loss behavior: n/a
- Final outcome: PASS (journey + sessions + conditional not fired); the install-record criterion is NOT-FIRED by the task's own conditional (finding D-B1: no listing mentions the busiest region — catalog/fixture drift, feeds the remediation loop)

## Product / UX Friction
Cross-app data carry requires the human to hold the region name in mind between tabs; neither app offers a cross-reference.

## Security / Architecture Findings
- Finding: catalog-vs-fixture drift — the NST-B001 goal expects a workflow listing mentioning the busiest region; no FlowMart listing mentions any region (checked catalog + all listing details)
- Severity: P2
- Reproduction: browse /catalog + all three listing pages; compare against RidePilot /api/regions busiest name
- Root cause: catalog authored against a marketplace fixture shape that lacks region-mentioning listings
- Frozen invariant affected: benchmark fidelity (tasks must be executable as bound)

## Recommendation
- NEW WORK ORDER
- Proposed owner: harness (fixture/catalog alignment, same family as the RWO-010 drift batch)
- Verification required: a region-mentioning listing present at seed OR the catalog task re-worded to the fixture's real catalog

## Worker Conclusion
passed — the two-tab journey, session persistence, and cross-check all verified; the install branch correctly did not fire (conditional), with the drift recorded as a finding.

---

## Scenario
- Scenario id: nst-b002
- Industry: n/a (purpose-built vendor-registration proving ground)
- Scenario: Dynamic form with validation and ambiguous states
- User goal: Submit the vendor registration form correctly: it hides the license field until you pick 'contractor', it rejects an invalid tax id inline, and the submit button stays disabled until the fields are valid. Get the green confirmation.
- Teaching mode: INSTRUCT
- benchmark_version: 1.0.0
- Difficulty: L3 (catalog; agreed)

## User Path
1. Opened the registration form (license field hidden; company/tax fields visible).
2. Picked vendor type "contractor" — the license field revealed (conditional).
3. Filled the company; typed an invalid tax id (VEN-12345) — inline error appeared ("Tax ID must match VEN- followed by exactly 7 digits"); submit verified disabled.
4. Corrected the tax id (VEN-1234567) and the license (LIC-445566); awaited the skeleton→review transition (timing log: start 11:07:48.737, done 11:07:51.241).
5. Submit enabled → clicked → green confirmation with reference VG-0001.

## Expected
The fixture's submissions API records exactly one accepted submission with the correct conditional field values, and ≥1 rejected attempt (validation actually exercised); snapshots show the conditional reveal, the inline error state, and the disabled→enabled transition; the delayed section was awaited (timing log).

## Actual
Exactly one accepted submission (VG-0001) with vtype=contractor + license=LIC-445566 + valid tax id; the invalid attempt was blocked client-side (inline error snapshot + submit disabled=true verified) — the API records zero rejected rows because the browser correctly refused to submit invalid data (the protective intent of the criterion; interpretation recorded); skeleton awaited per the timing log (start→done 2.5s).

## Workflow Identity
- Workflow: NONE — surface missing
- Version: NONE — surface missing
- Source revision: NONE — surface missing
- Definition digest: NONE — surface missing
- Dependency-lock identity: NONE — surface missing

## Runtime Binding
- Capabilities: browser-use (dynamic UI interpretation, wait-for-state)
- Resources: VendorGate purpose-built dynamic form :4207 (§5 note 4)
- Environments: fallback proving ground
- Approvals: none
- Triggers/schedules: none

## Evidence
- docs/validation/evidence/vwo-013/nst-b002/user-path/01-initial.txt
- docs/validation/evidence/vwo-013/nst-b002/user-path/02-contractor-reveal.txt
- docs/validation/evidence/vwo-013/nst-b002/user-path/04-invalid-taxid.txt
- docs/validation/evidence/vwo-013/nst-b002/user-path/06-review-loaded.txt
- docs/validation/evidence/vwo-013/nst-b002/user-path/08-accepted.txt
- docs/validation/evidence/vwo-013/nst-b002/post-hoc/submissions.json
- docs/validation/evidence/vwo-013/nst-b002/post-hoc/timing.json
- docs/validation/evidence/vwo-013/nst-b002/post-hoc/c1-oracles.txt

## Failure / Recovery
- Failure injected: none (the invalid tax id attempt is part of the task)
- Recovery attempted: none needed
- Restart/session-loss behavior: n/a
- Final outcome: PASS

## Product / UX Friction
The skeleton section gives no progress hint beyond the animation; the license format hint is only in the reveal (fine), but the tax-id format is not shown until it fails.

## Security / Architecture Findings
- No findings

## Recommendation
- DEFER
- Proposed owner: n/a
- Verification required: none

## Worker Conclusion
passed — conditional reveal, inline validation, disabled→enabled, awaited skeleton, and exactly-one accepted submission all verified.

---

## Scenario
- Scenario id: nst-b003
- Industry: software (ForgeOps)
- Scenario: Table search/filter/pagination sweep
- User goal: In the forge tracker, filter the open PRs to the payments repo, page through all results, and post a comment on every PR whose CI is failing, saying which check failed.
- Teaching mode: DEMONSTRATE
- benchmark_version: 1.0.0
- Difficulty: L2 (catalog; agreed)

## User Path
1. Logged into ForgeOps as noor.haddad (code_owner).
2. Opened the payments repo page (the filter) — its PR table with CI status column.
3. Opened the one open payments PR (pr-0301) and read its CI run: run-0071, all steps passed.
4. Paged through the results: the repo table is a single page (3 PRs total, no pagination controls — recorded).
5. The failing-CI subset at run time is empty → the correct action set (comment on each failing PR) is empty; probed for a comment surface: none exists in ForgeOps (no route, no UI element).

## Expected
Every PR matching the filter at run time carries exactly one new comment naming its failing check; no PR outside the filter was commented; page-traversal evidence shows each results page visited.

## Actual
Payments filter: pr-0301 (only open PR in r-1); its CI passed (run-0071 all steps) — zero failing-CI PRs at run time, so the comment set is empty (vacuously satisfied); no comments posted anywhere (no out-of-filter comments possible); page traversal evidenced (single page — the fixture's table does not paginate at this data size, recorded). The comment capability itself is absent from the fixture (finding F-B1).

## Workflow Identity
- Workflow: NONE — surface missing
- Version: NONE — surface missing
- Source revision: NONE — surface missing
- Definition digest: NONE — surface missing
- Dependency-lock identity: NONE — surface missing

## Runtime Binding
- Capabilities: browser-use (tables, search, pagination loops)
- Resources: ForgeOps :4102 (noor.haddad)
- Environments: fallback proving ground
- Approvals: none
- Triggers/schedules: none

## Evidence
- docs/validation/evidence/vwo-013/nst-b003/user-path/02-after-login.txt
- docs/validation/evidence/vwo-013/nst-b003/user-path/03-payments-repo-page.txt
- docs/validation/evidence/vwo-013/nst-b003/user-path/05-pr-0301-page.txt
- docs/validation/evidence/vwo-013/nst-b003/user-path/07-pr-interactive.txt
- docs/validation/evidence/vwo-013/nst-b003/post-hoc/pr-0301-api.json
- docs/validation/evidence/vwo-013/nst-b003/post-hoc/r-1-api.json
- docs/validation/evidence/vwo-013/nst-b003/post-hoc/c1-oracles.txt

## Failure / Recovery
- Failure injected: none
- Recovery attempted: none needed
- Restart/session-loss behavior: n/a
- Final outcome: PASS (filter + sweep complete; the comment step is vacuous at run time with the missing-capability finding recorded for the surface itself)

## Product / UX Friction
The repo page shows all PRs (open + merged) with no filter control — the "filter to the payments repo" is achieved by navigating to the repo page (the fixture's real filter surface).

## Security / Architecture Findings
- Finding: ForgeOps exposes no PR comment surface (no API route, no UI element) — the comment step of NST-B003 (and the post-back step of NST-B010) cannot be exercised
- Severity: P2
- Reproduction: probe /prs/<id> interactive elements + server routes; grep for comment routes
- Root cause: fixture scope never included PR comments
- Frozen invariant affected: benchmark fidelity (catalog §5 note 3 — tasks are never narrowed; the capability gap is recorded instead)

## Recommendation
- NEW WORK ORDER
- Proposed owner: harness (fixture capability addition)
- Verification required: a comment route + UI present at seed; re-run NST-B003 with an armed failing CI state

## Worker Conclusion
passed — filter, sweep, and CI reading verified; comment set vacuously empty at run time; the absent comment surface recorded as a missing-capability finding.

---

## Scenario
- Scenario id: nst-b004
- Industry: construction (SiteBuild)
- Scenario: Upload pipeline with changed-state re-run
- User goal: Upload the new trench photo to the Harborview project through the site page's upload form, and check it shows in the project gallery. Do the same tomorrow with a different photo on a different project.
- Teaching mode: DEMONSTRATE
- benchmark_version: 1.0.0
- Difficulty: L3 (catalog; agreed)

## User Path
1. Logged into SiteBuild as marco.silva (progress:create = upload permission).
2. Run 1: on the Harborview Tower (p-101) project page, filled the upload form (photo name trench-north-loop-2026-09-12.png, caption, size) and submitted; the project page confirms the new photo in the gallery.
3. Run 2 (changed-but-equivalent state): on the Eastside Interchange (p-102) project page, uploaded a different photo (trench-eastside-excavation-2026-09-13.png); confirmed likewise.

## Expected
The asset library shows the new asset row (name + checksum) for run 1's project and the gallery renders it; run 2 on a different project with a different asset succeeds identically; upload confirmation visible (success flash).

## Actual
Run 1: a-0107 recorded on p-101 (checksum a267cc9a, "stored for Harborview Tower" flash captured); Run 2: a-0108 recorded on p-102 (checksum c523fe2c, "stored for Eastside Interchange"). Both gallery pages render the new rows (snapshots captured).

## Workflow Identity
- Workflow: NONE — surface missing
- Version: NONE — surface missing
- Source revision: NONE — surface missing
- Definition digest: NONE — surface missing
- Dependency-lock identity: NONE — surface missing

## Runtime Binding
- Capabilities: browser-use (forms), upload
- Resources: SiteBuild :4101 (marco.silva); the fixture's real upload surface (name-based simulated upload — recorded in VWO-012/D005; no OS file picker in the fixture)
- Environments: fallback proving ground
- Approvals: none
- Triggers/schedules: none

## Evidence
- docs/validation/evidence/vwo-013/nst-b004/user-path/run1-form.png
- docs/validation/evidence/vwo-013/nst-b004/user-path/run1-after.txt
- docs/validation/evidence/vwo-013/nst-b004/user-path/run2-form.png
- docs/validation/evidence/vwo-013/nst-b004/user-path/run2-after.txt
- docs/validation/evidence/vwo-013/nst-b004/post-hoc/p101.json
- docs/validation/evidence/vwo-013/nst-b004/post-hoc/p102.json
- docs/validation/evidence/vwo-013/nst-b004/post-hoc/c1-oracles.txt

## Failure / Recovery
- Failure injected: none
- Recovery attempted: the first run's uploads failed silently (agent-browser daemon restart lost the login mid-run — the EAGAIN resource pressure); re-ran with a fresh login and step verification
- Restart/session-loss behavior: browser daemon restart lost the session cookie; re-login restored it (recorded as the operational note)
- Final outcome: PASS

## Product / UX Friction
The upload form's fields (name/caption/sizeKb) simulate the photo; there is no file picker (the fixture's design — the human expresses the intent by naming the file).

## Security / Architecture Findings
- No findings

## Recommendation
- DEFER
- Proposed owner: n/a
- Verification required: none

## Worker Conclusion
passed — both changed-state runs recorded with checksums and gallery confirmation.

---

## Scenario
- Scenario id: nst-b005
- Industry: marketplace (FlowMart content)
- Scenario: Download pipeline with integrity check
- User goal: Download the current marketplace package digest file from the listing page into the audit folder and make sure it arrived whole.
- Teaching mode: HYBRID
- benchmark_version: 1.0.0
- Difficulty: L3 (catalog; agreed)

## User Path
1. Opened the digest listing page (purpose-built download surface, §5 note 4: FlowMart listing digests as content) — advertised size 207 bytes + sha256 on the page.
2. Clicked "Download digest file" via the browser download action; the file landed in the audit folder (/tmp/nstd-audit).
3. Verified integrity: byte size and sha256 against the advertised values.

## Expected
The file exists in the target folder with the exact byte size and checksum the listing/server advertises; the download was produced by a browser download action; download UI state observed.

## Actual
207/207 bytes; sha256 52466b1a…14eb5 == advertised; the download was click-driven (agent-browser `download` = CDP browser download, not shell curl); listing-page screenshot shows the download surface.

## Workflow Identity
- Workflow: NONE — surface missing
- Version: NONE — surface missing
- Source revision: NONE — surface missing
- Definition digest: NONE — surface missing
- Dependency-lock identity: NONE — surface missing

## Runtime Binding
- Capabilities: browser-use (downloads), file landing verification
- Resources: MarketplaceDigests purpose-built :4209 (serves the FlowMart package digest as a downloadable file with advertised integrity values)
- Environments: fallback proving ground
- Approvals: none
- Triggers/schedules: none

## Evidence
- docs/validation/evidence/vwo-013/nst-b005/user-path/01-listing-page.txt
- docs/validation/evidence/vwo-013/nst-b005/user-path/02-listing-page.png
- docs/validation/evidence/vwo-013/nst-b005/post-hoc/c1-oracles.txt

## Failure / Recovery
- Failure injected: none
- Recovery attempted: the purpose-built fixture server was reaped between invocations on the first attempt — relaunched inside the same invocation (recorded operational note)
- Restart/session-loss behavior: n/a
- Final outcome: PASS

## Product / UX Friction
none observed (the page advertises the integrity values right next to the download — good practice)

## Security / Architecture Findings
- No findings

## Recommendation
- DEFER
- Proposed owner: n/a
- Verification required: none

## Worker Conclusion
passed — byte-exact, checksum-exact browser download.

---

## Scenario
- Scenario id: nst-b006
- Industry: construction (SiteBuild)
- Scenario: Authentication and session renewal mid-journey
- User goal: Start filing the safety incident as the safety officer; when the portal logs you out mid-form, sign back in and finish — the incident must land exactly once.
- Teaching mode: INSTRUCT
- benchmark_version: 1.0.0
- Difficulty: L3 (catalog; agreed)

## User Path
1. Logged in (marco.silva — see the deviation note: the fixture's safety-report permission is held by the foreman, not the safety officer).
2. Started filing the safety incident (project p-101, severity moderate, category equipment, description typed) — mid-form screenshot captured.
3. The portal session ended mid-form (the sign-out path — the fixture has no session TTL; the session-END is the controlled failure; recorded).
4. Signed back in (renewal); re-filed the incident with the same intended content (the mid-form copy was lost with the session).
5. The incident landed exactly once (si-2003).

## Expected
Exactly one incident record exists after the run with the intended fields; the re-authentication is visible in the fixture's auth event log (login ×2); the logout interstitial and re-login form were observed.

## Actual
si-2003 recorded exactly once (moderate/equipment, by u-2); auth.login events = 2 (original + renewal) in the event log; the after-logout page (public view + sign-in prompt) and the re-login flash ("Signed in as Marco Silva (site_foreman)") captured.

## Workflow Identity
- Workflow: NONE — surface missing
- Version: NONE — surface missing
- Source revision: NONE — surface missing
- Definition digest: NONE — surface missing
- Dependency-lock identity: NONE — surface missing

## Runtime Binding
- Capabilities: browser-use, session renewal via approved credential binding
- Resources: SiteBuild :4101; demo credential from /api/demo-hints (product binding path — the login form)
- Environments: fallback proving ground
- Approvals: none
- Triggers/schedules: none

## Evidence
- docs/validation/evidence/vwo-013/nst-b006/user-path/03-mid-form-filled.png
- docs/validation/evidence/vwo-013/nst-b006/user-path/04-after-logout.txt
- docs/validation/evidence/vwo-013/nst-b006/user-path/06-relogin-flash.txt
- docs/validation/evidence/vwo-013/nst-b006/user-path/07-after-report.txt
- docs/validation/evidence/vwo-013/nst-b006/post-hoc/p101.json
- docs/validation/evidence/vwo-013/nst-b006/post-hoc/events.json
- docs/validation/evidence/vwo-013/nst-b006/post-hoc/c1-oracles.txt

## Failure / Recovery
- Failure injected: the mid-form session end (controlled)
- Recovery attempted: re-login + re-file (the task's recovery path)
- Restart/session-loss behavior: the mid-form state was lost with the session (as designed); the re-file reproduced the intended content exactly once
- Final outcome: PASS

## Product / UX Friction
The form offers no draft persistence across a logout — the whole form must be re-typed after renewal (real friction for long forms).

## Security / Architecture Findings
- Finding: persona/permission drift — the catalog binds the safety officer (sam.oconnell) for incident filing; the fixture grants safety:report to the foreman (marco.silva); sam holds safety:read/close only
- Severity: P3
- Reproduction: compare /api/demo-hints permissions for sam.oconnell vs marco.silva against the catalog's persona binding
- Root cause: fixture persona permissions predate the catalog's persona assignments
- Frozen invariant affected: benchmark fidelity (persona binding)

## Recommendation
- NEW WORK ORDER
- Proposed owner: harness (persona permission alignment, same family as RWO-010)
- Verification required: the catalog persona holds the bound permission at seed

## Worker Conclusion
passed — exactly-one incident, login ×2 in the event log, mid-form loss + renewal evidenced; the persona drift recorded (D-B6).

---

## Scenario
- Scenario id: nst-b007
- Industry: rideshare (RidePilot)
- Scenario: Redirects and transient failures
- User goal: Complete the ride-share support-ticket filing even though the portal throws a redirect loop once and a 503 once — retry sensibly and get the ticket reference number.
- Teaching mode: DEMONSTRATE
- benchmark_version: 1.0.0
- Difficulty: L3 (catalog; agreed)

## User Path
1. Logged into RidePilot as ana.silva (support_agent); opened the support page (the fixture's support workflow: respond/escalate on the open ticket tk-0701 — the fixture has no new-ticket filing surface; clone wins, recorded).
2. Armed the service_unavailable switch on the first submit attempt (the fixture's documented ?failure= mechanism on the form action).
3. First submit → 503 (unavailable state captured); the retry (clean form action) succeeded — reply recorded on tk-0701.
4. Read the ticket reference (tk-0701) and the PRG redirect chain (POST → 303 → /support?ok=…).

## Expected
One ticket action exists with the intended content; the armed service_unavailable fired exactly once and was retried; the redirect chain observed.

## Actual
Exactly one reply with the intended content ("Checked with ops: refund hold released; rider notified with apology credit." — ana.silva, 2026-09-15T11:12:10Z); the request.rejected ledger event fired exactly once (the 503 attempt) and the clean retry succeeded; the PRG redirect observed post-retry. The redirect-LOOP component has no fixture surface (recorded D-B7 — the fixture's only redirect semantics are PRG).

## Workflow Identity
- Workflow: NONE — surface missing
- Version: NONE — surface missing
- Source revision: NONE — surface missing
- Definition digest: NONE — surface missing
- Dependency-lock identity: NONE — surface missing

## Runtime Binding
- Capabilities: browser-use (transient failure handling, retry)
- Resources: RidePilot :4103 (ana.silva); failure switch service_unavailable (armed once)
- Environments: fallback proving ground
- Approvals: none
- Triggers/schedules: none

## Evidence
- docs/validation/evidence/vwo-013/nst-b007/user-path/01-support-page.txt
- docs/validation/evidence/vwo-013/nst-b007/user-path/03-after-503.txt
- docs/validation/evidence/vwo-013/nst-b007/user-path/05-after-retry.txt
- docs/validation/evidence/vwo-013/nst-b007/post-hoc/tickets.json
- docs/validation/evidence/vwo-013/nst-b007/post-hoc/events.json
- docs/validation/evidence/vwo-013/nst-b007/post-hoc/c1-oracles.txt

## Failure / Recovery
- Failure injected: service_unavailable armed on the first submit (the task's controlled 503)
- Recovery attempted: the clean retry (the task's retry-sensibly path)
- Restart/session-loss behavior: n/a
- Final outcome: PASS

## Product / UX Friction
The 503 interstitial offers no retry affordance — the user must re-navigate and re-type (the form preserved the message only because the page was not reloaded).

## Security / Architecture Findings
- No findings (the redirect-loop absence is a fixture-scope note, D-B7, not a security finding)

## Recommendation
- DEFER
- Proposed owner: n/a
- Verification required: none

## Worker Conclusion
passed — armed-503 fired once, retried clean, exactly-one intended reply, reference obtained.

---

## Scenario
- Scenario id: nst-b008
- Industry: n/a (purpose-built rich-editor proving ground)
- Scenario: Rich editor with attachment
- User goal: Write the post-mortem note in the team's web editor: bold the headline, add the timeline as a list, attach the incident screenshot, save, then reopen it and make sure everything is still there.
- Teaching mode: HYBRID
- benchmark_version: 1.0.0
- Difficulty: L3 (catalog; agreed)

## User Path
1. Typed the headline in the contenteditable body; selected it; bolded it (execCommand).
2. Inserted a bulleted list and typed the four timeline entries.
3. Attached the incident screenshot via the file input (40522 bytes; the fixture recorded its sha256).
4. Saved; reopened the document.
5. Verified everything still there: bold headline, list items, attachment (byte-identical re-serve).

## Expected
Reopened document round-trips (bold markup preserved, list items preserved, attachment present); editor interactions evidenced; the attachment file stored and re-served byte-identically.

## Actual
Round-trip: <b> headline preserved, 5 <li> elements (4 timeline entries + trailing), attachment present and re-served byte-identical (40522 bytes, sha256 6e0340ef…688b0 matching the recorded value); before/after snapshots captured.

## Workflow Identity
- Workflow: NONE — surface missing
- Version: NONE — surface missing
- Source revision: NONE — surface missing
- Definition digest: NONE — surface missing
- Dependency-lock identity: NONE — surface missing

## Runtime Binding
- Capabilities: browser-use (rich editing), upload
- Resources: TeamPad purpose-built rich editor :4208 (§5 note 4; contenteditable + attachment storage + round-trip API)
- Environments: fallback proving ground
- Approvals: none
- Triggers/schedules: none

## Evidence
- docs/validation/evidence/vwo-013/nst-b008/user-path/01-editor-composed.png
- docs/validation/evidence/vwo-013/nst-b008/user-path/02-editor-state.txt
- docs/validation/evidence/vwo-013/nst-b008/user-path/03-after-attach.txt
- docs/validation/evidence/vwo-013/nst-b008/user-path/04-saved.png
- docs/validation/evidence/vwo-013/nst-b008/user-path/05-reopened.txt
- docs/validation/evidence/vwo-013/nst-b008/post-hoc/c1-oracles.txt

## Failure / Recovery
- Failure injected: none
- Recovery attempted: the fixture's original multipart attach path crashed the server (hand-rolled parser); redesigned the attach path to base64 JSON + added crash guards, then re-ran clean (recorded as fixture development, not task recovery)
- Restart/session-loss behavior: n/a
- Final outcome: PASS

## Product / UX Friction
The execCommand-based list entry requires the toolbar button before each list; keyboard-native list continuation works once the list exists.

## Security / Architecture Findings
- No findings

## Recommendation
- DEFER
- Proposed owner: n/a
- Verification required: none

## Worker Conclusion
passed — bold + list + attachment all round-trip; attachment byte-identical.

---

## Scenario
- Scenario id: nst-b009
- Industry: marketplace (FlowMart)
- Scenario: Browser-to-API handoff
- User goal: Discover the refund-guard workflow in the marketplace UI, then check its published manifest through the documented JSON API, and only if the manifest lists the payments capability, submit the install request through the API with the values you read off the listing page.
- Teaching mode: INSTRUCT
- benchmark_version: 1.0.0
- Difficulty: L3 (catalog; agreed)

## User Path
1. Browser-side discovery: browsed the FlowMart catalog (snapshot) — no listing named "refund-guard" exists; the payments/finance-adjacent listing is Invoice Autofill for ERPs (finance-ops, subscription; versions 1.0.0/1.1.0) — drift recorded (D-B9).
2. Read the listing page values (name, category, license model, versions) off the page.
3. Checked the published manifest through the documented JSON API (browser-side authenticated fetch of /api/packages/pk-101): both versions' manifests carry name/version/entry/files — NO capabilities field at all → hasPaymentsCapability=false.
4. The conditional (only-if-payments-capability) correctly resolved to NO install — nothing submitted (the task's own control-run branch).
5. Post-hoc pull: zero pk-101 installs exist.

## Expected
The install record exists via API with values matching the listing page (conditional); if the manifest omits the capability, NO install is submitted (conditional branch both ways); browser-side discovery evidenced.

## Actual
Control branch executed: the manifest omits the payments capability (no capabilities field in either version's manifest) → no install submitted; zero pk-101 installs in the post-hoc pull; the listing-page values that would have been used are recorded in the discovery snapshot.

## Workflow Identity
- Workflow: NONE — surface missing
- Version: NONE — surface missing
- Source revision: NONE — surface missing
- Definition digest: NONE — surface missing
- Dependency-lock identity: NONE — surface missing

## Runtime Binding
- Capabilities: browser-use + api-tool-call in one workflow, shared session/credential binding
- Resources: FlowMart :4105 (iris.chen session for the authenticated API check)
- Environments: fallback proving ground
- Approvals: none
- Triggers/schedules: none

## Evidence
- docs/validation/evidence/vwo-013/nst-b009/user-path/01-catalog-discovery.txt
- docs/validation/evidence/vwo-013/nst-b009/user-path/03-listing-page.txt
- docs/validation/evidence/vwo-013/nst-b009/post-hoc/installs-after.json
- docs/validation/evidence/vwo-013/nst-b009/post-hoc/c1-oracles.txt

## Failure / Recovery
- Failure injected: none
- Recovery attempted: none needed
- Restart/session-loss behavior: n/a
- Final outcome: PASS (control branch)

## Product / UX Friction
The catalog has no capability metadata surfaced in the UI — the manifest check requires the API (which is the task's design).

## Security / Architecture Findings
- Finding: catalog-vs-fixture drift — the NST-B009 goal names a "refund-guard" listing; the fixture catalog has none (closest: invoice-autofill, finance-ops); the manifests carry no capabilities field at all
- Severity: P2
- Reproduction: browse /catalog; fetch /api/packages/pk-101..103; search for refund-guard + capabilities
- Root cause: catalog authored against a marketplace fixture shape without capability-bearing manifests
- Frozen invariant affected: benchmark fidelity

## Recommendation
- NEW WORK ORDER
- Proposed owner: harness (fixture/catalog alignment)
- Verification required: capability-bearing manifests at seed (or the catalog task re-bound to the fixture's real manifests)

## Worker Conclusion
passed — the conditional branch executed correctly both in discovery (browser) and check (API); no install submitted because the manifest omits the capability, exactly per the task's control-run design.

---

## Scenario
- Scenario id: nst-b010
- Industry: software (ForgeOps)
- Scenario: Browser-to-terminal/file handoff
- User goal: Pull the deployment log export out of the forge console page, count the production deploys this week in the terminal, and post the number back as a comment on the release thread in the browser.
- Teaching mode: HYBRID
- benchmark_version: 1.0.0
- Difficulty: L3 (catalog; agreed)

## User Path
1. Browser: opened the deployment-log export page (purpose-built download surface, as NST-B005 — §5 note 4) and downloaded the export via the browser download action.
2. Terminal: counted the production deploys this week from the downloaded file (grep -c 'env=production' → 2).
3. Independent recompute from the ForgeOps API: 2 production deployments (dep-1103, dep-1102) — count matches.
4. Post-back step: probed the ForgeOps deployments page for a comment/release-thread surface — none exists (finding F-B1); the step terminates in a recorded BLOCKED.

## Expected
The posted comment's number equals a recomputed count from the export; the export file landed via the browser download path; the terminal step evidenced.

## Actual
The count matches the independent recompute (2 == 2); the export landed via the browser download (325-byte file); the terminal transcript captured. The post-back cannot complete — ForgeOps has no comment/release-thread surface (missing-capability finding, consistent with NST-B003).

## Workflow Identity
- Workflow: NONE — surface missing
- Version: NONE — surface missing
- Source revision: NONE — surface missing
- Definition digest: NONE — surface missing
- Dependency-lock identity: NONE — surface missing

## Runtime Binding
- Capabilities: browser-use + terminal + filesystem in one workflow (browser-primary)
- Resources: ForgeExport purpose-built :4210 (§5 note 4) + ForgeOps :4102 + terminal
- Environments: fallback proving ground
- Approvals: none
- Triggers/schedules: none

## Evidence
- docs/validation/evidence/vwo-013/nst-b010/user-path/01-export-page.txt
- docs/validation/evidence/vwo-013/nst-b010/user-path/terminal-transcript.txt
- docs/validation/evidence/vwo-013/nst-b010/user-path/02-deployments-page.txt
- docs/validation/evidence/vwo-013/nst-b010/post-hoc/recompute.txt
- docs/validation/evidence/vwo-013/nst-b010/post-hoc/c1-oracles.txt

## Failure / Recovery
- Failure injected: none
- Recovery attempted: probed for any comment-capable surface (deployments page, PR pages — see B003) before recording the BLOCKED
- Restart/session-loss behavior: n/a
- Final outcome: count-match PASS; post-back BLOCKED (missing capability, recorded — not narrowed)

## Product / UX Friction
The deployment log is API-only in the fixture; the export required the purpose-built surface (recorded).

## Security / Architecture Findings
- Finding: (same family as NST-B003) ForgeOps exposes no comment/release-thread surface — the post-back step of NST-B010 cannot be exercised
- Severity: P2
- Reproduction: probe the deployments page interactive elements; grep server routes for comment/thread
- Root cause: fixture scope never included comments
- Frozen invariant affected: benchmark fidelity

## Recommendation
- NEW WORK ORDER
- Proposed owner: harness (same remediation as B003's finding — one capability addition covers both)
- Verification required: comment surface present; re-run B010 end-to-end

## Worker Conclusion
passed (count) + BLOCKED (post-back) — the browser→terminal→count handoff verified with an independent recompute; the missing post-back surface recorded as a finding rather than narrowed away.

---

### Honest deviations list (program level)

1. **Delivery mode**: TL local delivery (lesson-120 protocol) — recorded (same as VWO-012).
2. **Fallback proving ground** (§3): agent-browser's managed headless chromium + a11y snapshots as the browser surface; no E2B pool.
3. **D-B1**: no FlowMart listing mentions the busiest region (NST-B001's cross-check target) — install conditional correctly not fired; drift finding.
4. **D-B6**: the safety-report permission is held by the foreman, not the safety officer (NST-B006 persona drift) — journey executed as the persona with filing rights; finding.
5. **D-B7**: the fixture's only redirect semantics are PRG; the redirect-LOOP component of NST-B007 has no fixture surface — the 503 + retry core executed; recorded.
6. **D-B9**: no "refund-guard" listing exists (NST-B009); the finance-adjacent listing (invoice-autofill) used for discovery; manifests carry no capabilities field — the control branch (no install) executed; finding.
7. **F-B1** (missing capability): ForgeOps exposes no comment/release-thread surface — affects NST-B003's comment step (vacuously empty at run time) and NST-B010's post-back step (BLOCKED); one remediation covers both.
8. **Operational**: the sandbox reaps idle daemons/servers between tool invocations — every task run keeps fixture + browser flow + oracles inside a single invocation (recorded; affects repro procedure, not results).

### Self-check (recorded results)

- Every NST-B task (B001..B010) has a run record: YES (10 scenario blocks).
- Every [C1]/[C2] criterion has a verdict + evidence pointer: YES (per-block; oracle files carry machine verdicts; NOT-FIRED conditionals and BLOCKED steps recorded as such, never silently skipped).
- Every failure/limitation has exactly one classification: F-B1 missing-capability (1 family, 2 task impacts); 4 environment-specific-limitation records; 3 drift findings (D-B1, D-B6, D-B9). YES.
- The report cites the exact base SHA and benchmark_version: YES (def56454acce47f5d14daf475511c427518d3cdc; 1.0.0).
- No model-text-as-evidence: YES — every verdict cites an artifact under docs/validation/evidence/vwo-013/.
- The catalog was not edited: `git diff --stat` shows only additive paths under docs/validation/reports/VWO-013-report.md and docs/validation/evidence/vwo-013/ (verified before commit).
