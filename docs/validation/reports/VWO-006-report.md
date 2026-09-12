# VWO-006 Report — Workflow Consumer and Automation Operator Validation

## Identity

- Work Order: VWO-006 (Wave 1, `docs/validation/work-orders/VWO-006.md`)
- Worker persona: workflow consumer / automation operator (real-user tester; no
  codex-rs/** or semantic-contract modifications)
- Base branch: main
- Base SHA: 0d314fd09cec70d000f33c81286edcd8cbf72501
- Base SHA verification: `git rev-parse 0d314fd^{commit}` == `0d314fd09cec70d000f33c81286edcd8cbf72501` == origin/main HEAD at clone time (clone WINS over packet excerpts)
- Head SHA: see git rev-parse vwo-006/consumer-automation-validation — single
  delivery commit on top of base 0d314fd (a commit cannot embed its own hash;
  the exact full SHA is recorded in the delivery bundle and the completion
  message)
- Validation environment: agent sandbox fallback proving ground (VWO-001
  record); node v24.19.0, git 2.47.3, python 3.12; browser modality via
  agent-browser 0.35.0 + Playwright chromium 1234 (headless); terminal + file
  API verified; E2B absent (recorded fallback). The pinned Rust toolchain
  (1.95.0) was ABSENT at worker start and was remediated during this run
  (rustup minimal profile) — annotated in the capability-check evidence.
- E2B template/workspace identity, if used: N/A — E2B/Composio integration
  absent (VWO-001 §2 fallback record governs)
- Codex Universal runtime/application SHA: NONE — no Codex Universal
  workflow-consumer product surface exists in this repository state to build
  or launch: the M4/M6 workflow family (workflow-forge, workflow-app,
  workflow-triggers, workflow-distribution, workflow-durable,
  workflow-evolution) is library-only — no [[bin]] target in any workflow
  crate, no `codex-workflow-*` dependency in cli/tui/app-server/exec, zero
  `workflow` methods in app-server-protocol (probe evidence:
  `docs/validation/evidence/VWO-006/marketplace-discover-install-configure/post-hoc/20260911T215237Z-codex-teaching-surface-probe.txt`).
  Building the CLI/app-server would not expose workflow operations; no SHA is
  fabricated (see finding F-1 in every scenario block).
- Date/time window: 2026-09-11T21:41Z–2026-09-11T22:02Z (UTC; fixture clock
  `state.meta.today` = 2026-09-12 by VWO-002 design)

### Global verification record (commands + exact results)

1. Fixture startup: `bash docs/validation/fixtures/run-all.sh` → all five
   apps healthy on 127.0.0.1:4101–4105.
2. `bash docs/validation/fixtures/verify-sweep.sh` (canonical:
   `run-all.sh --reset` first, per fixtures README §9) → **RESULT: 59 passed,
   0 failed** (evidence:
   `docs/validation/evidence/VWO-006/_infra/post-hoc/20260911T220122Z-verify-sweep-canonical-result.txt`).
   Procedure note, recorded for honesty: one mid-run sweep re-execution
   WITHOUT re-seeding produced 46/13 — all 13 were duplicate-state 409s on
   golden create-operations in the four non-marketplace worlds the sweep
   itself had mutated earlier (the sweep does not self-reset; it requires
   freshly seeded apps). Not a fixture defect; the canonical reset+sweep is
   59/59.
3. `bash docs/validation/proving-ground/capability_check.sh` → **RESULT: PASS
   — required capabilities verified** (exit 0) with 2 fidelity failures
   (E2B/Composio absent; xdotool input injection absent — both are recorded
   fallback conditions that gate only under `--strict`). Remediation note:
   check #4 (pinned Rust toolchain) failed at worker start (cargo not on
   PATH); fixed in-run by installing toolchain 1.95.0; re-run passes (evidence:
   `docs/validation/evidence/VWO-006/_infra/post-hoc/20260911T220222Z-capability-check-sandbox-profile.txt`).
4. Secrets refusal kept intact: a probe artifact containing fake
   credential-looking lines was REFUSED by `capture-evidence.sh` (exit 1,
   nothing written; transcript with redacted fakes captured at
   `docs/validation/evidence/VWO-006/_infra/post-hoc/20260911T220042Z-secrets-refusal-selftest.txt`).
5. Report lint: `python3 docs/validation/scripts/validate-report.py
   docs/validation/reports/VWO-006-report.md --catalog
   docs/validation/scenarios/SCENARIO-CATALOG.md` → verdict recorded at the
   end of this report (PASS required; warnings explained below).

### Scenarios executed (3 of 3 teaching modes, all FlowMart port 4105)

| Scenario id | Mode (catalog binding) | Role here | Verdict |
|---|---|---|---|
| marketplace-discover-install-configure | DEMONSTRATE | primary owner (VWO-006) | fixture path passed; teaching step blocked (F-1) |
| marketplace-upgrade-rollback | INSTRUCT | primary owner (VWO-006) | fixture path passed; inference step blocked (F-1) |
| adv-environment-failure-rebinding | HYBRID | secondary exerciser (catalog "Also exercised by: VWO-006") — chosen as the HYBRID pick, recorded per work-order | demo phase passed; instruct submission + reconciliation blocked (F-1); rebind human path blocked by fixture defect (F-2); invariant oracle verified |

### Unique findings summary (detail in scenario blocks)

- P0: 0
- P1: 2 — F-1 (Codex Universal workflow-consumer product surface absent),
  F-2 (FlowMart UI configure form silently no-ops)
- P2: 2 — F-3 (no capability/resource/compatibility contract before
  install), F-4 (no schedule / webhook-event-trigger / execution-monitoring /
  uninstall surfaces anywhere)
- P3: 2 — F-5 (install form carries no binding fields), F-6 (unvalidated
  binding-key namespace accepts semantic-shadowing keys as inert data)

### WO-006 acceptance mapping (each bullet → scenarios + evidence)

- discover: SC1 steps 1–2 (catalog search, compare, listing) — user-path
  step01/step02 snapshots.
- understand: PARTIAL — SC1 step 2 (digest, provenance, license, immutable
  version history visible); machine-readable capabilities/resources/
  compatibility contract absent (F-3).
- install: SC1 step 3 (install ins-0402 v2.3.1, entitlement auto-grant,
  role+entitlement gates verified: steps 4–5).
- authorize: SC1 steps 4–5 (stale_entitlement block; member permission
  block), SC2 step 7 (entitlement-blocked upgrade), SC3 configure authority.
- execute: NOT SATISFIABLE — no execution surface exists (F-1/F-4); installs
  are commercial records only (VWO-002 scope).
- recover: SC2 steps 4/5/7 + SC3 step 2 (fail-safe 503, stale-pin conflict,
  entitlement block, explicit rollback, clean retry) at the store layer;
  execution-level recovery untestable (F-1/F-4).
- upgrade: SC2 step 3, SC3 step 3 (explicit, pinned, audited).
- rollback: SC2 step 5 (explicit, history-recorded).
- "without needing repository internals": FAILS for Codex Universal — no
  consumer product surface exists at all (F-1); HOLDS on the FlowMart store
  surface for every store-side operation performed (all evidence user-path
  snapshots were produced through pages/forms only).
- "Version identity stays pinned": HOLDS everywhere tested — SC2 pin trail
  (2.3.1 → 2.4.0 → 2.3.1, every move explicit + history), SC3 oracle
  (rebind left pin 2.4.0 and digest sha256:cadb075e36500b7c unchanged;
  smuggled semantic fields stored as inert config data).
- "resource rebinding never mutates workflow semantics": HOLDS — SC3
  post-hoc rebind invariant oracle (pin, packageId, published digest all
  immutable across rebind; binding values changed).

---

## Scenario

- Scenario id: marketplace-discover-install-configure
- Industry: marketplace
- Scenario: discover, inspect and install a marketplace workflow with
  explicit bindings, taught by demonstration (DEMONSTRATE)
- Teaching mode: DEMONSTRATE
- User goal: as Iris Chen (Northwind org admin), discover a workflow in the
  catalog, inspect its digest/provenance/license/version history BEFORE
  installing, install the pinned version with explicit resource/account
  bindings for the organization, watch the entitlement and role gates behave,
  and configure the install — demonstrating the consumer path for Codex
  Universal to observe.

## User Path

1. Opened `http://localhost:4105/login` (persona table with demo credentials
   visible on the page), signed in as `iris.chen`; opened `/catalog`, searched
   `q=triage` (1 result) and browsed the full catalog for comparison
   (license/latest/stars columns across all 3 listings).
2. Opened the listing `/listings/support-triage-assistant`: read the digest
   (`sha256:04cf049e110cc2cc`), provenance (repo, commit `c7d8e9f`, built/signed
   by vera.osei), immutable version history (2.3.1), license model
   (subscription), stats and description — BEFORE installing.
3. Clicked **Install v2.3.1** as org admin: install `ins-0402` created at
   v2.3.1 for Northwind Logistics with auto-granted trial entitlement
   `ent-0504` (until 2026-10-12); publisher notification delivered; history
   entry "installed 2.3.1 by Iris Chen".
4. Attempted to install `pk-101` (Invoice Autofill) for Northwind: natural
   rejection `[stale_entitlement]` — "Entitlement ent-0502 … is expired (valid
   until 2026-08-01) — installation requires an active license."
5. Signed in as `ravi.gupta` (org member, review:write + install:read only):
   catalog renders **View** instead of Install — no install affordance for a
   member; post-hoc API-twin probe confirms the natural `[permission_denied]`
   403 ("lacks permission install:manage") behind the absent affordance.
6. As `iris.chen`, used the install card's **Config keys to merge (JSON)**
   form to apply bindings (queue, confidenceThreshold, routingMode,
   notifyChannel): flash claims success ("Install ins-0402 configured (no
   keys)") but the Config cell remains `{}` — the human-path configure
   silently persists NOTHING (finding F-2); the same operation through the
   API twin persists the bindings (marked diagnostic continuation).
7. Attempted the catalog's Codex Universal step (replay steps 1–4 while a
   teaching surface observes; review the inferred consumer workflow): step 7
   NOT performed — no teaching/consumer product surface exists in this
   environment (probe transcript in post-hoc evidence; finding F-1).

## Expected

Per the catalog entry: discovery and inspection possible before install;
provenance/digest visible on the listing; install requires org admin (role
gate) + valid entitlement (license gate) with the expired trial rejected with
entitlement id + expiry; member blocked; configuration persists; the
DEMONSTRATE replay produces a compiled consumer workflow (search → inspect →
install → configure, with entitlement + role gates) for review.

## Actual

Steps 1–5 behaved exactly as the catalog expects (search/compare; digest +
provenance + immutable version history + license visible pre-install; install
ins-0402 @ 2.3.1 with auto-trial ent-0504; natural stale_entitlement and
permission blocks both observed; publisher notification delivered). Step 6
DID NOT persist bindings through the human path — the configure form's JSON
textarea submits a string, the operation expects an object, and the result is
a misleading success flash with zero keys merged while a "configured" history
entry is still recorded (fixture defect F-2; API-twin continuation marked in
post-hoc evidence persisted queue/confidenceThreshold/routingMode/
notifyChannel). Step 7 was impossible: no Codex Universal
teaching/observation surface exists (F-1) — nothing was fabricated and no
internal workflow IR was pre-built.

## Workflow Identity

- Workflow: pk-102 "Support Ticket Triage Assistant" (FlowMart package;
  NO Codex Universal workflow exists — consumer surface missing, F-1)
- Version: 2.3.1 (immutable published release; install pinned ins-0402)
- Source revision: provenance commit c7d8e9f (source repo
  git.example.internal/vera/support-triage-assistant, built/signed by
  vera.osei)
- Definition digest: sha256:04cf049e110cc2cc (manifest digest shown on the
  listing and equal in the package API pull)
- Dependency-lock identity: NONE — surface missing (FlowMart packages are
  opaque artifacts by VWO-002 design; the dependency-lock surface belongs to
  the missing Codex Universal consumer surface, F-1)

## Runtime Binding

- Capabilities: browser use of the FlowMart store surface (catalog, listing,
  installs pages/forms); install:manage + install:configure authority
  (org admin); Codex Universal teaching/compile capability NOT exercised —
  surface missing (F-1)
- Resources: FlowMart fixture app (marketplace family, port 4105); seeded
  users iris.chen (org_admin), ravi.gupta (org_member), vera.osei
  (publisher); org Northwind Logistics (o-1); install ins-0402; entitlements
  ent-0504 (auto trial) and ent-0502 (expired trial); bound config keys
  (queue, confidenceThreshold, routingMode, notifyChannel — persisted via
  marked diagnostic continuation only, F-2)
- Environments: agent sandbox fallback proving ground (no E2B workspace);
  fixture clock state.meta.today = 2026-09-12; five VWO-002 apps on
  127.0.0.1:4101–4105
- Approvals: org-admin install gate (member blocked at the affordance level +
  natural 403 behind it); entitlement gate (expired trial blocked with
  entitlementId + validUntil); publisher notified on install — all decisions
  explicit and observable
- Triggers/schedules: none exercisable — no schedule surface, no
  webhook/event-trigger surface exists anywhere in this environment (F-4);
  the only trigger-adjacent signal observed is the publisher-notification on
  install

## Evidence

- docs/validation/evidence/VWO-006/marketplace-discover-install-configure/user-path/20260911T214946Z-step01-login-persona-table.txt
- docs/validation/evidence/VWO-006/marketplace-discover-install-configure/user-path/20260911T214959Z-step01-catalog-search-triage.txt
- docs/validation/evidence/VWO-006/marketplace-discover-install-configure/user-path/20260911T220024Z-step01-catalog-compare-all-listings.txt
- docs/validation/evidence/VWO-006/marketplace-discover-install-configure/user-path/20260911T215029Z-step02-listing-inspect-digest-provenance.txt
- docs/validation/evidence/VWO-006/marketplace-discover-install-configure/user-path/20260911T215038Z-step03-install-success-record.txt
- docs/validation/evidence/VWO-006/marketplace-discover-install-configure/user-path/20260911T215054Z-step04-install-stale-entitlement-rejected.txt
- docs/validation/evidence/VWO-006/marketplace-discover-install-configure/user-path/20260911T215111Z-step05-member-no-install-affordance.txt
- docs/validation/evidence/VWO-006/marketplace-discover-install-configure/user-path/20260911T215220Z-step06-configure-attempt-no-keys-persisted.txt
- docs/validation/evidence/VWO-006/marketplace-discover-install-configure/post-hoc/20260911T215237Z-codex-teaching-surface-probe.txt
- docs/validation/evidence/VWO-006/marketplace-discover-install-configure/post-hoc/20260911T215258Z-api-twin-diagnostics-configure-member-di.txt
- docs/validation/evidence/VWO-006/marketplace-discover-install-configure/post-hoc/20260911T215306Z-events-feed-scenario-trail.txt
- docs/validation/evidence/VWO-006/marketplace-discover-install-configure/post-hoc/20260911T215339Z-reset-verification.txt
- docs/validation/evidence/VWO-006/marketplace-discover-install-configure/post-hoc/20260911T215952Z-consumer-lifecycle-surface-inventory.txt

## Failure / Recovery

- Failure injected: natural expired-trial install (ent-0502, pk-101) →
  `[stale_entitlement]` 409 with entitlement id + expiry (recovery path shown:
  renew/grant, then install); natural member install attempt → blocked;
  configure-form defect (F-2) discovered on the human path
- Recovery attempted: entitlement recovery path NOT taken further (renewal is
  the documented marketplace-admin path; out of this scenario's scope —
  the block itself is the oracle and it held); F-2 worked around by a MARKED
  diagnostic continuation through the API twin (same shared operation
  `inst.configureInstall`) to prove bindings CAN persist and to keep the
  scenario's downstream state meaningful; no silent simulation
- Restart/session-loss behavior: not attempted — store-level scenario; the
  durable restart adversarial scenario is adv-session-loss-restart (VWO-007)
- Final outcome: discovery/inspection/install/gates all pass on the fixture;
  configure-with-bindings fails on the human path (F-2); teaching replay
  impossible (F-1)

## Product / UX Friction

The store's consumer path is genuinely usable: search, compare, listing
inspection (digest/provenance/license/history), install, upgrade/rollback
forms, flashes, notifications and the event feed are all human-observable and
clear. Friction points: (1) after completing a clean install there is nowhere
to go for the "teach Codex this routine" step — the acceptance question dies
at a missing surface, and a normal person has no way to even express the
request (F-1); (2) the configure form LOOKS like it works (success flash) but
persists nothing — the worst kind of friction because it is silent (F-2);
(3) the install form takes no bindings, so "install with explicit
resource/account bindings" is really "install, then configure" (F-5); (4) the
listing's pre-install inspection is prose + digest only — no
capabilities/resources/compatibility declaration a consumer could verify
against their org before committing (F-3).

## Security / Architecture Findings

- Finding: F-1 — the Codex Universal workflow-consumer product surface is
  absent. The M4/M6 workflow family (workflow-forge discovery/install,
  workflow-distribution commerce/entitlement, workflow-durable installs,
  workflow-triggers install/rebind/schedule/event, workflow-app lifecycle)
  exists as library crates only: no [[bin]] target, no dependency from
  cli/tui/app-server/exec, zero workflow methods in app-server-protocol
  (the app-server "marketplace" methods are the plugin/skills marketplace —
  a different concept). The acceptance question — "can a non-creator user
  discover, understand, install, authorize, execute, recover, upgrade and
  rollback a workflow without needing repository internals?" — FAILS
  outright at the Codex Universal layer: the step a normal person cannot
  complete is every Codex-Universal-side step of this scenario (here: user
  path step 7, the DEMONSTRATE replay/compile), and the only alternative is
  repository internals (Rust library calls), which the acceptance forbids.
- Severity: P1
- Reproduction: complete user-path steps 1–6 in FlowMart; then attempt to
  locate any Codex Universal teaching/observation surface (CLI subcommand,
  TUI view, app-server method, web UI) — none exists. Probe transcript:
  post-hoc/20260911T215237Z-codex-teaching-surface-probe.txt.
- Root cause: the workflow crates were landed as libraries (WO-009/010/011)
  without any user-facing surface wiring; no follow-up surface Work Order has
  been executed.
- Frozen invariant affected: none violated (absence, not violation); recorded
  per VALIDATION-PROGRAM.md §8 — missing surfaces are reported, never
  fabricated.

- Finding: F-2 — FlowMart's UI configure form silently no-ops. The "Config
  keys to merge (JSON)" textarea POSTs urlencoded, so the operation receives
  a STRING while `configureInstall` merges only objects; the result is a
  success flash "configured (no keys)", an unchanged `{}` Config cell, and a
  misleading "configured" history/event entry with zero keys. The step a
  normal person cannot complete is user-path step 6 (apply explicit resource/
  account bindings) — the exact WO-006 mandatory scenario "Install using
  explicit resource/account bindings" is blocked on the human path (the API
  twin is the only working path, and catalog rules permit it only as post-hoc
  diagnostics).
- Severity: P1
- Reproduction: sign in as iris.chen → /installs → paste any JSON object into
  the config textarea → Apply config → flash says configured (no keys);
  Config cell still {} (user-path/20260911T215220Z-step06-… and
  post-hoc/20260911T215258Z-api-twin-diagnostics-…).
- Root cause: fixture defect — shared runtime parses urlencoded forms into
  string values; ops-install.js `configureInstall` requires
  `typeof b.config === 'object'`. One-line fix: JSON.parse string values for
  the config field (install + configure).
- Frozen invariant affected: none (fixture plumbing, not a semantic
  invariant); but it blocks exercising the consumer-bindings human path this
  Work Order is required to validate.

- Finding: F-3 — no capability/resource/compatibility declaration surface
  before install. The listing shows digest, provenance, license model,
  immutable version history, stats and prose description, but nothing a
  consumer could verify organizationally (required capabilities, resource
  types, compatibility constraints) before installing. The step a normal
  person cannot complete is "understand" in the WO acceptance sense —
  inspection is limited to trust markers, not requirements.
- Severity: P2
- Reproduction: open any listing pre-install; the overview card contains no
  capabilities/resources/compatibility fields
  (user-path/20260911T215029Z-step02-…).
- Root cause: FlowMart packages are deliberately opaque artifacts (VWO-002
  forbidden list); the consumer-side compatibility surface belongs to the
  missing Codex Universal consumer surface (F-1).
- Frozen invariant affected: none (absence recorded so it is not lost when
  F-1 is remediated).

- Finding: F-4 — the consumer lifecycle surfaces for schedule, webhook/event
  trigger, execution monitoring and uninstall/reinstall do not exist anywhere
  in this environment (FlowMart has no such routes by design and the Codex
  Universal workflow-triggers crate that implements ScheduleSpec /
  IncomingTrigger / poll_schedules is library-only). The steps a normal
  person cannot complete are the WO-006 mandatory scenarios "Schedule a
  workflow" and "Trigger one by webhook/event", plus execution observation
  and uninstall.
- Severity: P2
- Reproduction: inventory probe across the install card contents, the
  FlowMart route list and the crate wiring —
  post-hoc/20260911T215952Z-consumer-lifecycle-surface-inventory.txt.
- Root cause: same as F-1 (library-only implementation, no surface wiring).
- Frozen invariant affected: none (absence).

- Finding: F-5 — the install form carries no binding fields (hidden
  packageId + expectedVersion only), so bindings can only be applied
  post-install via configure (which is itself broken on the human path,
  F-2). Friction, not incorrectness.
- Severity: P3
- Reproduction: inspect the catalog/listing install forms
  (user-path/20260911T215038Z-step03-… shows Config = {} immediately after
  install).
- Root cause: fixture UI design; the "explicit bindings at install" surface
  belongs to the missing consumer surface (F-1).
- Frozen invariant affected: none.

## Recommendation

- F-1 (P1): NEW WORK ORDER — build the workflow-consumer product surface
  (discover/inspect/install/bind/schedule/trigger/execute/upgrade/rollback/
  uninstall) over the existing library seams (workflow-triggers
  `WorkflowTriggerPlane::install/discover/rebind_resource/poll_schedules`,
  workflow-distribution commerce, workflow-durable stores), exposed through
  the app-server protocol and/or CLI, with the FlowMart fixture as the store
  backend. Proposed owner: Tech Lead to assign (app-server/CLI surface
  ownership). Verification required: re-run all three VWO-006 scenarios;
  every Codex-Universal-side user-path step must complete without repository
  internals.
- F-2 (P1): FIX NOW — one-line fixture fix (JSON.parse string `config` in
  install/configure ops; VWO-002 surface, Tech Lead applies). Verification
  required: re-run this scenario's step 6 through the UI; keys must persist
  and the flash/history must reflect them.
- F-3 (P2): NEW WORK ORDER — fold into the F-1 consumer-surface WO: the
  pre-install surface must render a machine-readable
  capabilities/resources/compatibility contract. Proposed owner: same as
  F-1. Verification required: consumer can verify org compatibility before
  install on the listing/install screen.
- F-4 (P2): NEW WORK ORDER — fold into the F-1 consumer-surface WO: surface
  schedules, webhook/event triggers, run/execution monitoring and uninstall
  (the library semantics already exist and are idempotent-by-design).
  Proposed owner: same as F-1. Verification required: the WO-006 mandatory
  "Schedule a workflow" and "Trigger one by webhook/event" scenarios run
  end-to-end.
- F-5 (P3): NEW WORK ORDER — fold into F-1 (install-time binding fields on
  the install surface). Proposed owner: same as F-1. Verification required:
  bindings expressible at install time through the UI.

## Worker Conclusion

The scenario is **blocked** at the Codex Universal layer (user-path step 7 —
teaching/compile surface missing, F-1) and partially blocked on the fixture
human path (step 6 configure no-op, F-2); every store-side step passed
cleanly with all authorization gates behaving exactly as the catalog
specifies. Not a pass: the DEMONSTRATE mode cannot be completed end-to-end
by a normal person while F-1 stands.

---

## Scenario

- Scenario id: marketplace-upgrade-rollback
- Industry: marketplace
- Scenario: upgrade a pinned install to a new immutable release, reproduce a
  failed upgrade, roll back explicitly (INSTRUCT)
- Teaching mode: INSTRUCT
- User goal: as Petra Voss (Acme Retail org admin, install:manage) owning
  seeded install ins-0401 (pk-102 @ 2.3.1), upgrade to the newly published
  2.4.0, reproduce failure modes (stale expectedCurrentVersion,
  entitlement-blocked, registry-down), and roll back explicitly — described
  as a policy the system should infer: upgrades and rollbacks are explicit,
  pinned, and auditable; failures never leave half-installed state.

## User Path

1. As `vera.osei` (publisher), opened the listing's **Publish a new version**
   form, set version `2.4.0` with changelog, published: new immutable version
   visible with its own digest `sha256:cadb075e36500b7c` and commit
   `c107403`; both 2.3.1 and 2.4.0 listed; installers notified.
2. Signed in as `petra.voss`; opened `/installs`: install `ins-0401` shows
   "2.4.0 — upgrade available" with the upgrade form (target select + hidden
   expectedCurrentVersion rendered from the current pin 2.3.1).
3. Submitted the upgrade in a fresh tab: `ins-0401` moved 2.3.1 → 2.4.0;
   flash "Install ins-0401 upgraded 2.3.1 → 2.4.0"; history entry recorded
   ("upgraded 2.3.1 → 2.4.0 by Petra Voss"); "up to date" shown; org-admin
   notification delivered; `install.upgraded` event carries the 2.4.0 digest.
4. Re-submitted the STALE upgrade form kept open from before the upgrade
   (two-tab, natural optimistic-concurrency mismatch): rejected
   `[data_conflict]` — "Install ins-0401 moved: you expected v2.3.1 but it
   is at v2.4.0." Install stays at 2.4.0, no duplicate history entry, no
   partial state.
5. Clicked **Rollback to previous**: `ins-0401` moved 2.4.0 → 2.3.1 as its
   own history entry ("rolled_back 2.4.0 → 2.3.1 by Petra Voss"); flash
   confirms; "2.4.0 — upgrade available" returns; `install.rolled_back`
   event recorded.
6. Wrote the INSTRUCT policy text verbatim (explicit-pin upgrade/rollback,
   fail-safe, auditable history, named-error recovery — full text in
   user-path evidence) and attempted to submit it to the Codex Universal
   instruction surface: step NOT performed — surface missing (F-1); the
   inference-evaluation the mode requires is impossible; text recorded as
   user-path evidence per TEACHING-MODE-MATRIX §3.
7. Armed `?failure=stale_entitlement` on the upgrade form (via form-action
   modification, the documented per-request switch mechanism): upgrade
   blocked — "Entitlement … is stale/revoked (simulated): upgrade is blocked
   for Acme Retail. [stale_entitlement]"; pin stays 2.3.1, history
   unchanged, zero partial state. Also armed
   `?failure=service_unavailable`: "The registry-blob-store service is
   unavailable (simulated)… [service_unavailable]"; pin stays 2.3.1, zero
   partial state.
8. Opened `/events` as the human-visible monitoring surface: the full
   version.published → install.upgraded → install.rolled_back trail with
   actors, subjects and digests is visible to any signed-in user.

## Expected

Per the catalog entry: publisher publishes a new immutable version; the
consumer sees upgrade-available; upgrade succeeds only with valid entitlement
+ current version pin; re-upgrading with a stale expectedCurrentVersion is
rejected with `[data_conflict]`; rollback is explicit and recorded; failed
upgrades (entitlement, registry) leave the install at its previous pin with
no partial state; history is auditable; the INSTRUCT policy description is
inferred into an explicit-pin upgrade/rollback workflow for review.

## Actual

Every store-side expectation was met exactly (steps 1–5, 7, 8): the pin moved
only through explicit, history-recorded transitions (2.3.1 → 2.4.0 → 2.3.1);
all three failure modes rejected cleanly with named, actionable errors and
zero partial state; the event feed exposed the complete auditable trail.
Step 6 could not be performed: no Codex Universal instruction/teaching
surface exists (F-1) — the policy text is preserved verbatim as user-path
evidence and the inference evaluation is recorded as impossible rather than
simulated.

## Workflow Identity

- Workflow: pk-102 "Support Ticket Triage Assistant" (FlowMart package; NO
  Codex Universal workflow exists — consumer surface missing, F-1)
- Version: release 2.4.0 published this run (digest
  sha256:cadb075e36500b7c, commit c107403); install ins-0401 pin trail
  2.3.1 → 2.4.0 → 2.3.1 (every move explicit + recorded)
- Source revision: provenance commits c7d8e9f (2.3.1) and c107403 (2.4.0)
- Definition digest: 2.3.1 sha256:04cf049e110cc2cc; 2.4.0
  sha256:cadb075e36500b7c (both equal between listing page, package API and
  the install.upgraded event payload)
- Dependency-lock identity: NONE — surface missing (same gap as scenario 1)

## Runtime Binding

- Capabilities: browser use of the FlowMart store surface; version:publish
  (publisher), install:manage (org admin) authorities; INSTRUCT submission/
  inference capability NOT exercisable — surface missing (F-1)
- Resources: FlowMart fixture app (port 4105); users vera.osei (publisher),
  petra.voss (Acme org admin); org Acme Retail (o-2); install ins-0401
  (seeded, pk-102 @ 2.3.1, entitlement ent-0501 active until 2027-05-01)
- Environments: agent sandbox fallback proving ground; synthetic
  registry-blob-store dependency (the named provider that fails under
  service_unavailable)
- Approvals: org-admin upgrade and rollback gates (forms only render for
  install:manage); entitlement validity gate on every commercial move;
  publisher version gate (version:publish)
- Triggers/schedules: upgrade-available signal triggered by version.published
  (notification delivered to installers + installs page state) — a real
  product trigger within the store's scope; the taught policy's
  upgrade-available trigger could not be inferred by any system (F-1);
  schedule/webhook surfaces absent (F-4)

## Evidence

- docs/validation/evidence/VWO-006/marketplace-upgrade-rollback/user-path/20260911T215418Z-step01-publish-240-immutable-version.txt
- docs/validation/evidence/VWO-006/marketplace-upgrade-rollback/user-path/20260911T215437Z-step02-upgrade-available-ins0401.txt
- docs/validation/evidence/VWO-006/marketplace-upgrade-rollback/user-path/20260911T215451Z-step03-upgrade-success-history-entry.txt
- docs/validation/evidence/VWO-006/marketplace-upgrade-rollback/user-path/20260911T215507Z-step04-stale-expectedcurrent-conflict.txt
- docs/validation/evidence/VWO-006/marketplace-upgrade-rollback/user-path/20260911T215516Z-step05-explicit-rollback-history.txt
- docs/validation/evidence/VWO-006/marketplace-upgrade-rollback/user-path/20260911T215533Z-step06-instruct-policy-text.txt
- docs/validation/evidence/VWO-006/marketplace-upgrade-rollback/user-path/20260911T215558Z-step07-upgrade-entitlement-blocked-no-partial.txt
- docs/validation/evidence/VWO-006/marketplace-upgrade-rollback/user-path/20260911T215608Z-step07-upgrade-service-unavailable-failsafe.txt
- docs/validation/evidence/VWO-006/marketplace-upgrade-rollback/user-path/20260911T215933Z-monitoring-surface-event-feed.txt
- docs/validation/evidence/VWO-006/marketplace-upgrade-rollback/post-hoc/20260911T215629Z-events-pin-digest-checks.txt
- docs/validation/evidence/VWO-006/marketplace-upgrade-rollback/post-hoc/20260911T215630Z-reset-verification.txt

## Failure / Recovery

- Failure injected: three — natural stale `expectedCurrentVersion`
  (two-tab) → `[data_conflict]` naming both versions; simulated
  `stale_entitlement` on upgrade → blocked, pin unchanged, zero partial
  state; simulated `service_unavailable` on upgrade → named
  registry-blob-store dependency, pin unchanged, zero partial state
- Recovery attempted: per the catalog — re-read current version (the fresh
  tab's form re-renders expectedCurrentVersion=2.4.0, the stale submit is
  discarded); plain retry after provider "recovery" (the switch is
  per-request, so the clean re-submit IS the recovery — verified in scenario
  3 step 3 on the equivalent path); explicit rollback as the governed
  downgrade path
- Restart/session-loss behavior: not attempted — store-level scenario;
  durable restart belongs to adv-session-loss-restart (VWO-007)
- Final outcome: every failure failed closed with a named, actionable error
  and zero partial state; the pin only ever moved through explicit recorded
  actions; rollback explicit and auditable

## Product / UX Friction

Store-side friction in this scenario is essentially zero: the upgrade form
carries its optimistic-concurrency pin invisibly and correctly; the two-tab
stale submit produces a human-readable conflict that says exactly what moved
and what to do; the rollback is one explicit click with its own history row.
The friction is again the missing Codex Universal half: the INSTRUCT policy
text has nowhere to go — a normal person cannot hand their upgrade/rollback
policy to the system, so the mode's core value (inference review) is
unreachable (F-1). Minor note: the upgrade form's hidden
expectedCurrentVersion is good discipline but invisible to the user; a
normal person must trust the conflict message rather than see the pin they
are holding (acceptable; the error message compensates).

## Security / Architecture Findings

- Finding: F-1 (recurrence in this scenario) — the INSTRUCT
  instruction-submission and inference-review surface is absent (same root
  cause and acceptance failure as scenario 1; the step a normal person
  cannot complete is user-path step 6). The policy text is preserved as
  user-path evidence; no inference was fabricated.
- Severity: P1
- Reproduction: write the policy text; attempt to submit it through any
  Codex Universal surface (CLI/TUI/app-server/UI) — none exists (probe:
  scenario 1 post-hoc surface probe, same repo state).
- Root cause: library-only workflow crates, no surface wiring (see scenario
  1 F-1).
- Frozen invariant affected: none violated (absence).

## Recommendation

- F-1 (P1): NEW WORK ORDER — same consumer-surface Work Order as scenario 1
  (the INSTRUCT path: instruction submission + inferred-workflow review
  surface with semantics/dependencies/resources/conditions/approvals/
  triggers fields). Proposed owner: Tech Lead to assign. Verification
  required: re-run this scenario; the policy text must produce an
  inferable, reviewable workflow whose field-by-field evaluation is
  capturable as user-path evidence.

## Worker Conclusion

The scenario **passed** on every store-side behavior (explicit-pin upgrade,
natural data_conflict, entitlement-blocked and registry-down fail-safes,
explicit auditable rollback, event-feed monitoring) and is **blocked** only
at the INSTRUCT submission/inference step (F-1). Overall verdict for the
teaching-mode requirement: blocked by a missing product surface, with the
fixture-side semantics fully verified.

---

## Scenario

- Scenario id: adv-environment-failure-rebinding
- Industry: adversarial
- Scenario: environment/provider failure during upgrade and compatible
  rebinding without semantic mutation (HYBRID) — chosen as this worker's
  HYBRID scenario from the catalog (VWO-006 is its recorded secondary
  exerciser)
- Teaching mode: HYBRID
- User goal: as Iris Chen (Northwind org admin), establish a consumer
  install with instructions plus demonstration, survive a registry-provider
  failure during upgrade (fail-safe, then clean retry), rebind the install
  to a compatible environment through configuration only, and prove the
  version pin, digest and provenance never change across the rebind —
  including against a deliberately semantically-mutating rebind payload.

## User Path

1. (instruct phase) Wrote the HYBRID instruction text verbatim — the
   install-and-rebind policy including "rebinding must change the binding
   values, never the workflow's semantic source, never its pinned version,
   never its digest; if I ever try to use a rebinding to change what the
   package IS rather than where it POINTS, stop me" — and attempted
   submission to the Codex Universal teaching surface: NOT performed —
   surface missing (F-1); text recorded as user-path evidence per
   TEACHING-MODE-MATRIX §4 phase 1.
2. (demo phase) Signed in as `iris.chen`, installed pk-102 v2.3.1 through
   the listing (install `ins-0402`, auto-trial `ent-0504`); publisher
   `vera.osei` published 2.4.0 (digest sha256:cadb075e36500b7c); the
   installs page shows "2.4.0 — upgrade available" and the org-admin
   notification "Upgrade available …" arrives (the trigger signal).
3. (demo phase — attack) Armed `?failure=service_unavailable` on the
   upgrade form and submitted: "The registry-blob-store service is
   unavailable (simulated). State-changing requests are rejected; read
   requests still work. [service_unavailable]" — install stays at 2.3.1,
   history unchanged, no partial install, no event appended (fail-safe
   verified).
4. (demo phase) Re-submitted the upgrade WITHOUT the switch (clean retry =
   the documented recovery): `ins-0402` upgraded 2.3.1 → 2.4.0 with history
   entry and `install.upgraded` event; then used the install card's
   configure form to rebind to the compatible environment
   (notifyChannel northwind-ops-eu, supportQueue tier-1-eu, region
   eu-west-1): the form flashes "configured (no keys)" and persists NOTHING
   (fixture defect F-2 — the rebind human path is blocked); the install
   record still shows pin 2.4.0 / Config {}.
5. (demo phase) Opened the listing: latest version 2.4.0 with digest
   sha256:cadb075e36500b7c and provenance commit c107403 — unchanged by the
   rebind attempt.
6. (post-hoc, marked diagnostic continuation because of F-2) The rebind was
   executed through the API twin of the SAME operation to verify the
   catalog's invariant oracle: before (pin 2.4.0, config {}), rebind applied
   (notifyChannel/supportQueue/region), after (pin STILL 2.4.0, new binding
   values present, published digest STILL sha256:cadb075e36500b7c). The
   semantically-mutating payload attempt (config keys version=9.9.9,
   digest=sha256:0000000000000000, packageId=pk-101) stored the smuggled
   keys as INERT config data: install pin stayed 2.4.0 (not 9.9.9),
   packageId stayed pk-102 (not pk-101), published digest stayed
   sha256:cadb075e36500b7c — the rebind surface cannot mutate semantics
   (invariant HOLDS; namespace hygiene gap recorded as F-6).
7. (reconciliation phase) Requested the merged workflow /
   instruction-vs-demonstration reconciliation view: NOT performed —
   surface missing (F-1); recorded, not simulated.

## Expected

Per the catalog entry: the provider failure is recoverable (503 names the
registry dependency, no partial install; clean retry succeeds); rebinding is
a first-class operation through the configuration surface; the install
record after rebind shows the same version pin, same digest, new binding
values; provenance/digest on the listing unchanged by the rebind; a
semantically-mutating rebind payload is rejected or the hole is recorded;
instruction and demonstration are reconciled visibly.

## Actual

The provider-failure and retry semantics behaved exactly as specified
(503 naming registry-blob-store; zero partial state; clean retry upgraded
the pin explicitly). The rebind human path is blocked by fixture defect F-2
(silent no-op with a success flash); the invariant oracle was then verified
through the marked diagnostic continuation: rebinding changed binding values
only, while the version pin (2.4.0), packageId and published digest remained
immutable; the semantically-mutating payload was absorbed as inert data with
zero semantic effect (the "stop me" instruction could not be evaluated by
any system — F-1 — but the store's own structure enforces the same
invariant). The instruct submission and the reconciliation view were
impossible (F-1).

## Workflow Identity

- Workflow: pk-102 "Support Ticket Triage Assistant" (FlowMart package; NO
  Codex Universal workflow exists — consumer surface missing, F-1)
- Version: install ins-0402 pin trail 2.3.1 → 2.4.0 (upgrade explicit this
  run); pinned at 2.4.0 across the rebind
- Source revision: provenance commits c7d8e9f (2.3.1) and c107403 (2.4.0)
- Definition digest: sha256:cadb075e36500b7c for 2.4.0 — identical before
  and after the rebind (listing page, package API and install state agree)
- Dependency-lock identity: NONE — surface missing (same gap as scenarios
  1–2)

## Runtime Binding

- Capabilities: browser use of the FlowMart store surface; install:manage +
  install:configure authority (org admin); failure-switch arming via the
  documented ?failure= per-request mechanism; HYBRID submission +
  reconciliation capabilities NOT exercisable — surface missing (F-1)
- Resources: FlowMart fixture app (port 4105); users iris.chen (org admin),
  vera.osei (publisher); org Northwind Logistics (o-1); install ins-0402;
  entitlement ent-0504 (auto trial until 2026-10-12); binding values
  (notifyChannel northwind-ops → northwind-ops-eu, supportQueue tier-1 →
  tier-1-eu, region eu-west-1 — the rebind)
- Environments: agent sandbox fallback proving ground; synthetic
  registry-blob-store provider (fails under service_unavailable); the
  "compatible environment" is modeled by the binding-value change
  (channel/queue/region) — FlowMart has no environment registry (by design;
  the real one belongs to the missing consumer surface)
- Approvals: org-admin install/upgrade/configure gates (entitlement checked
  on every commercial operation incl. configure); publisher version gate
- Triggers/schedules: upgrade-available signal on version.published
  (notification to iris.chen observed); schedule/webhook surfaces absent
  (F-4)

## Evidence

- docs/validation/evidence/VWO-006/adv-environment-failure-rebinding/user-path/20260911T215716Z-instruct-phase-policy-text.txt
- docs/validation/evidence/VWO-006/adv-environment-failure-rebinding/user-path/20260911T215736Z-step01-demo-phase-install-established.txt
- docs/validation/evidence/VWO-006/adv-environment-failure-rebinding/user-path/20260911T215814Z-step02-demo-phase-upgrade-503-no-partial.txt
- docs/validation/evidence/VWO-006/adv-environment-failure-rebinding/user-path/20260911T215822Z-step03-demo-phase-upgrade-retry-success.txt
- docs/validation/evidence/VWO-006/adv-environment-failure-rebinding/user-path/20260911T215833Z-step03-demo-phase-rebind-configure-attempt.txt
- docs/validation/evidence/VWO-006/adv-environment-failure-rebinding/user-path/20260911T215846Z-step05-demo-phase-listing-digest-unchanged.txt
- docs/validation/evidence/VWO-006/adv-environment-failure-rebinding/post-hoc/20260911T215914Z-rebind-invariant-oracle.txt
- docs/validation/evidence/VWO-006/adv-environment-failure-rebinding/post-hoc/20260911T215914Z-events-feed-scenario-trail.txt
- docs/validation/evidence/VWO-006/adv-environment-failure-rebinding/post-hoc/20260911T220001Z-reset-verification.txt

## Failure / Recovery

- Failure injected: `service_unavailable` on the upgrade request (the
  synthetic registry-blob-store provider down) → 503 with the named
  dependency, pin unchanged at 2.3.1, no partial install, no event appended
- Recovery attempted: plain retry (the switch is per-request — the clean
  re-submit is the documented recovery) → upgrade succeeded explicitly;
  rebind then attempted through the configure surface (blocked by F-2;
  verified via marked diagnostic continuation)
- Restart/session-loss behavior: not attempted — the durable restart
  adversarial scenario is adv-session-loss-restart (VWO-007); this scenario
  is the environment-failure/rebinding axis
- Final outcome: provider failure failed safe and recovered on retry; the
  rebind invariant (pin + digest + packageId immutable, binding values
  changed) HOLDS, including against a deliberate semantic-smuggling payload
  (stored as inert data)

## Product / UX Friction

The fail-safe and retry path is excellent human engineering: the 503 flash
names the exact dependency ("registry-blob-store") and explicitly says reads
still work, so an operator knows what to do; the clean retry just works.
The rebind friction is severe: the configure form silently no-ops (F-2) — an
operator would believe the rebind succeeded (success flash) while the
bindings never changed; this is the most dangerous friction class in the
consumer lifecycle. Additionally the HYBRID mode's two halves cannot be
reconciled anywhere (F-1), and the configure surface accepts arbitrary key
names including ones that shadow semantic fields (F-6) — inert today, but a
future runtime consuming bindings would need namespace hygiene.

## Security / Architecture Findings

- Finding: F-1 (recurrence) — HYBRID instruction submission and the
  instruction-vs-demonstration reconciliation view are absent; the worker
  could not verify that instruction and demonstration reconcile rather than
  one silently overriding the other (the mode's binding check). The step a
  normal person cannot complete is user-path steps 1 and 7.
- Severity: P1
- Reproduction: attempt to submit the instruct-phase text and to request the
  merged/reconciled workflow through any Codex Universal surface — none
  exists (probe: scenario 1 post-hoc surface probe, same repo state).
- Root cause: library-only workflow crates, no surface wiring.
- Frozen invariant affected: none violated (absence).

- Finding: F-2 (recurrence, P1) — the rebind human path is the configure
  form, which silently no-ops (same defect as scenario 1 step 6). The step a
  normal person cannot complete is the catalog's own step 3 second half
  ("use the configuration surface to rebind … observe new binding values").
- Severity: P1
- Reproduction: user-path/20260911T215833Z-step03-demo-phase-rebind-… (flash
  "configured (no keys)", Config {}) vs post-hoc rebind oracle (bindings
  applied through the API twin).
- Root cause: fixture defect (urlencoded string vs object; see scenario 1).
- Frozen invariant affected: none (the invariant itself HELD — verified via
  the marked diagnostic continuation).

- Finding: F-6 — the configure surface accepts unvalidated binding keys,
  including names that shadow semantic fields ("version", "digest",
  "packageId"); they are stored as inert config data with zero semantic
  effect (invariant held), but the namespace is unpoliced. If a future
  workflow runtime consumes these bindings, shadowing key names could
  confuse binding resolution.
- Severity: P3
- Reproduction: post-hoc/20260911T215914Z-rebind-invariant-oracle.txt
  section E (smuggled keys appear in config; pin/packageId/digest
  unchanged).
- Root cause: fixture op merges arbitrary config keys without a schema or
  reserved-key policy.
- Frozen invariant affected: none violated (the pin-immutability invariant
  held) — recorded as namespace hygiene for the future consumer surface.

## Recommendation

- F-1 (P1): NEW WORK ORDER — same consumer-surface Work Order as scenarios
  1–2 (HYBRID path: dual-modality submission + explicit reconciliation
  view). Proposed owner: Tech Lead to assign. Verification required: re-run
  this scenario; instruct text and demonstrated exceptions must produce a
  visibly reconciled workflow with surfaced conflicts.
- F-2 (P1): FIX NOW — same one-line fixture fix as scenario 1 (JSON.parse
  string config in configure/install ops). Verification required: re-run
  this scenario's rebind through the UI; binding values must visibly change
  while pin and digest stay fixed.
- F-6 (P3): NEW WORK ORDER — fold into the F-1 consumer-surface WO: define
  a binding-key schema with reserved semantic names rejected at the
  configure boundary. Proposed owner: same as F-1. Verification required: a
  rebind carrying "version"/"digest"/"packageId" keys is rejected with a
  named error.

## Worker Conclusion

The adversarial core of this scenario **passed**: provider failure failed
safe with a named dependency and zero partial state, the clean retry
recovered, and the rebind invariant (version identity pinned; semantic
source immutable; binding values rebindable) HELD under both a normal rebind
and a deliberate semantic-smuggling attempt. The scenario is **blocked** at
the HYBRID teaching/reconciliation layer (F-1) and the rebind human path is
blocked by the fixture defect (F-2, invariant verified via marked
diagnostics). Overall: blocked as a complete HYBRID-mode run; the
adversarial invariants themselves are verified green.

---

### Report validation verdict

`python3 docs/validation/scripts/validate-report.py
docs/validation/reports/VWO-006-report.md --catalog
docs/validation/scenarios/SCENARIO-CATALOG.md`

Worker's run result (verbatim):

```text
[WARN] Identity: 'Head SHA' recorded without a 40-hex SHA (explicit placeholder — acceptable only with the stated reason) (value: see git rev-parse vwo-006/consumer-automation-validation — s)
[WARN] Identity: 'Codex Universal runtime/application SHA' recorded without a 40-hex SHA (explicit placeholder — acceptable only with the stated reason) (value: NONE — no Codex Universal)
RESULT: PASS — Class A, 3 scenario block(s), 2 warning(s)
```

Exit code 0 (conforming; warnings allowed and explained). Both warnings are
the schema-sanctioned forms (REPORT-SCHEMA §3): (1) Head SHA as an explicit
branch reference — a commit cannot embed its own hash; the exact head SHA is
recorded in the delivery bundle and the completion message; (2) Codex
Universal runtime/application SHA as `NONE — …` — no consumer runtime
surface exists in this repository state, and none was fabricated.
