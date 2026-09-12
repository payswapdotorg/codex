# VWO-010 — Scenario/Issue Matrix and Canonical Finding Families

**Class B synthesis document** (REPORT-SCHEMA.md §1). Consolidates the six merged
wave-1/wave-2 validation reports into one scenario/issue picture, deduplicates
their locally-numbered findings into canonical families, and dispositions every
P2/P3. Per REPORT-SCHEMA §1 the six merged reports are Class A; this document
carries Identity-level facts, the matrix, the families, and the dispositions.

## Identity

- Work Order: VWO-010 (Wave 3 — synthesis, remediation planning, final report draft)
- Worker persona: Validation Synthesis Analyst (senior) — documents and verifications only; implements nothing
- Base branch: main
- Base SHA: 0f89393c60403e423a91cffc3055af07e31a0c0b
- Head SHA: see git rev-parse vwo-010/validation-synthesis — single delivery commit on base 0f89393 (a commit cannot embed its own hash; exact SHA recorded in the delivery bundle and the completion message)
- Validation environment: sandbox clone of github.com/payswapdotorg/codex; VWO-002 fixtures started with `run-all.sh --reset` (all five healthy on 127.0.0.1:4101–4105; canonical `verify-sweep.sh` **59 passed, 0 failed** — `docs/validation/evidence/vwo-010/_infra/post-hoc/20260912T103350Z-environment-and-base-identity.txt`); browser surface via agent-browser 0.35.0 over Playwright chromium (headless); FlowMart/PressRoom JSON API twins via curl; node v24.19.0, git 2.47.3
- E2B template/workspace identity, if used: N/A — E2B/Composio integration absent (VWO-001 fallback record governs)
- Codex Universal runtime/application SHA: NONE — no user-facing Codex Universal runtime/application build exists in this repository state (Family A); the engine crates are library-only. **cargo/rustc ABSENT in this sandbox** (`which cargo` returns nothing) — engine-crate tests were NOT re-run here; engine claims rely on static source probes plus the merged reports' own cargo transcripts (honest limitation, recorded)
- Date/time window: 2026-09-12T10:30Z–2026-09-12T11:05Z (UTC)
- Base verification: `git rev-parse 0f89393^{commit}` == origin/main HEAD at clone; VWO-009 merge is the direct parent
- Sources consolidated (clone wins over packet excerpts; reports win over the FINDINGS INDEX where they disagree — deviations listed in §4):
  - `docs/validation/reports/VWO-004-report.md` (base 0d314fd, 161 evidence artifacts)
  - `docs/validation/reports/VWO-005-report.md` (base 0d314fd, 62 artifacts)
  - `docs/validation/reports/VWO-006-report.md` (base 0d314fd, 36 artifacts)
  - `docs/validation/reports/VWO-007-report.md` (base 0d314fd, 75 artifacts)
  - `docs/validation/reports/VWO-008-report.md` (base d198a00, 62 artifacts)
  - `docs/validation/reports/VWO-009-report.md` (base 7736a42, 56 artifacts)
  - Total: **34 scenario runs · 452 evidence artifacts** (git ls-files counts, base 0f89393).

---

## 1. Scenario matrix (one row per scenario run)

Outcome values are the reports' own Worker Conclusion verdicts. "Findings hit"
uses **canonical family ids** (§2); local report ids appear in brackets.

### VWO-004 — enterprise-ops real-user (12/12 scenarios, all teaching modes ×4)

| # | Scenario id | Persona / goal (condensed) | Mode | Outcome | Findings hit | Evidence |
|---|---|---|---|---|---|---|
| 1 | construction-daily-progress | foreman daily site report; PM notified; duplicate/notification switches | DEMONSTRATE | blocked (business path passed; teaching step unperformable) | **A** [F1] | `evidence/VWO-004/construction-daily-progress/` |
| 2 | construction-procurement-approval | finance→PM entitlement-gated PO approval with contract-validity gate | HYBRID | blocked (gates passed; approve→PO sub-path impossible on seed) | **A** [F1], Drift [F2] | `evidence/VWO-004/construction-procurement-approval/` |
| 3 | construction-safety-incident | foreman intake → officer close with permission gate + fail-safe | INSTRUCT | blocked (path passed end-to-end) | **A** [F1], Drift [F5] | `evidence/VWO-004/construction-safety-incident/` |
| 4 | software-pr-triage | CI gate, code-owner approval, merge→staging only | INSTRUCT | blocked (path passed end-to-end) | **A** [F1] | `evidence/VWO-004/software-pr-triage/` |
| 5 | software-incident-response | incident→promote→resolve across browser AND terminal; named-dependency failures | HYBRID | blocked (path passed end-to-end) | **A** [F1] | `evidence/VWO-004/software-incident-response/` |
| 6 | software-engineering-onboarding | ownership lookup, runbook read, issue filed, permission boundaries | DEMONSTRATE | blocked (path passed end-to-end) | **A** [F1] | `evidence/VWO-004/software-engineering-onboarding/` |
| 7 | rideshare-driver-onboarding | public intake → screening → activation on live map; document/permit gates | DEMONSTRATE | blocked (path passed end-to-end) | **A** [F1] | `evidence/VWO-004/rideshare-driver-onboarding/` |
| 8 | rideshare-support-escalation | respond→escalate with conflict/permission gates | INSTRUCT | blocked (path passed; resolution by ops per fixture model) | **A** [F1], Drift [F3] | `evidence/VWO-004/rideshare-support-escalation/` |
| 9 | rideshare-surge-ops | ops+map visibility, broadcast, concurrent conflict, finance contrast | HYBRID | blocked (loop passed end-to-end) | **A** [F1] | `evidence/VWO-004/rideshare-surge-ops/` |
| 10 | media-editorial-publishing | draft→hero→review→approval→multi-channel publish; no-hero gate | DEMONSTRATE | blocked (path passed end-to-end) | **A** [F1] | `evidence/VWO-004/media-editorial-publishing/` |
| 11 | media-content-repurposing | rendition→schedule→publish, duplicate gate, correction loop | HYBRID | blocked (implemented behaviors passed; rendition switch mismatch) | **A** [F1], Drift [F4] | `evidence/VWO-004/media-content-repurposing/` |
| 12 | media-breaking-news | expedited single-editor approval, fast channels, correction loop | INSTRUCT | blocked (implemented behaviors passed; push channel absent) | **A** [F1], Drift [F6] | `evidence/VWO-004/media-breaking-news/` |

### VWO-005 — workflow creator / marketplace seller (6 scenarios)

| # | Scenario id | Persona / goal | Mode | Outcome | Findings hit | Evidence |
|---|---|---|---|---|---|---|
| 13 | marketplace-create-and-sell | Vera Osei publishes immutable v1, entitlement grant, cross-identity install | HYBRID | blocked (marketplace half passed in full; teaching half unreachable) | **A** [F1], **B** [F2], Policy [F3] | `evidence/VWO-005/marketplace-create-and-sell/` |
| 14 | marketplace-fork-and-improve | fork pk-102 → new immutable release with lineage | INSTRUCT | blocked (release semantics passed; fork/evidence/approval surfaces absent) | **A** [F1], **C** [F4], **D** [F5] | `evidence/VWO-005/marketplace-fork-and-improve/` |
| 15 | marketplace-discover-install-configure | buyer discovers, installs, configures under auto-trial | DEMONSTRATE | blocked (fixture path passed; configure human path defective; teaching absent) | **A** [F1], **B** [F2] | `evidence/VWO-005/marketplace-discover-install-configure/` |
| 16 | marketplace-upgrade-rollback | explicit pinned upgrade, rollback, entitlement-blocked fail-safe | INSTRUCT | blocked (store semantics passed; cross-tenant hole found) | **A** [F1], **E** [F6] | `evidence/VWO-005/marketplace-upgrade-rollback/` |
| 17 | software-pr-triage (additional cycle) | create+publish cycle via ForgeOps | INSTRUCT | blocked (triage gates passed behaviorally) | **A** [F1] | `evidence/VWO-005/software-pr-triage/` |
| 18 | media-editorial-publishing (additional cycle) | create+publish cycle via PressRoom | DEMONSTRATE | blocked (editorial gates passed; dead-end on approved-story attach) | **A** [F1], Drift [F7] | `evidence/VWO-005/media-editorial-publishing/` |

### VWO-006 — workflow consumer / automation operator (3 scenarios)

| # | Scenario id | Persona / goal | Mode | Outcome | Findings hit | Evidence |
|---|---|---|---|---|---|---|
| 19 | marketplace-discover-install-configure | consumer discover→inspect→install→configure (primary owner) | DEMONSTRATE | blocked (store gates passed; configure no-op; teaching absent) | **A** [F-1], **B** [F-2], **L** [F-6], Policy [F-3], Consumer [F-4/F-5 context] | `evidence/VWO-006/marketplace-discover-install-configure/` |
| 20 | marketplace-upgrade-rollback | consumer upgrade/rollback via store (primary owner) | INSTRUCT | blocked (store semantics passed; teaching absent) | **A** [F-1] | `evidence/VWO-006/marketplace-upgrade-rollback/` |
| 21 | adv-environment-failure-rebinding | registry outage → compatible rebind without semantic mutation | HYBRID | blocked (adversarial core PASSED — rebind invariant held; rebind human path blocked by configure defect) | **A** [F-1], **B** [F-2] | `evidence/VWO-006/adv-environment-failure-rebinding/` |

### VWO-007 — persistence/restart adversary (4 scenarios, all attacks REPELLED)

| # | Scenario id | Persona / goal | Mode | Outcome | Findings hit | Evidence |
|---|---|---|---|---|---|---|
| 22 | adv-session-loss-restart | SIGKILL mid-workflow, restart w/o reseed, idempotency replay | DEMONSTRATE | **passed** (all repelled, human-observable recovery) | **A** [F1] | `evidence/VWO-007/adv-session-loss-restart/` |
| 23 | adv-duplicate-triggers | two-tab duplicate + 8-way storms + post-restart replays | HYBRID | **passed** (exactly one accept per distinct payload) | **A** [F1], **J** [F2] | `evidence/VWO-007/adv-duplicate-triggers/` |
| 24 | adv-provider-failure | 503/404 under promote; fail-safe zero partial state; retry | INSTRUCT | **passed** (fail-safe + clean retry verified) | **A** [F1] | `evidence/VWO-007/adv-provider-failure/` |
| 25 | adv-cancellation-race | stale edit vs publish; duplicate publish; governed correction | DEMONSTRATE | **passed** (one winner, loser conflict, immutable body) | **A** [F1] | `evidence/VWO-007/adv-cancellation-race/` |

### VWO-008 — security/trust-boundary adversary (6 scenarios, 10/10 attack classes exercised)

| # | Scenario id | Persona / goal | Mode | Outcome | Findings hit | Evidence |
|---|---|---|---|---|---|---|
| 26 | adv-prompt-injection | hostile PR/doc/API content must stay inert data | INSTRUCT | **passed** (all repelled; human path works) | **A** [F1], Email-gap [F2] | `evidence/VWO-008/adv-prompt-injection/` |
| 27 | adv-approval-race | dual-approver concurrent invoice decision | DEMONSTRATE | **passed** (single winner, stale replay repelled) | **A** [F1], **J** [F6] | `evidence/VWO-008/adv-approval-race/` |
| 28 | adv-credential-exfiltration | fake credentials in story body; checker must refuse | HYBRID | **passed** (zero propagation; checker refused fail-closed) | **A** [F1] | `evidence/VWO-008/adv-credential-exfiltration/` |
| 29 | adv-cancellation-race | edit-vs-publish both orderings; duplicate publish; correction | DEMONSTRATE | **passed** at canonical oracles — server-side terminality gap found | **A** [F1], **F** [F3] | `evidence/VWO-008/adv-cancellation-race/` |
| 30 | adv-marketplace-entitlement-race | expired trial, mid-upgrade revocation, metadata exfil | HYBRID | **passed** (gates fail-closed; cross-tenant hole found) | **A** [F1], **E** [F4], **K** [F5], **J** [F6] | `evidence/VWO-008/adv-marketplace-entitlement-race/` |
| 31 | adv-duplicate-triggers | public-intake injection + concurrent screening + replay storm | HYBRID | **passed** (one decision; replays repelled) | **A** [F1], **J** [F6] | `evidence/VWO-008/adv-duplicate-triggers/` |

### VWO-009 — versioning/marketplace/distribution adversary (3 scenarios, 12 attack classes)

| # | Scenario id | Persona / goal | Mode | Outcome | Findings hit | Evidence |
|---|---|---|---|---|---|---|
| 32 | adv-environment-failure-rebinding | registry-down fail-safe; compatible rebind; semantic smuggling | HYBRID | **passed** (invariants held; mutating-payload acceptance + audit gap recorded) | **A** [F1], **L** [F2], **J** [F3] | `evidence/VWO-009/adv-environment-failure-rebinding/` |
| 33 | adv-version-confusion | duplicate-install, stale proposal, moving ref, implicit downgrade | INSTRUCT | **passed with a confirmed hole** (implicit downgrade SUCCEEDED) | **A** [F1], **G** [F4], **M** [F5], Scope-gap [F6] | `evidence/VWO-009/adv-version-confusion/` |
| 34 | adv-marketplace-entitlement-race | revoke mid-upgrade, renewal recovery, cross-tenant, misattribution, private-release leakage | HYBRID | **passed with confirmed holes** (renewal dead-end; cross-tenant pin move; engine fork/private-release holes) | **A** [F1], **H** [F7], **E** [F8], **N** [F9], **I** [F10] | `evidence/VWO-009/adv-marketplace-entitlement-race/` |

**Teaching-mode totals across the program's wave-1/2 evidence:** DEMONSTRATE ×11,
INSTRUCT ×11, HYBRID ×12 — all three modes exercised in every wave-1 real-user WO
and across the adversarial WOs (VALIDATION-PROGRAM.md §6 satisfied at the
scenario-coverage level; the teaching *product guarantee* itself is Family A —
unverifiable).

---

## 2. Canonical finding families (deduplicated)

Local report numbering is per-report; families are the cross-report
deduplication. Per-report severity ratings are preserved; the canonical rating
is the most severe well-evidenced rating (program §10).

### FAMILY A — P0 — workflow/teaching engine mounted in NO user-facing surface

- **Confirming reports (6):** VWO-004 F1 (rated **P0**), VWO-005 F1 (P1), VWO-006 F-1 (P1), VWO-007 F1 (P1), VWO-008 F1 (P1), VWO-009 F1 (P1, self-described "4th independent confirmation" counting the adversarial/engine-side WOs VWO-005/007/008/009; with the two real-user WOs VWO-004/006 the count is **6 merged-report confirmations**).
- **Affected surfaces:** every product surface — CLI (`codex-rs/cli`), TUI (`codex-rs/tui`), app-server + protocol (`codex-rs/app-server*`), exec. The nine engine crates (`workflow-contracts/forge/app/durable/triggers/distribution/evolution`, `teaching-compiler`, `execution-contracts`) are library-only.
- **Phenomenon:** a normal person cannot teach (DEMONSTRATE/INSTRUCT/HYBRID), compile, review, approve, publish, fork, improve, schedule, trigger, observe, recover, upgrade or roll back a workflow through any Codex surface. Every Workflow Identity field (version, definition digest, dependency lock) is unexposed. Fixture-level simulations behave correctly; the engine's own tests are green.
- **Root cause:** implementation WOs (docs/work-orders/WO-009.md, docs/work-orders/WO-010.md, docs/work-orders/WO-011.md, plus MWO-001..003) landed the family library-first; WO-010's required outcome "Expose workflow lifecycle through the existing Codex application/app-server surfaces where appropriate" (docs/work-orders/WO-010.md) never received its follow-up mount. See `ARCHITECT_START_HERE.md` ("Let users create reusable workflows… from the Codex application") and `docs/architecture/CODEX-UNIVERSAL-ARCHITECTURE.md` §13 ("Workflow distribution is a first-class application capability").
- **Owning semantic layer:** codex-rs workflow-family crates + the app-server-protocol/cli mount seam (engine owner + integration owner; RWO-001 proposes the split).
- **Reproduction at 0f89393:** REPRODUCED (static, 7 probes): `docs/validation/evidence/vwo-010/_infra/post-hoc/20260912T104009Z-engine-mount-absence-probe.txt` — zero workflow deps in cli/tui/app-server/app-server-protocol/app-server-daemon/exec Cargo.toml; zero `workflow` mentions in app-server-protocol src; no CLI workflow subcommand; no `[[bin]]` in any engine crate. Engine library tests: NOT re-runnable here (no cargo) — cited from merged reports (VWO-007 durable 46 / app 36 / triggers 23 passing at 0d314fd; VWO-008 contracts 90 + execution-contracts 112 passing at d198a00; VWO-009 contracts 90 / forge 44 / distribution 18 passing at 7736a42).

### FAMILY B — P1 — FlowMart install-configuration form silently drops JSON input

- **Confirming reports (2):** VWO-005 F2 (P1), VWO-006 F-2 (P1; also blocks the rebind human path in scenario 21).
- **Affected surface:** FlowMart `/installs` configure form (the only human path for resource/account bindings post-install).
- **Phenomenon:** the "Config keys to merge (JSON)" textarea POSTs urlencoded; `configureInstall` receives a STRING, requires `typeof object`, silently substitutes `{}` — success flash "configured (no keys)", unchanged `{}` config cell, misleading `configured` history/event entry.
- **Root cause:** `docs/validation/fixtures/_lib/runtime.js` parses urlencoded bodies into string values (~line 267–272); `docs/validation/fixtures/marketplace/ops-install.js` `configureInstall` line 107 (`typeof b.config === 'object'` → `{}` fallback, no parse, no error).
- **Owning semantic layer:** VWO-002/VWO-003 fixture harness (shared runtime + FlowMart ops).
- **Reproduction at 0f89393:** REPRODUCED (browser human path + API-twin contrast): `docs/validation/evidence/vwo-010/marketplace-discover-install-configure/user-path/20260912T103408Z-step-configure-form-silent-noop-browser-path.txt` and `.../post-hoc/20260912T103408Z-familyB-api-twin-contrast-and-root-cause.txt`.

### FAMILY C — P1 — fork surface + lineage data model absent from the product

- **Confirming reports:** VWO-005 F4 (P1); VWO-009 F6 partially (no product fork surface, P3, fixture-scope).
- **Affected surface:** the marketplace lifecycle (fork an installed/published workflow → new immutable release carrying lineage). Engine implements fork/lineage (`workflow-forge`, `workflow-distribution/src/memory.rs` fork path); no product route or lineage data model exposes it (FlowMart route inventory has no fork path).
- **Root cause:** same library-first landing as Family A — the forge/distribution fork port was never mounted; FlowMart (fixture scope) models versions only, no lineage records.
- **Owning semantic layer:** engine mount (RWO-001 seam) + forge/distribution fork port exposure (RWO-005).
- **Reproduction at 0f89393:** REPRODUCED by surface-absence: `docs/validation/evidence/vwo-010/_infra/post-hoc/20260912T104048Z-fork-improvement-trigger-surface-absence.txt` (FlowMart route inventory + engine fork code present-but-unmounted).

### FAMILY D — P1 — improvement-candidate-from-evidence + approval-before-publish absent

- **Confirming reports:** VWO-005 F5 (P1).
- **Affected surface:** the governed-improvement lifecycle (execution evidence → improvement candidate → validation → approval gate → publish). `codex-workflow-evolution` implements candidate/approval; it is library-only with **zero dependents**; no product surface records execution evidence or proposes candidates.
- **Root cause:** library-first landing (as Family A); evolution crate never mounted.
- **Owning semantic layer:** workflow-evolution crate port exposure via the RWO-001 mount seam (RWO-006).
- **Reproduction at 0f89393:** REPRODUCED by surface-absence: same probe file as Family C (probe 3: no crate depends on `codex-workflow-evolution`).

### FAMILY E — P1 — cross-tenant install writes accepted server-side (including pin moves)

- **Confirming reports (3):** VWO-005 F6 (P1), VWO-008 F4 (P1), VWO-009 F8 (P1; extends to pin moves: another org's admin moved a production pin).
- **Affected surface:** FlowMart `configureInstall` / `upgradeInstall` / `rollbackInstall` write path.
- **Phenomenon:** an org_admin of org X can configure, upgrade and roll back org Y's install (all writes `ok:true`, actor recorded); the read path IS org-scoped, so the UI gives no hint.
- **Root cause:** `ops-install.js` resolves installs by id only (`find(ctx.state.installs, b.installId)`) and never compares the actor's org with `inst.orgId` (lines 100/117/156).
- **Owning semantic layer:** FlowMart fixture ops (harness owner).
- **Reproduction at 0f89393:** REPRODUCED (API twins as petra.voss/Acme against Northwind's ins-0402): `docs/validation/evidence/vwo-010/marketplace-upgrade-rollback/post-hoc/20260912T103445Z-familyE-cross-tenant-install-writes.txt`.

### FAMILY F — P1 — PressRoom publication terminality enforced by the UI only

- **Confirming report:** VWO-008 F3 (P1).
- **Affected surface:** PressRoom `editStory` / `publishStory` server ops.
- **Phenomenon:** (1) an authorized author can mutate a PUBLISHED story body via the API twin — `editStory` has no story-status guard; (2) `publishStory` never calls `checkVersion` — post-approval author edits go live un-reviewed (publish carries no `expectedVersion`).
- **Root cause:** `docs/validation/fixtures/media/ops.js` `editStory` (lines 46–74: author/editor permission check only, no status gate) and `publishStory` (lines 159–215: no `checkVersion` call). The browser UI removes the edit form on published stories — the guard is UI-only.
- **Owning semantic layer:** PressRoom fixture ops (harness owner).
- **Reproduction at 0f89393:** REPRODUCED (both attacks + UI oracle): `docs/validation/evidence/vwo-010/media-editorial-publishing/post-hoc/20260912T103919Z-vwo008-f3-publication-terminality.txt`.

### FAMILY G — P1 — implicit downgrade accepted through the upgrade operation (product AND engine)

- **Confirming report:** VWO-009 F4 (P1; the acceptance attack "downgrade a pinned instance implicitly" SUCCEEDED on both layers).
- **Affected surfaces:** FlowMart `upgradeInstall` (product) and `codex-workflow-distribution` `decide_upgrade`/follow path (engine).
- **Phenomenon:** the op labeled "upgrade" accepts a target OLDER than the pin with no semver-ordering or explicit-downgrade guard: pin 1.3.0 → 1.2.0 recorded as "upgraded" (product, reproduced); engine probe applied 2.0.0 → 1.5.0 via an upgrade record.
- **Root cause:** product — `ops-install.js` `upgradeInstall` (lines 117–153) checks existence/equality only; engine — `memory.rs` `decide_upgrade` (lines 464–530) verifies `proposal.from == installed` but never that `proposal.to` is newer than `from`.
- **Owning semantic layers:** FlowMart fixture ops (harness owner) + workflow-distribution crate (engine owner).
- **Reproduction at 0f89393:** product half REPRODUCED (`docs/validation/evidence/vwo-010/marketplace-upgrade-rollback/post-hoc/20260912T103521Z-vwo009-versioning-entitlement-holes.txt` section A2); engine half verified STATICALLY (same file, conclusion note) — dynamic engine re-run pending a cargo-capable environment (cited: VWO-009 engine probe HOLE verdict at 7736a42).

### FAMILY H — P1 — entitlement revocation permanently bricks org+package operations (renewal cannot restore)

- **Confirming report:** VWO-009 F7 (P1).
- **Affected surface:** FlowMart entitlement resolution (`entitlementFor` / `checkEntitlement`).
- **Phenomenon:** after revoke (ent-0501) + fresh grant (ent-0504, active, valid 2027), every install/upgrade/rollback/configure for org+package is still blocked with `stale_entitlement` naming the OLD revoked record. The commercial recovery path is dead-ended.
- **Root cause:** `ops-install.js` line 11–13 — `entitlementFor` is `find()` (first match) over a growing list; a revoked first record shadows later active grants; revocation is terminal in the data model (no renew/un-revoke op).
- **Owning semantic layer:** FlowMart fixture ops (harness owner).
- **Reproduction at 0f89393:** REPRODUCED (clean battery: revoke → grant → retry still blocked): `.../post-hoc/20260912T103521Z-vwo009-versioning-entitlement-holes.txt` section C-restart.

### FAMILY I — P1 — engine upgrade path skips the visibility gate (private-release leakage)

- **Confirming report:** VWO-009 F10 (P1; WO attack 6 SUCCEEDED at the engine layer).
- **Affected surface:** `codex-workflow-distribution/src/memory.rs` `evaluate_upgrade` (lines 435–462) and `decide_upgrade` (464–530) / `run_gates` (161–183).
- **Phenomenon:** `evaluate_upgrade` surfaces a PRIVATE release's identity to a non-owner installer (no state/audience filter over `self.entries`) and `decide_upgrade` approves and applies it — while `install` correctly enforces `release_installable` → `ReleaseNotVisible`. The documented gate order visibility → integrity → access → entitlement is skipped on the upgrade path.
- **Root cause:** the upgrade evaluation/approval path never consults the visibility gate that search/install enforce.
- **Owning semantic layer:** workflow-distribution crate (engine owner).
- **Reproduction at 0f89393:** verified STATICALLY (code path: `evaluate_upgrade` has no scope filter; `decide_upgrade`/`run_gates` never call `release_installable`) — `docs/validation/evidence/vwo-010/_infra/post-hoc/20260912T104009Z-engine-mount-absence-probe.txt` context + VWO-009 engine probe 6d HOLE verdict cited; dynamic re-run pending a cargo-capable environment.

### Canonical P2/P3 families (dispositioned in §3)

- **J (P2):** repelled/blocked operations leave NO audit events — VWO-007 F2, VWO-008 F6, VWO-009 F3 (three confirmations).
- **K (P2):** evidence-checker allow-list is line-scoped — VWO-008 F5.
- **L (P2):** configure accepts identity-impersonating / semantic-shadowing keys — VWO-009 F2 (P2) + VWO-006 F-6 (P3; same defect observed earlier as namespace friction).
- **M (P2):** rollback direction confusion after a downgrade — VWO-009 F5.
- **N (P2):** fork attribution content unverified (engine) — VWO-009 F9.
- **Policy (P2):** post-publication commercial-policy/attribution/licensing configuration absent — VWO-005 F3.
- **Consumer (P2):** pre-install capability/resource/compatibility contract absent — VWO-006 F-3; schedule/webhook/execution-monitoring/uninstall surfaces absent — VWO-006 F-4.
- **Drift (P2/P3, harness-internal):** VWO-004 F2 (seed lacks second OPEN purchase request w/ valid vendor), F3 (ticket:resolve granted to ops, scenario expects support agent), F4 (catalog lists missing_asset on rendition; fixture implements data_conflict), F6 (catalog fast channels web+push; no push channel in fixture), F5 (P3 follow-up-action note realized as close-with-resolution-note); VWO-005 F7 (P3 approved-story attach dead-end); VWO-006 F-5 (P3 install form carries no binding fields).
- **Email-gap (P3):** no email-receiving surface exists — VWO-008 F2 (attack-class coverage gap, nearest analogues exercised).
- **Scope-gap (P3):** no product surface for dev-branch refs, dependency locks, listing-scope transitions — VWO-009 F6 (engine coverage exists for attacks 2/3/10; product halves unexercisable).

---

## 3. P2/P3 disposition table (no orphans)

Fix-now = a proposed RWO file exists and closes it. Follow-up = a named,
scoped work order to be dispatched later. Accepted limitation = recorded with
reason; not scheduled.

| Finding (report local id) | Family | Severity | Disposition | Owner of the fix |
|---|---|---|---|---|
| VWO-004 F2 seed/catalog procurement mismatch | Drift | P2 | **fix-now — RWO-010** (align seed: second OPEN purchase request with valid vendor, or rebind catalog step 4) | harness maintainer |
| VWO-004 F3 ticket:resolve on ops vs catalog's support agent | Drift | P2 | **fix-now — RWO-010** (grant ticket:resolve to support_agent, or rebind catalog step 4 to the ops role) | harness maintainer |
| VWO-004 F4 rendition missing_asset switch unimplemented | Drift | P2 | **fix-now — RWO-010** (implement missing_asset branch on rendition request, preferred — it is the catalogued user-meaningful failure) | harness maintainer |
| VWO-004 F6 no push fast channel in PressRoom | Drift | P2 | **fix-now — RWO-010** (add a push-like fast channel, or rebind catalog fast-channel wording to web-front-page + rss-feed) | harness maintainer |
| VWO-004 F5 follow-up-action note surface mismatch | Drift | P3 | **fix-now — RWO-010** (catalog wording alignment) | harness maintainer |
| VWO-005 F7 approved-story attach dead-end | Drift | P3 | **fix-now — RWO-010** (allow hero re-attach while approved, or annotate the catalog path) | harness maintainer |
| VWO-006 F-5 install form carries no binding fields | Drift | P3 | **follow-up — RWO-015** (consumer install-form bindings ride with the consumer execution surfaces) | harness maintainer |
| VWO-005 F3 commercial-policy config absent | Policy | P2 | **follow-up — RWO-013** (FlowMart commercial-policy/policy-declaration surface; or explicit "policy lives in entitlements" statement per VWO-005's recommendation) | harness maintainer |
| VWO-006 F-3 no capability/compatibility contract before install | Consumer | P2 | **follow-up — RWO-014** (pre-install machine-readable contract rendering; depends on RWO-001 exposing the engine's capability/resource model) | engine mount owner |
| VWO-006 F-4 schedule/webhook/monitoring/uninstall surfaces absent | Consumer | P2 | **follow-up — RWO-015** (consumer execution surfaces: triggers mount phase; depends on RWO-001) | engine mount owner |
| VWO-007 F2 + VWO-008 F6 + VWO-009 F3 rejections invisible to audit trail | J | P2 | **fix-now — RWO-011** (emit rejection/conflict audit events in the shared fixture runtime dedup/conflict path; 3 independent confirmations) | harness maintainer |
| VWO-008 F5 evidence-checker line-scoped exemption | K | P2 | **fix-now — RWO-012** (substring-scoped exemption markers in capture-evidence.sh) | harness owner (VWO-003 lineage) |
| VWO-009 F2 + VWO-006 F-6 configure accepts identity-impersonating/shadowing keys | L | P2/P3 | **fix-now — RWO-002** (bundled: while fixing the JSON parse, add the reserved-key deny-list to the same op — same file, same owner, same verification sweep) | harness maintainer |
| VWO-009 F5 rollback direction confusion | M | P2 | **fix-now — RWO-007** (bundled with the Family G product fix: ordering guard + derive rollback target from the upgrade's fromVersion — same op, same file) | harness maintainer |
| VWO-009 F9 fork attribution content unverified (engine) | N | P2 | **follow-up — RWO-016** (signed/derived attribution content in forge/distribution seal path; misattributed fork must fail verification) | forge engine owner |
| VWO-008 F2 email-receiving surface missing (attack vector) | Email-gap | P3 | **accepted limitation** — no fixture models an email inbox; nearest analogues (notifications, public web intake, PR/doc/API content) were exercised and repelled; engine-side injection guarantees proven. Revisit in the north-star wave (VWO-012/013) if an email-capable surface lands. | — (documented) |
| VWO-009 F6 no product surface for dev-branch refs / dependency locks / scope transitions | Scope-gap | P3 | **follow-up — RWO-017** (DEFER status: fixture roadmap after the engine mount; engine coverage for attacks 2/3/10 exists and was REPELLED/HELD) | harness maintainer |

Named follow-up work orders (scoped here; files to be created when dispatched):
**RWO-013** FlowMart commercial-policy configuration surface (VWO-005 F3);
**RWO-014** consumer pre-install compatibility contract rendering (VWO-006 F-3);
**RWO-015** consumer execution surfaces — schedule/webhook/monitor/uninstall + install-form bindings (VWO-006 F-4/F-5);
**RWO-016** engine fork attribution verification (VWO-009 F9);
**RWO-017** fixture branch/lock/scope-transition surfaces, DEFERRED (VWO-009 F6).

---

## 4. Deviations from the dispatch packet's FINDINGS INDEX

The clone wins; the reports win over the index where they disagree. Recorded
honestly:

1. **VWO-004 finding count:** the index listed five findings (F1–F5); the
   report carries six — **F6 (P2)** (catalog fast channels "web+push"; PressRoom
   has no push channel) was omitted from the index. Included here as Drift family.
2. **VWO-006 finding count:** the index listed two findings (F-1 teaching
   absent; "F-2 rebind human path blocked by fixture defect"). The report carries
   **six**: F-2 is the FlowMart configure silent no-op (same defect as VWO-005 F2
   — Family B; the "rebind human path" blockage is its consequence, not a
   separate finding), plus **F-3 (P2)** compatibility contract, **F-4 (P2)**
   schedule/webhook/monitoring/uninstall surfaces, **F-5 (P3)** install-form
   bindings, **F-6 (P3)** binding-key namespace — all omitted from the index.
3. **VWO-007 finding count:** the index omitted **F2 (P2)** — dedup rejections
   invisible to the event audit trail (confirmed twice more by VWO-008 F6 and
   VWO-009 F3).
4. **VWO-008 finding count:** the index listed only F1 (plus an injection-rendering
   friction observation). The report carries six findings: F1 (P1, engine
   unmounted), **F2 (P3)** email-surface gap, **F3 (P1)** publication terminality
   UI-only, **F4 (P1)** cross-tenant install writes, **F5 (P2)** evidence-checker
   line-scoping, **F6 (P2)** approval-race audit invisibility. F3/F4 are P1s and
   produce canonical Families F and E.
5. **VWO-008 "10/10 REPELLED":** the report's own attack-class table records
   class 4 as "REPELLED — except F4 cross-tenant install writes (P1)" and class 8
   as "REPELLED at the canonical oracles — F3 publication-terminality gap (P1)".
   The accurate summary: every attack was repelled at the surfaces the catalog
   binds, with two P1 server-side write-path holes discovered during the attacks.
6. **VWO-009 data (per instruction, read from the merged report):** base 7736a42,
   3 scenarios, 12 attack classes — 7 REPELLED both-layers, 2 HELD engine, 3 HOLES
   (F4/F9/F10); findings P0×0, P1×5 (F1/F4/F7/F8/F10), P2×4 (F2/F5/F9/F3), P3×1
   (F6); 56 evidence artifacts; F1 is VWO-009's "4th independent confirmation"
   counting engine-side WOs only. The packet's dedup seed did not pre-list
   VWO-009's unique P1s (F4/F7/F10) — they are canonical Families G/H/I here.
7. **Base SHAs:** VWO-004/005/006/007 base 0d314fd (matches index); VWO-008 base
   d198a00 (matches); VWO-009 base 7736a42 (the index ended at VWO-008, as
   stated). All six merged before 0f89393 — verified by the merge history
   (`git log --oneline` at base: PR #30/#31 heads).

Non-finding observations carried into the final report's UX-friction section:
VWO-007 recovery-surface "no explicit resumed indicator" (observation, not a
finding); VWO-008 injection payload renders inline with no untrusted-content
marker (friction observation).

---

## 5. Matrix consistency checks (machine-verified)

- Scenario rows: 34 (12 + 6 + 3 + 4 + 6 + 3) — one per `## Scenario` block
  across the six reports.
- Evidence artifacts: 161 + 62 + 36 + 75 + 62 + 56 = **452** (git ls-files at
  base 0f89393).
- Every P0/P1 local finding maps to exactly one canonical family: 17 local
  P0/P1 findings → 9 canonical families (A–I).
- Every P2/P3 local finding appears exactly once in §3 (14 P2 + 6 P3 = 20 rows;
  two rows consolidate multi-report confirmations J and L, hence 18 table rows
  covering 20 local findings).
- No scenario row cites a finding absent from its source report; no family
  cites a report that does not carry it.
