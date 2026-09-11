# VWO-002 — Synthetic Enterprise Application Ecosystem — Implementation Report

## Identity

- Work Order: VWO-002 (Wave 0, proving-ground bootstrap)
- Worker persona: VWO-002 Worker (validation-infrastructure specialist)
- Base branch: `main` @ `926c177a5515a1bf1316d7ef1bacb9b6d2d53611` (verified: `git rev-parse 926c177a5^{commit}` = `origin/main` HEAD at dispatch)
- Branch: `vwo-002/synthetic-apps`
- Head SHA: see `git rev-parse vwo-002/synthetic-apps` (commit `feat(validation): VWO-002 synthetic enterprise application ecosystem`)
- Validation environment: agent sandbox (Linux, node v24.19.0, bun 1.3.14); apps bound to 127.0.0.1 ports 4101-4105
- **Status:** MERGED via PR (squash) — independently verified by the Tech Lead: verify-sweep 59/59 on six consecutive clean runs (node v24.19.0) after one integration fix to the sweep's expect() helper (`echo | grep -q` under `set -o pipefail` SIGPIPE-raced — see the Tech-Lead verification addendum at the end of this report)
- Date/time window: 2026-09-11 (fixture clock; real UTC timestamps in git)
- Scope: `docs/validation/fixtures/**`, `docs/validation/reports/VWO-002-report.md`, `docs/validation/reports/vwo-002-screens/*` only. No codex-rs changes, no semantic-contract changes.

## Deliverable

Five self-contained, stateful, browser-accessible synthetic enterprise applications under `docs/validation/fixtures/`, sharing a tiny std-lib runtime (`_lib/runtime.js`, `_lib/web.js`):

| Family | App | Port | Entry | Terminal surface |
|---|---|---|---|---|
| construction | SiteBuild Field Suite | 4101 | http://localhost:4101/ | — |
| software (Google-like) | ForgeOps | 4102 | http://localhost:4102/ | `software/deploy-console.js` (xterm-able interactive client) |
| ride-share | RidePilot | 4103 | http://localhost:4103/ | — |
| media | PressRoom | 4104 | http://localhost:4104/ | — |
| marketplace | FlowMart | 4105 | http://localhost:4105/ | — |

Catalog with inventory, users/roles, seed data, failure controls, endpoints and reset procedure: `docs/validation/fixtures/README.md`. Per-family failure tables: `docs/validation/fixtures/<family>/FAILURES.md`. Reproducible launcher: `fixtures/run-all.sh` (start/stop/reset). Deterministic app-level self-test: `fixtures/verify-sweep.sh` (fixture smoke test — explicitly NOT the VWO-003 validation harness).

## Implementation summary

- **Runtime (`_lib/runtime.js`, 506 LoC, `_lib/web.js`, 219 LoC):** std-lib only (`node:http/fs/path/crypto`) — zero external dependencies, no database, no SaaS. HTTP router with path params; JSON-file state with atomic tmp+rename writes under `<family>/data/` (gitignored, auto-seeded); per-user session tokens/passwords assembled at runtime from fragments (no literal credentials in source); event feed with cap + `since`/`type` filters; in-app notifications; idempotency tracking; deterministic failure-switch guards; server-rendered HTML shell with shared CSS (sticky footer, responsive, semantic landmarks, ARIA roles, `sr-only` where applicable). Runs under node ≥18 **and** bun (verified).
- **Uniform app contract (all five families):** HTML pages + plain form POST → 303 redirect with flash (PRG — no client JavaScript required for any human path); every mutation has a JSON API twin (`POST /api/...`) hitting the same domain op, so API/tool paths and human paths produce identical state and events; common endpoints `/healthz`, `/login`, `/api/login`, `/api/whoami`, `/api/demo-hints`, `/events` (JSON by default, HTML for browsers), `/api/events`, `/failures`, `/api/failures`, `POST /reset`, `/api/reset`.
- **Deterministic failure switches (all 7 canonical, all 5 families):** `service_unavailable` (global 503 on mutations with named synthetic dependency), `notification_failure` (op succeeds; notification/broadcast/social delivery stored `failed`), `missing_asset` (404 dangling asset), `permission_denied` (403 even for authorized users), `data_conflict` (409 concurrent-change/optimistic-concurrency), `duplicate_event` (409 replay; idempotencyKey replays always rejected), `stale_entitlement` (409 expired license/contract/permit/entitlement). Activated per request via `?failure=<switch>` or `X-Failure-Switch` header; unknown names → 400 with known list; switches take precedence over natural record state (documented contract). Natural (non-simulated) equivalents exist in every family's seed (expired SteelCo contract, missing insurance doc, expired driver permit, expired partner-app license, expired marketplace trial, self-approval policy, one-per-day progress reports, immutable published versions…).
- **Fixture clock:** each world believes today = 2026-09-12 (`state.meta.today`), so date-dependent seed semantics are deterministic regardless of real wall clock; event timestamps are real.
- **Families:** construction (projects/tasks, site photos, contractors, procurement w/ approval + PO issue, invoices, safety incidents, documents, notifications); software (git forge w/ PRs + CI runs, merge gate needing approval+green CI, staging→production promotion approval, rollback, incidents → service health, issues, docs w/ optimistic versioning, teams/ownership directory); rideshare (public driver intake, screening w/ document checks, activation w/ permit validity, support tickets w/ escalation, surge ops + driver broadcasts, SVG region map, payouts w/ permit gate, incidents); media (story lifecycle draft→review→approve→publish with hero-asset gate, asset repository + renditions, multi-channel distribution w/ per-channel license validity, corrections + re-distribution, social scheduling/posting with engagement); marketplace (catalog/search, listing + immutable versions with sha256 manifest digests + provenance records, install w/ auto-trial entitlement, configuration, upgrade/rollback, entitlement grant/revoke, provenance verification, one-review-per-org).
- **Desktop-capable surface:** `software/deploy-console.js` — honest interactive readline terminal client over the ForgeOps API (login, deployments, promote, rollback, incidents, events, arming a failure switch for the next command). Works in a real xterm and with piped stdin for scripted runs. It is a client only; E2B desktop environments are VWO-001's surface.

## Verification — commands and exact results

All commands run from `docs/validation/fixtures/` in the sandbox (node v24.19.0; bun 1.3.14). Servers were started fresh from seeds unless noted.

### 1. Boot + health

```
$ ./run-all.sh --reset          # starts all five apps, health-checks each, resets to seed
All five VWO-002 fixture apps are running: ... (construction 4101, software 4102, rideshare 4103, media 4104, marketplace 4105)

$ curl -s http://localhost:4101/healthz     (one of five, all equivalent)
{ "ok": true, "family": "construction", "app": "SiteBuild Field Suite", "port": 4101, "version": "1.0.0", ... }

$ cd construction && bun server.js --port 4199 && curl -s localhost:4199/healthz
{ "ok": true, "family": "construction", ... }      # bun compatibility verified, then killed
```

### 2. Deterministic sweep (committed script): 59/59 PASS

```
$ ./verify-sweep.sh
== construction (SiteBuild, 4101) ==
  PASS  golden: submit daily progress (HTTP 200)
  PASS  switch service_unavailable (HTTP 503)
  PASS  switch notification_failure (HTTP 200)
  PASS  switch missing_asset (HTTP 404)
  PASS  switch permission_denied (HTTP 403)
  PASS  switch data_conflict (HTTP 409)
  PASS  switch duplicate_event (HTTP 409)
  PASS  switch stale_entitlement (HTTP 409)
  PASS  natural stale_entitlement (SteelCo contract expired) (HTTP 409)
== software (ForgeOps, 4102) ==
  PASS  golden: open PR + auto-CI (HTTP 200)
  PASS  golden: code owner approves (HTTP 200)
  PASS  golden: merge → staging deploy (HTTP 200)
  PASS  golden: promote to production (HTTP 200)
  PASS  switch service_unavailable (HTTP 503)
  ... (all 7 switches PASS)
  PASS  natural permission_denied (self-approval blocked) (HTTP 403)
== rideshare (RidePilot, 4103) ==
  PASS  golden: public intake (no auth) (HTTP 200)
  PASS  golden: screening approval (HTTP 200)
  PASS  golden: driver activation (HTTP 200)
  ... (all 7 switches PASS)
  PASS  natural stale_entitlement (da-0303 expired permit) (HTTP 409)
  PASS  natural duplicate_event (same plate twice) (HTTP 409)
== media (PressRoom, 4104) ==
  PASS  golden: writer creates draft (HTTP 200)
  PASS  golden: attach hero image (HTTP 200)
  PASS  golden: submit for review (HTTP 200)
  PASS  golden: editor approves (HTTP 200)
  PASS  golden: publish to channels (HTTP 200)
  ... (all 7 switches PASS — stale_entitlement is a partial publish: 200 + failed channel job)
  PASS  natural missing_asset (approved, no hero) (HTTP 404)
== marketplace (FlowMart, 4105) ==
  PASS  golden: Northwind installs (active entitlement) (HTTP 200)
  PASS  golden: publisher releases 1.3.0 (HTTP 200)
  PASS  golden: upgrade 1.2.0 → 1.3.0 (HTTP 200)
  PASS  golden: rollback to 1.2.0 (HTTP 200)
  PASS  golden: provenance digest verifies (HTTP 200)
  ... (all 7 switches PASS)
  PASS  natural stale_entitlement (Northwind expired trial) (HTTP 409)

RESULT: 59 passed, 0 failed
```

Representative raw outputs (exact):

```
# construction golden path (API twin of the browser form)
$ curl -s "http://localhost:4101/api/progress?token=<marco>" -H 'content-type: application/json' \
    -d '{"projectId":"p-101","reportDate":"2026-09-13","percentComplete":65,"taskIds":["t-101c"],"photoIds":["a-101"]}'
{ "ok": true, "message": "Progress report pr-4002 saved for Harborview Tower (65%).", ...,
  "notification": { "status": "delivered", ... } }

# notification_failure switch — op succeeds, notification observable as failed
$ curl -s ".../api/progress?failure=notification_failure&token=<marco>" -d '{...}'
{ "ok": true, "warning": "Saved, but the notification to the project manager could not be delivered ...",
  "notification": { "status": "failed", ... } }
$ curl -s ".../api/events?type=notification.failed"
{ "events": [ { "type": "notification.failed", "summary": "Notification \"...\" to dana.reyes FAILED to deliver (simulated switch)" } ] }

# natural stale_entitlement — approving PR prq-5001 (SteelCo contract expired 2026-08-01 under fixture clock)
{ "ok": false, "error": "stale_entitlement",
  "message": "Vendor contract for SteelCo Fabrication expired on 2026-08-01 — ..." }  HTTP 409

# software golden chain (event feed excerpt, newest first)
ev-0013 service.version_bumped   | Service payments-api now serving version 4.8.2
ev-0012 deployment.promoted      | payments-service 4.8.2 promoted to production by Raj Patel (from dep-1104)
ev-0009 pr.merged                | PR #303 "..." merged by Raj Patel; payments-service staging → 4.8.2
ev-0007 pr.approved              | PR #303 "..." approved by Noor Haddad

# marketplace rollback
$ curl -s ".../api/installs/rollback?token=<iris>" -d '{"installId":"ins-0402"}'
{ "ok": true, "message": "Install ins-0402 rolled back 1.3.0 → 1.2.0." }

# reset determinism
$ curl -s -X POST http://localhost:4101/api/reset   → { "ok": true, "message": "Seed state restored.", ... }
$ curl -s .../api/projects → percentComplete back to seed values [62, 35, 88]
```

### 3. Real-browser verification (agent-browser CLI over Chromium)

Golden human path per family — login → navigate → fill form → submit → visible result (cookies, no client JS):

| App | Browser steps (exact) | Observed result |
|---|---|---|
| SiteBuild | `open /login` → fill marco.silva/demo-pass-marco → click Sign in → `open /projects/p-101` → set % = 68, notes → click "Submit daily report" | 303 → `/projects/p-101?ok=Progress+report+pr-4003+saved+...+%2868%25%29`; flash + report row + `68%` visible in DOM |
| SiteBuild (failure) | click "Submit daily report" again (same date) | 303 → `?err=...already+exists...+%5Bduplicate_event%5D`; red flash visible: "A progress report for p-101 on 2026-09-12 already exists — daily reports are one per project per day. [duplicate_event]" |
| ForgeOps | login ari.klein → `open /repos/r-1` → fill PR title/branch → click "Open PR" | 303 → `/prs/pr-0305?ok=PR+%23304+created%3B+CI+run+run-0074+passed.`; PR page shows CI pipeline `passed` |
| RidePilot | (cookies cleared, no login) `open /intake` → fill name/phone/plate → click "Submit application" | 303 → `/intake?ok=Application+da-0306+submitted+for+Downtown+Core...`; flash visible |
| PressRoom | login hana.kim → `open /desk` → fill title/body → click "Create draft" | 303 → `/stories/st-0207?ok=Story+st-0207+...+created+as+a+draft.`; story page renders |
| FlowMart | login iris.chen → `open /catalog?q=triage` → click "Install 2.4.0" | 303 → `/installs?ok=Installed+%22Support+Ticket+Triage+Assistant%22+v2.4.0...+ins-0403...`; installs page shows both org installs |
| FlowMart (natural failure) | `open /catalog?q=warehouse` → click "Install 1.3.0" (org already has active install) | 303 → `?err=...already+has+an+active+install...+%5Bduplicate_event%5D`; red flash visible |

Full-page screenshots committed under `docs/validation/reports/vwo-002-screens/` (construction-project.png, software-pr.png, rideshare-map.png, media-story.png, marketplace-installs.png).

### 4. Terminal desktop console

```
$ cd software && printf 'login raj.patel demo-pass-raj\ndeployments r-1\nswitch data_conflict\npromote dep-1102\nquit\n' | node deploy-console.js
signed in as Raj Patel (release_manager). ...
deployments (5): dep-1106 production 4.8.1 succeeded ... dep-1102 production 4.8.1 succeeded ...
failure switch armed for next command: data_conflict
FAILED 409: data_conflict — Deployment dep-1102 was already promoted by another release manager (simulated).
```

Interactive (TTY) mode prompts `forgeops>` and is fully xterm-able; piped mode serializes commands and exits cleanly.

### 5. Static checks

- `node --check` passes on all 29 fixture JS files.
- `bash -n` passes on `run-all.sh` and `verify-sweep.sh`.
- File sizes: every app file ≤ 262 LoC; shared infra `_lib/runtime.js` is 506 LoC (largest file in the change — shared plumbing reused by all five apps, documented exception to the per-app ~400 LoC guidance).
- No credentials: `grep` for `password`/`token` literals in source finds only fragment arrays (`tokenParts`/`passwordParts`) that the runtime joins; assembled values exist only at runtime and are surfaced through `/api/demo-hints` and the login page.

## Acceptance-criteria evidence (packet → artifacts)

| VWO-002 requirement | Evidence |
|---|---|
| Required families: construction | `fixtures/construction/` — projects/tasks, progress/photos, contractors, budgets/procurement, invoices, safety, documents, notifications; sweep 9/9 |
| … software/Google-like | `fixtures/software/` — git forge, issues, CI dashboard, deployment console (+terminal client), incident console, service dashboard, docs, ownership directory; sweep 13/13 |
| … ride-share | `fixtures/rideshare/` — onboarding (public intake), support, ops dashboard, trips/regions, SVG map, payments/earnings, incidents, communications (broadcasts); sweep 12/12 |
| … media | `fixtures/media/` — CMS/editorial, asset repository, image/video workflow (renditions), approvals, publication/distribution, corrections, social dashboard; sweep 13/13 |
| … marketplace | `fixtures/marketplace/` — catalog/search, listing, installation/configuration, entitlements/licenses, upgrades, rollback, attribution/provenance; sweep 13/13 |
| Real state, not static screenshots | JSON-file store, atomic writes, mutations from UI and API twins; `POST /reset` restores exact seed (verified: percentComplete [62,35,88] restored) |
| Browser-accessible interfaces | 7-row browser table above; 5 committed screenshots; server-rendered HTML with no client-side JS required |
| At least one desktop-capable application surface | `software/deploy-console.js` (interactive + piped, verified) — honestly labeled terminal app; E2B desktop is VWO-001's question |
| APIs/events corresponding to important UI operations | every `/ui/...` form has a `/api/...` twin (see `fixtures/README.md` §5); event feed verified (progress.submitted, pr.merged, deployment.promoted, service.version_bumped, driver.activated, story.published, distribution.sent, package.installed, entitlement.revoked, notification.failed …) |
| Seeded users, roles, permissions, representative data | 27 personas across 5 apps (`fixtures/README.md` §2, `/api/demo-hints`); permission checks enforced (self-approval blocks, intern/install:manage denials verified) |
| Deterministic failure switches ×7 | sweep: 35/35 switch cases PASS (7 per family) + natural equivalents; per-family FAILURES.md tables |
| Reset/seed capability between scenarios | `POST /reset`, `POST /api/reset`, dashboard button, `run-all.sh --reset`; `world.reset` event recorded each time |
| No production credentials or sensitive real-world data | fragment-assembled demo credentials only; all companies/persons fictional; `.gitignore` excludes runtime state |
| Forbidden: encoding workflow semantics into fixtures | NONE — fixtures model ordinary enterprise behavior; FlowMart packages are opaque artifacts (manifest+digest+provenance) with no step definitions and no execution (noted in `fixtures/marketplace/seed.js` + README §7) |
| Forbidden: creating a second workflow engine | NONE — `_lib/runtime.js` is HTTP/JSON/file plumbing (no scheduling, triggers, orchestration) |
| Forbidden: hard-coding the validation suite to one UI automation library | NONE — plain HTML forms; the verification sweep is pure HTTP; browser pass used agent-browser but any driver (or none) works |
| Forbidden: static mocks where stateful behavior is cheap | NONE — every page reflects live state (browser duplicate-submit, upgrade-available logic, per-channel publish failures all verified) |
| Acceptance: ≥1 complete workflow via a normal human interaction path + one failure path per family | golden + failure paths exercised per family via BOTH curl/API and a real browser (tables above); 59/59 sweep |
| Acceptance: workers can use the applications without modifying Codex Universal internals | zero codex-rs changes; `git diff --stat main..HEAD` touches only `docs/validation/fixtures/**` and `docs/validation/reports/**` |
| Verification deliverables (setup/reset, inventory, users/roles, seed data, failure controls, endpoints, report) | `fixtures/README.md` (all seven sections), per-family `FAILURES.md`, this report |

## Fidelity / limitations

- **Environment fidelity:** validated in the agent sandbox (localhost browser via Chromium + curl + terminal). VWO-001 owns the E2B desktop-capable proving ground; this work order's apps are environment-agnostic (any machine with node or bun). The desktop surface provided here is an honest local terminal application, not a claim of E2B desktop capability.
- The apps are deliberately small (each app file ≤ 262 LoC): realistic *behavior*, not realistic scale. Event feeds cap at 1000 entries; notifications are in-app only (no email/webhook side effects, by design — no external services).
- `service_unavailable` applies to all state-changing requests of the app carrying the switch (documented) rather than per-dependency granularity — deterministic and observable, at some cost in realism.
- Fixture clock (2026-09-12) means seed-relative dates are internally consistent; workers comparing to the real wall clock will see "past" dates by design.

## Known limitations

1. Single-user session per token; no concurrent-edit locking beyond optimistic versioning on stories/installs (documented natural `data_conflict` paths).
2. The event feed is per-app; there is no cross-family event bus (each app is independent by design — cross-app scenarios would exercise Codex Universal, not fixtures).
3. `verify-sweep.sh` is an app-level self-test for VWO-002 acceptance, explicitly not the VWO-003 validation harness/evidence infrastructure; it makes no claims about scenario catalogs or evidence schemas.
4. Auth is demo-grade (static per-user tokens, fragment-assembled): sufficient for permission/role testing, not a security model. It is documented as such everywhere.
5. The map is a schematic SVG grid, not a geo-accurate map.

## Deferred items

- None within VWO-002 scope. Adjacent surfaces intentionally left to their owners: E2B/desktop environments (VWO-001), validation harness/scenario catalog/evidence schema (VWO-003), all human-workflow validation (VWO-004+).

## Risks

- **Port collisions** in shared sandboxes: defaults are 4101-4105; every app accepts `--port`/`FIXTURE_PORT`. Low risk.
- **Browser-driver assumptions**: pages avoid client-side JS on core paths, so driver choice stays open; if a future consumer adds JS, they own that coupling. Low risk.
- **Seed drift vs. report**: this report documents ids (prq-5001, da-0303, st-0205, ent-0502 …) tied to the committed seed; changing seeds later invalidates cited ids (acceptable — seeds are versioned in git).

## Worker conclusion

VWO-002 is implemented and verified: five stateful synthetic enterprise applications with seeded personas, complete golden human paths (browser + API), all seven deterministic failure switches per family (59/59 scripted checks), reset/seed capability, a terminal desktop surface, and full catalog documentation — with no workflow semantics, no second engine, no UI-automation coupling, and no real credentials. Ready for the Tech Lead's verification and harvest.

---

## Tech-Lead verification addendum (integration station, 2026-09-11)

- **Bundle acceptance:** strict base `926c177a5515a1bf1316d7ef1bacb9b6d2d53611` = origin/main at dispatch; single delivery commit; scope exactly `docs/validation/fixtures/**` + `docs/validation/reports/**` (48 files, +5,709); zero codex-rs changes.
- **Independent run (not a trust of the worker's evidence):** all five apps booted, health-checked, reset deterministically and exercised under node v24.19.0 on the Tech Lead machine. `verify-sweep.sh` reproduced the worker's 59/59 — after fixing ONE test-harness bug the sweep itself shipped with (see next).
- **Integration fix (the only change the Tech Lead made to the delivery):** `expect()` used `echo "$body" | grep -q "$sub"` under `set -uo pipefail`. `grep -q` exits on the first match; on larger response bodies `echo` takes SIGPIPE and exits 141, and pipefail reports the whole pipeline as failed — so a PASSING assertion randomly reported `missing '<sub>'` while the printed body visibly contained the substring (reproduced: "missing '1.3.0'" under a body containing `1.3.0`; failure sets varied run-to-run). Replaced with `grep -q "$sub" <<< "$body"` (no pipe, no SIGPIPE, identical BRE semantics). After the fix: **59/59 on six consecutive clean reset→sweep runs**. The applications themselves were deterministic and correct the whole time; no app code was touched.
- **Credential hygiene independently checked:** seeds store only fragment arrays (`['demo','pass','marco']`), assembled at runtime — no literal credential strings in source.
- **Terminal client verified live:** ForgeOps deploy-console (login, deployments, switch arming) against a running server.
