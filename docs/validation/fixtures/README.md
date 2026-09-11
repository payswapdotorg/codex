# VWO-002 Synthetic Enterprise Application Ecosystem — Catalog

Five self-contained, stateful, browser-accessible enterprise applications used as the proving ground for the Codex Universal human-workflow validation program (`docs/validation/VALIDATION-PROGRAM.md` §4). Work order: `docs/validation/work-orders/VWO-002.md`. Report: `docs/validation/reports/VWO-002-report.md`.

**Everything here is a fake demo world.** All users, companies, data and credentials are synthetic. No production credentials or real-world data exist anywhere. Demo passwords/session tokens are *assembled at runtime from fragments* (see `_lib/runtime.js`) so no literal credential strings appear in source.

## 1. Application inventory

| Family | App | Port | Entry URL | Start command |
|---|---|---|---|---|
| construction | SiteBuild Field Suite | 4101 | http://localhost:4101/ | `node construction/server.js` (or `bun`) |
| software (Google-like) | ForgeOps | 4102 | http://localhost:4102/ | `node software/server.js` |
| ride-share | RidePilot | 4103 | http://localhost:4103/ | `node rideshare/server.js` |
| media | PressRoom | 4104 | http://localhost:4104/ | `node media/server.js` |
| marketplace | FlowMart | 4105 | http://localhost:4105/ | `node marketplace/server.js` |

- Start/stop everything: `./run-all.sh` / `./run-all.sh --stop`; start + reset to seed: `./run-all.sh --reset` (background, logs and PIDs in `fixtures/.run/`).
- Any port can be overridden per app with `--port <n>` (all commands also honor `FIXTURE_PORT`).
- **Desktop-capable surface:** `software/deploy-console.js` — an honest, xterm-able interactive terminal client (readline over the ForgeOps HTTP API; works interactively *and* piped for scripted runs). It is a client only and holds no state. E2B desktop environments are VWO-001's surface, not this one.
- Shared plumbing: `_lib/runtime.js` (HTTP router, JSON-file store, auth, event feed, deterministic failure-switch guards) and `_lib/web.js` (server-rendered HTML shell, CSS, shared pages). Std-lib only — **zero external dependencies, no npm install, no database, no external SaaS**. Runs under node ≥ 18 or bun.

## 2. Users, roles, permissions (seeded personas)

Every app serves its persona table at `GET /api/demo-hints` and renders it on `/login`. Login: HTML form (`POST /login`, sets a session cookie) or JSON (`POST /api/login {username, password}` → bearer token; also accepted as `?token=` for deep links).

| App | Username | Role | Key permissions |
|---|---|---|---|
| SiteBuild | `dana.reyes` | project_manager | po:approve, invoice:approve, project/progress read |
| SiteBuild | `marco.silva` | site_foreman | progress:create, safety:report |
| SiteBuild | `priya.nair` | finance_accountant | invoice:approve |
| SiteBuild | `sam.oconnell` | safety_officer | safety:read, safety:close |
| SiteBuild | `lena.hart` | contractor_lead | read-only (external contractor) |
| ForgeOps | `ari.klein` | engineer | pr:create, ci:run, issue create/close |
| ForgeOps | `noor.haddad` | code_owner | pr:review, pr:merge, doc:write |
| ForgeOps | `raj.patel` | release_manager | deploy:promote, deploy:rollback, pr:merge |
| ForgeOps | `mei.chen` | sre_oncall | incident:open, incident:resolve, doc:write |
| ForgeOps | `tobi.oyelaran` | intern | issue:create only (low-privilege for denied-permission tests) |
| RidePilot | `farah.khan` | ops_director | driver screen/activate, surge, payouts, incidents |
| RidePilot | `jules.moreau` | regional_ops_manager | driver screen/activate, surge, broadcasts |
| RidePilot | `ana.silva` | support_agent | ticket respond/escalate (cannot screen drivers) |
| RidePilot | `kwame.mensah` | finance_analyst | payout:approve |
| PressRoom | `camille.dubois` | editor_in_chief | story:approve, story:publish, correction:apply |
| PressRoom | `diego.morales` | section_editor | story:approve, story:publish |
| PressRoom | `hana.kim` | staff_writer | story:create/edit (cannot approve own; cannot publish) |
| PressRoom | `omar.bhatt` | photo_editor | asset:upload, rendition:request |
| PressRoom | `nia.roberts` | social_distribution_manager | social:schedule, social:post |
| FlowMart | `vera.osei` | publisher | listing:publish, version:publish, entitlement:revoke |
| FlowMart | `dan.kowalski` | marketplace_admin | entitlement:grant/revoke, audit:verify |
| FlowMart | `iris.chen` | org_admin (Northwind Logistics) | install:manage/configure |
| FlowMart | `ravi.gupta` | org_member (Northwind) | review:write, install:read |
| FlowMart | `mia.torres` | auditor | audit:verify |
| FlowMart | `petra.voss` | org_admin (Acme Retail) | install:manage |

## 3. Seed data summary

| App | Seeded world |
|---|---|
| SiteBuild | 3 projects (Harborview Tower 62%, Eastside Interchange 35%, Riverside Clinic 88%), 8 tasks, 6 assets/photos, 1 progress report, 2 purchase requests (one with an **expired vendor contract** — SteelCo, natural `stale_entitlement`), 2 POs, 2 invoices, 2 safety incidents, 3 contractors, documents, notifications |
| ForgeOps | 3 repos (payments-service, web-checkout, search-indexer), 3 teams, 4 services with versions/health, 3 PRs (one merged), 3 CI runs, 3 deployments (staging+production lineage), 2 artifacts with digests, 2 incidents (one resolved, one mitigated), 3 issues, 3 internal docs, ownership directory |
| RidePilot | 4 regions with surge/demand + SVG map, 3 driver applications (submitted / screening-with-missing-insurance / approved-with-expired-permit), 3 active drivers (one with an **expired permit** — natural `stale_entitlement` on payouts), 2 support tickets (open/escalated), 4 trips, 3 payments, 1 incident, 1 broadcast |
| PressRoom | 5 distribution channels (one with an **expired license** — partner-app, natural `stale_entitlement`), 4 assets with renditions (image/video), 5 stories across the full lifecycle (draft → in_review → approved → published; one approved **without hero image** — natural `missing_asset`), 3 distribution jobs, 2 social posts, 1 correction |
| FlowMart | 2 orgs, 3 packages with immutable version history + sha256 manifest digests + provenance (source repo/commit/builder/signer), 3 entitlements (one **expired trial** for Northwind — natural `stale_entitlement`), 1 install with history, 1 review |

**Fixture clock:** every world believes "today" is `state.meta.today = 2026-09-12` (see each `seed.js`), so seed dates ("contract expired 2026-08-01") are deterministic regardless of the real wall clock. Event timestamps use real time.

## 4. Failure controls (deterministic, per-request)

Activate by appending `?failure=<switch>` to **any** request (API *or* HTML form) or by sending header `X-Failure-Switch: <switch>`. The switch applies only to the request that carries it. Unknown names return HTTP 400 with the known list. Machine-readable docs: `GET /api/failures`. HTML docs: `/failures`. Per-family tables: `docs/validation/fixtures/<family>/FAILURES.md`.

Seven canonical switches exist in **every** family:

| Switch | Meaning | Typical symptom |
|---|---|---|
| `service_unavailable` | a named synthetic dependency (photo store, CI runner pool, dispatch/payout gateway, media pipeline, registry blob store) is down | HTTP 503 on all mutations; reads keep working |
| `notification_failure` | the operation succeeds but the in-app notification / broadcast / platform delivery fails | HTTP 200 + `warning`; notification row `failed`; `notification.failed` event |
| `missing_asset` | a referenced asset (photo, document, hero image, artifact, package archive) cannot be resolved | HTTP 404 `missing_asset` |
| `permission_denied` | the permission check fails even for a user who would normally pass | HTTP 403 `permission_denied` with `simulated:true` |
| `data_conflict` | the record was concurrently changed / optimistic-concurrency mismatch | HTTP 409 `data_conflict` |
| `duplicate_event` | the request is treated as a replay of an already-processed event | HTTP 409 `duplicate_event` (idempotency: requests carrying an `idempotencyKey` are recorded; replays are always rejected) |
| `stale_entitlement` | an entitlement/license/contract/permit is expired or revoked | HTTP 409 `stale_entitlement` |

Contract details (identical in all five apps):
- switches take precedence over natural record state, so switch behavior is deterministic regardless of current state;
- every failure is *observable*: distinct HTTP status + machine error code + (where meaningful) event-feed entry and/or persisted record (e.g. failed notification rows stay visible);
- natural (non-simulated) equivalents exist for most switches and are documented per family in `FAILURES.md`.

## 5. Endpoints (per family)

Common surface in **every** app (provided by `_lib/runtime.js`):

| Endpoint | Method | Purpose |
|---|---|---|
| `/` | GET | HTML dashboard (browser entry) |
| `/login` | GET/POST | HTML login (persona table included) |
| `/logout` | POST | sign out |
| `/healthz` | GET | JSON health (family, app, port, uptime, event count) |
| `/events` (also `/api/events`) | GET | event feed — JSON by default; `Accept: text/html` or `?format=html` renders HTML. Supports `?limit=&since=&type=` |
| `/failures` (also `/api/failures`) | GET | failure-switch documentation (HTML / JSON) |
| `/api/login` | POST | JSON login → bearer token |
| `/api/whoami` | GET | current user (token/cookie) |
| `/api/demo-hints` | GET | seeded personas + runtime-assembled demo passwords |
| `/api/events`, `/api/reset`, `POST /reset` | GET/POST | machine event feed / reset world to seed |

Family-specific surface (UI pages + JSON API twins — every HTML form has a JSON twin under `/api/...` so the human path and the API path are the same operation):

| App | UI pages | Mutations (JSON `POST /api/...` + form `POST /ui/...` twins) |
|---|---|---|
| SiteBuild | `/projects`, `/projects/:id`, `/procurement`, `/safety`, `/notifications` | `progress`, `assets/upload`, `procurement/decide`, `invoices/approve`, `safety/report`, `safety/close` |
| ForgeOps | `/repos`, `/repos/:id`, `/prs/:id`, `/deployments`, `/incidents`, `/services`, `/issues`, `/docs`, `/docs/:id`, `/teams`, `/notifications` | `prs/create`, `prs/ci`, `prs/approve`, `prs/merge`, `deployments/promote`, `deployments/rollback`, `incidents/open`, `incidents/resolve`, `issues/create`, `issues/close`, `docs/update` |
| RidePilot | `/onboarding`, `/intake` (public), `/support`, `/ops`, `/map`, `/payments`, `/incidents`, `/notifications` | `intake/apply` (public), `applications/screen`, `drivers/activate`, `tickets/respond`, `tickets/escalate`, `tickets/resolve`, `regions/surge`, `payments/approve`, `incidents/report`, `incidents/resolve` |
| PressRoom | `/desk`, `/stories/:id`, `/assets`, `/social`, `/channels`, `/notifications` | `stories/create`, `stories/edit`, `stories/attach`, `stories/submit`, `stories/decide`, `stories/publish`, `stories/correct`, `assets/upload`, `assets/rendition`, `social/schedule`, `social/publish` |
| FlowMart | `/catalog`, `/listings/:slug`, `/publisher`, `/installs`, `/entitlements`, `/notifications` | `listings/publish`, `versions/publish`, `provenance/verify`, `reviews/create`, `installs/install`, `installs/configure`, `installs/upgrade`, `installs/rollback`, `entitlements/grant`, `entitlements/revoke` |

Read-only JSON APIs exist for every listing (`/api/projects`, `/api/repos`, `/api/drivers`, `/api/stories`, `/api/catalog?q=`, etc. — see each `server.js`).

## 6. Reset / seed procedure

- Between scenarios: `curl -X POST http://localhost:<port>/reset` (HTML flow, signs you out) or `POST /api/reset` (JSON). Both restore the exact seed state and append a `world.reset` event.
- The dashboard also has a **Reset demo world** button (human path).
- Whole ecosystem: `./run-all.sh --reset`.
- State lives in `docs/validation/fixtures/<family>/data/state.json` (gitignored, atomic tmp+rename writes). Deleting the `data/` directory re-seeds on next boot. Fresh clone → first boot auto-seeds.

## 7. Scope boundaries (VWO-002 forbidden list)

- **No workflow semantics are encoded in these fixtures.** The apps model ordinary enterprise behavior (approvals, submissions, installs, licenses). FlowMart specifically treats packages as opaque artifacts (manifest + digest + provenance) and never defines or executes workflows.
- **No second workflow engine.** `_lib/runtime.js` is disposable HTTP/JSON plumbing — routing, file persistence, auth, event log, failure guards. It has no scheduling, no triggers, no orchestration semantics.
- **Not hard-coded to any UI automation library.** The UIs are plain server-rendered HTML with standard form POSTs — drivable by any browser automation (or none, via the JSON twins).
- **No static screenshots/mock-only pages.** Every UI action mutates real JSON state and appends events.

## 8. Golden + failure paths (one per family, verified in the VWO-002 report)

| App | Golden human path | Failure path (example) |
|---|---|---|
| SiteBuild | foreman signs in → project page → submits daily progress (tasks/photo/%) → report saved, project % updated, PM notified | same submit with `?failure=notification_failure` → saved but notification row `failed`; approving prq-5001 → natural `stale_entitlement` (SteelCo contract expired) |
| ForgeOps | engineer opens PR (CI auto-passes) → code owner approves → release manager merges (staging deploy + artifact) → promotes to production → service version bumps → rollback available | merge with `?failure=data_conflict`; engineer approving own PR → natural `permission_denied`; promote with `?failure=stale_entitlement` |
| RidePilot | applicant uses public `/intake` → ops screens (docs verified) → activates driver → appears in directory + map + welcome broadcast | screen with `?failure=missing_asset` (or approve da-0302, insurance missing); activate da-0303 → natural `stale_entitlement` (expired permit) |
| PressRoom | writer creates draft → attaches hero → submits → editor approves → publishes to channels → social manager schedules/posts → correction re-distributes | publish with `?failure=missing_asset`; publish to partner-app → natural `stale_entitlement` (expired channel license); writer approving own story → natural `permission_denied` |
| FlowMart | buyer searches catalog → installs (trial entitlement auto-granted) → configures → publisher releases new version → buyer upgrades → rolls back → auditor verifies provenance digest | Northwind installing pk-101 → natural `stale_entitlement` (expired trial); org member installing → natural `permission_denied`; install with stale `expectedVersion` → natural `data_conflict` |

## 9. File map

```
docs/validation/fixtures/
├── README.md                 ← this catalog
├── run-all.sh                ← start/stop/reset all five apps
├── verify-sweep.sh           ← deterministic app-level self-test (golden + all 7 switches × 5 families; NOT the VWO-003 harness)
├── _lib/runtime.js           ← shared std-lib runtime (router, store, auth, events, failure guards)
├── _lib/web.js               ← shared HTML shell/CSS/helpers + login/events/failures pages
├── construction/  server.js seed.js ops.js ui.js FAILURES.md .gitignore
├── software/      server.js seed.js ops.js ops-infra.js ui.js ui2.js deploy-console.js FAILURES.md .gitignore
├── rideshare/     server.js seed.js ops.js ops-ops.js ui.js FAILURES.md .gitignore
├── media/         server.js seed.js ops.js ops-dist.js ui.js FAILURES.md .gitignore
└── marketplace/   server.js seed.js ops.js ops-install.js ui.js FAILURES.md .gitignore
```

Each app file is well under ~400 LoC (largest app file: 262 LoC; the shared runtime `_lib/runtime.js` is 506 LoC of plumbing reused by all five apps — a documented exception). Runtime state (`<family>/data/`) is gitignored and reproducible from seeds. Reproduce the VWO-002 acceptance run: `./run-all.sh --reset && ./verify-sweep.sh` → `RESULT: 59 passed, 0 failed`.
