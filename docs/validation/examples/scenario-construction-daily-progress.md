# Worked Example — Catalog Scenario File (filled)

**Purpose:** this file demonstrates the COMPLETE scenario-entry format at
full detail — it is the unambiguous template for (a) reading what a catalog
entry gives you, and (b) adding a NEW scenario to
`docs/validation/scenarios/SCENARIO-CATALOG.md`. It duplicates the catalog
entry for `construction-daily-progress` with every field fully populated and
annotated with what makes it checkable.

**Status: EXAMPLE ONLY — the canonical entry is the one in
`docs/validation/scenarios/SCENARIO-CATALOG.md` §4.1. When running this
scenario, follow the catalog entry; this file exists to disambiguate the
format, not to add a second registry.**

---

## Binding block (machine-checkable — exact format required)

```yaml
scenario-id: construction-daily-progress
industry: construction
teaching-mode: DEMONSTRATE
app-dir: docs/validation/fixtures/construction
app-name: SiteBuild Field Suite
port: 4101
terminal-surface: none
primary-owner: VWO-004
```

Annotating each line (what the field promises, and who checks it):

- `scenario-id` — the stable slug (`[a-z0-9-]+`). Referenced verbatim in
  reports (`- Scenario id:`) and as the evidence directory name
  `evidence/<VWO-ID>/<scenario-id>/`. Checked by
  `validate-report.py --catalog`.
- `industry` — one of construction/software/rideshare/media/marketplace
  (industry rows) or `adversarial` (cross-cutting rows). Maps the entry to
  the coverage table (SCENARIO-CATALOG.md §6) cross-checked against
  prompt-02.
- `teaching-mode` — the canonical mode; TEACHING-MODE-MATRIX.md gives the
  per-mode procedure for this value. Deviations in a run are validator
  warnings (strict: failures).
- `app-dir` — MUST exist in the repository (checked by the VWO-003
  scenario↔fixture cross-check; no dangling bindings).
- `app-name`, `port` — the VWO-002 fixture identity (see
  `docs/validation/fixtures/README.md` §1); the port is the default, every
  app accepts `--port`.
- `terminal-surface` — `none`, or the fixture's terminal client path when
  the scenario uses it (only `software/deploy-console.js` exists today).
- `primary-owner` — the Work Order whose mandatory list this scenario
  serves; the "Also exercised by" column of the catalog selection table
  lists secondary owners.

## Persona

Marco Silva, site foreman (`marco.silva`, role `site_foreman`, permissions
`progress:create`, `safety:report`). He runs the Harborview Tower site
(`p-101`, 62% complete at seed). Secondary: Dana Reyes, project manager
(`dana.reyes`) — the notification recipient.

*A new worker reads this and knows exactly which seeded user to sign in as;
the persona table also renders on the fixture's `/login` page and at
`/api/demo-hints`.*

## User goal

Submit today's site progress report (percent complete, tasks completed,
photos, notes) so the PM sees it without a phone call — and, while doing
exactly that, teach Codex Universal the daily-reporting routine in
DEMONSTRATE mode (the system observes the foreman doing his normal work).

## User path (human-observable steps)

1. Open `http://localhost:4101/login`; sign in as `marco.silva` with the
   demo password from the login page's persona table (or `/api/demo-hints`).
2. Open the project page `/projects/p-101` (Harborview Tower).
3. Fill the daily report form: percent complete `68`, task `t-101c`, photo
   `a-101`, notes "Slab pour on 13 finished"; click **Submit daily report**.
4. Observe the green flash ("Progress report pr-XXXX saved … (68%)"), the
   new report row, and the updated project percent.
5. Check `/notifications` (as `marco.silva`, or sign out and in as
   `dana.reyes`): the PM notification must show `delivered`.
6. **Codex Universal surface (DEMONSTRATE):** open the product's teaching
   surface, replay steps 2–5 while it observes, then request the compiled
   workflow and review what it inferred (steps, resource binding to
   SiteBuild, notification side-effect, daily cadence).
7. Submit the same report again for the same date — observe the duplicate
   rejection flash.

*Every step names a page/button a human can see; none is an internal API
call. Step 6 names the product surface generically — if it does not exist in
the current runtime, record "teaching surface missing" as a finding instead
of fabricating it.*

## Expected observable outcomes

- The report row appears on the project page and the project percent is
  updated (62 → 68).
- The PM's notification row exists with status `delivered`.
- Re-submission for the same date produces a red flash:
  "A progress report for p-101 on 2026-09-12 already exists — daily reports
  are one per project per day. [duplicate_event]" (HTTP 409 semantics).
- The compiled workflow (if the surface exists) captures: the three
  form fields as inputs, one-report-per-project-per-day, and the PM
  notification as a side-effect.
- A normal person completes this without knowing anything about workflow
  internals — if any step requires internal knowledge, that is a finding.

## Failure switches & recovery expectation

| Injection | Expected behavior | Recovery expectation |
|---|---|---|
| `?failure=notification_failure` on the submit | save succeeds; warning visible; notification row `failed`; `notification.failed` event | human informed, not silent; next-day submit works; no state corruption |
| `?failure=service_unavailable` on the submit | 503 with named dependency `site-photo-store`; NO partial report row | retry after the switch clears succeeds |
| natural duplicate (same date) | 409 `[duplicate_event]` flash | the human understands: one report per day |
| `?failure=duplicate_event` on any other mutation | 409 replay rejection | request rejected as replay, state unchanged |

## Evidence to capture

**user-path/ (what the human saw):**

- `…Z-step01-login.png` or transcript of the login page with the persona table
- `…Z-step03-report-form.png` — the filled form before submit
- `…Z-step04-report-saved.png` — flash + report row + updated percent
- `…Z-step05-notification-delivered.txt` — the notification row text
- `…Z-step06-compiled-workflow.png` — the proposal/review screen (or the
  "surface missing" note)
- `…Z-step07-duplicate-rejected.png` — the red flash

**post-hoc/ (diagnostics, only after the human path):**

- `…Z-events-progress-submitted.txt` — `GET /api/events?type=progress.submitted`
- `…Z-project-state.txt` — `GET /api/projects/p-101` (percent + report list)
- `…Z-reset-verification.txt` — `POST /api/reset` then percent back to 62
  (seed value), proving deterministic reseed

## Approvals / triggers exercised

- No approval gate for the foreman (creation only) — the PM's visibility via
  notification is the human gate the workflow must preserve.
- The taught workflow carries: a **daily schedule trigger** (report each
  day), **one-per-day idempotency**, and a **notification side-effect** to
  the PM — exactly the semantics DEMONSTRATE must capture.

---

## How a new worker uses this in minutes (the acceptance test)

1. Select the scenario in SCENARIO-CATALOG.md §1 (row 1) — persona, mode,
   app, port are in the row and the entry.
2. `bash docs/validation/scripts/reset-scenario.sh construction` — app
   restarted and reseeded.
3. Execute user-path steps 1–7 above, capturing per the evidence list.
4. Record actual outcomes vs the expected list; classify deltas per
   REPORT-SCHEMA.md §13 severity vocabulary.
5. Write the scenario block into `docs/validation/reports/VWO-004-report.md`
   (worked shape: `docs/validation/examples/VWO-XXX-example-report.md`).
6. `python3 docs/validation/scripts/validate-report.py <report> --catalog
   docs/validation/scenarios/SCENARIO-CATALOG.md` — conforming.
