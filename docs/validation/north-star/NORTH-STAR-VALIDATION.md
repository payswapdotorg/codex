# Codex Universal — North-Star Validation (Final Gate Report)

**Work Order:** VWO-017 — Universal Computer Automation North-Star Gate
**Status:** COMPLETE
**Verdict:** `NORTH_STAR_PARTIALLY_SUPPORTED`
**Evidence base (exact):** repository `payswapdotorg/codex`, main @
`9f5eaffff` (all evidence cited below exists at this SHA; every merged
VWO/RWO/APP report and its evidence tree is reachable from it).
**Benchmark version:** 1.0.0 (BENCHMARK-CATALOG.md, unchanged since the
wave-4 runs — no bump between the runs and this synthesis).
**Date:** 2026-09-15
**Depends on:** VWO-010 (merged — the wave-0..3 synthesis this builds on) ·
VWO-011..015 (merged — benchmark + all four scripted/human-authored lanes)
· RWO-001..012 (merged — remediation) · APP-001..004 (merged — product
delivery) · **VWO-016: NOT EXECUTED — operator-gated** (see §3 and
`generalization-results.md`; the executor is barred from self-authoring
held-out briefs by BENCHMARK-CATALOG §7 anti-cheating rules).

---

## 0. The north star under test

> Codex Universal should be capable of automating arbitrary computer-based
> work that a human can perform, subject to the capabilities,
> authorization, resources, interfaces and environment actually available.

Per the work order and SCORING §9, a finite benchmark cannot prove an
infinite task space. This report determines whether the implemented
architecture and the observed validation evidence justify calling the
system a **general-purpose computer-work automation platform** — and
separates what is demonstrated from what is not.

---

## 1. The eight required verdict classes

| # | Class | Verdict | Basis (all pointers at `9f5eaffff`) |
|---|---|---|---|
| 1 | **Semantic universality** — one workflow model spans the tested environments | **DEMONSTRATED (scripted + human-authored tiers)** | One `WorkflowControlPlane` served every lane: VWO-012 desktop GUI (10/10), VWO-013 browser (10/10), VWO-014 terminal/files (10/10), VWO-015 cross-application (8/8) — 38/38 task runs through the same teach→compile→review→approve→publish→run→settle lifecycle, one durable state root (`~/.codex/workflow/`), one IR (workflow-contracts), zero benchmark-specific engines (rule 11: none created — verified by the RWO-001 mount diff and every lane's harness). |
| 2 | **Interaction universality** — computer actions beyond browser automation | **DEMONSTRATED** | Native desktop GUI (Xvfb + xdotool + GUI-mode chromium: D001–D010 incl. dialogs, keyboard-only, mouse, clipboard, file pickers, document/spreadsheet editing, crash recovery), terminal/shell (T001–T010 incl. process inspection, SIGSTOP wedge + recovery, git ops, archives, CLI auth), filesystem documents, browser beyond the five wave-1 fixtures (purpose-built VendorGate/TeamPad/MarketplaceDigests/ForgeExport + multi-tab, uploads/downloads, rich editors, session renewal). |
| 3 | **Environment universality** — workflows mix environment modalities | **DEMONSTRATED** | NST-X lane 8/8: ≥3-modality tasks (browser+GUI+terminal+API), two concurrent authenticated browser sessions on different origins (X005), concurrent dual-surge fanout with millisecond overlap + approval precedence (X006), long-running resumable across a kill -9 (X004), cross-app join via API twins (X005). |
| 4 | **Capability/resource universality** — discover/bind without provider-semantics leakage | **DEMONSTRATED (task level)** | Every lane's teach pipeline inferred capabilities and proposed bindings with `requiresApproval` semantics; T009 exercised the CLI credential-binding path end-to-end (credential material never in evidence — redacted); VWO-008 engine parity suites (execution-contracts 112/0, workflow-contracts 90/0 incl. forged-grant/foreign-binding/no-credential-material); env-adapters/resource-providers exercised through the product path across lanes (A12 transition recorded in `coverage-matrix.md`). |
| 5 | **Teaching universality** — DEMONSTRATE / INSTRUCT / HYBRID teach novel workflows | **DEMONSTRATED (scripted + human-authored tiers)** | All three modes covered in every lane (lane mode tables in VWO-012..015 reports); 9 human-authored tasks (goal text only, no action script — G2 provenance: D002, D007, D009, B002, B008, B010, T004, T007, X007) all PASS — a human-stated goal yielded a working workflow through the product with no script. |
| 6 | **Execution reliability** — success, recovery, restart, idempotency | **DEMONSTRATED** | Adversarial batteries (VWO-007: SIGKILL/restart/idempotency replay, session loss, duplicate triggers, cancellation race, provider failure — all REPELLED at engine level); task-level recovery: armed-503 + retry (X001, T010, B007), kill -9 mid-run → checkpoint resume → exactly-one summary (X004), stale-checkpoint supersede (T010), mid-form logout → renewal → re-file exactly-once (B006), close/relaunch durability (APP-003 step 8-9, fresh-machine); rejection/conflict audit events on every 409/403/503 (RWO-011). |
| 7 | **Generalization** — performance on held-out human goals | **NOT EXERCISED — OPERATOR-GATED** | VWO-016 (the NST-H held-out lane) did not execute: the catalog §7 anti-cheating procedure requires ≥3 independent human authors (incl. one unfamiliar with workflow contracts), a sealed ≥12-task pool, stratified sampling, SHA-256 commitment ledger — and **the executor is barred from authoring the held-out briefs**. No such briefs were delivered to the executor within the program window. See `generalization-results.md` for the honest G3 record. |
| 8 | **Product usability** — ordinary users without architectural knowledge | **DEMONSTRATED** | APP-003 fresh-machine run 22/22 (`docs/product-delivery/evidence/app-003/run-20260915T185129Z/`): a user with a sanitized PATH (no cargo/rust), isolated HOME, no source checkout downloaded the release from GitHub, verified the published sha256, installed via the documented `install.sh`, launched, taught + published a workflow from natural-language instructions, ran it to terminal `completed`, closed/relaunched with durable state, received an actionable error on unusable model config (workflow surface unaffected), and uninstalled cleanly per the documented procedure. Release `rust-v0.1.0` (34 assets) is live and checksum-verified. |

**Class-7 aggregate:** 7 of 8 classes demonstrated at task level through the
product path; the 8th (held-out generalization) is unexercised by
operator gating — not by a product or architecture failure.

---

## 2. Required claim-boundary separation (README "Universal claim boundary")

### 2.1 Directly demonstrated

- **Substrate**: Xvfb/xdotool/GUI chromium, File API, fixture runtime
  (VWO-001; 59/59 fixture sweep, VWO-002/003 and re-verified in VWO-005/006).
- **Five-app enterprise ecosystem, human-path business behavior**: 34
  scenario runs, VWO-004..009 (22 teaching-layer blocks at the time — all
  since unblocked by RWO-001 and re-proven at product level in wave-4).
- **Engine libraries under adversarial load**: contracts/forge/app/durable/
  triggers/distribution/execution-contracts cargo suites + the VWO-007/008/009
  attack batteries (tamper, forged grants, races, replay — repelled; the
  3 wave-era distribution HOLES closed by RWO-007/009 and re-proven).
- **Product path, all four scripted lanes**: 38/38 PASS (D 10/10 · B 10/10 ·
  T 10/10 · X 8/8) at benchmark v1.0.0 through the mounted CLI/app-server
  surfaces (RWO-001), including the 9 human-authored G2 tasks.
- **Recovery/adaptation at task level**: every recovery/adaptation criterion
  goal-critical-passed with the stressor evidenced as actually exercised
  (armed 503s, kill -9, stale checkpoints, mid-form session loss).
- **Human-in-the-loop at product level**: approval-before-publish ordering
  (X001), editor review + approval + deploy notification (X002), digest-gated
  install with tampered-control branch (X003), GUI review window (X001).
- **Product delivery**: downloadable/launchable/verifiable/installable/
  uninstallable release with user documentation (APP-001..004; verdict
  `DOWNLOADABLE_AND_LAUNCHABLE`).

### 2.2 Strongly generalized from held-out tasks

**NONE — the class is unexercised.** No held-out brief was authored by an
independent human within the program window; per catalog §7 the executor
cannot author them. No claim here is available, and none is made. (The G2
human-authored tier — goals without scripts — is recorded under 2.1, one
provenance tier below held-out.)

### 2.3 Remains untested

- The entire held-out (NST-H) class: unseen-goal success/recovery/
  adaptation rates by teaching mode, changed-state re-execution on unseen
  goals, human-intervention reasons on unseen goals (VWO-016 procedure
  defined and sealed-ready; zero tasks enumerated by design).
- MCP modality (A16/E11 — no MCP server surface exists in the environment;
  revisit trigger recorded).
- E2B/Composio connected proving ground (E13 — credentials never available;
  the user-space fallback profile was validated instead and its 7 fidelity
  substitutions recorded, VWO-012).
- macOS/Windows platforms (the release builds Linux musl x86_64/aarch64
  only — org signing environments unavailable; APP-002 §2).
- Real-model agent-surface operation (sandbox egress is provider-
  region-blocked; the agent surface was validated for actionable error
  behavior and the workflow/teaching surface — which needs no model
  credentials — was validated fully; a model-connected run of the agent
  surface remains untested here).

### 2.4 Impossible without unavailable capabilities (environment class)

- MCP tasks: no `.mcp.json`/MCP server exists in the sandbox (VWO-001 §1).
- E2B/Composio: integration credentials do not exist and may not be
  invented (program §3 fallback; VWO-001 §2).
- Provider model access: egress region blocked (403 "Country, region, or
  territory not supported") — affects only the model-backed agent surface,
  not the deterministic teaching/workflow surfaces.

### 2.5 Blocked by product limitations rather than architecture

- **F-B1 (missing-capability, fixture class)**: ForgeOps exposes no
  comment/release-thread surface — hit by NST-B003's comment step (vacuous
  sweep executed honestly) and NST-B010's post-back step (BLOCKED sub-step
  recorded). Fixture-scope remediation candidate; no engine involvement.
- **11 recorded environment drifts** (all environment-specific-limitation,
  none engine): D-B1 catalog/fixture listing drift; D-B6 persona
  capability drift (executed as an authorized persona, recorded); D-B7
  redirect-loop component absent; D-B9 refund-guard listing absent; D-T4
  route-shape drift; D-T8 no-seeded-typo premise (structural execution);
  D-T10 + D-X1 reads-exempt failure switch arms; D-X2 notification persona
  routing; D-X7 metadata-only media assets; D-T4/D-X7 fixture data shape.
  Each feeds the harness-alignment/remediation loop per catalog §5 note 3.

### 2.6 Product defects (found, none engine-semantic)

- Zero `workflow-semantic-defect` findings across all 38 lane runs (the
  5-way failure accounting in each lane report).
- Fixture/catalog drift set (§2.5) + F-B1 — all fixture-scope.
- Product friction (first-class benchmark output, recorded, non-blocking):
  `codex exec` reads piped stdin by design (scripts must close stdin —
  documented in USER-GUIDE §10); `install.sh`'s single api.github.com
  metadata call is sensitive to shared-IP rate limits (manual path
  documented); the `workflow` group is flagged `[experimental]` in CLI help.

### 2.7 Architecture defects

- None open from the executed lanes. The wave-0..3 finding families A–M
  (SCENARIO-ISSUE-MATRIX, VWO-010) were all remediated by RWO-001..012 and
  re-proven (mount, engine hardening, distribution semantics, evidence
  checker, rejection audits), and the wave-4 lanes surfaced no new
  architecture-family findings.
- Architectural areas where evidence is silent (not defects — untested):
  held-out generalization (class 7), MCP integration (out-of-scope v1).

---

## 3. Why the verdict is `NORTH_STAR_PARTIALLY_SUPPORTED`

The work order's gate definitions:

- `NORTH_STAR_SUPPORTED` requires "held-out generalization **and** no
  critical blockers". Class 7 is unexercised (operator-gated, §1) — the
  SUPPORTED bar is therefore not met, regardless of the strength of the
  scripted-tier evidence (SCORING §9: no universality claim may be derived
  from scripted or human-authored passes).
- `NORTH_STAR_NOT_SUPPORTED` requires "critical modality, workflow,
  recovery, or generalization gaps". No such gap was found: all interaction
  modalities present in the environment were exercised and passed (classes
  1–6, 8), recovery is proven at engine and task level, and the one open
  generalization gap is a *missing execution* of a ready procedure, not an
  observed failure.
- Therefore: **broad automation works (demonstrated across four lanes, all
  teaching modes, human-authored goals, adversarial recovery, and a
  delivered product), but material generality remains unproven (the
  held-out class never ran; MCP and non-Linux platforms untested)** — the
  definition of `NORTH_STAR_PARTIALLY_SUPPORTED`.

The verdict follows evidence, not roadmap intent (work-order "Final gate"
discipline). The single decisive missing artifact is the VWO-016 execution:
the procedure is defined, sealed, and ready (BENCHMARK-CATALOG §7); what is
missing is ≥3 independent human authors' briefs — a gate only the operator
can open.

---

## 4. Coverage reconciliation

`coverage-matrix.md` (this directory, lowercase — the reconciled ledger)
transitions every row the executed evidence covers from
`benchmark-pending` to `validated-with-evidence` at `9f5eaffff`, and keeps
held-out/MCP/E2B/platform rows explicitly pending or out-of-scope with
reasons. Per SCORING §5-4, only PASS transitions rows — all 38 lane task
runs were PASS, so every row their evidence covers transitions.

## 5. Generalization record

`generalization-results.md` records the three provenance tiers separately:
G1 scripted (demonstrated), G2 human-authored (demonstrated, 9/9), G3
held-out (not exercised — operator-gated; per-mode/changed-state/
intervention statistics unavailable and never fabricated).

## 6. How to close the gap to `NORTH_STAR_SUPPORTED`

1. Operator delivers ≥10 held-out briefs per the catalog §7 procedure
   (≥12 authored pool, ≥3 independent authors incl. one workflow-contracts
   outsider, stratified sampling grid, SHA-256 commitment ledger).
2. VWO-016 executes them through the product path (all three teaching
   modes across the pool; changed-state re-runs; human-intervention
   recording; environment evidence only — never model text).
3. VWO-017 reconciles: if held-out results demonstrate
   success/recovery/adaptation consistent with the scripted tiers and no
   new architecture family opens, classes 1–8 are all demonstrated and the
   verdict escalates to `NORTH_STAR_SUPPORTED` (subject to the remaining
   out-of-scope modalities staying explicitly bounded).

## 7. Evidence set (exact, at `9f5eaffff`)

- Merged lane reports: `docs/validation/reports/VWO-001-report.md` … `VWO-009-report.md`, `VWO-012-report.md` … `VWO-015-report.md` (the wave-0..3 lane reports); VWO-010 = `SCENARIO-ISSUE-MATRIX.md` (the wave-0..3 synthesis); VWO-011 = `docs/development-state/reports/VWO-011-report.md` (the benchmark construction report)
- Wave-4 evidence trees: `docs/validation/evidence/vwo-01{2,3,4,5}/`
- Prior-wave evidence trees: `docs/validation/evidence/VWO-00{4..9}/`, `vwo-010/`
- Benchmark framework: `docs/validation/north-star/{TASK-TAXONOMY,BENCHMARK-CATALOG,SCORING,COVERAGE-MATRIX}.md` (v1.0.0)
- Wave-0..3 synthesis: `docs/validation/reports/SCENARIO-ISSUE-MATRIX.md`
- Remediation records: `docs/development-state/reports/RWO-001-report.md` … `RWO-012-report.md` (all twelve) + docs(state) merge commits (PRs #34–#45)
- Product delivery: `docs/product-delivery/` (reports, evidence, USER-GUIDE; release `rust-v0.1.0` on GitHub, 34 assets, sha256-verified)
- Merge records: `docs/validation/evidence/_wave4/merge-records.txt`, `docs/product-delivery/evidence/_app-program/merge-records.txt`

**Explicit verdict: `NORTH_STAR_PARTIALLY_SUPPORTED`.**
