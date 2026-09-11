# VWO-XXX Example Report — construction-daily-progress (Class A)

**THIS IS A WORKED EXAMPLE, NOT A REAL RUN.** It demonstrates a conforming
Class A report (single scenario block) that passes
`python3 docs/validation/scripts/validate-report.py --catalog` against the
real catalog, with example evidence files under
`docs/validation/examples/evidence/VWO-XXX/`. Copy the SHAPE, never the
values. Runtime behavior described here is illustrative.

## Identity

- Work Order: VWO-XXX (example — substitute your real WO id)
- Worker persona: enterprise operations user (example)
- Base branch: main
- Base SHA: e2a5e968581e4feb60b754a848824d7c05a583a9
- Head SHA: 0000000000000000000000000000000000000000
- Validation environment: agent sandbox fallback proving ground (browser
  via headless chromium; no E2B workspace)
- E2B template/workspace identity, if used: N/A — E2B integration absent
  (fallback record, VWO-001)
- Codex Universal runtime/application SHA: NONE — example: teaching/compile
  surface not exposed in this environment; recorded as missing product
  surface (see Actual), not fabricated
- Date/time window: 2026-09-12T10:00Z–2026-09-12T10:45Z (UTC)

## Scenario

- Scenario id: construction-daily-progress
- Industry: construction
- Scenario: daily site progress report teaching (DEMONSTRATE)
- Teaching mode: DEMONSTRATE
- User goal: submit Harborview Tower's daily progress report as the site
  foreman and teach the routine by demonstration

## User Path

1. Opened `http://localhost:4101/login` and signed in as `marco.silva`
   (persona table on the login page).
2. Opened `/projects/p-101`, filled the daily report form (68%, task
   `t-101c`, photo `a-101`, notes) and clicked **Submit daily report**.
3. Observed the confirmation flash and the updated project percent.
4. Opened `/notifications` and read the PM notification row.
5. Attempted the same-day re-submission and observed the duplicate
   rejection flash.
6. Opened the Codex Universal teaching surface to replay steps 2–4 under
   observation — SURFACE NOT FOUND (no teaching surface exposed in this
   runtime); recorded instead of fabricating.

## Expected

The report saves and is visible on the project page; the project percent
updates (62 → 68); the PM notification shows `delivered`; a same-day
re-submission is rejected with a clear `[duplicate_event]` flash; the
teaching surface observes steps 2–4 and offers a compiled workflow
capturing the daily cadence, the form inputs, and the notification
side-effect.

## Actual

Steps 1–5 behaved as expected (example transcript
`user-path/…step04-report-saved.txt`; duplicate flash
`user-path/…step07-duplicate-rejected.txt`). Step 6 could not be
performed: no teaching surface is exposed by the current runtime in this
environment — the workflow could not be taught by demonstration. Recorded
as a missing product surface (P1 below), not worked around with internal
workflow IR.

## Workflow Identity

- Workflow: NONE — surface missing (no workflow could be created)
- Version: NONE — surface missing
- Source revision: NONE — surface missing
- Definition digest: NONE — surface missing
- Dependency-lock identity: NONE — surface missing

## Runtime Binding

- Capabilities: browser use (fixture app navigation); teaching capability
  NOT exercised — surface missing
- Resources: SiteBuild Field Suite fixture app (construction family,
  port 4101), seeded user marco.silva
- Environments: agent sandbox fallback proving ground; no E2B workspace
- Approvals: none required for creation; PM notification is the human
  gate (observed `delivered`)
- Triggers/schedules: daily-report cadence intended; NOT exercised —
  teaching surface missing

## Evidence

- docs/validation/examples/evidence/VWO-XXX/construction-daily-progress/user-path/20260912T101500Z-step04-report-saved.txt
- docs/validation/examples/evidence/VWO-XXX/construction-daily-progress/user-path/20260912T101530Z-step07-duplicate-rejected.txt
- docs/validation/examples/evidence/VWO-XXX/construction-daily-progress/post-hoc/20260912T101600Z-reset-verification.txt

## Failure / Recovery

- Failure injected: same-day duplicate submission (natural) and
  `?failure=notification_failure` on the submit (switch)
- Recovery attempted: re-observation of notification state after the
  switch (row shows `failed` with visible warning; save intact)
- Restart/session-loss behavior: not attempted — no runtime process to
  restart in this example (see adv-session-loss-restart for the real test)
- Final outcome: fixture-side behavior correct; teaching path blocked by
  the missing surface

## Product / UX Friction

The foreman's app path is clear (login → project → form → flash). The
friction point is the missing teaching surface: a normal person has
nowhere to go after completing the demo steps — no "teach Codex this
routine" entry point exists in the product surface they can see.

## Security / Architecture Findings

- Finding: teaching surface absent — the DEMONSTRATE mode cannot be
  exercised end-to-end by a normal person; the acceptance question fails
  at step 6 (a normal person cannot use Codex Universal end-to-end
  without internals because no product surface exposes the capability at
  all).
- Severity: P1
- Reproduction: complete user-path steps 1–5 in SiteBuild; attempt to
  locate any teaching/observation surface; none is exposed.
- Root cause: runtime surface not present in this environment build.
- Frozen invariant affected: none (absence, not violation); recorded per
  VALIDATION-PROGRAM.md §8 (report missing surfaces, do not fabricate).

## Recommendation

- NEW WORK ORDER
- Proposed owner: Tech Lead (surface ownership TBD by architecture)
- Verification required: re-run this scenario after the surface exists;
  user-path step 6 must complete without internals.

## Worker Conclusion

The scenario is **blocked** by a missing product surface (teaching);
fixture-side steps passed. Not a pass: the acceptance question cannot be
answered while step 6 is impossible.
