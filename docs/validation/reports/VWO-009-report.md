# VWO-009 — Versioning, Marketplace, and Distribution Adversary Report

## Identity
- Work Order: VWO-009 (wave-2 adversarial validator; versioning/marketplace/distribution attacks against frozen identity, pin, lineage and entitlement contracts)
- Worker persona: versioning/marketplace/distribution adversary — attacks the product surface with mutated publications, moving refs, stale proposals, implicit downgrades, mid-flight entitlement revocations and cross-tenant writes; reports truthfully
- Base branch: main
- Base SHA: 7736a42e57a48c67d97b40350001e759e78661ae
- Head SHA: see git rev-parse vwo-009/versioning-distribution-adversary (single commit on base 7736a42)
- Validation environment: sandbox fallback proving ground per VALIDATION-PROGRAM.md §3 — VWO-002 five-app fixture harness (FlowMart 4105 marketplace driven via agent-browser 0.35.0 with per-persona sessions, plus curl for the documented JSON API twins); the codex workflow versioning/marketplace/distribution contract crates attacked as LIBRARIES via `cargo test` (rustc/cargo 1.95.0, pinned by codex-rs/rust-toolchain.toml) plus a standalone adversarial probe crate at /home/z/vwo009-probe (path dependencies on codex-workflow-contracts/forge/distribution, NO codex-rs source modification — working tree clean at base); E2B/Composio absent = documented fallback profile (capability_check.sh final run PASS; verify-sweep.sh 59/59 after seed reset)
- Codex Universal runtime/application SHA: NONE — no user-facing Codex Universal runtime/application build exists in this environment; the workflow versioning/distribution engine is library-only (Finding F1, re-verifying VWO-005 F1 / VWO-007 F1 / VWO-008 F1); the library source under test is the repository tree at base SHA 7736a42e57a48c67d97b40350001e759e78661ae
- Date/time window: 2026-09-12T07:51:47Z to 2026-09-12T09:40:06Z (UTC)

## Scenario
- Scenario id: adv-environment-failure-rebinding
- Industry: adversarial (proving ground: marketplace / FlowMart, port 4105)
- Scenario: the environment/provider bound to an installed workflow fails during install/upgrade (registry down) — the operator must rebind to a compatible environment WITHOUT mutating the workflow's semantic source or version pin
- Teaching mode: HYBRID
- User goal: Iris Chen (Northwind org admin) upgrades a pinned install through a registry outage (fail-safe, no partial state), then rebinds the install's environment/binding parameters through configuration while the version pin, package digest and published provenance stay byte-identical — and a rebind carrying a semantically-mutating payload must be rejected or recorded as a hole

## User Path
1. Recorded the INSTRUCT instruction text (binding pins, rebind policy, 503 fail-safe expectations) verbatim — no Codex teaching surface exists at base 7736a42 (standing F1), so the text is evidence, not a submission.
2. Signed in as `iris.chen` on http://localhost:4105/login, searched the catalog ("triage") and opened the Support Ticket Triage Assistant listing (pk-102, latest 2.3.1, digest sha256:04cf049e110cc2cc).
3. Installed pk-102 for Northwind Logistics — install `ins-0402` pinned at v2.3.1 with auto-granted trial entitlement `ent-0504`.
4. As `vera.osei` (publisher), published 2.4.0 — the listing showed both immutable version rows (2.3.1 / 2.4.0 digest sha256:cadb075e36500b7c) and org admins received upgrade-available notifications.
5. As iris, clicked **Upgrade to 2.4.0** with the registry-down switch armed (`?failure=service_unavailable`) — HTTP 503 `service_unavailable` naming the `registry-blob-store` dependency; the install stayed pinned at 2.3.1 with NO partial state.
6. Retried the same upgrade cleanly (switch is per-request) — "Install ins-0402 upgraded 2.3.1 → 2.4.0."; the pin moved only via this explicit decision.
7. Rebound the install's environment parameters through the configure surface: the browser form silently dropped the JSON payload ("configured (no keys)" — VWO-005 F2 pattern re-verified), so the documented API twin `POST /api/installs/configure` carried the rebind (queue tier-2-emea, confidenceThreshold 0.85, environment registry-eu-1) — same op code `install:configure`.
8. Observed the rebound record: same pin 2.4.0, same entitlement, same digests — only binding values changed; the listing digest panel and the auditor's provenance verification (MATCH for 2.4.0) confirmed the published identity was untouched by the rebind.
9. Step 6 of the catalog (rebind carrying a SEMANTICALLY-MUTATING payload): the API twin accepted attacker-style identity-impersonating config keys (version/digest/targetVersion/manifest) with `ok:true` — the pin and published digests did NOT move, but the keys persisted in the config record (recorded as finding F2, see post-hoc attack artifact).

## Expected
Provider failure is recoverable: the 503 upgrade fails safely with the named registry dependency, zero partial install, and a clean retry succeeds; rebinding is a first-class operation through the configure surface; the version pin, package digest and published provenance NEVER change across a rebind; a rebind carrying semantically-mutating payload keys is refused (or the hole is recorded as a finding).

## Actual
Every immutability invariant held. The registry-down upgrade returned HTTP 503 `service_unavailable` naming `registry-blob-store`; ins-0402 stayed at 2.3.1 with no upgrade record and no event; the clean retry moved the pin explicitly (2.3.1 → 2.4.0). The rebind (via the API twin after the browser form dropped the JSON) changed only binding values: pin 2.4.0, entitlement ent-0504, published digests 2.3.1=sha256:04cf049e110cc2cc and 2.4.0=sha256:cadb075e36500b7c all byte-identical before/after; provenance verification still returned MATCH for 2.4.0. The mutating-payload rebind (catalog step 6) was NOT rejected: `POST /api/installs/configure` accepted `{"version":"2.3.1","digest":"sha256:0123…","targetVersion":"1.0.0","manifest":{…}}` into the config record with `ok:true` — the install's actual pin field stayed 2.4.0 (the keys are inert data, not authority), but the product persisted and displays attacker-written identity-impersonating fields inside the install record (finding F2). The 503 rejection produced NO audit event (finding F3, standing gap). Engine-side parity: `cargo test -p codex-workflow-contracts --lib` 90 passed / 0 failed, `-p codex-workflow-forge --lib` 44 passed / 0 failed, `-p codex-workflow-distribution` 10 + 8 passed / 0 failed; the 12-attack engine probe battery repelled the dependency-lock tamper, moving-ref authority, and branch-move attacks (see program-level summary).

## Workflow Identity
- Workflow: Support Ticket Triage Assistant (marketplace package pk-102) installed as ins-0402 for Northwind Logistics; engine-side counterpart: codex-workflow-forge install records + codex-workflow-distribution install/upgrade contracts — library-only, no product mount (Finding F1)
- Version: 2.3.1 → 2.4.0 (immutable published versions; the pin moved only via the explicit upgrade decision)
- Source revision: NONE — surface missing (provenance records a per-release commit hash — 2.4.0 commit 57408a3 — but no user-addressable source revision of the executing workflow; engine crates tested at base SHA as libraries)
- Definition digest: 2.3.1 = sha256:04cf049e110cc2cc, 2.4.0 = sha256:cadb075e36500b7c (published manifest digests, exposed on the listing panel and verified by provenance checks — unchanged by rebind and by the mutating-payload attack)
- Dependency-lock identity: NONE — surface missing (FlowMart models manifest+digest only; no dependency-lock surface exists — engine probe attack 3 covers the lock contract, REPELLED)

## Runtime Binding
- Capabilities: install:manage + install:configure (iris.chen, org_admin), version:publish (vera.osei, publisher), audit:verify (mia.torres, auditor)
- Resources: package pk-102, install ins-0402, entitlement ent-0504 (trial), org Northwind Logistics (o-1), synthetic registry-blob-store dependency
- Environments: FlowMart browser surface (http://localhost:4105) + documented JSON API twins; the "environment" being rebound = the install's binding parameters (queue, confidenceThreshold, environment)
- Approvals: consumer rebinding authority (install:manage/install:configure); the explicit-upgrade decision as the control-plane move for pin changes; entitlement gate active throughout
- Triggers/schedules: version.published → upgrade-available notification to org admins; install.upgraded → notification; no scheduled triggers in this scenario

## Evidence
- docs/validation/evidence/VWO-009/adv-environment-failure-rebinding/user-path/20260912T090022Z-instruct-phase-binding-pins-and-rebind-r.txt
- docs/validation/evidence/VWO-009/adv-environment-failure-rebinding/user-path/20260912T090037Z-step01-iris-signs-in.txt
- docs/validation/evidence/VWO-009/adv-environment-failure-rebinding/user-path/20260912T090041Z-step02-catalog-search-triage.txt
- docs/validation/evidence/VWO-009/adv-environment-failure-rebinding/user-path/20260912T090045Z-demo-phase-step03-listing-pin-and-digest.txt
- docs/validation/evidence/VWO-009/adv-environment-failure-rebinding/user-path/20260912T090055Z-demo-phase-step04-install-pinned-231-wit.txt
- docs/validation/evidence/VWO-009/adv-environment-failure-rebinding/user-path/20260912T090110Z-demo-phase-step05-publisher-releases-240.txt
- docs/validation/evidence/VWO-009/adv-environment-failure-rebinding/user-path/20260912T090126Z-demo-phase-step06-upgrade-503-registry-d.txt
- docs/validation/evidence/VWO-009/adv-environment-failure-rebinding/user-path/20260912T090132Z-demo-phase-step07-clean-retry-upgrade-su.txt
- docs/validation/evidence/VWO-009/adv-environment-failure-rebinding/user-path/20260912T090146Z-demo-phase-step08-rebind-form-silently-d.txt
- docs/validation/evidence/VWO-009/adv-environment-failure-rebinding/user-path/20260912T090204Z-demo-phase-step09-rebound-record-same-pi.txt
- docs/validation/evidence/VWO-009/adv-environment-failure-rebinding/user-path/20260912T090226Z-demo-phase-step05b-auditor-provenance-ve.txt
- docs/validation/evidence/VWO-009/adv-environment-failure-rebinding/user-path/20260912T090245Z-demo-phase-step10-listing-digest-panel-a.txt
- docs/validation/evidence/VWO-009/adv-environment-failure-rebinding/post-hoc/20260912T090210Z-rebind-api-twin-and-record.txt
- docs/validation/evidence/VWO-009/adv-environment-failure-rebinding/post-hoc/20260912T090258Z-mutating-payload-rebind-attack-result.txt
- docs/validation/evidence/VWO-009/adv-environment-failure-rebinding/post-hoc/20260912T090349Z-events-digest-invariants-reset.txt
- docs/validation/evidence/VWO-009/adv-environment-failure-rebinding/post-hoc/20260912T090401Z-reset-verification.txt

## Failure / Recovery
- Failure injected: `service_unavailable` switch on the upgrade request (registry-blob-store down, HTTP 503 with the named dependency); the browser configure form's JSON-dropping bug (VWO-005 F2 pattern) also forced the API-twin path for the rebind.
- Recovery attempted: clean retry of the SAME upgrade after the per-request switch cleared — succeeded and moved the pin explicitly; rebinding then carried the environment change.
- Restart/session-loss behavior: not exercised (VWO-007's scope); the fixture is stateless across persona session switches.
- Final outcome: upgrade recovered through retry; rebind completed with pin 2.4.0 / digests / provenance byte-identical; the mutating-payload rebind was accepted into config (F2) but moved no authority field; world reset restored the seed.

## Product / UX Friction
- The browser configure form SILENTLY drops JSON payloads ("configured (no keys)" with a success flash) — a normal person cannot rebind bindings through the visible UI and must know the API twin exists (VWO-005 F2 re-verified at base 7736a42).
- The 503 error is clear and actionable (names the dependency, states reads still work).
- The install card renders the raw config JSON inline, so the F2 identity-impersonating keys are user-visible but indistinguishable from legitimate binding keys — no reserved-field warning.

## Security / Architecture Findings
- Finding: F1 — the workflow versioning/marketplace/distribution engine (codex-workflow-contracts/forge/distribution) is mounted in NO user-facing product surface: no workflow/teaching CLI subcommand, protocol method, or TUI surface exists at base 7736a42. Every versioning guarantee this WO verifies at the engine layer (immutable published identities, lock-covered seals, pin authority, gate ordering) is unobtainable by a normal person using Codex, and the Workflow Identity fields (source revision, dependency lock) have no product surface. Re-verification of the standing VWO-005 F1 / VWO-007 F1 / VWO-008 F1 at base 7736a42.
- Severity: P1
- Reproduction: surface search + the INSTRUCT instruction text for this scenario had no surface to be submitted to (recorded as evidence instead); engine crates only reachable as libraries.
- Root cause: engine crates exist and pass adversarial tests but were never mounted into a product surface (architecture-incomplete integration).
- Frozen invariant affected: workflow authority surfacing (a person cannot see or govern the version identity their "workflow" runs).
- Finding: F2 — configure accepts semantically-mutating payload keys: `POST /api/installs/configure` merges arbitrary keys into the install config record — attacker-written `version`, `digest`, `targetVersion`, and `manifest` fields persist with `ok:true` and render in the UI. The authoritative pin field is separate and did NOT move (the keys are inert data), but the product persists identity-impersonating fields inside the install record, which can mislead operators and downstream integrations that read config.
- Severity: P2
- Reproduction: step 6 of the user path — request/response captured verbatim in the post-hoc attack artifact (ok:true, config shows the attacker manifest).
- Root cause: configureInstall merges an unvalidated object patch with no reserved-key deny-list; the schema treats all config keys as opaque.
- Frozen invariant affected: reconfiguration must never touch installed version/digest/provenance (held for authority fields, violated for the persisted record's impersonation fields).
- Finding: F3 — repelled/blocked operations leave NO audit events: the 503 upgrade rejection, the stale_entitlement blocks, and the data_conflict rejections produce zero event-feed entries — the audit trail shows only successes (standing VWO-007 F2 / VWO-008 F6 re-confirmed in the distribution context).
- Severity: P2
- Reproduction: event-feed pull immediately after the 503 upgrade: no event between the notification and the successful retry (see events-digest artifact).
- Root cause: fixture ops emit events only on success paths.
- Frozen invariant affected: every state-relevant decision should be auditable (failures included).

## Recommendation
- FIX NOW
- Proposed owner: F1 — Tech Lead (standing wave-3 mount decision, now fourth independent confirmation across VWO-005/007/008/009); F2 — FlowMart fixture owner (bounded: reserved-key deny-list in configureInstall; retest: re-submit the mutating payload and expect 400 with the reserved key named); F3 — fixture harness owner (NEW WORK ORDER candidate shared with VWO-008 F6: rejection/conflict event emission).
- Verification required: mount a minimal workflow surface exposing version/digest/lock identity (F1); re-run adv-environment-failure-rebinding step 6 expecting refusal (F2); re-run the 503 upgrade and observe a rejected-operation event (F3).

## Worker Conclusion
passed — the registry failure failed safe with no partial state, the clean retry recovered, rebinding moved binding values only, and pin/digest/provenance stayed byte-identical across rebind and even across the mutating-payload attack; the mutating-payload ACCEPTANCE into config (F2), the missing teaching/engine surface (F1), and the rejection-audit gap (F3) are recorded as findings rather than repelled attacks.

## Scenario
- Scenario id: adv-version-confusion
- Industry: adversarial (proving ground: marketplace / FlowMart, port 4105)
- Scenario: corrupt version identity — install v2 while v1 is still running, attempt an implicit downgrade, present a stale upgrade proposal, and try to install a moving ref — all must fail or require explicit control; digest and pin stay verifiable at every step
- Teaching mode: INSTRUCT
- User goal: Petra Voss (Acme org admin) enforces version governance on ins-0401 (pinned 2.3.1): one active install per org+package, forward-only explicit upgrades, downgrades only as labeled rollback decisions, immutable published digests, no moving-ref authority — while Vera Osei releases 2.4.0

## User Path
1. Recorded the INSTRUCT version-governance instruction text verbatim (no teaching surface exists — standing F1) and verified the seed state: install ins-0401 pinned at 2.3.1, entitlement ent-0501 active, latest published 2.3.1 — "up to date".
2. As `vera.osei`, published 2.4.0 through the listing's publish form — flash "Version 2.4.0 of 'Support Ticket Triage Assistant' published (1 installer(s) notified)"; the versions table (immutable, newest first) now shows 2.4.0 sha256:cadb075e36500b7c above 2.3.1 sha256:04cf049e110cc2cc.
3. As `petra.voss`, clicked **Install v2.4.0** on the listing (installing v2 while v1 is active) — rejected: "Acme Retail already has an active install (ins-0401, v2.3.1) … upgrade it instead of installing again. [duplicate_event]".
4. Attempted the implicit downgrade through the configure surface: the browser form dropped the JSON payload (inert, VWO-005 F2 pattern), so the API twins carried the attack — install targeting 2.3.1 (duplicate_event guard), install with a moving ref "latest"/"main" (version-mismatch guard, see post-hoc), and configure carrying targetVersion/version/digest keys (accepted into config, pin untouched — F2 pattern re-verified).
5. Submitted a stale upgrade proposal through the API twin (expectedCurrentVersion 2.2.0 while ins-0401 is at 2.3.1) — rejected [data_conflict] "Install ins-0401 moved: you expected v2.2.0 but it is at v2.3.1."
6. Upgraded PROPERLY through the browser form — "Install ins-0401 upgraded 2.3.1 → 2.4.0."; the install card shows 2.4.0 "up to date" and the history row "upgraded 2.3.1 → 2.4.0 by Petra Voss"; the pinned 2.4.0 digest equals the published digest sha256:cadb075e36500b7c (verified via /api/packages and the provenance check).
7. Replayed the exact successful proposal verbatim after the pin moved — now stale — rejected [data_conflict] "you expected v2.3.1 but it is at v2.4.0." (the natural stale-proposal oracle).
8. THE DOWNGRADE ATTACK: with the pin at 2.4.0, submitted an upgrade targeting the OLDER version 2.3.1 (fresh expectedCurrentVersion) through the API twin — ACCEPTED: the pin moved 2.4.0 → 2.3.1 and the history recorded it as "upgraded 2.4.0 → 2.3.1" (hole — finding F4; the browser withholds the form when up-to-date, but the op code has no ordering guard). Then clicked **Rollback to previous** in the browser — it rolled 2.3.1 → 2.4.0 (FORWARD), because after a downgrade-as-upgrade the "previous" version is 2.4.0 (finding F5).

## Expected
No moving-ref installs; one active install per org+package; upgrades move the pin forward only via explicit decisions with the expected version; a stale proposal conflicts; downgrades happen only as explicit, labeled rollback decisions; published digests immutable and verifiable at every step.

## Actual
The duplicate install was repelled with [duplicate_event] naming the active install; the stale proposals (both the fabricated 2.2.0 and the verbatim replay after the pin moved) were repelled with [data_conflict] naming the current version; the proper explicit upgrade moved the pin exactly onto the published 2.4.0 (digest equality verified); moving-ref strings ("latest", "main") were refused as install authority (version mismatch); re-publishing existing versions (mutate-published-definition attack) was repelled with [duplicate_event] "published versions are immutable" and provenance verification returned MATCH for both versions. The implicit-downgrade attack SUCCEEDED: `POST /api/installs/upgrade` with targetVersion 2.3.1 while pinned at 2.4.0 returned ok:true and moved the pin down, recorded as an "upgraded" history entry (finding F4 — mirrored by the engine probe's follow-policy HOLE: the distribution crate's follow policy surfaced 1.5.0 from a 2.0.0 pin and applied it, semver ordering unchecked). After the hole, the browser rollback moved the pin FORWARD to 2.4.0 labeled "rolled back" (finding F5). The 503/conflict rejections again produced no audit events (F3 pattern). Engine parity (fresh at base): contracts 90/0, forge 44/0, distribution 10+8/0 all passing; the engine probe's version attacks (mutate-after-publish ×2 variants, branch move, lock tamper ×2 variants, moving ref, stale proposal, failed-upgrade rollback, over-install v2-while-v1, implicit downgrade) — REPELLED/HELD except the downgrade HOLE.

## Workflow Identity
- Workflow: Support Ticket Triage Assistant (pk-102) installed as ins-0401 for Acme Retail; engine-side counterpart: codex-workflow-distribution install pin/upgrade/follow contracts + codex-workflow-forge publish/version-integrity contracts — library-only, no product mount (Finding F1)
- Version: 2.3.1 → 2.4.0 explicit; then 2.4.0 → 2.3.1 via the F4 hole; then 2.3.1 → 2.4.0 via the F5 forward rollback
- Source revision: NONE — surface missing (per-release provenance commit hashes exist — 2.4.0 commit 57408a3 — but are not user-addressable workflow source revisions)
- Definition digest: 2.3.1 = sha256:04cf049e110cc2cc, 2.4.0 = sha256:cadb075e36500b7c — immutable through every operation including the downgrade hole (digests never recomputed or overwritten)
- Dependency-lock identity: NONE — surface missing (no lock surface in FlowMart; engine probe attack 3 REPELLED — locks are identity-covered, re-sealing yields a NEW identity, incomplete locks refused)

## Runtime Binding
- Capabilities: install:manage + install:configure (petra.voss), version:publish (vera.osei), audit:verify (mia.torres for provenance checks)
- Resources: package pk-102, install ins-0401, entitlement ent-0501, org Acme Retail (o-2)
- Environments: FlowMart browser surface + documented JSON API twins (same op codes as the forms)
- Approvals: explicit version-pin authority — upgrades carry expectedCurrentVersion (optimistic concurrency); one-active-install guard; published-version immutability gate
- Triggers/schedules: version.published → upgrade-available notification; install.upgraded/rolled_back → org-admin notifications

## Evidence
- docs/validation/evidence/VWO-009/adv-version-confusion/user-path/20260912T090421Z-instruct-phase-version-governance-rules.txt
- docs/validation/evidence/VWO-009/adv-version-confusion/user-path/20260912T090432Z-step01-acme-install-pinned-231.txt
- docs/validation/evidence/VWO-009/adv-version-confusion/user-path/20260912T092930Z-step02-demo-phase-step02-vera-publishes-240.txt
- docs/validation/evidence/VWO-009/adv-version-confusion/user-path/20260912T092943Z-step03-demo-phase-step03-duplicate-install-reje.txt
- docs/validation/evidence/VWO-009/adv-version-confusion/user-path/20260912T093005Z-step04-demo-phase-step04-configure-form-drops-j.txt
- docs/validation/evidence/VWO-009/adv-version-confusion/user-path/20260912T093104Z-step06-demo-phase-step06-explicit-upgrade-240.txt
- docs/validation/evidence/VWO-009/adv-version-confusion/user-path/20260912T093153Z-step08-demo-phase-step08-rollback-moves-forward.txt
- docs/validation/evidence/VWO-009/adv-version-confusion/post-hoc/20260912T092723Z-engine-probe-version-attacks.txt
- docs/validation/evidence/VWO-009/adv-version-confusion/post-hoc/20260912T093022Z-step04-api-twin-downgrade-impersonation.txt
- docs/validation/evidence/VWO-009/adv-version-confusion/post-hoc/20260912T093049Z-step05a-stale-upgrade-proposal-rejected.txt
- docs/validation/evidence/VWO-009/adv-version-confusion/post-hoc/20260912T093112Z-step05b-replayed-proposal-stale-rejected.txt
- docs/validation/evidence/VWO-009/adv-version-confusion/post-hoc/20260912T093133Z-step07-implicit-downgrade-via-upgrade-ho.txt
- docs/validation/evidence/VWO-009/adv-version-confusion/post-hoc/20260912T093212Z-attack-mutate-published-definition-repel.txt
- docs/validation/evidence/VWO-009/adv-version-confusion/post-hoc/20260912T093226Z-events-pin-history-digests-reset.txt
- docs/validation/evidence/VWO-009/adv-version-confusion/post-hoc/20260912T093304Z-reset-verification.txt

## Failure / Recovery
- Failure injected: natural oracles — duplicate install (duplicate_event), stale upgrade proposals (data_conflict ×2), version-mismatch guards for moving refs; plus the adversarial downgrade itself (the hole).
- Recovery attempted: after the downgrade hole moved the pin to 2.3.1, the browser rollback returned the install to 2.4.0 — but labeled "rolled back 2.3.1 → 2.4.0" (forward), documenting F5.
- Restart/session-loss behavior: not exercised (VWO-007's scope).
- Final outcome: pin restored to 2.4.0, digests immutable throughout, the downgrade hole and rollback confusion documented as findings, seed state restored by reset-scenario.sh.

## Product / UX Friction
- The install card hides the upgrade form exactly when the install is up to date — good guard-rail UX — but the op behind it still accepts downgrades, so the withheld form is the ONLY downgrade protection a normal person gets (and it is client-side only).
- The rollback button label ("Rollback to previous") does not say WHICH version; after the F4 hole it silently means the NEWER version.
- Stale-proposal and duplicate-install errors name the current state (currentVersion / existing install id) — clear and actionable.
- The version-mismatch error for moving refs ("you selected latest but the latest published version is 1.1.0") is technically accurate but mildly confusing to a person who typed "latest" expecting a floating tag.

## Security / Architecture Findings
- Finding: F4 — implicit downgrade of a pinned instance through the UPGRADE operation: `upgradeInstall` (FlowMart) and the distribution crate's follow/apply path (engine probe HOLE) both accept a target version OLDER than the installed pin with no semver-ordering or explicit-downgrade guard. The pin moved 2.4.0 → 2.3.1 recorded as "upgraded" (product) and 2.0.0 → 1.5.0 via an applied upgrade record (engine). The WO acceptance "downgrade a pinned instance implicitly" SUCCEEDED on both surfaces; the catalog oracle "downgrades only via explicit rollback" is violated. A normal person cannot rely on "upgrade means forward" — the operation's name lies about its effect.
- Severity: P1
- Reproduction: step 8 user path (API twin, ok:true, history "upgraded 2.4.0 → 2.3.1"); engine probe attack 12 (follow policy surfaced and applied 1.5.0 from a 2.0.0 pin).
- Root cause: no ordering check between targetVersion and the current pin on the upgrade/apply path (both layers); rollback exists as the sanctioned downgrade control but nothing prevents the upgrade op from moving down.
- Frozen invariant affected: pins move only via explicit control-plane decisions, and a "downgrade" must be an explicit labeled decision (implicit downgrade = pin corruption class).
- Finding: F5 — rollback direction confusion after a downgrade: `rollbackInstall` selects "the last history entry whose toVersion differs from the current pin" — after an F4-style downgrade that entry is the NEWER version, so the op labeled "rollback" moves the pin FORWARD (observed: "rolled back 2.3.1 → 2.4.0"). The history-based "previous" is order-blind.
- Severity: P2
- Reproduction: step 8 user path (browser rollback after the hole; flash and history captured).
- Root cause: rollback derives the target from history recency, not from the semantic previous pin or an explicit recorded rollback point.
- Frozen invariant affected: rollback is the explicit, labeled downgrade authority — its direction must be deterministic and human-predictable.
- Finding: F6 — missing product surfaces for three WO attack classes: FlowMart has no development-branch/ref surface (attack 2: move/delete dev branch after release — provenance records only a per-release commit hash), no dependency-lock surface (attack 3: tamper lock), and no listing scope/visibility-transition operation (attack 10: marketplace transition altering semantics). Those attacks are exercised engine-side only (REPELLED/HELD by the probe); the product coverage for them is absent by fixture scope.
- Severity: P3
- Reproduction: grep of the fixture surface (no fork/branch/lock/visibility ops exist); engine probe attacks 2, 3, 10 verdicts in the _infra battery artifact.
- Root cause: VWO-002 fixture scope decision (store surface only).
- Frozen invariant affected: none (coverage gap, not a boundary violation).

## Recommendation
- FIX NOW
- Proposed owner: F4 — distribution engine owner + FlowMart fixture owner (bounded: reject targetVersion ≤ current pin on the upgrade/apply path unless an explicit downgrade flag is present; mirror the same guard in follow_policy; retest: re-run step 8 expecting a 409 naming the downgrade path, and re-run engine attack 12 expecting refusal); F5 — fixture owner (bounded: derive rollback target from the last "upgraded" entry's fromVersion, not the last differing toVersion; retest: after a legitimate 2.3.1 → 2.4.0 upgrade, rollback must land on 2.3.1); F6 — DEFER to fixture roadmap (P3 coverage gap; engine coverage exists).
- Verification required: re-run adv-version-confusion steps 8 with the guard active (F4); verify rollback direction after both a normal upgrade and the (now-rejected) downgrade attempt (F5); product-surface roadmap decision for branch/lock/visibility surfaces (F6).

## Worker Conclusion
passed with a confirmed hole — duplicate-install, stale-proposal, moving-ref and mutate-publication attacks were all repelled with typed errors and immutable digests, and the explicit upgrade landed exactly on the published artifact; the implicit-downgrade attack SUCCEEDED through the upgrade op (F4, mirrored engine-side by the follow-policy hole) and the subsequent rollback moved the pin forward (F5) — recorded as findings, not repelled attacks.

## Scenario
- Scenario id: adv-marketplace-entitlement-race
- Industry: adversarial (proving ground: marketplace / FlowMart, port 4105)
- Scenario: entitlement changes DURING commercial operations — an expired trial blocks install; an admin revokes an org's entitlement mid-upgrade — operations must fail closed, no partial install, executable content untouched by commercial gates
- Teaching mode: HYBRID
- User goal: Iris Chen (buyer org admin) and Petra Voss (Acme org admin) run installs while Dan Kowalski (marketplace admin) revokes entitlements mid-operation and Vera Osei (publisher) releases a new version — every commercial gate must fail closed with the entitlement id named, pins and digests untouched, and a renewed entitlement must let the blocked operation retry cleanly

## User Path
1. Recorded the INSTRUCT entitlement-gate instruction text verbatim (no teaching surface — standing F1), signed in as `iris.chen`, and established the Northwind install (ins-0402 v2.3.1, auto-granted trial ent-0504) — Acme's ins-0401 (ent-0501) is the seed counterpart.
2. As iris, clicked **Install v1.1.0** on the Invoice Autofill listing — blocked by the natural expired trial: "Entitlement ent-0502 for 'Invoice Autofill for ERPs' (Northwind Logistics) is expired (valid until 2026-08-01) — installation requires an active license. [stale_entitlement]".
3. As `vera.osei`, published 2.4.0 — both org admins received upgrade-available notifications ("2 installer(s) notified").
4. THE RACE: as `petra.voss`, opened the upgrade form on /installs; as `dan.kowalski`, revoked ent-0501 ("Mid-upgrade commercial revocation…") while the form was open; petra then clicked **Upgrade to 2.4.0** — blocked: "Entitlement ent-0501 for 'Support Ticket Triage Assistant' (Acme Retail) is revoked — upgrade requires an active license. [stale_entitlement]"; the pin stayed 2.3.1 with zero partial state (history: installed only).
5. Observed the entitlement ledger as dan — "Entitlement ent-0501 revoked." with the reason recorded.
6. As vera (audit:verify), ran provenance verification — "Provenance verified … @2.4.0: digest sha256:cadb075e36500b7c matches the published manifest" — the package's digests/versions were NOT altered by the revocation or the blocked upgrade.
7. RECOVERY ATTEMPT: as dan, granted Acme a fresh subscription entitlement (ent-0505, valid until 2027); as petra, retried the upgrade — STILL BLOCKED by the revoked ent-0501: the gates resolve the org+package entitlement to the FIRST record (the revoked one), so the renewal grant is shadowed (finding F7 — the catalog's "renew → retry cleanly" recovery fails).
8. Post-hoc battery (API twins): the deterministic mid-upgrade block (`?failure=stale_entitlement` on iris's ins-0402 upgrade) — blocked, pin unchanged, no partial state; double-revoke of ent-0501 — [data_conflict] "already revoked"; cross-tenant writes — petra (Acme) configured NORTHWIND's ins-0402 (ok:true, queue overwritten) and then UPGRADED it (pin 2.3.1 → 2.4.0) — the install ops have no actor-org checks (finding F8).

## Expected
Commercial gates fail closed: the expired trial blocks install with the entitlement id and date; a mid-upgrade revocation blocks the upgrade with the entitlement id, leaves the pin unchanged, creates no partial state, and lands in the ledger; digests/versions untouched; a renewed entitlement lets the same operation succeed; double-revoke conflicts; install operations are org-scoped.

## Actual
Every fail-closed gate held: the expired-trial install was blocked naming ent-0502 and its expiry; the mid-upgrade revocation blocked petra's upgrade naming ent-0501 with the pin at 2.3.1 and zero partial state; the ledger recorded the revocation with reason; provenance verification confirmed 2.4.0's digest unchanged; the deterministic switch variant blocked identically; the double-revoke returned [data_conflict]. The RECOVERY FAILED: after dan granted ent-0505, petra's retry was still blocked by the revoked ent-0501 — entitlement resolution picks the first org+package record and a later active grant cannot supersede a revoked one, so revocation permanently bricks that org+package's commercial operations through the product surface (finding F7). Cross-tenant attacks SUCCEEDED: petra (Acme org_admin) wrote config into Northwind's ins-0402 and then moved its pin 2.3.1 → 2.4.0 — configure/upgrade/rollback perform no actor-org checks (finding F8, re-verifying VWO-008 F4 at base 7736a42 in the versioning context, now including the pin-move op). Blocked operations again left no audit events (F3 pattern). Engine-side, the marketplace attack extract shows: entitlement removal during installation REPELLED (InstallRefused, sealed record HELD byte-identical), scope transitions HELD (sealed version/digest/pin unchanged across Public→Private→Shared→Public), and two engine HOLES — fork misattribution accepted (attacker-written attribution content, lineage chain intact — finding F9) and the upgrade-path visibility bypass (evaluate_upgrade surfaced a PRIVATE release to a foreign installer and decide_upgrade approved it — finding F10).

## Workflow Identity
- Workflow: Support Ticket Triage Assistant (pk-102) as ins-0401 (Acme) and ins-0402 (Northwind); engine-side counterpart: codex-workflow-distribution marketplace/entitlement/visibility contracts + codex-workflow-forge fork records — library-only, no product mount (Finding F1)
- Version: 2.3.1 (both installs) with 2.4.0 published; the cross-tenant attack moved ins-0402 to 2.4.0
- Source revision: NONE — surface missing (same as prior blocks)
- Definition digest: 2.3.1 = sha256:04cf049e110cc2cc, 2.4.0 = sha256:cadb075e36500b7c — byte-identical across every entitlement change, blocked operation, and cross-tenant write
- Dependency-lock identity: NONE — surface missing

## Runtime Binding
- Capabilities: install:manage/install:configure (iris.chen, petra.voss), entitlement:grant + entitlement:revoke (dan.kowalski, marketplace admin), version:publish (vera.osei), audit:verify (vera.osei in-session)
- Resources: pk-102 + ins-0401/ins-0402, ent-0501 (revoked), ent-0502 (expired), ent-0504 (trial), ent-0505 (shadowed grant), pk-101/ent-0502 for the expired-trial install, orgs o-1/o-2
- Environments: FlowMart browser (per-persona sessions) + documented JSON API twins
- Approvals: the commercial entitlement gates on install/upgrade/rollback/configure (fail-closed, entitlement id named); grant/revoke authority scoped to entitlement:grant/revoke holders
- Triggers/schedules: version.published → upgrade-available notifications; entitlement.granted/revoked → org-admin notifications; blocked operations produce no events (F3)

## Evidence
- docs/validation/evidence/VWO-009/adv-marketplace-entitlement-race/user-path/20260912T093329Z-instruct-phase-entitlement-gates-fail-cl.txt
- docs/validation/evidence/VWO-009/adv-marketplace-entitlement-race/user-path/20260912T093345Z-step01-demo-phase-step01-iris-signs-in.txt
- docs/validation/evidence/VWO-009/adv-marketplace-entitlement-race/user-path/20260912T093356Z-step02-demo-phase-step02-northwind-install-esta.txt
- docs/validation/evidence/VWO-009/adv-marketplace-entitlement-race/user-path/20260912T093407Z-step03-demo-phase-step03-expired-trial-blocks-i.txt
- docs/validation/evidence/VWO-009/adv-marketplace-entitlement-race/user-path/20260912T093416Z-step04-demo-phase-step04-vera-publishes-240.txt
- docs/validation/evidence/VWO-009/adv-marketplace-entitlement-race/user-path/20260912T093609Z-step05-demo-phase-step05-midupgrade-revocation-.txt
- docs/validation/evidence/VWO-009/adv-marketplace-entitlement-race/user-path/20260912T093610Z-step06-demo-phase-step06-entitlement-ledger-rev.txt
- docs/validation/evidence/VWO-009/adv-marketplace-entitlement-race/user-path/20260912T093626Z-step07-demo-phase-step07-provenance-digests-unt.txt
- docs/validation/evidence/VWO-009/adv-marketplace-entitlement-race/user-path/20260912T093718Z-step08-demo-phase-step08-renewal-retry-still-bl.txt
- docs/validation/evidence/VWO-009/adv-marketplace-entitlement-race/post-hoc/20260912T092723Z-engine-probe-marketplace-attacks.txt
- docs/validation/evidence/VWO-009/adv-marketplace-entitlement-race/post-hoc/20260912T093823Z-posthoc-attack-battery-and-events.txt
- docs/validation/evidence/VWO-009/adv-marketplace-entitlement-race/post-hoc/20260912T093830Z-reset-verification.txt

## Failure / Recovery
- Failure injected: the natural expired trial (ent-0502) on install; the mid-upgrade revocation of ent-0501 (race between dan's revoke and petra's upgrade submission); the deterministic `stale_entitlement` switch on iris's upgrade; the double-revoke replay.
- Recovery attempted: renewal via a fresh admin grant (ent-0505) followed by petra's retry — FAILED: the revoked ent-0501 still shadows the gate (finding F7); no renew/un-revoke operation exists.
- Restart/session-loss behavior: not exercised (VWO-007's scope); the state is fixture-persistent and revocation survived all session switches.
- Final outcome: all commercial gates failed closed with entitlement ids named and zero partial state; the recovery path is broken (F7); cross-tenant writes succeeded (F8); digests and published versions untouched throughout; seed restored by reset-scenario.sh.

## Product / UX Friction
- The blocked-upgrade error names the entitlement id, the status, and the required action — good fail-closed UX — but after the renewal grant the SAME error still names the OLD revoked ent-0501, which a normal person cannot reconcile with the ledger that shows a fresh active ent-0505 (F7).
- The entitlement ledger renders grant and revoke rows clearly (status pills, reasons, validUntil).
- The cross-tenant success (F8) is silent: no UI hints that petra is operating another org's install.

## Security / Architecture Findings
- Finding: F7 — revocation permanently bricks org+package operations; renewal cannot restore: `entitlementFor` resolves the FIRST entitlement record for org+package, so a revoked record shadows later active grants; there is no renew/un-revoke op. After dan granted ent-0505 (subscription, valid 2027), every install/upgrade/rollback/configure for Acme+pk-102 was still blocked by ent-0501 — the commercial lifecycle (re-license after revocation, i.e. the vendor selling again) is dead-ended through the product surface. The acceptance question fails: an org admin and a marketplace admin follow the documented recovery path and the operation still fails.
- Severity: P1
- Reproduction: step 7-8 user path (grant ent-0505 → retry → still [stale_entitlement] naming ent-0501).
- Root cause: entitlement lookup is first-match over a growing list with no status/supersession logic; revocation is terminal by data model.
- Frozen invariant affected: commercial gates fail closed (held) AND recover by design (violated) — gates must block on stale entitlements but accept renewed authority.
- Finding: F8 — cross-tenant install writes including PIN MOVES: configureInstall/upgradeInstall/rollbackInstall perform no actor-org check — petra.voss (Acme org_admin) successfully configured Northwind's ins-0402 (config overwritten) and upgraded it (pin 2.3.1 → 2.4.0, history recorded under her id). Re-verifies VWO-008 F4 (cross-tenant install writes) at base 7736a42 and extends it to the versioning context: another org's admin can move YOUR production pin.
- Severity: P1
- Reproduction: post-hoc battery sections 5-6 (ok:true for both the configure and the upgrade on ins-0402 as petra).
- Root cause: the install ops resolve the install by id only; org membership is never compared to the install's orgId.
- Frozen invariant affected: resource scoping — capability (install:manage) does not grant cross-tenant authority over another org's resources.
- Finding: F9 — fork attribution content is unverified (engine): a fork record with attacker-written attribution text was accepted (empty attribution is refused — "a fork must carry upstream attribution" — and the forked_from lineage chain correctly points at the real upstream), but the attribution CONTENT is free text with no verification or signature. A marketplace consumer reading the fork sees attacker-authored provenance.
- Severity: P2
- Reproduction: engine probe attack 5b (HOLE verdict, verbatim output in the marketplace extract).
- Root cause: attribution is a required-but-unvalidated string field; only the structural lineage is enforced.
- Frozen invariant affected: forks preserve lineage (structurally held); attribution must be trustworthy surface (violated).
- Finding: F10 — upgrade-path visibility bypass (engine): `evaluate_upgrade` surfaced a PRIVATE release's identity to a non-owner installer and `decide_upgrade` APPROVED and applied it — the documented gate order (visibility → integrity → access → entitlement) skips the visibility gate on the upgrade path, while search and direct install correctly return zero results / ReleaseNotVisible. Cross-owner private-release discovery leakage — WO attack 6 — succeeded at the engine layer.
- Severity: P1
- Reproduction: engine probe attack 6d (HOLE verdict: leaked identity sha256:99b5a40f… + applied=true).
- Root cause: the upgrade evaluation path never consults the visibility gate that search/install enforce.
- Frozen invariant affected: access/entitlement gates cannot mutate executable content AND cannot bypass visibility — private releases must be invisible to foreign principals on every path.

## Recommendation
- FIX NOW
- Proposed owner: F7 — fixture owner (bounded: entitlement resolution should select the newest ACTIVE entitlement for org+package, or grant should mark superseded records; retest: revoke → grant → retry upgrade must succeed); F8 — fixture owner (bounded: org guard on configure/upgrade/rollback — same bounded fix as VWO-008 F4; retest: re-run the cross-tenant probes expecting 403); F9 — forge engine owner (NEW WORK ORDER candidate: signed/derived attribution content; retest: misattributed fork must fail verification); F10 — distribution engine owner (bounded: add the visibility gate to evaluate/decide_upgrade in documented order; retest: engine attack 6d must refuse with ReleaseNotVisible).
- Verification required: re-run adv-marketplace-entitlement-race steps 7-8 (recovery succeeds, F7) and the cross-tenant probes (403, F8); re-run engine attacks 5b/6d expecting refusal (F9/F10).

## Worker Conclusion
passed with confirmed holes — every commercial gate failed closed with entitlement ids named, zero partial state, and immutable digests (the mid-upgrade revocation race resolved exactly as specified), but the renewal-recovery path is dead-ended by entitlement shadowing (F7), cross-tenant install writes and pin moves succeeded (F8), and the engine accepted a misattributed fork (F9) and approved a private release on the upgrade path (F10) — recorded as findings, not repelled attacks.

---

**Program-Level Summary (VWO-009)**

**Attack-class coverage (all twelve WO-required attacks)**

| # | Attack class | Where exercised | Verdict |
|---|---|---|---|
| 1 | mutate published definition after publication | FlowMart re-publish 2.4.0/2.3.1 with tampered changelog (duplicate_event, immutable); engine: VersionIntegrity on publish, store-tamper recomputation, republish-same-identity | REPELLED (both layers) |
| 2 | move/delete development branch after release | engine only (no product branch surface — F6): branch moved to a new commit, released id still verifies, install pin HELD | REPELLED (engine) / surface missing |
| 3 | tamper dependency lock | engine only (no product lock surface — F6): tampered lock digest mismatch, incomplete lock refused, re-seal = NEW identity | REPELLED (engine) / surface missing |
| 4 | install a moving ref as executable authority | FlowMart: expectedVersion "latest"/"main" refused (version mismatch); engine: authority is a typed immutable WorkflowVersionId, no moving-ref field | REPELLED (both layers) |
| 5 | fork without attribution/lineage | engine only (no product fork surface — F6): empty attribution refused, upstream byte-identical, lineage intact; misattributed content accepted | **HOLE — F9** (misattribution) |
| 6 | cross-owner/private-release discovery leakage | engine: anonymous/foreign search zero results, foreign install ReleaseNotVisible — but upgrade path leaked + approved a private release; product: all listings public by design (no visibility surface) | **HOLE — F10** (upgrade path) |
| 7 | entitlement removal during installation | FlowMart: natural expired-trial install block, mid-upgrade revocation block, deterministic switch block (entitlement id named, no partial state); engine: InstallRefused, sealed record HELD | REPELLED (both layers) |
| 8 | stale upgrade proposal | FlowMart: fabricated 2.2.0 proposal + verbatim replay after pin move (data_conflict ×2); engine: StaleUpgradeProposal | REPELLED (both layers) |
| 9 | failed upgrade followed by rollback | FlowMart: 503 registry-down upgrade (fail-safe, clean retry succeeded); rollback op exercised (direction confusion after F4 noted); engine: gate-refused upgrade leaves pin, explicit v1→v2→v1 history, v1/v2 byte-identical | HELD (product fail-safe + engine) |
| 10 | marketplace transition altering executable semantics | engine only (no product scope-transition surface — F6): Public→Private→Shared→Public left sealed version, digest and pin byte-identical; no-op scope change = IllegalTransition | HELD (engine) / surface missing |
| 11 | install v2 while v1 is still running | FlowMart: duplicate install rejected [duplicate_event] (one active install per org+package); engine: AlreadyInstalled on both forge and distribution paths | REPELLED (both layers) |
| 12 | downgrade a pinned instance implicitly | FlowMart: upgrade op accepted targetVersion 2.3.1 from pin 2.4.0 (UI withholds the form; op code unguarded); engine: follow policy surfaced and applied 1.5.0 from 2.0.0 | **HOLE — F4** (both layers) |

**Findings summary**

| Id | Severity | One-line | Disposition |
|---|---|---|---|
| F1 | P1 | workflow versioning/distribution engine unmounted — no product surface for any identity/pin/entitlement guarantee (4th independent confirmation) | FIX NOW (standing wave-3 mount decision) |
| F4 | P1 | implicit downgrade through the upgrade op — no semver/ordering guard at product OR engine follow path | FIX NOW (bounded: ordering guard + explicit-downgrade flag) |
| F7 | P1 | revocation permanently bricks org+package ops — renewed grants are shadowed by the first (revoked) entitlement record | FIX NOW (bounded: newest-active entitlement resolution) |
| F8 | P1 | cross-tenant install writes incl. pin moves — configure/upgrade/rollback lack actor-org checks (VWO-008 F4 re-verified at 7736a42, pin-move newly demonstrated) | FIX NOW (bounded: org guard on the three ops) |
| F10 | P1 | upgrade-path visibility bypass — evaluate/decide_upgrade skip the visibility gate, leaking and approving a private release to a foreign installer | FIX NOW (bounded: visibility gate in documented order) |
| F2 | P2 | configure accepts identity-impersonating keys (version/digest/targetVersion/manifest) into the install config record | FIX NOW (bounded: reserved-key deny-list) |
| F5 | P2 | rollback direction confusion — after a downgrade-as-upgrade, "rollback" moves the pin forward | FIX NOW (bounded: derive target from the upgrade's fromVersion) |
| F9 | P2 | fork attribution content unverified — attacker-written provenance text accepted (lineage chain intact) | NEW WORK ORDER (signed/derived attribution) |
| F3 | P2 | repelled/blocked operations leave no audit events (VWO-007 F2 / VWO-008 F6 re-confirmed in the distribution context) | NEW WORK ORDER (rejection/conflict events) |
| F6 | P3 | no product surface for dev-branch refs, dependency locks, listing-scope transitions, forks, or private visibility (attacks 2, 3, 10, 5, 6 product halves) | DEFER (fixture roadmap; engine coverage exists) |

**Proven guarantees (fixture + engine parity)**
Published version identities are immutable and content-derived: re-publishing an existing version is refused at both layers, tampered records fail digest recomputation, and re-sealing tampered content yields a NEW identity rather than an overwrite. Installations are explicitly pinned: moving refs are never authority, over-installs are refused, the pin moves only through explicit decisions, and a failed (503) upgrade leaves zero partial state with a clean retry. Stale proposals conflict with the current pin named. Commercial gates fail closed with entitlement ids and never touch executable content (digests byte-identical across every entitlement change and blocked operation). Scope transitions leave sealed versions byte-identical (engine). Engine parity at base 7736a42: contracts 90/0, forge 44/0, distribution 10+8/0 — all passing, plus the standalone 12-attack probe battery (13/13 tests green, verdicts REPELLED/HELD except the three documented HOLEs F4/F9/F10).

**No-secret attestation**
No secret entered ordinary workflow evidence or source: every committed artifact was captured through capture-evidence.sh (no refusals triggered in this WO — all attack payloads used fake digests, example manifest names, and demo fixture tokens); all fixture credentials used are the documented demo-pass personas; no real credentials were used or introduced anywhere in this run.
