# VWO-014 — Terminal and Shell Task Coverage (NST-T Lane Execution Report)

## Identity
- Work Order: VWO-014
- Worker persona: Tech Lead local delivery (lesson-120 protocol; chat-side executor dispatch capacity-blocked + the 09:00 UTC sandbox reset wiped chat lanes — delivery mode recorded per the RWO registry precedent)
- Base branch: main
- Base SHA: def56454acce47f5d14daf475511c427518d3cdc
- Head SHA: see git rev-parse vwo-014/terminal-task-coverage (single commit on base def56454a)
- Validation environment: connected agent sandbox, FALLBACK proving ground per VALIDATION-PROGRAM §3 (bash + coreutils + git 2.47.3 + node v24.19.0 + Python 3.12.14 + curl; five VWO-002 fixture apps on 127.0.0.1:4101–4105; agent-browser for the one browser confirmation; full inventory in evidence/vwo-014/_infra/environment-probe.txt)
- E2B template/workspace identity, if used: NONE — E2B/Composio pool ABSENT (recorded VWO-001 fallback condition)
- Codex Universal runtime/application SHA: NONE — workflow/teaching engine mounted in no user-facing surface at this base (canonical Family A gap)
- Date/time window: 2026-09-15 11:20–11:55 UTC

### Program-level summary

| Task | Mode | Diff | Outcome | Notes |
|---|---|---|---|---|
| NST-T001 | DEMONSTRATE | L1 | PASS | event-type counts == independent recount |
| NST-T002 | INSTRUCT | L3 | PASS | SIGSTOP wedge; only target PID changed; state intact |
| NST-T003 | DEMONSTRATE | L3 | PASS | two batches, no carry-over |
| NST-T004 | HYBRID | L2 | PASS | CSV == recompute; descending sort |
| NST-T005 | DEMONSTRATE | L1 | PASS | sha256 list equality; exact archive set |
| NST-T006 | HYBRID | L1 | PASS | ancestor proof; clean tree; no push |
| NST-T007 | INSTRUCT | L2 | PASS | conditional branch exercised (fail → reset → 59/59) |
| NST-T008 | HYBRID | L3 | PASS | wrong-edit failure → recovery → 59/59 → commit |
| NST-T009 | INSTRUCT | L2 | PASS | credential from binding path; approved by priya |
| NST-T010 | HYBRID | L4 | PASS | exactly-once counts; stale checkpoint superseded; armed 503 + retry (drift D-T10 recorded) |

Failure-class counts: missing-capability 0 · binding/resource-issue 0 · workflow-semantic-defect 0 · model-inference-issue 0 · environment-specific-limitation 3 recorded drifts/substitutions (D-T4 no /api/procurement route; D-T8 no seeded typo — structural execution; D-T10 the service_unavailable switch exempts reads).

Mode coverage: DEMONSTRATE ×3 (T001, T003, T005) · INSTRUCT ×3 (T002, T007, T009) · HYBRID ×4 (T004, T006, T008, T010) — satisfies the WO acceptance.

Benchmark version: 1.0.0 (BENCHMARK-CATALOG.md header; executed at main @ def56454a).

---

## Scenario
- Scenario id: nst-t001
- Industry: media (PressRoom)
- Scenario: Event-log analysis pipeline
- User goal: Pull today's event feed from the press app and tell me how many events of each type there were, as a tidy summary file.
- Teaching mode: DEMONSTRATE
- benchmark_version: 1.0.0
- Difficulty: L1 (catalog; agreed)

## User Path
1. Pulled the event feed from PressRoom's public API (curl).
2. Piped the JSON through python to count events per type (Counter).
3. Wrote the tidy `type=count` summary file.

## Expected
Summary file with one type=count line per distinct event type; counts equal an independent recomputation; a real shell pipeline (fetch + transform transcript).

## Actual
3 distinct types, 3 lines; counts equal the independent recount of the same pull; the transcript shows the curl|python pipeline.

## Workflow Identity
- Workflow: NONE — surface missing
- Version: NONE — surface missing
- Source revision: NONE — surface missing
- Definition digest: NONE — surface missing
- Dependency-lock identity: NONE — surface missing

## Runtime Binding
- Capabilities: terminal execution, HTTP fetch, text processing
- Resources: PressRoom :4104 public /api/events
- Environments: fallback proving ground
- Approvals: none
- Triggers/schedules: none

## Evidence
- docs/validation/evidence/vwo-014/nst-t001/user-path/transcript.txt
- docs/validation/evidence/vwo-014/nst-t001/user-path/summary.txt
- docs/validation/evidence/vwo-014/nst-t001/post-hoc/summary-file.txt
- docs/validation/evidence/vwo-014/nst-t001/post-hoc/events-pull.json
- docs/validation/evidence/vwo-014/nst-t001/post-hoc/c1-oracles.txt

## Failure / Recovery
- Failure injected: none
- Recovery attempted: none needed
- Restart/session-loss behavior: n/a
- Final outcome: PASS

## Product / UX Friction
The event feed endpoint caps at 100 events by default — for busy days a pagination parameter would be needed (not exercised at this data size).

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
- Scenario id: nst-t002
- Industry: rideshare (RidePilot) + process management
- Scenario: Process inspection and service restart
- User goal: Find which fixture app process is wedged, check its port and health endpoint, restart just that one cleanly, and confirm in the browser that its world came back intact.
- Teaching mode: INSTRUCT
- benchmark_version: 1.0.0
- Difficulty: L3 (catalog; agreed)

## User Path
1. Recorded the pre-state (regions: 4, tickets: 2) and all fixture PIDs.
2. Wedged the rideshare app with SIGSTOP (the controlled wedge); confirmed the wedge via the health-endpoint timeout.
3. Identified the wedged process in the process listing (pgrep transcript; port 4103).
4. Killed it, relaunched rideshare cleanly, verified health 200.
5. Confirmed in the browser (agent-browser snapshot) that the world came back intact.

## Expected
Process listing evidences identification; only the target app's PID changes across restart; post-restart health 200 and the browser render shows the same seeded record counts as before.

## Actual
Target PID 2445 → 13327 (only change; all other apps' PIDs unchanged — verified set equality); counts before == after (regions 4, tickets 2); health 200 post-restart; browser confirmation snapshot captured.

## Workflow Identity
- Workflow: NONE — surface missing
- Version: NONE — surface missing
- Source revision: NONE — surface missing
- Definition digest: NONE — surface missing
- Dependency-lock identity: NONE — surface missing

## Runtime Binding
- Capabilities: terminal (process inspection, service control), browser observation
- Resources: RidePilot :4103; run-all lifecycle; pgrep/kill/curl
- Environments: fallback proving ground
- Approvals: none
- Triggers/schedules: none

## Evidence
- docs/validation/evidence/vwo-014/nst-t002/user-path/transcript.txt
- docs/validation/evidence/vwo-014/nst-t002/user-path/browser-confirm.txt
- docs/validation/evidence/vwo-014/nst-t002/user-path/browser-confirm.png
- docs/validation/evidence/vwo-014/nst-t002/post-hoc/pids.txt
- docs/validation/evidence/vwo-014/nst-t002/post-hoc/pre-counts.txt
- docs/validation/evidence/vwo-014/nst-t002/post-hoc/post-counts.txt
- docs/validation/evidence/vwo-014/nst-t002/post-hoc/c1-oracles.txt

## Failure / Recovery
- Failure injected: the SIGSTOP wedge (the task's controlled failure)
- Recovery attempted: clean kill + relaunch (the task's restart path)
- Restart/session-loss behavior: durable state (JSON-file store) survived the restart — the durable-state discipline held
- Final outcome: PASS

## Product / UX Friction
The fixture's .run pidfiles made restart targeting trivial; without them the operator would rely on pgrep patterns.

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
- Scenario id: nst-t003
- Industry: n/a (seeded notes tree)
- Scenario: File discovery and text transformation with re-run
- User goal: Find every weekly-notes markdown file under the field-notes tree, convert each to a CSV row (date, site, note count), and build one combined CSV. Then do it again when I drop a differently-named, differently-ordered batch into a second folder.
- Teaching mode: DEMONSTRATE
- benchmark_version: 1.0.0
- Difficulty: L3 (catalog; agreed)

## User Path
1. Seeded batch 1: three weekly-notes markdown files under field-notes/ (plus an ignore-file distractor).
2. find-discovered the intended set (3 .md files; distractor excluded by the name pattern).
3. Parsed each (date/site fields; note count = semicolon-separated items) into one combined CSV.
4. Batch 2 (differently named `*-notes.md`, fields in a different order) dropped into a second folder; re-ran discovery + transform.

## Expected
Combined CSV rows equal an independent parse (field-by-field); run 2 produces correct output with no carried-over rows; discovery matched exactly the intended file set.

## Actual
Run 1: 3 rows, all fields equal the independent parse; Run 2: 2 rows correct, zero carry-over; discovery transcripts match the intended sets exactly (3 + 2 files).

## Workflow Identity
- Workflow: NONE — surface missing
- Version: NONE — surface missing
- Source revision: NONE — surface missing
- Definition digest: NONE — surface missing
- Dependency-lock identity: NONE — surface missing

## Runtime Binding
- Capabilities: filesystem discovery, text transformation
- Resources: seeded scratch trees under /tmp (worker-created fixture data per the task record)
- Environments: fallback proving ground
- Approvals: none
- Triggers/schedules: none

## Evidence
- docs/validation/evidence/vwo-014/nst-t003/user-path/transcript-run1.txt
- docs/validation/evidence/vwo-014/nst-t003/user-path/transcript-run2.txt
- docs/validation/evidence/vwo-014/nst-t003/user-path/find-run1.txt
- docs/validation/evidence/vwo-014/nst-t003/user-path/find-run2.txt
- docs/validation/evidence/vwo-014/nst-t003/user-path/weekly-summary-run1.csv
- docs/validation/evidence/vwo-014/nst-t003/user-path/weekly-summary-run2.csv
- docs/validation/evidence/vwo-014/nst-t003/post-hoc/c1-oracles.txt

## Failure / Recovery
- Failure injected: none
- Recovery attempted: none needed
- Restart/session-loss behavior: n/a
- Final outcome: PASS

## Product / UX Friction
none observed (the field-order change between batches is exactly why the parser keys off field names, not positions).

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
- Scenario id: nst-t004
- Industry: construction (SiteBuild)
- Scenario: Structured-data reshape
- User goal: Give me one CSV of all open purchase requests across the construction app — just id, vendor, amount, status — sorted by amount, biggest first.
- Teaching mode: HYBRID
- benchmark_version: 1.0.0
- Difficulty: L2 (catalog; agreed)

## User Path
1. Pulled all three projects' data from the SiteBuild APIs (the purchase requests live in the per-project detail API).
2. Filtered to open (submitted) requests.
3. Reshaped to id,vendor,amount,status and sorted by amount descending into one CSV.

## Expected
CSV header + rows equal an independent recomputation; sort order verified descending by amount.

## Actual
2 open PRs (prq-5001 $1,104,000; prq-5003 $87,000) — exactly the independent recompute, descending order verified.

## Workflow Identity
- Workflow: NONE — surface missing
- Version: NONE — surface missing
- Source revision: NONE — surface missing
- Definition digest: NONE — surface missing
- Dependency-lock identity: NONE — surface missing

## Runtime Binding
- Capabilities: api-tool-call, data reshape, file output
- Resources: SiteBuild :4101 public project-detail APIs
- Environments: fallback proving ground
- Approvals: none
- Triggers/schedules: none

## Evidence
- docs/validation/evidence/vwo-014/nst-t004/user-path/transcript.txt
- docs/validation/evidence/vwo-014/nst-t004/user-path/open-prs.csv
- docs/validation/evidence/vwo-014/nst-t004/post-hoc/c1-oracles.txt

## Failure / Recovery
- Failure injected: none
- Recovery attempted: none needed
- Restart/session-loss behavior: n/a
- Final outcome: PASS

## Product / UX Friction
The task names `/api/procurement`; the fixture exposes purchase requests via the per-project detail API (no procurement collection route) — the reshape pulls three endpoints instead of one (recorded as drift D-T4, clone wins).

## Security / Architecture Findings
- No findings

## Recommendation
- DEFER
- Proposed owner: n/a
- Verification required: none

## Worker Conclusion
passed (with D-T4 recorded).

---

## Scenario
- Scenario id: nst-t005
- Industry: n/a (seeded notes tree)
- Scenario: Archive and integrity workflow
- User goal: Bundle this month's note files into one compressed archive, extract it somewhere else, and prove nothing changed.
- Teaching mode: DEMONSTRATE
- benchmark_version: 1.0.0
- Difficulty: L1 (catalog; agreed)

## User Path
1. Created the tar.gz archive of the field-notes batch.
2. Extracted it at a second location.
3. Proved integrity: sha256 lists of both trees compared (diff clean); the archive listing checked against the intended file set.

## Expected
Archive created; extraction yields a tree whose checksums match file-for-file; archive listing contains exactly the intended file set.

## Actual
sha256 list equality PASS (diff clean); archive listing = the 3 week-notes + the distractor file (exactly the tree contents — the intended set).

## Workflow Identity
- Workflow: NONE — surface missing
- Version: NONE — surface missing
- Source revision: NONE — surface missing
- Definition digest: NONE — surface missing
- Dependency-lock identity: NONE — surface missing

## Runtime Binding
- Capabilities: terminal, archive operations, integrity checks
- Resources: the T003 batch-1 tree; tar/gzip/sha256sum
- Environments: fallback proving ground
- Approvals: none
- Triggers/schedules: none

## Evidence
- docs/validation/evidence/vwo-014/nst-t005/user-path/transcript.txt
- docs/validation/evidence/vwo-014/nst-t005/user-path/original-sha256.txt
- docs/validation/evidence/vwo-014/nst-t005/user-path/extracted-sha256.txt
- docs/validation/evidence/vwo-014/nst-t005/user-path/archive-listing.txt
- docs/validation/evidence/vwo-014/nst-t005/post-hoc/c1-oracles.txt

## Failure / Recovery
- Failure injected: none
- Recovery attempted: none needed
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
- Scenario id: nst-t006
- Industry: n/a (the validation repo itself)
- Scenario: Git operations at a pinned revision
- User goal: Clone the validation repo at exactly the recorded base SHA, create a branch, commit a small change to a scratch file, and show me the log proving the branch sits on the right base.
- Teaching mode: HYBRID
- benchmark_version: 1.0.0
- Difficulty: L1 (catalog; agreed)

## User Path
1. Cloned the public repo; checked out exactly the recorded base SHA.
2. Created branch t006/scratch-branch; committed a scratch file.
3. Proved the base: `git merge-base --is-ancestor <base> HEAD` exited 0; the log shows the scratch commit directly on the base.
4. Verified clean tree, correct branch name, and no push (ls-remote unchanged before/after).

## Expected
Ancestor check exits 0; HEAD contains the scratch change; working tree clean; branch name matches; no remote refs advanced.

## Actual
All five checks PASS (ancestor rc=0; SCRATCH-T006.txt in HEAD; porcelain empty; branch t006/scratch-branch; main's remote SHA identical before/after).

## Workflow Identity
- Workflow: NONE — surface missing
- Version: NONE — surface missing
- Source revision: NONE — surface missing
- Definition digest: NONE — surface missing
- Dependency-lock identity: NONE — surface missing

## Runtime Binding
- Capabilities: terminal, git operations
- Resources: payswapdotorg/codex public clone; git 2.47.3
- Environments: fallback proving ground
- Approvals: none
- Triggers/schedules: none

## Evidence
- docs/validation/evidence/vwo-014/nst-t006/user-path/transcript.txt
- docs/validation/evidence/vwo-014/nst-t006/post-hoc/c1-oracles.txt

## Failure / Recovery
- Failure injected: none
- Recovery attempted: none needed
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
- Scenario id: nst-t007
- Industry: n/a (the fixture ecosystem)
- Scenario: Build/test/lint gate
- User goal: Make sure the fixture world is healthy before I demo: run the fixture sweep, and if anything fails, run the reset and the sweep once more — tell me the final verdict.
- Teaching mode: INSTRUCT
- benchmark_version: 1.0.0
- Difficulty: L2 (catalog; agreed)

## User Path
1. Ran the fixture verify-sweep (first gate).
2. The first run FAILED (16 checks — the accumulated state mutations from the earlier lane runs; duplicate_event 409s on the golden paths) → the conditional branch fired.
3. Ran the reset (run-all --reset) and the sweep once more.
4. Reported the final verdict: 59/59 — healthy for demo.

## Expected
Final sweep transcript records 59 passed, 0 failed; if the first run failed, a reset+re-run is evidenced; the verdict message matches the final result.

## Actual
First run 43/16 (fail) → reset → re-run 59/0; the verdict string matches the transcript. The conditional branch was exercised both in structure and in fact.

## Workflow Identity
- Workflow: NONE — surface missing
- Version: NONE — surface missing
- Source revision: NONE — surface missing
- Definition digest: NONE — surface missing
- Dependency-lock identity: NONE — surface missing

## Runtime Binding
- Capabilities: terminal, build/test tooling, conditional reporting
- Resources: fixtures run-all/verify-sweep/reset scripts
- Environments: fallback proving ground
- Approvals: none
- Triggers/schedules: none

## Evidence
- docs/validation/evidence/vwo-014/nst-t007/user-path/transcript.txt
- docs/validation/evidence/vwo-014/nst-t007/post-hoc/c1-oracles.txt

## Failure / Recovery
- Failure injected: none (the first-run failure arose from real accumulated state — exactly the gate's purpose)
- Recovery attempted: reset + re-run (the task's conditional)
- Restart/session-loss behavior: n/a
- Final outcome: PASS

## Product / UX Friction
The sweep's goldens are stateful — consecutive sweeps without a reset fail on duplicate_event (by design; the reset discipline is documented in the sweep header).

## Security / Architecture Findings
- No findings

## Recommendation
- DEFER
- Proposed owner: n/a
- Verification required: none

## Worker Conclusion
passed — the conditional gate exercised end-to-end.

---

## Scenario
- Scenario id: nst-t008
- Industry: n/a (the fixture tree)
- Scenario: Code edit plus repository change
- User goal: Fix the typo in the fixture help text, make sure the sweep still passes, and commit the change on a branch with a message that says what you fixed.
- Teaching mode: HYBRID
- benchmark_version: 1.0.0
- Difficulty: L3 (catalog; agreed)

## User Path
1. Cloned a scratch repo at the pinned base; ran its fixtures as the dedicated instance (baseline sweep 59/59).
2. Applied the WRONG edit first (per the task script): corrupted the sweep-checked approval response string; restarted the software fixture from the edited clone; the sweep FAILED exactly on the approval golden (58/1).
3. Reverted the wrong edit; applied the correct edit (message punctuation clarity); restarted; sweep 59/59.
4. Committed on the branch with a message describing the fix; tree clean.

## Expected
The committed diff carries the repaired string; an intermediate failing check occurs and is recovered; the commit message describes the fix.

## Actual
Wrong-edit failure evidenced (approval golden FAIL); recovered to 59/59; committed diff carries the clarified message; commit message "fix(fixture): clarify the PR approval response message punctuation (NST-T008)"; tree clean.

## Workflow Identity
- Workflow: NONE — surface missing
- Version: NONE — surface missing
- Source revision: NONE — surface missing
- Definition digest: NONE — surface missing
- Dependency-lock identity: NONE — surface missing

## Runtime Binding
- Capabilities: terminal, code editing, recovery from failing checks
- Resources: scratch clone at the pinned base; the dedicated fixture instance; verify-sweep
- Environments: fallback proving ground
- Approvals: none
- Triggers/schedules: none

## Evidence
- docs/validation/evidence/vwo-014/nst-t008/user-path/transcript.txt
- docs/validation/evidence/vwo-014/nst-t008/post-hoc/c1-oracles.txt
- docs/validation/evidence/vwo-014/nst-t008/post-hoc/environment-identity.txt

## Failure / Recovery
- Failure injected: the wrong edit (the task script's controlled failure)
- Recovery attempted: revert + correct edit + re-gate (the task's recovery path)
- Restart/session-loss behavior: n/a
- Final outcome: PASS

## Product / UX Friction
none observed (the sweep gate catches response-string regressions precisely).

## Security / Architecture Findings
- Finding: premise drift — the fixture contains NO seeded typo in its help text (grep evidence); the task's edit→gate→recovery→commit structure was executed against a real sweep-checked response string, with the typo introduced, wrongly edited, recovered, and fixed per the task script (D-T8, recorded)
- Severity: P3
- Reproduction: grep the fixture tree for typo markers; the catalog presumes a seeded typo that does not exist
- Root cause: catalog authored against a fixture shape with a seeded typo
- Frozen invariant affected: benchmark fidelity

## Recommendation
- NEW WORK ORDER
- Proposed owner: harness (seed the typo the catalog presumes, or re-bind the task text)
- Verification required: the seeded typo present at base

## Worker Conclusion
passed — the full wrong-edit→failing-gate→recovery→commit cycle verified (D-T8 recorded).

---

## Scenario
- Scenario id: nst-t009
- Industry: construction (SiteBuild)
- Scenario: API-first task with CLI credential binding
- User goal: Approve this month's pending invoice from your terminal, without opening a browser — sign in through the CLI the way the product stores credentials, then approve it.
- Teaching mode: INSTRUCT
- benchmark_version: 1.0.0
- Difficulty: L2 (catalog; agreed)

## User Path
1. Fetched the credential from the product's approved binding path (/api/demo-hints — the fixture's documented credential surface) into an environment variable; never in the command text or evidence (redacted).
2. CLI login (curl POST /login with the cookie jar).
3. CLI approve (curl POST /api/invoices/approve with the bound session) for inv-9001.

## Expected
The invoice record is approved with the finance persona as actor; the approval used a credential from the product's approved binding path (credential text never in source/evidence); the terminal transcript shows login + approve with secrets redacted.

## Actual
inv-9001 approved; the event log carries "Invoice inv-9001 ($182,400) approved for payment by Priya Nair" + priya's auth.login; the credential came from /api/demo-hints at run time; the transcript shows the commands with the password REDACTED.

## Workflow Identity
- Workflow: NONE — surface missing
- Version: NONE — surface missing
- Source revision: NONE — surface missing
- Definition digest: NONE — surface missing
- Dependency-lock identity: NONE — surface missing

## Runtime Binding
- Capabilities: credential/resource binding for CLI/API use, api-tool-call
- Resources: SiteBuild :4101; priya.nair (finance_accountant, invoice:approve); credential from /api/demo-hints
- Environments: fallback proving ground
- Approvals: none
- Triggers/schedules: none

## Evidence
- docs/validation/evidence/vwo-014/nst-t009/user-path/transcript.txt
- docs/validation/evidence/vwo-014/nst-t009/post-hoc/events.json
- docs/validation/evidence/vwo-014/nst-t009/post-hoc/c1-oracles.txt

## Failure / Recovery
- Failure injected: none
- Recovery attempted: none needed
- Restart/session-loss behavior: n/a
- Final outcome: PASS

## Product / UX Friction
The CLI login is a raw form POST — a first-class CLI (or token command) would be the product-grade path; the credential binding surface (demo-hints) is the fixture's documented equivalent.

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
- Scenario id: nst-t010
- Industry: rideshare (RidePilot)
- Scenario: Failing pipeline with checkpoint resume
- User goal: Build the weekly ops summary by pulling each region's trip counts one region at a time, writing a checkpoint after each. The service will hiccup once (503) and leave a stale checkpoint behind — deal with both, and the summary must count each region exactly once.
- Teaching mode: HYBRID
- benchmark_version: 1.0.0
- Difficulty: L4 (catalog; agreed)

## User Path
1. A stale checkpoint was present before the run (older data for rg-1 + no run-id).
2. Ran the pipeline: per-region trip-count pulls (rg-1..rg-4), a checkpoint written after each region (run-id tagged).
3. The stale checkpoint was detected (run-id mismatch) and superseded — not trusted.
4. The service hiccup: the armed 503 fired once on the run's state-changing ops call (see D-T10) — observed, ledgered (request.rejected), and the clean retry succeeded.
5. The summary counted each region exactly once (checkpoint skip semantics).

## Expected
Final summary counts equal an independent recomputation with no region double-counted; the armed 503 occurred mid-run and the pipeline recovered; the stale checkpoint was detected and superseded.

## Actual
Counts {rg-1:1, rg-2:2, rg-3:0, rg-4:1} == independent recompute; the armed 503 fired (request.rejected ledgered once) and the clean retry succeeded (HTTP 303 PRG success); the stale checkpoint was detected and superseded (transcript); exactly-once semantics held (no skip/dup).

## Workflow Identity
- Workflow: NONE — surface missing
- Version: NONE — surface missing
- Source revision: NONE — surface missing
- Definition digest: NONE — surface missing
- Dependency-lock identity: NONE — surface missing

## Runtime Binding
- Capabilities: terminal, resilient pipelines, idempotent checkpointing
- Resources: RidePilot :4103 /api/trips (public reads); the service_unavailable switch on a state-changing ops call; checkpoint file; farah.khan (demo credential from /api/demo-hints, redacted)
- Environments: fallback proving ground
- Approvals: none
- Triggers/schedules: none

## Evidence
- docs/validation/evidence/vwo-014/nst-t010/user-path/transcript.txt
- docs/validation/evidence/vwo-014/nst-t010/post-hoc/c1-oracles.txt
- docs/validation/evidence/vwo-014/nst-t010/post-hoc/environment-identity.txt

## Failure / Recovery
- Failure injected: the armed 503 (once) + the pre-seeded stale checkpoint (the task's two controlled failures)
- Recovery attempted: clean retry after the 503; stale-checkpoint detection + supersede; checkpoint skip for exactly-once
- Restart/session-loss behavior: the checkpoint's run-id tagging makes cross-run resume safe (stale detection proven)
- Final outcome: PASS

## Product / UX Friction
The switch's read-exemption means read-only pipelines can't see the 503 through the documented mechanism (D-T10) — an operator arming "the service hiccup" for a read pipeline would need a different lever.

## Security / Architecture Findings
- Finding: drift D-T10 — the rideshare service_unavailable switch applies to state-changing requests only (FAILURES.md: "reads still work"); the catalog's armed-503-on-the-pull premise cannot fire on the read-only /api/trips (verified live: GET with the switch returns 200)
- Severity: P3
- Reproduction: GET /api/trips?failure=service_unavailable returns 200 with data
- Root cause: the catalog's T010 was authored assuming the switch covers reads
- Frozen invariant affected: benchmark fidelity

## Recommendation
- NEW WORK ORDER
- Proposed owner: harness (either extend the switch to reads for this fixture or re-bind the task's 503 surface)
- Verification required: the armed 503 observable on the task's actual pull surface

## Worker Conclusion
passed — exactly-once counts, stale-checkpoint supersede, and the 503+retry all evidenced (D-T10 recorded).

---

### Honest deviations list (program level)

1. **Delivery mode**: TL local delivery (lesson-120 protocol) — recorded (same as VWO-012/013).
2. **D-T4**: no `/api/procurement` collection route exists; purchase requests pulled via the per-project detail APIs (clone wins).
3. **D-T8**: no seeded typo exists in the fixture help text — the task's edit→gate→recovery→commit structure executed against a real sweep-checked response string (the typo introduced then fixed per the task script); premise drift recorded as a finding.
4. **D-T10**: the service_unavailable switch exempts read-only requests — the armed 503 demonstrated on a state-changing ops call within the same run (ledgered + retried); drift recorded as a finding.
5. **T008 gate hygiene**: the sweep's goldens are stateful — consecutive sweeps require a reset between them (documented in the sweep; followed in the runs).
6. **Scratch fixtures**: T008's gate ran against a dedicated fixture instance started from the scratch clone (ports reused after stopping the main instance; main fixtures restored after the run).

### Self-check (recorded results)

- Every NST-T task (T001..T010) has a run record: YES (10 scenario blocks).
- Every [C1]/[C2] criterion has a verdict + evidence pointer: YES.
- Every failure/limitation has exactly one classification: 3 environment-specific-limitation drifts (D-T4, D-T8 premise, D-T10) + 2 P3 findings recommending harness alignment. YES.
- The report cites the exact base SHA and benchmark_version: YES.
- No model-text-as-evidence: YES — every verdict cites an artifact under docs/validation/evidence/vwo-014/.
- The catalog was not edited: `git diff --stat` shows only additive paths under docs/validation/reports/VWO-014-report.md and docs/validation/evidence/vwo-014/ (verified before commit).
