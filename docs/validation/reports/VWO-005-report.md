# VWO-005 Report — Workflow Creator and Marketplace Seller Validation

Class A scenario-run report. Six catalog scenarios executed through the real
application surfaces (browser over the VWO-002 fixtures), evidence under
`docs/validation/evidence/VWO-005/`. The consolidated summary (execution,
verification, acceptance mapping, findings) is carried as Identity-section
bullets because the schema fixes the heading sequence to Identity + whole
scenario blocks.

## Identity

- Work Order: VWO-005 (Wave 1, `docs/validation/work-orders/VWO-005.md`)
- Worker persona: workflow creator / marketplace seller — Vera Osei (publisher) primary; lifecycle counterparts Dan Kowalski (marketplace admin), Iris Chen and Petra Voss (org admins / buyers), Ravi Gupta (org member), Mia Torres (auditor), Noor Haddad + Ari Klein (software), Hana Kim + Camille Dubois (media)
- Base branch: main
- Base SHA: 0d314fd09cec70d000f33c81286edcd8cbf72501
- Head SHA: see git rev-parse vwo-005/marketplace-seller-validation — single delivery commit on top of base 0d314fd09cec70d000f33c81286edcd8cbf72501 (a commit cannot embed its own hash)
- Validation environment: agent sandbox fallback proving ground per VWO-001 §2 (E2B/Composio integration ABSENT; node v24.19.0, bun 1.3.14, python 3.12.14, rustc 1.95.0 pinned; browser = agent-browser 0.35.0 over Playwright chromium; fixtures on 127.0.0.1:4101-4105)
- E2B template/workspace identity, if used: N/A — E2B/Composio integration absent (VWO-001 fallback record governs)
- Codex Universal runtime/application SHA: NONE — no user-facing Codex Universal runtime/application surface exists at this SHA: the eight workflow-plane crates (teaching-compiler, workflow-app, workflow-forge, workflow-distribution, workflow-triggers, workflow-evolution, workflow-durable, workflow-contracts) are library-only, depended on by no CLI/TUI/app-server/MCP crate (evidence `_infra/product-surface-dependency-probe`); the family itself cargo-checks clean at base (evidence `_infra/m4-workflow-family-cargo-check.txt`)
- Date/time window: 2026-09-11T22:12Z–2026-09-11T22:50Z (UTC)

Consolidated execution summary (bullets, machine-checked fields above):

- Scenarios run (6, in execution order): `marketplace-create-and-sell` (HYBRID, FlowMart) · `marketplace-fork-and-improve` (INSTRUCT, FlowMart) · `marketplace-discover-install-configure` (DEMONSTRATE, FlowMart) · `marketplace-upgrade-rollback` (INSTRUCT, FlowMart — catalog primary owner VWO-006, exercised by VWO-005 per the WO mission and the catalog §1 "also exercised by" column) · `software-pr-triage` (INSTRUCT, ForgeOps — additional create+publish cycle) · `media-editorial-publishing` (DEMONSTRATE, PressRoom — additional create+publish cycle).
- Teaching modes covered: HYBRID ×1, INSTRUCT ×3, DEMONSTRATE ×2 — all three modes exercised, per WO and VALIDATION-PROGRAM.md §6.
- Lifecycle transitions REACHED through normal application surfaces: listing publish immutable v1 (digest visible) → creation-time attribution/licensing → admin entitlement grant → marketplace discovery via search → install under a DIFFERENT consumer identity (buyer org, auto/admin entitlement) → improvement release as new immutable version on the origin listing → duplicate-version and stale-pin rejections → auditor provenance verification (digest MATCH) → explicit pinned upgrade → explicit recorded rollback → entitlement-blocked upgrade failing safe (pin unchanged). Commercial decisions (entitlement grant/revoke, install/upgrade) never altered published definitions (digests byte-stable; recomputed MATCH post-hoc in every scenario).
- Lifecycle transitions NOT REACHED (missing product surfaces, recorded not simulated): teach → compile → review → approve → publish of a workflow definition (no teaching/compile surface anywhere in the product); fork with lineage (no fork surface or lineage data model); improvement candidate from execution evidence with validation/approval-before-publish (no execution-evidence surface, no proposal gate); post-publication commercial-policy configuration (creation-time license model only).
- Verification commands and exact results: (1) `bash docs/validation/fixtures/run-all.sh` → all five apps healthy on 4101-4105; (2) `bash docs/validation/fixtures/verify-sweep.sh` → `RESULT: 59 passed, 0 failed` (transcript `_infra/fixture-startup-verify-sweep`); (3) `bash docs/validation/proving-ground/capability_check.sh` → first run `RESULT: FAIL` (cargo absent — required check), after pinned rustup 1.95.0 install → `RESULT: PASS — required capabilities verified` (E2B + xdotool fidelity failures remain recorded per the VWO-001 fallback; transcript `_infra/capability-check-pass`); (4) `cargo check -p codex-workflow-contracts -p codex-workflow-forge -p codex-workflow-app -p codex-workflow-triggers -p codex-workflow-evolution -p codex-workflow-distribution -p codex-workflow-durable` → `Finished \`dev\` profile … in 5m 37s`, 0 errors (transcript `_infra/m4-workflow-family-cargo-check`); (5) `python3 docs/validation/scripts/validate-report.py docs/validation/reports/VWO-005-report.md --catalog docs/validation/scenarios/SCENARIO-CATALOG.md` → `RESULT: PASS — Class A, 6 scenario block(s), 2 warning(s)` (the two schema-permitted warnings: Head SHA recorded as branch-reference; runtime/application SHA recorded as NONE-with-reason — both honest placeholders per REPORT-SCHEMA §3).
- Findings summary (7 unique, canonical home scenario in brackets): P0 ×0 · P1 ×5 — F1 teaching/compile/review/approve surface absent product-wide [marketplace-create-and-sell]; F2 FlowMart install-configuration form silently drops JSON input [marketplace-create-and-sell]; F4 fork surface + lineage data model absent [marketplace-fork-and-improve]; F5 improvement-candidate-from-execution-evidence + approval-before-publish surface absent [marketplace-fork-and-improve]; F6 cross-tenant install mutation accepted server-side in FlowMart [marketplace-upgrade-rollback] · P2 ×1 — F3 post-publication commercial-policy/attribution/licensing configuration absent [marketplace-create-and-sell] · P3 ×1 — F7 PressRoom approved-story attach dead-end [media-editorial-publishing]. Per-block Severity lines below count UNIQUE canonical findings only; other blocks reference them.
- Missing product surfaces (recorded, not simulated): (a) any teaching/compile/review/approve surface for DEMONSTRATE, INSTRUCT or HYBRID (no CLI subcommand, no app-server protocol method, no UI; the workflow plane is mounted by no user-facing crate); (b) workflow definition identities exposed to end users (source revision pin, definition digest of executable semantics, dependency-lock identity — only opaque package digests exist in FlowMart); (c) fork + lineage; (d) execution-evidence records and improvement-proposal gates; (e) post-publication commercial-policy configuration; (f) an install-configure human path that actually persists config (form defect F2).
- WO acceptance mapping: "Published executable semantics remain byte/integrity-stable through marketplace transitions" → every scenario's digest checks (pk-0104@1.0.0 sha256:4d61dd23bfcbb8a5 recomputed MATCH after grant/install/configure; pk-102@2.3.1/2.3.2/2.4.0 digests stable and auditor-verified MATCH through upgrade/rollback) — held. "Forks preserve lineage" → could NOT be evaluated: no fork surface exists (F4) — failed by absence, recorded. "Commercial decisions never alter workflow definition" → held at the package level: entitlement grant/revoke, install, upgrade, rollback, configure left manifest/provenance/digest byte-identical (post-hoc integrity artifacts). "Upgrades and rollback remain explicit" → held behaviorally: pinned expectedCurrentVersion enforced ([data_conflict] on stale pin), explicit recorded rollback only, entitlement-blocked upgrade fails safe. "The worker can perform the lifecycle through normal application surfaces" → NOT met for the teach→compile→review→approve half (F1): every other transition was performed through browser forms as a normal person.
- Deliverables: this report + 62 evidence artifacts under `docs/validation/evidence/VWO-005/` (6 scenario directories + `_infra`), zero changes outside `docs/validation/evidence/VWO-005/**` and `docs/validation/reports/VWO-005-report.md`; no codex-rs/** or semantic-contract changes; repo tree kept free of build artifacts (cargo target cleaned).

## Scenario

- Scenario id: marketplace-create-and-sell
- Industry: marketplace
- Scenario: create and sell a monthly sales-report workflow (HYBRID) — the VWO-005 anchor
- Teaching mode: HYBRID
- User goal: as Vera Osei (publisher), teach the monthly sales-report workflow, publish immutable v1, configure attribution/licensing/commercial policy, and have a different consumer identity (Iris Chen, Northwind) discover and install it under entitlement

## User Path

1. Signed in as `vera.osei` through the `/login` form (persona table rendered on the page).
2. HYBRID instruct phase: wrote the monthly sales-report instruction text (monthly ops-app totals pull → one-page report → finance notify, retry-once on ops failure) and attempted to submit it to the Codex Universal teaching surface — no such surface exists anywhere reachable by a normal user; the FlowMart `/publisher` console explicitly states it is the store surface only; recorded instead of fabricating.
3. Created the listing "Monthly Sales Report" (slug `monthly-sales-report`, category finance-ops, license model subscription) through the publisher form — observed listing `pk-0104` published at v1.0.0 with immutable version entry, manifest digest `sha256:4d61dd23bfcbb8a5`, and provenance (commit `de1cbb1`, built/signed by vera.osei).
4. Probed post-publication configuration of attribution/licensing/commercial policy on the listing and publisher pages — no edit/policy surface exists; attribution and licensing were fixed at creation; the only commercial objects an admin can manage are separate entitlement records.
5. Signed in as `dan.kowalski` (marketplace admin) and granted Northwind Logistics a subscription entitlement `ent-0504` for the listing through the `/entitlements` grant form.
6. Signed in as `iris.chen` (Northwind org admin): searched `/catalog?q=sales`, discovered the listing, installed v1.0.0 (install `ins-0402` under `ent-0504`), then attempted to configure the install through the `/installs` form — the JSON config input was silently discarded ("configured (no keys)").
7. HYBRID demo-phase exception: as vera.osei re-submitted the same listing slug — rejected with `[duplicate_event]`; nothing was published twice.

## Expected

Per the catalog entry: the listing plus immutable version with digest and provenance is visible; the buyer discovers the listing through catalog search and installs it under an entitlement; commercial policy is visible on the listing; executable semantics stay unchanged through the transitions; and the HYBRID teaching surface reconciles the initial instructions with the demonstrated duplicate-slug exception into a compiled workflow whose monthly schedule trigger and finance notification carry through to what is published.

## Actual

Fixture-side marketplace lifecycle behaved correctly end-to-end: listing published at v1.0.0 with digest `sha256:4d61dd23bfcbb8a5` and provenance visible; admin entitlement `ent-0504` granted; buyer discovered the listing via `/catalog?q=sales` and installed it as `ins-0402` under that entitlement; the duplicate listing slug was rejected (`[duplicate_event]`); post-hoc digest recompute (fixture algorithm) MATCHed after every transition and the events `listing.published`, `entitlement.granted`, `package.installed`, `install.configured` were all recorded. Three deviations from expected: (1) the HYBRID teaching half could not run at all — no product surface accepts instructions or observes demonstrations, so no workflow was created, compiled, reviewed or approved, and nothing workflow-shaped was published (F1); (2) the install configuration form dropped its input — a "configured" history entry and success flash were recorded while the config stayed `{}` (F2); (3) commercial policy beyond the creation-time license model has no configuration surface at all (F3).

## Workflow Identity

- Workflow: NONE — teaching/compile surface missing (no workflow could be created); nearest product identity is FlowMart listing pk-0104 "Monthly Sales Report", an opaque package artifact with no workflow definition
- Version: 1.0.0 (FlowMart package version — immutable, duplicate-publish rejected)
- Source revision: NONE — surface missing (provenance commit de1cbb1 is fixture-generated provenance, not a pinned workflow source revision)
- Definition digest: sha256:4d61dd23bfcbb8a5 (FlowMart manifest digest — the integrity-stable identity that survived all marketplace transitions; recomputed MATCH post-hoc)
- Dependency-lock identity: NONE — surface missing

## Runtime Binding

- Capabilities: browser use (fixture navigation, forms, flashes); teaching/compile/review/approve capability NOT exercised — surface missing
- Resources: FlowMart fixture app (marketplace family, port 4105); seeded personas vera.osei (publisher), dan.kowalski (admin), iris.chen (buyer org admin)
- Environments: agent sandbox fallback proving ground; no E2B workspace
- Approvals: publisher listing gate (listing:publish) at creation; admin entitlement grant (entitlement:grant); entitlement validity gate enforced at install (ent-0504 active → install allowed)
- Triggers/schedules: intended monthly schedule + finance notification carried by the taught workflow — NOT exercised (teaching surface missing); FlowMart deliberately has no execution semantics

## Evidence

- docs/validation/evidence/VWO-005/marketplace-create-and-sell/user-path/20260911T222728Z-step01-login-page-persona-table.txt
- docs/validation/evidence/VWO-005/marketplace-create-and-sell/user-path/20260911T222756Z-step02-instruct-phase-no-teaching-surface.txt
- docs/validation/evidence/VWO-005/marketplace-create-and-sell/user-path/20260911T222756Z-step02-publisher-console-no-teach-input.txt
- docs/validation/evidence/VWO-005/marketplace-create-and-sell/user-path/20260911T222812Z-step03-listing-published-v1-digest-provenance.txt
- docs/validation/evidence/VWO-005/marketplace-create-and-sell/user-path/20260911T222842Z-step04-attribution-licensing-creation-only.txt
- docs/validation/evidence/VWO-005/marketplace-create-and-sell/user-path/20260911T222905Z-step05-admin-grants-buyer-entitlement.txt
- docs/validation/evidence/VWO-005/marketplace-create-and-sell/user-path/20260911T222917Z-step06-buyer-discovers-listing-via-search.txt
- docs/validation/evidence/VWO-005/marketplace-create-and-sell/user-path/20260911T222929Z-step06-buyer-installs-under-entitlement.txt
- docs/validation/evidence/VWO-005/marketplace-create-and-sell/user-path/20260911T223024Z-step06-configure-form-drops-input.txt
- docs/validation/evidence/VWO-005/marketplace-create-and-sell/user-path/20260911T223024Z-step06-configure-form-drops-input-note.txt
- docs/validation/evidence/VWO-005/marketplace-create-and-sell/user-path/20260911T223102Z-step07-demo-phase-duplicate-slug-rejected.txt
- docs/validation/evidence/VWO-005/marketplace-create-and-sell/post-hoc/20260911T223144Z-lifecycle-event-pulls.txt
- docs/validation/evidence/VWO-005/marketplace-create-and-sell/post-hoc/20260911T223144Z-digest-integrity-and-rootcause.txt

## Failure / Recovery

- Failure injected: natural duplicate listing slug (HYBRID demo-phase exception); configure-form input loss surfaced as product defect F2
- Recovery attempted: duplicate slug → cleanly rejected, existing listing untouched, nothing published twice; configure → re-attempted through the form (input dropped again); root cause isolated post-hoc — the HTML form POSTs a urlencoded string while the domain op accepts only object config, so the API twin succeeds where the human path always records "no keys"
- Restart/session-loss behavior: not attempted — no Codex Universal runtime process exists to restart (runtime surface absent); fixture restarts between scenarios were handled by reset-scenario.sh with state persisted to disk (world.reset recorded); restart/session-loss is VWO-007's owned adversarial surface
- Final outcome: fixture marketplace lifecycle PASS (publish v1 → grant → discover → install → duplicate-reject, digest byte-stable throughout); the teach→compile→review→approve→publish workflow lifecycle BLOCKED by the missing teaching surface; consumer configuration path defective (F2)

## Product / UX Friction

The publisher's and buyer's app paths are clear and legible (login → form → flash → record), and the immutable version table with digest and provenance gives a normal person real integrity visibility. The friction points: after creating a listing there is nowhere to go for the actual workflow — the scope note on /publisher openly says execution belongs to "the platform consuming them", but that platform exposes no surface to a normal user (nowhere to submit instructions, nothing to review, no approval step); the configure textarea accepts JSON, confirms success, and silently keeps the empty config — the worst kind of friction because it looks like it worked; and commercial policy beyond the license model simply has no fields anywhere.

## Security / Architecture Findings

- Finding: F1 — the Codex Universal teaching/compile/review/approve surface is absent from every user-facing surface at this SHA: the codex CLI subcommand enum has no workflow/teaching command, the app-server protocol defines no workflow/teaching method, the TUI/MCP surfaces mount nothing from the workflow plane, and the eight workflow crates are library-only (dependency probe shows only env-adapters, resource-providers, eval-compat and the family itself depend on them). The acceptance question fails at its first step: a normal person cannot teach, compile, review, approve, or publish a workflow without understanding internals — there is no product surface at all, so the entire WO lifecycle's workflow half is unreachable.
- Severity: P1
- Reproduction: complete user-path steps 1–7 as any persona; attempt to locate any workflow-teaching, instruction, observation, compile-proposal, review or approval surface in FlowMart (pages: /, /catalog, /listings/:slug, /publisher, /installs, /entitlements, /notifications) and in the Codex product (CLI --help surface enumerated from source at base; app-server protocol; TUI); none exists. Post-hoc: workspace Cargo.toml dependency scan (product-surface-dependency-probe artifact).
- Root cause: the workflow plane (M4 family, implemented and cargo-check-clean) has never been mounted by any user-facing crate — an integration gap between the library plane and the product shell, not a defect inside the workflow crates.
- Frozen invariant affected: none violated (absence, not violation); recorded per VALIDATION-PROGRAM.md §8 — report missing surfaces, never fabricate. Precedent: the worked example report classifies the missing teaching surface as P1.
- Finding: F2 — the FlowMart install-configuration form silently discards its JSON input: the op accepts only object-typed config, the form posts a urlencoded string, so every human-path configure records a "configured (no keys)" history entry and a success flash while the config stays {}. A normal person cannot configure an installed package through the UI at all.
- Severity: P1
- Reproduction: as iris.chen on /installs, fill "Config keys to merge (JSON)" with {"opsApp":"forgeops-4102",…} and click "Apply config" — flash "Install ins-0402 configured (no keys)", Config cell {}, history entry "configured" (user-path artifacts step06); API twin with an object body succeeds with the same keys (post-hoc root-cause artifact).
- Root cause: ops-install.js configureInstall() guards `typeof b.config === 'object'`, but readBody() parses form-urlencoded bodies into strings; no JSON.parse bridge exists on the form path (VWO-002 fixture defect).
- Frozen invariant affected: none (fixture defect; commercial decisions still never altered published definitions).
- Finding: F3 — post-publication configuration of attribution, licensing and commercial policy does not exist: the license model is chosen once at listing creation (free/per-seat/subscription), there is no pricing field, no revenue-share/trial-length/seats policy fields, and no way to change attribution after creation; commercial administration is limited to granting/revoking separate entitlement records.
- Severity: P2
- Reproduction: as vera.osei open /listings/monthly-sales-report and /publisher after publishing — enumerate every form: only new-listing and new-version forms exist; the step04 user-path artifact records the full probe.
- Root cause: FlowMart models commercial policy only as creation-time listing fields plus the entitlement lifecycle (VWO-002 scope decision).
- Frozen invariant affected: none (scope gap, recorded not simulated — the catalog's "configure … commercial policy" step has no surface).

## Recommendation

- NEW WORK ORDER — F1: mount the existing workflow plane behind at least one user-facing surface (CLI subcommand, app-server method pair, or UI) exposing teach→compile→review→approve→publish for all three teaching modes; proposed owner: Tech Lead to scope and dispatch a bounded integration WO against current main; verification required: re-run all six VWO-005 scenarios — the instruct/demo/replay steps must complete without internals, and compiled workflow identities (source revision, definition digest, dependency lock) must become user-visible.
- FIX NOW — F2: one-line fixture remediation (parse the form config field as JSON in configureInstall or bridge in the form route); proposed owner: bounded VWO-002 follow-up remediation WO; verification required: human-path configure persists keys, flash lists them, and the wave-2 entitlement-race scenarios (VWO-008/009) re-run clean.
- NEW WORK ORDER — F3: commercial-policy surface (pricing/policy fields or an explicit statement that policy lives entirely in entitlements); proposed owner: same fixture-remediation WO as F2 (one FlowMart remediation bundle); verification required: catalog step 4 of this scenario becomes performable or the catalog entry is corrected.

## Worker Conclusion

The scenario is **blocked** as a workflow-lifecycle test (the teaching/compile/review/approve half cannot be performed by any normal user — F1) while the fixture-side marketplace half passed in full: immutable v1 published with digest and provenance, entitlement granted, buyer discovery and install under a different identity, duplicate rejection, and byte-stable executable-package semantics through every transition; the consumer configuration step is additionally defective (F2). Not a pass: the WO's acceptance question cannot be answered while the workflow half is unreachable.

## Scenario

- Scenario id: marketplace-fork-and-improve
- Industry: marketplace
- Scenario: fork an existing published workflow, improve it from execution evidence, and produce a new immutable release preserving lineage (INSTRUCT)
- Teaching mode: INSTRUCT
- User goal: as Vera Osei (publisher, fork origin owner), fork the Support Ticket Triage Assistant (pk-102), propose an evidence-driven triage improvement, and publish a new immutable release only after validation/approval — with Mia Torres (auditor) verifying provenance/lineage

## User Path

1. Reset the marketplace fixture (`reset-scenario.sh marketplace`); signed in as `vera.osei`; searched `/catalog?q=triage`; opened the Support Ticket Triage Assistant listing (`pk-102`, latest 2.3.1).
2. Searched every reachable page for a fork affordance — the listing page offers overview, immutable versions, provenance verification, version publish, install and review forms; NO fork button, no lineage display, no forkOf/parent field anywhere (the publisher console's listing form creates only fresh listings).
3. INSTRUCT: wrote the fork-and-improve instruction text (fork pk-102@2.3.1; from execution evidence route repeat-contact tickets to the senior queue; publish only after validation and approval) and attempted to submit it — no teaching surface exists to accept it; additionally no execution-evidence records exist anywhere (packages are opaque artifacts) and no proposal/validate/approve-before-publish flow exists.
4. Exercised what exists instead, explicitly NOT a simulated fork: as vera.osei published the improvement as a NEW immutable version 2.3.2 on the ORIGIN listing with the evidence-driven changelog; observed the new version entry, digest `sha256:8e2afc2ef9d9698a`, and provenance.
5. Signed in as `mia.torres` (auditor); ran provenance verification from the listing page — verified MATCH with the published digest.
6. Immutability and conflict probes: re-published version 2.3.2 → natural `[duplicate_event]` "already exists — published versions are immutable"; armed `?failure=data_conflict` on the version-publish form → `[data_conflict]` rejected; signed in as `iris.chen` (buyer) and observed the version-publish form is absent for her role, with the post-hoc API twin confirming the server-side 403 (`permission_denied`, required `version:publish`).

## Expected

Per the catalog entry: forking produces a new listing with attribution/lineage to pk-102; the INSTRUCT inference produces an improvement-candidate workflow with validation-before-publish and approval gates; the fork's v1.0.0 is immutable (duplicate version rejected); provenance verification passes and shows the fork parent; a non-publisher cannot publish versions.

## Actual

The fixture-side immutable-release and verification halves behaved correctly: version 2.3.2 published (digest `sha256:8e2afc2ef9d9698a`, provenance recorded, installer notified), duplicate version rejected, stale-conflict switch rejected, auditor provenance verification returned MATCH, and the version-publish permission gate held both in the UI (form absent for the buyer) and server-side (403 with the required permission). The fork-and-improve core could not be performed: there is no fork surface and no lineage data model at all (state inspection confirms no forkOf/parent/lineage keys exist anywhere in the marketplace store), so "forks preserve lineage" could not even be evaluated (F4); there is no execution-evidence source and no improvement-candidate/approval-before-publish surface, so the governed improvement loop is entirely absent (F5); and the INSTRUCT submission itself had no surface to accept it (F1, canonical home in the create-and-sell block).

## Workflow Identity

- Workflow: NONE — teaching/compile surface missing; nearest product identity is FlowMart listing pk-102 "Support Ticket Triage Assistant" (origin) — no fork entity could be created
- Version: 2.3.1 (origin, seed) and 2.3.2 (improvement release on origin, published during the run)
- Source revision: NONE — surface missing (fixture-generated provenance commits only)
- Definition digest: sha256:8e2afc2ef9d9698a (v2.3.2 manifest digest, auditor-verified MATCH); origin v2.3.1 digest sha256:04cf049e110cc2cc recomputed MATCH post-hoc
- Dependency-lock identity: NONE — surface missing

## Runtime Binding

- Capabilities: browser use; fork/lineage, execution-evidence analysis and improvement-proposal capabilities NOT exercised — surfaces missing
- Resources: FlowMart fixture app (port 4105); personas vera.osei (publisher), mia.torres (auditor), iris.chen (buyer contrast)
- Environments: agent sandbox fallback proving ground; no E2B workspace
- Approvals: publisher version gate (version:publish) enforced in UI and server-side; auditor verification surface (audit:verify) exercised with MATCH result; the intended improvement-candidate approval gate could NOT be exercised — surface missing (F5)
- Triggers/schedules: intended evidence-driven improvement trigger + approval-before-publish — NOT exercised (surfaces missing)

## Evidence

- docs/validation/evidence/VWO-005/marketplace-fork-and-improve/user-path/20260911T223258Z-step02-pk102-listing-no-fork-affordance.txt
- docs/validation/evidence/VWO-005/marketplace-fork-and-improve/user-path/20260911T223258Z-step03-instruct-no-fork-no-improvement-surface.txt
- docs/validation/evidence/VWO-005/marketplace-fork-and-improve/user-path/20260911T223312Z-step04-improvement-release-on-origin-232.txt
- docs/validation/evidence/VWO-005/marketplace-fork-and-improve/user-path/20260911T223341Z-step06-duplicate-version-rejected-immutable.txt
- docs/validation/evidence/VWO-005/marketplace-fork-and-improve/user-path/20260911T223341Z-step06-data-conflict-on-version-publish.txt
- docs/validation/evidence/VWO-005/marketplace-fork-and-improve/user-path/20260911T223417Z-step05-auditor-provenance-verification-match.txt
- docs/validation/evidence/VWO-005/marketplace-fork-and-improve/user-path/20260911T223450Z-step06-buyer-no-publish-version-surface.txt
- docs/validation/evidence/VWO-005/marketplace-fork-and-improve/post-hoc/20260911T223509Z-permission-gate-events-digests-lineage-a.txt

## Failure / Recovery

- Failure injected: natural duplicate version 2.3.2; `?failure=data_conflict` armed on the version-publish form (documented fixture switch contract)
- Recovery attempted: duplicate → rejected, existing version untouched (immutability held); data_conflict → rejected with the simulated concurrent-release message; both left the published 2.3.2 digest unchanged
- Restart/session-loss behavior: not attempted — no Codex Universal runtime process exists to restart; fixture reseed between scenarios via reset-scenario.sh (world.reset recorded)
- Final outcome: immutable-release + provenance-verification half PASS; fork-with-lineage half BLOCKED by absent surfaces (F4, F5); the WO's "improvement candidate … published only after validation/approval" transition could not be performed anywhere

## Product / UX Friction

A publisher wanting to build on someone's (or their own) published workflow has literally no path: the listing page ends at "publish a new version on the same listing", and a seller who wants a derived product must hand-create a fresh listing with no connection to the origin — lineage, attribution and credit are impossible to express in the UI. The auditor's verification surface, by contrast, is exemplary for a normal person: one button, a clear MATCH statement with the digest. The INSTRUCT intent also has nowhere to go: describing an improvement in words has no input surface, and even if it did, there is no execution evidence in the marketplace to ground it.

## Security / Architecture Findings

- Finding: F4 — the marketplace has no fork surface and no lineage data model: no fork affordance on any page, no forkOf/parent/lineage keys anywhere in the store, and publishListing creates only fresh root listings. The WO acceptance "forks preserve lineage" is untestable because forks cannot exist; a normal person cannot perform the mandated fork-and-new-immutable-release transition at all (the workaround — publishing on the origin listing — explicitly does NOT preserve lineage and was performed only as the nearest existing behavior, never recorded as a fork).
- Severity: P1
- Reproduction: as vera.osei open /catalog and the pk-102 listing; enumerate every form and button (step02 artifact); post-hoc state scan for forkOf/lineage/parent keys returns none (post-hoc artifact §4).
- Root cause: FlowMart scope decision — packages are opaque artifacts with no derivation model (VWO-002 forbidden list keeps workflow semantics out, but lineage is a store concern, not a workflow-engine concern).
- Frozen invariant affected: none violated inside existing surfaces; the missing capability blocks a WO-mandated lifecycle transition (recorded per VALIDATION-PROGRAM.md §8).
- Finding: F5 — there is no execution-evidence surface, no improvement-candidate proposal, and no validation/approval-before-publish gate anywhere: FlowMart records commercial events only (installs/upgrades/entitlements), so "produce an improvement candidate from execution evidence and publish only after validation/approval" has neither a data source nor a governance flow. The governed-improvement half of the WO objective is unreachable through normal surfaces.
- Severity: P1
- Reproduction: as vera.osei attempt to locate any execution/evidence/proposal/approval surface across /catalog, /listings/:slug, /publisher, /installs, /entitlements, /notifications (step03 artifact); none exists; packages carry only manifest+digest+provenance.
- Root cause: execution evidence belongs to the Codex Universal workflow plane, which is not mounted anywhere user-facing (compounds F1); the store side never modeled a proposal/upgrade-recommendation object.
- Frozen invariant affected: none violated (absence); the "publish only after validation/approval" governance invariant could not be exercised in either direction.

## Recommendation

- NEW WORK ORDER — F4: add a fork surface with explicit lineage records (forkOf package+version, preserved attribution) to the marketplace fixture, or explicitly re-scope the catalog scenario if forking is deliberately out of the store's scope; proposed owner: fixture-remediation WO (FlowMart bundle with F2/F6); verification required: fork produces a new listing whose lineage to pk-102 is user-visible and survives provenance verification.
- NEW WORK ORDER — F5: define the improvement-candidate lifecycle (evidence source, proposal, validation, explicit approval gate before publish) — this is a product-surface requirement owned by the Codex Universal evolution plane, so it should ride the F1 integration WO; proposed owner: Tech Lead (surface integration WO); verification required: re-run this scenario — the improvement must be publishable only after an explicit approval, with the approval evidenced.

## Worker Conclusion

The scenario is **blocked**: the fork, the evidence-driven improvement candidate, and the approval-before-publish gate are all missing product surfaces (F4, F5 — plus the shared missing teaching surface F1), so the mandated lifecycle cannot be performed by a normal person. What exists behaved correctly and is fully evidenced: the new immutable release 2.3.2 on the origin listing, duplicate/conflict rejections, the version-publish permission gate (UI and server-side), and the auditor's digest verification MATCH.

## Scenario

- Scenario id: marketplace-discover-install-configure
- Industry: marketplace
- Scenario: discover a workflow in the catalog, inspect it before installing, install with entitlement as the org admin, and configure it (DEMONSTRATE)
- Teaching mode: DEMONSTRATE
- User goal: as Iris Chen (Northwind org admin), discover pk-102 via catalog search, inspect its digest/provenance/version history/licensing BEFORE installing, install it under entitlement, and configure it — demonstrated for Codex Universal to observe the consumer path (catalog primary owner VWO-006; run here as the VWO-005 WO's DEMONSTRATE-mode marketplace cycle)

## User Path

1. Reset the marketplace fixture; signed in as `iris.chen` (org admin, Northwind Logistics); searched `/catalog?q=triage`.
2. Opened the listing and inspected it BEFORE install: manifest digest `sha256:04cf049e110cc2cc`, provenance card (source repo, commit, built/signed by), immutable version history, license model.
3. Installed v2.3.1 for Northwind through the listing's install form — install `ins-0402` recorded with an auto-granted 30-day trial entitlement `ent-0504` (until 2026-10-12).
4. Attempted to install pk-101 (Invoice Autofill) for Northwind — natural `[stale_entitlement]` rejection naming entitlement `ent-0502`, expired 2026-08-01 (the documented recovery is renew/grant then install; the admin-grant path was exercised in the create-and-sell scenario).
5. Signed in as `ravi.gupta` (Northwind org member) and opened the same listing — NO install surface is rendered for him (role gate: install:manage required); he retains the review form; the post-hoc API-twin probe confirmed the server-side 403 with the required permission.
6. As `iris.chen`, attempted to configure `ins-0402` through the /installs form — the JSON input was silently dropped ("configured (no keys)", Config cell {}).
7. DEMONSTRATE replay attempt: no Codex Universal observation/compile surface exists to watch steps 1–6 or produce the inferred consumer workflow — recorded, not simulated.

## Expected

Per the catalog entry: discovery and inspection are possible before install with provenance/digest visible on the listing; install requires org admin plus a valid entitlement; the pk-101 install is blocked with the entitlement id and expiry; the org member is blocked with the required permission; the configuration persists; and replaying the path under observation yields a compiled consumer workflow (search → inspect → install → configure with entitlement and role gates).

## Actual

Every fixture gate behaved exactly as the catalog describes: discovery via search, full pre-install inspection (digest/provenance/versions/licensing visible), install `ins-0402` recorded under auto-trial `ent-0504`, the pk-101 install blocked with `[stale_entitlement]` naming `ent-0502` and its expiry, and the org member blocked by role (UI hides the surface; server enforces 403 `permission_denied` with required `install:manage`). Two deviations: the configuration did NOT persist through the human path (F2 — the form drops its JSON input while recording a success flash and a "configured" history entry), and the DEMONSTRATE replay was impossible because no observation/compile surface exists (F1 — canonical home in the create-and-sell block; no new unique finding here).

## Workflow Identity

- Workflow: NONE — teaching/compile surface missing; nearest product identity is FlowMart listing pk-102 "Support Ticket Triage Assistant" as installed by Northwind (opaque package)
- Version: 2.3.1 (installed pin)
- Source revision: NONE — surface missing
- Definition digest: sha256:04cf049e110cc2cc (pk-102@2.3.1 manifest digest — displayed pre-install and unchanged post-install/configure)
- Dependency-lock identity: NONE — surface missing

## Runtime Binding

- Capabilities: browser use; observation/compile capability NOT exercised — surface missing (DEMONSTRATE mode's core loop)
- Resources: FlowMart fixture app (port 4105); personas iris.chen (org admin), ravi.gupta (org member contrast)
- Environments: agent sandbox fallback proving ground; no E2B workspace
- Approvals: org-admin install gate (install:manage) enforced in UI and server-side; entitlement gate enforced on install (active trial ent-0504 allows; expired ent-0502 blocks with id + expiry)
- Triggers/schedules: intended schedule/trigger configuration step of the consumer workflow — NOT exercised (teaching surface missing; the configure surface itself is defective F2)

## Evidence

- docs/validation/evidence/VWO-005/marketplace-discover-install-configure/user-path/20260911T223535Z-step01-step01-catalog-search-triage.txt
- docs/validation/evidence/VWO-005/marketplace-discover-install-configure/user-path/20260911T223541Z-step02-step02-listing-inspect-digest-provenance.txt
- docs/validation/evidence/VWO-005/marketplace-discover-install-configure/user-path/20260911T223553Z-step03-step03-install-with-auto-trial.txt
- docs/validation/evidence/VWO-005/marketplace-discover-install-configure/user-path/20260911T223607Z-step04-step04-expired-trial-rejected.txt
- docs/validation/evidence/VWO-005/marketplace-discover-install-configure/user-path/20260911T223721Z-step05-step05-member-no-install-surface.txt
- docs/validation/evidence/VWO-005/marketplace-discover-install-configure/user-path/20260911T223742Z-step06-step06-configure-attempt-drops-input.txt
- docs/validation/evidence/VWO-005/marketplace-discover-install-configure/user-path/20260911T223755Z-step07-step07-demo-replay-no-observation-surfac.txt
- docs/validation/evidence/VWO-005/marketplace-discover-install-configure/post-hoc/20260911T223805Z-events-install-state-role-gate.txt

## Failure / Recovery

- Failure injected: natural expired-trial install (pk-101 / ent-0502); configure-form input loss (defect F2, reproduced on the consumer path)
- Recovery attempted: expired trial → blocked with entitlement id + expiry (recovery path is renew/grant then install; demonstrated in the create-and-sell scenario via the admin grant); configure → retried via form, input dropped again; API-twin confirmation isolated the root cause (create-and-sell post-hoc artifact)
- Restart/session-loss behavior: not attempted — no Codex Universal runtime process exists to restart; fixture reseed between scenarios via reset-scenario.sh
- Final outcome: discovery/inspection/install/entitlement/role gates all PASS; configuration persistence FAILED through the human path (F2); DEMONSTRATE observation impossible (F1)

## Product / UX Friction

The consumer path is the best-designed part of the fixture: search, inspect-then-install, explicit rejection messages that name the exact entitlement id and expiry or the exact required permission — a normal person can understand every denial. The two friction points are severe: the configure step lies to the user (success flash, empty config), and after finishing the whole demonstration there is no "teach Codex this routine" moment — the DEMONSTRATE mode's entire value proposition (the system watches you work) does not exist as a product surface.

## Security / Architecture Findings

- No findings — no NEW unique findings in this scenario block: the missing teaching/observation surface is F1 and the configure-input loss is F2, both recorded canonically in the marketplace-create-and-sell block and reproduced here on the consumer path (user-path step06 and step07 artifacts); the entitlement and role gates themselves held correctly in both UI and server-side checks.

## Recommendation

- NEW WORK ORDER — the consumer-path gaps are covered by the canonical recommendations in the marketplace-create-and-sell block (F1 via the surface-integration WO; F2 via the fixture-remediation WO); no scenario-unique disposition is required; proposed owner: as per those blocks; verification required: re-run this scenario after those WOs land — configure must persist keys and the replay must produce a compiled consumer workflow.

## Worker Conclusion

The scenario is **blocked** as a DEMONSTRATE-mode teaching test (no observation/compile surface exists — F1) with the consumer fixture path otherwise passing every gate (discover → inspect → install under auto-trial → expired-trial rejection → member role-block); the configuration step is defective on the human path (F2). Partial pass at the fixture level, blocked at the product level.

## Scenario

- Scenario id: marketplace-upgrade-rollback
- Industry: marketplace
- Scenario: upgrade a pinned install to a newly released version, reproduce a failed upgrade, and roll back explicitly (INSTRUCT)
- Teaching mode: INSTRUCT
- User goal: as Petra Voss (Acme Retail org admin) owning seeded install ins-0401 (pk-102 @ 2.3.1), upgrade to the newly released 2.4.0, hit a stale-pin conflict, roll back explicitly, and see an entitlement-blocked upgrade fail safe — described in words for the system to infer explicit-pin upgrade/rollback semantics (catalog primary owner VWO-006; exercised by VWO-005 per the WO mission)

## User Path

1. Reset the marketplace fixture; as `vera.osei` (publisher) published version 2.4.0 of pk-102 through the listing's version form — observed the new immutable version (digest `sha256:cadb075e36500b7c`) and "1 installer(s) notified".
2. Signed in as `petra.voss`; opened `/installs` — observed "2.4.0 — upgrade available" for install `ins-0401` (pinned at 2.3.1).
3. Upgraded `ins-0401` to 2.4.0 through the installs page form (explicit hidden `expectedCurrentVersion=2.3.1` pin + target version) — success flash, version moved to 2.4.0, history entry and `install.upgraded` event recorded.
4. Re-submitted the same upgrade carrying the stale pin `expectedCurrentVersion=2.3.1` (reproducing what a stale browser tab posts) — natural `[data_conflict]` "Install ins-0401 moved: you expected v2.3.1 but it is at v2.4.0" — rejected; the correct recovery (re-read current version, re-issue) is exactly what the page then shows.
5. Rolled back `ins-0401` to 2.3.1 through the explicit "Rollback to previous" form — success flash, history entry and `install.rolled_back` event; the install again shows "upgrade available" (no implicit downgrade ever occurs).
6. INSTRUCT: wrote the upgrade/rollback policy text (explicit, pinned, auditable; failures never leave half-installed state) and attempted to submit it — no teaching surface exists (F1); the policy semantics were instead verified behaviorally through steps 2–5 and 7.
7. Armed `?failure=stale_entitlement` on the upgrade form and attempted the upgrade — `[stale_entitlement]` blocked with the commercial operation named; the install pin stayed at 2.3.1 (fail-safe, no partial state).

## Expected

Per the catalog entry: the upgrade proceeds only with a valid entitlement and the current-version pin; the stale `expectedCurrentVersion` upgrade is rejected with `[data_conflict]` and the recovery is re-read + re-issue; rollback is explicit, recorded and auditable; the entitlement-blocked upgrade leaves the install at its previous pin with no partial state; and the INSTRUCT-described policy is inferred as a workflow with an upgrade-available trigger and explicit approval steps.

## Actual

Every behavioral half of the policy held exactly: pinned upgrade succeeded and was recorded; the stale-pin retry was rejected with `[data_conflict]` naming both versions; the rollback was explicit, recorded in history and events, and returned the pin to 2.3.1; the entitlement-blocked upgrade failed safe leaving the pin untouched; digests of both published versions stayed byte-stable through all install transitions (post-hoc check). The INSTRUCT half could not run — no surface accepts the policy description (F1, canonical home in the create-and-sell block). One new unique finding was produced post-hoc after the user path: the upgrade/rollback/configure mutations enforce role but NOT org ownership — a Northwind org admin (iris.chen) successfully upgraded Acme Retail's install `ins-0401` through the documented JSON API twin (HTTP 200), moving another org's pin (F6 below; the probe's state effect was left recorded in the post-hoc artifact and the fixture was reset afterwards).

## Workflow Identity

- Workflow: NONE — teaching/compile surface missing; nearest product identity is the seeded install ins-0401 of FlowMart listing pk-102 (commercial record, not a workflow)
- Version: 2.3.1 → 2.4.0 → 2.3.1 (the install pin's explicit transitions; package versions 2.3.1 and 2.4.0 both immutable)
- Source revision: NONE — surface missing
- Definition digest: sha256:04cf049e110cc2cc (pk-102@2.3.1) and sha256:cadb075e36500b7c (pk-102@2.4.0) — both byte-stable through every install transition
- Dependency-lock identity: NONE — surface missing

## Runtime Binding

- Capabilities: browser use (including arming the documented `?failure=` switch on the form action — a user-reachable fixture contract); policy-inference capability NOT exercised — surface missing
- Resources: FlowMart fixture app (port 4105); personas vera.osei (publisher), petra.voss (Acme org admin), iris.chen (Northwind org admin — cross-tenant probe)
- Environments: agent sandbox fallback proving ground; no E2B workspace
- Approvals: org-admin upgrade/rollback gates (install:manage) exercised; entitlement validity gate on upgrade (blocked when stale — fail-safe); publisher version gate on the 2.4.0 release
- Triggers/schedules: upgrade-available notification triggered by version publish (publisher → installer org admin, observed "1 installer(s) notified"); the policy's explicit approval steps are behavioral only — no inferred workflow exists

## Evidence

- docs/validation/evidence/VWO-005/marketplace-upgrade-rollback/user-path/20260911T223901Z-step01-step01-publisher-releases-240.txt
- docs/validation/evidence/VWO-005/marketplace-upgrade-rollback/user-path/20260911T223909Z-step02-step02-upgrade-available-ins0401.txt
- docs/validation/evidence/VWO-005/marketplace-upgrade-rollback/user-path/20260911T223921Z-step03-step03-upgrade-success-history.txt
- docs/validation/evidence/VWO-005/marketplace-upgrade-rollback/user-path/20260911T223934Z-step04-step04-stale-expectedcurrentversion-reje.txt
- docs/validation/evidence/VWO-005/marketplace-upgrade-rollback/user-path/20260911T223941Z-step05-step05-explicit-rollback-recorded.txt
- docs/validation/evidence/VWO-005/marketplace-upgrade-rollback/user-path/20260911T223952Z-step06-step06-instruct-policy-no-surface.txt
- docs/validation/evidence/VWO-005/marketplace-upgrade-rollback/user-path/20260911T223958Z-step07-step07-entitlement-blocked-no-partial-st.txt
- docs/validation/evidence/VWO-005/marketplace-upgrade-rollback/post-hoc/20260911T224059Z-events-pin-history-digests-cross-tenant.txt

## Failure / Recovery

- Failure injected: natural stale `expectedCurrentVersion` upgrade (stale-pin replay); `?failure=stale_entitlement` armed on the upgrade form; post-hoc cross-tenant mutation probe (F6)
- Recovery attempted: stale pin → rejected `[data_conflict]` with both versions named; the page re-renders the correct current pin for re-issue (re-read + retry demonstrated by the subsequent successful operations); entitlement-blocked upgrade → install left untouched at 2.3.1, re-attemptable after entitlement recovery
- Restart/session-loss behavior: not attempted — no Codex Universal runtime process exists to restart; fixture reseed after the cross-tenant probe via reset-scenario.sh (state effect of the probe recorded in the post-hoc artifact before reset)
- Final outcome: explicit-pin upgrade/rollback semantics PASS behaviorally (pinned, auditable, fail-safe, digests stable); INSTRUCT policy inference BLOCKED (F1); new finding F6 — cross-tenant install mutation accepted server-side

## Product / UX Friction

The upgrade/rollback surface is genuinely good for a normal person: the pin is implicit but correct (hidden expectedCurrentVersion), denials name both versions, history is visible on the install card, and the entitlement-blocked failure explains itself. The friction points: the INSTRUCT policy author has nowhere to put the policy (F1); and the installs page hides other orgs' installs, which gives a false sense of tenant isolation that the mutation boundary does not actually enforce (F6) — the UI and the server disagree about the security model, which is worse than either alone.

## Security / Architecture Findings

- Finding: F6 — cross-tenant install mutation is accepted server-side: upgradeInstall/rollbackInstall/configureInstall in the FlowMart fixture check the acting user's role permission (install:manage) but never check that the install belongs to the actor's organization. A Northwind org admin (iris.chen) upgraded Acme Retail's install ins-0401 to 2.4.0 through the documented JSON API twin (HTTP 200, "Install ins-0401 upgraded 2.3.1 → 2.4.0"), moving another org's version pin and writing another org's history/events. The UI filters installs by org, so the human path does not surface it, but the documented API twin is part of the fixture contract ("every HTML form has a JSON twin … the same operation"), so a normal person using the API surface can mutate any org's installs. Tenant-boundary authorization failure on commercial records.
- Severity: P1
- Reproduction: POST /api/installs/upgrade with Authorization for iris.chen and body {installId: ins-0401, targetVersion: 2.4.0} → HTTP 200 with the upgraded install (full transcript and response JSON in the post-hoc artifact §4); contrast the role gate which correctly returned 403 for ravi.gupta.
- Root cause: the three install-mutation ops resolve the install by id and check only ctx.user permissions; orgForUser() is used for listing/scoping but never compared against inst.orgId on the mutation path (VWO-002 fixture defect).
- Frozen invariant affected: commercial-entitlement boundary (tenant isolation of commercial records) — a trust-boundary violation inside the fixture that wave-2 scenarios (VWO-008/009) would otherwise rely on.

## Recommendation

- FIX NOW — F6: add an org-ownership check (inst.orgId === orgForUser(state, ctx.user.id), with an admin override if intended) to upgradeInstall, rollbackInstall and configureInstall; proposed owner: bounded fixture-remediation WO (FlowMart bundle with F2/F4); verification required: the cross-tenant probe from the post-hoc artifact must return 403, and VWO-008/009 entitlement-race scenarios must re-run against the fixed boundary.

## Worker Conclusion

The scenario is **blocked** as an INSTRUCT-mode teaching test (no surface accepts the policy description — F1) while the behavioral semantics it describes all passed: explicit pinned upgrade, natural stale-pin rejection with correct recovery, explicit recorded rollback, entitlement-blocked fail-safe, and byte-stable digests; one new P1 security finding (F6, cross-tenant mutation) was produced and evidenced.

## Scenario

- Scenario id: software-pr-triage
- Industry: software
- Scenario: triage an open PR — CI gate, code-owner approval, merge with staging side-effect (INSTRUCT — additional create+publish cycle per the VWO-005 WO)
- Teaching mode: INSTRUCT
- User goal: as Noor Haddad (code owner), describe the PR-triage process in words for Codex Universal to infer its gates (CI must pass; a code owner other than the author must approve; merge deploys to staging, never straight to production), and exercise the triage through the ForgeOps human path

## User Path

1. Reset the software fixture (`reset-scenario.sh software`); signed in as `ari.klein`; opened `/repos/r-1` (payments-service).
2. Opened the pending PR `pr-0301` (Refund idempotency keys) — CI run `run-0071` visible as passed.
3. As the author, attempted to approve the own PR through the decision path — natural `[permission_denied]` (engineer lacks `pr:review`; the form's own note states self-approval is blocked by policy).
4. Signed in as `noor.haddad` (code owner); approved with a review note — approval recorded and visible; re-approving again → `[duplicate_event]` rejected.
5. Merged the PR — observed "PR #302 merged; payments-service 4.8.2 deployed to staging (deployment dep-1104)"; a second merge attempt → `[data_conflict]` "only open PRs can be merged" with no second deployment.
6. INSTRUCT: wrote the triage-process description (CI gate, non-self code-owner approval, merge→staging side-effect, never straight to production) and attempted to submit it — no teaching surface exists (F1, canonical home in the marketplace-create-and-sell block); the described gates were verified behaviorally through steps 2–5 instead.

## Expected

Per the catalog entry: the author's self-approval is blocked; the code-owner approval and merge are recorded; the staging deployment and service version bump are observable; the double-merge fails closed with no double deploy; and the INSTRUCT inference produces a workflow carrying the CI gate, the non-self owner gate, and the staging side-effect.

## Actual

All gates behaved exactly as described: CI visible as passed before merge; author approval blocked (`[permission_denied]`); code-owner approval recorded (with note and timestamp); duplicate approval rejected (`[duplicate_event]`); merge produced the staging deployment `dep-1104` + artifact `art-0902` + service version bump 4.8.1 → 4.8.2 (events `pr.approved`, `pr.merged`, `deployment.created` all recorded; production version untouched — never deployed straight to production); double-merge rejected `[data_conflict]` with no second deployment. The INSTRUCT submission itself had no surface to accept it (F1 — no new unique finding; the workflow-plane integration gap is canonical in the marketplace-create-and-sell block).

## Workflow Identity

- Workflow: NONE — teaching/compile surface missing; nearest product identity is the ForgeOps PR/merge record set (pr-0301 → merge → dep-1104/art-0902)
- Version: payments-service 4.8.2 (staging version produced by the merge; seed baseline 4.8.1)
- Source revision: NONE — surface missing (ForgeOps records branches/commits only as PR metadata, no workflow source pin)
- Definition digest: NONE — surface missing (artifact art-0902 has a build digest, not a workflow-definition digest)
- Dependency-lock identity: NONE — surface missing

## Runtime Binding

- Capabilities: browser use; instruction-based workflow creation NOT exercised — surface missing
- Resources: ForgeOps fixture app (software family, port 4102); personas ari.klein (engineer/author), noor.haddad (code owner)
- Environments: agent sandbox fallback proving ground; no E2B workspace (the ForgeOps terminal console exists but this scenario's catalog binding is terminal-surface: none)
- Approvals: code-owner approval gate (pr:review) with the self-approval prohibition enforced server-side (author 403); merge gate requiring ≥1 approval + passing CI (enforced; double-merge closed)
- Triggers/schedules: intended PR-opened + CI-completed event triggers — NOT exercised (teaching surface missing); the merge→staging side-effect is the behavioral stand-in and was fully observable

## Evidence

- docs/validation/evidence/VWO-005/software-pr-triage/user-path/20260911T224143Z-step01-step01-repo-payments-service.txt
- docs/validation/evidence/VWO-005/software-pr-triage/user-path/20260911T224219Z-step02-step02-pr-page-ci-status.txt
- docs/validation/evidence/VWO-005/software-pr-triage/user-path/20260911T224311Z-step03-step03-author-approval-blocked.txt
- docs/validation/evidence/VWO-005/software-pr-triage/user-path/20260911T224328Z-step04-step04-code-owner-approval-recorded.txt
- docs/validation/evidence/VWO-005/software-pr-triage/user-path/20260911T224335Z-step04-step04b-duplicate-approval-rejected.txt
- docs/validation/evidence/VWO-005/software-pr-triage/user-path/20260911T224346Z-step05-step05-merge-staging-deploy.txt
- docs/validation/evidence/VWO-005/software-pr-triage/user-path/20260911T224357Z-step05-step05b-double-merge-rejected.txt
- docs/validation/evidence/VWO-005/software-pr-triage/user-path/20260911T224408Z-step06-step06-instruct-triage-process-no-surfac.txt
- docs/validation/evidence/VWO-005/software-pr-triage/post-hoc/20260911T224424Z-events-gates-staging-sideeffect.txt

## Failure / Recovery

- Failure injected: natural self-approval attempt (author); natural duplicate approval; natural double-merge
- Recovery attempted: all three failed closed exactly as the catalog expects — self-approval 403 with the policy reason; duplicate approval 409; double-merge 409 with no second deployment or version bump
- Restart/session-loss behavior: not attempted — no Codex Universal runtime process exists to restart; fixture reseed between scenarios via reset-scenario.sh (post-reset markers verified in `_infra/final-reset-verification`)
- Final outcome: every described gate PASS behaviorally (CI, non-self owner approval, merge→staging-only side-effect, fail-closed duplicates); the INSTRUCT workflow creation BLOCKED (F1)

## Product / UX Friction

The triage path is clear and honest: the PR page shows CI status, the approval list, and forms whose own notes state the policies ("Self-approval is blocked by policy…", "Requires ≥1 approval and a passing CI run. Merging deploys the version to staging…"). A normal person can execute and understand the whole gate chain. The single friction point is the missing INSTRUCT surface: the process description a code owner would hand to the system has nowhere to go, so the "create an additional workflow using INSTRUCT" WO mandate cannot be satisfied as a workflow — only as a gate-behavior verification.

## Security / Architecture Findings

- No findings — no NEW unique findings: the only blocker is the product-wide missing teaching/compile surface (F1, canonical home in the marketplace-create-and-sell block; the step06 artifact records the submission attempt and the exact instruction text). All ForgeOps authorization gates held correctly (self-approval 403, duplicate approval 409, double-merge 409, production untouched).

## Recommendation

- NEW WORK ORDER — covered by the canonical F1 recommendation in the marketplace-create-and-sell block (surface-integration WO must make INSTRUCT submission possible); proposed owner: as per that block; verification required: re-run this scenario — the triage description must produce an inferred workflow whose CI gate, non-self owner gate and staging side-effect match the behavior verified here.

## Worker Conclusion

The scenario is **blocked** as an INSTRUCT-mode workflow-creation test (no teaching surface — F1) while every described triage gate passed behaviorally with full evidence (CI gate, self-approval block, code-owner approval, duplicate/conflict fail-closed, merge→staging-only deployment with version bump, production untouched).

## Scenario

- Scenario id: media-editorial-publishing
- Industry: media
- Scenario: publish a story end-to-end — draft, hero image, review, approval, multi-channel publication (DEMONSTRATE — additional create+publish cycle per the VWO-005 WO)
- Teaching mode: DEMONSTRATE
- User goal: as Hana Kim (staff writer) and Camille Dubois (editor-in-chief), demonstrate the editorial gates — writer/editor role split, hero gate, publish step, per-channel distribution — for Codex Universal to observe

## User Path

1. Reset the media fixture (`reset-scenario.sh media`); signed in as `hana.kim`; created draft `st-0206` ("Council approves riverside stadium funding") on `/desk`.
2. Attached hero asset `a-0301` to the story (hero renditions ready) — visible on the story page.
3. Submitted the story for review — `in_review` state visible.
4. As the writer, attempted to approve the own story — natural `[permission_denied]` (staff_writer lacks `story:approve`).
5. Signed in as `camille.dubois`: approved `st-0206`, then published it to web-front-page (1/1 channels delivered); additionally approved and published `st-0202` (hero attached during the run) to three channels — 2/3 delivered with the partner-app job FAILED `[stale_entitlement]` (distribution license expired 2026-06-01) — partial success with the failed job observable.
6. Attempted to publish seed story `st-0205` (approved, NO hero) — natural `[missing_asset]` "no hero image is attached. Publication blocked." — the story stays approved; noted that an approved story can never gain a hero through the UI (attach form requires draft/changes_requested/in_review), so the catalog's documented recovery (attach hero, publish) is unreachable through normal surfaces for approved stories.
7. DEMONSTRATE replay attempt: no Codex Universal observation/compile surface exists to watch steps 1–6 or produce the inferred editorial workflow — recorded, not simulated.

## Expected

Per the catalog entry: lifecycle states are visible at each step; the writer's self-approval is blocked; the no-hero publish is blocked with `[missing_asset]` and the story stays approved; publication fans out to channels with per-channel status (the expired partner-app license fails that channel while valid channels still receive the story); and replaying the path under observation yields a compiled editorial workflow with the role split, hero gate, publish step and per-channel distribution.

## Actual

Every editorial gate behaved as the catalog describes: draft → hero attach → in_review → writer self-approval blocked (`[permission_denied]`) → editor approval → publication with per-channel fan-out (st-0206 1/1 delivered; st-0202 2/3 delivered with partner-app FAILED `[stale_entitlement]` and the failed job visible); the no-hero publish of st-0205 was blocked (`[missing_asset]`) leaving the story approved; events `story.created`, `story.approved`, `story.published`, `distribution.failed` all recorded. Two deviations: the DEMONSTRATE replay was impossible (no observation/compile surface — F1, canonical home in the marketplace-create-and-sell block), and a new unique finding emerged: the approved-story attach dead-end (F7) — the documented recovery for the missing-hero block cannot be performed through any normal surface, because the attach form disappears once a story is approved and no surface moves an approved story back to an editable state.

## Workflow Identity

- Workflow: NONE — teaching/compile surface missing; nearest product identity is the PressRoom story record set (st-0206 through its lifecycle states)
- Version: st-0206 v2 (published), st-0202 v3 (published after hero attach + approval)
- Source revision: NONE — surface missing
- Definition digest: NONE — surface missing (PressRoom records no digests; assets carry checksums a-0301 bb89dc04-class only)
- Dependency-lock identity: NONE — surface missing

## Runtime Binding

- Capabilities: browser use; observation/compile capability NOT exercised — surface missing (DEMONSTRATE mode's core loop)
- Resources: PressRoom fixture app (media family, port 4104); personas hana.kim (staff writer), camille.dubois (editor-in-chief); asset a-0301; channels web-front-page, morning-newsletter, partner-app
- Environments: agent sandbox fallback proving ground; no E2B workspace
- Approvals: editor approval gate (story:approve) with the non-self rule enforced (writer 403); hero-asset gate enforced at publish (`[missing_asset]`); per-channel license gate enforced at distribution (partner-app failed, others delivered)
- Triggers/schedules: intended publication trigger + social scheduling — the social surface exists in the fixture (out of this run's path); the DEMONSTRATE-observed workflow trigger could NOT be exercised (teaching surface missing)

## Evidence

- docs/validation/evidence/VWO-005/media-editorial-publishing/user-path/20260911T224445Z-step01-step01-draft-created.txt
- docs/validation/evidence/VWO-005/media-editorial-publishing/user-path/20260911T224453Z-step02-step02-hero-attached.txt
- docs/validation/evidence/VWO-005/media-editorial-publishing/user-path/20260911T224459Z-step03-step03-submitted-for-review.txt
- docs/validation/evidence/VWO-005/media-editorial-publishing/user-path/20260911T224507Z-step03-step03-submitted-for-review.txt
- docs/validation/evidence/VWO-005/media-editorial-publishing/user-path/20260911T224516Z-step04-step04-writer-self-approval-blocked.txt
- docs/validation/evidence/VWO-005/media-editorial-publishing/user-path/20260911T224532Z-step05-step05-editor-approves.txt
- docs/validation/evidence/VWO-005/media-editorial-publishing/user-path/20260911T224539Z-step05-step05b-published-channel-fanout.txt
- docs/validation/evidence/VWO-005/media-editorial-publishing/user-path/20260911T224723Z-step05-step05c-partner-app-partial-success.txt
- docs/validation/evidence/VWO-005/media-editorial-publishing/user-path/20260911T224558Z-step06-step06-nohero-publish-blocked.txt
- docs/validation/evidence/VWO-005/media-editorial-publishing/user-path/20260911T224737Z-step07-step07-demo-replay-no-observation-surfac.txt
- docs/validation/evidence/VWO-005/media-editorial-publishing/post-hoc/20260911T224746Z-events-story-states.txt

## Failure / Recovery

- Failure injected: natural writer self-approval; natural no-hero publish (st-0205); natural expired partner-app channel license (partial distribution)
- Recovery attempted: self-approval → blocked, editor approval path used instead; no-hero publish → blocked, story stays approved (the documented recovery — attach a hero — was probed and found UNREACHABLE for approved stories through the UI: F7); expired channel → partial success with the failed job observable (documented recovery = renew license → republish/correct, not exercised — out of this scenario's path)
- Restart/session-loss behavior: not attempted — no Codex Universal runtime process exists to restart; fixture reseed between scenarios via reset-scenario.sh
- Final outcome: all editorial gates PASS behaviorally (role split, hero gate, per-channel license fan-out with observable partial failure); the DEMONSTRATE observation impossible (F1); one new P3 finding (F7, approved-story attach dead-end)

## Product / UX Friction

The editorial path is legible and the partial-failure behavior is excellent for a normal person (2/3 delivered + the failed channel named with its license expiry, story still published). The friction points: the approved-without-hero dead-end means an editor who hits `[missing_asset]` has no in-product recovery — the only visible options are the API or a world reset, neither of which is a normal-person path (F7); and, as in every other scenario, there is no "let Codex watch me do this" surface, so the DEMONSTRATE mode cannot deliver its compiled-workflow payoff (F1).

## Security / Architecture Findings

- Finding: F7 — PressRoom approved-story attach dead-end: the asset-attach form is rendered only for stories in draft/changes_requested/in_review, and no surface returns an approved story to an editable state. Consequently, an approved story that lacks a hero (the exact seed case st-0205 the catalog uses for the natural missing_asset path) can never satisfy the documented recovery ("attach hero, publish") through the UI — publication is permanently blocked for that story through normal surfaces, and the catalog's recovery expectation is unfalsifiable in the product.
- Severity: P3
- Reproduction: as camille.dubois open /stories/st-0205 (approved, no hero) — publish blocked `[missing_asset]`; enumerate the page: no attach form is rendered (attach requires draft/changes_requested/in_review per the fixture's ui.js); no un-approve/edit transition exists (step06 + step07 artifacts; source noted post-hoc).
- Root cause: state-machine scope in the VWO-002 fixture — the attach surface's status guard omits 'approved', and no reverse transition was modeled.
- Frozen invariant affected: none (fixture polish gap; the hero gate itself behaves correctly).

## Recommendation

- NEW WORK ORDER — F7: either allow hero attach for approved stories (one-line status-guard change) or model an explicit un-approve/edit transition; proposed owner: fixture-remediation WO (can bundle with the FlowMart items or ride the next VWO-002 revision); verification required: the st-0205 recovery path (attach hero → publish) completes through the UI after the fix.

## Worker Conclusion

The scenario is **blocked** as a DEMONSTRATE-mode teaching test (no observation/compile surface — F1) while the demonstrated editorial lifecycle passed every gate with full evidence (draft → hero → review → non-self approval → multi-channel publication with a correctly-failing expired-license channel, and the no-hero block leaving the story approved); one new P3 finding (F7) recorded on the unreachable recovery path.
