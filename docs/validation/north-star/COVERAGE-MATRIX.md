# VWO-011 — North-Star Coverage Matrices

**Work Order:** VWO-011 (Wave 4)
**Status:** CANONICAL — the running state-of-coverage ledger against
`BENCHMARK-CATALOG.md` v1.0.0. VWO-012 … VWO-016 update rows as their runs
land; VWO-017 reconciles these matrices into its final verdict artifacts.
**Base state:** everything below reflects repository state at base SHA
`2d37cc0ef7d075e877f370790f82ac9c70df26a4` (waves 0–3 merged: VWO-001..010;
remediation RWO-001..012 PROPOSED, in-flight, **NOT merged at this base — no
finding is marked fixed here**).
**Governing sources:** `docs/validation/north-star/README.md` (coverage
dimensions: semantic / interaction / environment / capability-resource /
teaching / reliability / generalization / usability) · VWO-011 packet (four
SEPARATE matrices required) · `SCENARIO-ISSUE-MATRIX.md` (the VWO-010
synthesis this ledger builds on).

---

## 0. How to read this ledger (status vocabulary, normative)

| Status | Meaning | Row requirements |
|---|---|---|
| **validated-with-evidence** | Exercised through the cited surface with committed, verifiable evidence at a known SHA. | MUST cite a report + scenario ID and/or an evidence path that exists at the cited SHA. Engine/library claims cite the merged reports' cargo transcripts (this sandbox has no cargo — static verification only here, per the VWO-010 honest-limitation precedent). |
| **benchmark-pending** | Not yet exercised at task level through the product; owned by a benchmark lane (NST-*) whose executing WO will exercise it. | MUST name the owning NST lane + executing WO. |
| **explicitly-out-of-scope-v1** | Deliberately excluded from benchmark v1.0.0 with a recorded reason and a revisit condition. | MUST state reason + revisit trigger. |

Every row carries **what remains untested** — the acceptance question a
reviewer must be able to answer (VWO-011 acceptance: "a reviewer can inspect
the catalog and determine exactly what portions of the computer-task space
have been validated, what remains untested, and how new task families will
be added").

**A note on "validated" in the wave-1/2 evidence:** the 34 scenario runs
validated *fixture business behavior* driven through browser/terminal human
paths, with teaching/product-layer steps BLOCKED by Family A (engine mounted
in no user-facing surface). Rows below state precisely which layer the
evidence covers. No row claims the teaching product guarantee.

---

## 1. ARCHITECTURE coverage matrix

Semantic planes/crates of `docs/architecture/CODEX-UNIVERSAL-ARCHITECTURE.md`
§2–§3, exercised vs not, at base `2d37cc0`.

| # | Plane / crate | Status | Evidence pointer (exists at base) | Owning benchmark lane | What remains untested |
|---|---|---|---|---|---|
| A1 | **Workflow Platform — `codex-workflow-contracts`** (semantic contracts) | validated-with-evidence (library: cargo test) | VWO-008-report.md (engine parity: `cargo test -p codex-workflow-contracts --lib` → 90 passed / 0 failed at d198a00, incl. forged-grant/foreign-binding/pre-READY/unbound-dispatch suites) + VWO-009-report.md (same 90/0 at 7736a42, dependency-lock tamper REPELLED) | product-path exercise: all NST lanes (via RWO-001 mount) | Contract enforcement through a USER-facing surface (nothing mounts it — Family A); contract behavior under GUI/terminal/file task shapes |
| A2 | **Workflow Platform — `codex-workflow-forge`** (authoring/compile/fork) | validated-with-evidence (library: cargo test) | VWO-009-report.md (`-p codex-workflow-forge --lib` → 44 passed / 0 failed at 7736a42) | product fork surface: NST-B/X marketplace tasks (RWO-005) | Teaching/compile path from any user surface (Family A); fork/lineage exposed in product (Family C — RWO-005 proposed, not merged) |
| A3 | **Workflow Platform — `codex-workflow-app`** (application composition) | validated-with-evidence (library: cargo test) | VWO-007-report.md (engine-attack battery incl. `-p codex-workflow-app` tests → 36 passed at 0d314fd) | product mount: all NST lanes (RWO-001) | App-plane behavior when driven by real users through DEMONSTRATE/INSTRUCT/HYBRID (Family A) |
| A4 | **Workflow Platform — `codex-teaching-compiler`** (DEMONSTRATE/INSTRUCT/HYBRID compile) | validated-with-evidence (compile-level only; behavioral coverage benchmark-pending) | Compiles clean as a workflow-app/triggers dependency: VWO-001-report.md family check (cargo check exit 0) + VWO-005 `_infra/post-hoc/20260911T221920Z-m4-workflow-family-cargo-check.txt`; no dedicated `cargo test -p codex-teaching-compiler` transcript exists in any merged report (marked unverified per no-fabrication) | teaching tasks in EVERY lane; teaching-mode coverage is a lane acceptance requirement | Teaching compilation exercised AT ALL through any surface (Family A blocks it — the single most important untested plane) |
| A5 | **Workflow Platform — `codex-workflow-durable`** (durable state/recovery) | validated-with-evidence (library: cargo test + adversarial) | VWO-007-report.md (46 tests passing at 0d314fd; 4/4 adversarial scenarios REPELLED — SIGKILL/restart/idempotency replay, `adv-session-loss-restart`, `adv-duplicate-triggers`, `adv-cancellation-race`, `adv-provider-failure`) | durable behavior through the product surface: NST-X004 (long-running resumable) + all recovery tasks | Durable guarantees when the workflow runs as a product (not library probes); long-running resumption at task level |
| A6 | **Workflow Platform — `codex-workflow-triggers`** (schedules/events) | validated-with-evidence (library: cargo test) | VWO-007-report.md (23 tests passing at 0d314fd) | trigger-driven tasks: NST-B/B006-style renewal, NST-X004 | Trigger firing through the product; schedule/webhook surfaces in the app (VWO-006 F-4, Consumer family) |
| A7 | **Workflow Platform — `codex-workflow-distribution`** (publish/install/upgrade/rollback) | validated-with-evidence (library: cargo test + adversarial WITH OPEN HOLES) | VWO-009-report.md (`-p codex-workflow-distribution` → 10 + 8 passed at 7736a42; 12-attack battery: 7 REPELLED both-layers, 2 HELD engine, 3 HOLES — Family G implicit downgrade, Family I visibility-gate skip; RWO-009 proposed, NOT merged) | distribution through product: NST-B/X marketplace tasks (B009, X003, X008) | Post-remediation re-proof of G/I; product-path install/upgrade/rollback with UI evidence |
| A8 | **Workflow Platform — `codex-workflow-evolution`** (evidence→improvement→approval) | benchmark-pending (compiled only; zero dependents; NO behavioral evidence) | Compiles in the family check (VWO-005 cargo-check artifact, above); zero dependents and no product surface: VWO-010 probe `docs/validation/evidence/vwo-010/_infra/post-hoc/20260912T104048Z-fork-improvement-trigger-surface-absence.txt` (probe 3) | improvement-lifecycle tasks (no v1.0.0 task targets it directly — recorded gap; RWO-006 proposes the port) | ALL of it: candidate-from-evidence, approval-before-publish (Family D — RWO-006 proposed, not merged) |
| A9 | **Execution contracts — `codex-execution-contracts`** (execution-plane contract) | validated-with-evidence (library: cargo test) | VWO-008-report.md (`cargo test -p codex-execution-contracts --lib` → 112 passed / 0 failed at d198a00, incl. undeclared-operation/missing-resource/no-credential-material suites) | execution under real adapters: NST-D/B/T/X tasks | Contract enforcement during real browser/GUI/terminal/API task execution (adapters unmounted) |
| A10 | **Execution Plane — `codex-browser-use-adapter`** | benchmark-pending (compiles as workflow-app dependency; no execution evidence) | Present in `codex-rs/workflow-app/Cargo.toml` (static, verifiable at base); no merged report exercises it | NST-B (VWO-013) — all browser tasks | Adapter behavior under browser task load; provider-semantics isolation (VWO-013 acceptance) |
| A11 | **Execution Plane — `codex-computer-use-adapter`** | benchmark-pending (compiles; no execution evidence) | Present in `codex-rs/workflow-app/Cargo.toml` (static); Xvfb/xdotool substrate probed: VWO-001-report.md §1 | NST-D (VWO-012) — all desktop tasks | Adapter behavior under GUI task load (windows/dialogs/keyboard/mouse) |
| A12 | **Execution Plane — env adapters / resource providers** (`env-adapters`, `resource-providers`) | benchmark-pending (compile-level; dependency-probe only) | VWO-005 `_infra/post-hoc/20260911T222529Z-product-surface-dependency-probe.txt` (they depend on teaching-compiler; nothing user-facing depends on them) | capability/resource binding tasks: NST-T009 (CLI auth binding), all TD-10 ≠ none tasks | Binding resolution, capability discovery, provider-semantics non-leakage in real tasks |
| A13 | **Codex Runtime app plane — CLI / TUI / app-server(+protocol/daemon) / exec mounts** | benchmark-pending (mount ABSENT — Family A, canonical P0) | VWO-010 synthesis SCENARIO-ISSUE-MATRIX.md §2 Family A + static probe `docs/validation/evidence/vwo-010/_infra/post-hoc/20260912T104009Z-engine-mount-absence-probe.txt` (zero workflow deps in cli/tui/app-server*/exec Cargo.toml; no CLI subcommand; no protocol mention) | ALL NST lanes (the mount is the prerequisite for every product-path task) | The entire user-facing lifecycle: teach → compile → review → approve → publish → install → run → recover → upgrade (RWO-001 proposed, NOT merged at base) |
| A14 | **Evidence Plane (product-native)** | benchmark-pending (fixture-harness evidence infra validated; workflow-native recording unmounted) | Harness: VWO-003 report + 452 artifacts across VWO-004…009 (SCENARIO-ISSUE-MATRIX §1); `capture-evidence.sh` discipline | evidence-emission tasks in every lane (SCORING.md §6 requires run evidence) | Workflow-attached execution evidence (the A8 evolution input) |
| A15 | **Human-gate plane** | benchmark-pending (operator-chat channel only — reduced fidelity) | VWO-001-report.md §1 human-gate row + §2.1 item 6 (fidelity limitation recorded); fixture-level approval gates exercised: `construction-procurement-approval`, `adv-approval-race` (VWO-004/VWO-008) | NST-X001/X002/X006 (human-in-the-loop) + NST-H draw | Product human-approval step inside a workflow (distinct from fixture approval UI) |
| A16 | **MCP modality (rmcp-client / codex-mcp crates)** | explicitly-out-of-scope-v1 | Absent from the proving ground: VWO-001-report.md §1 (MCP servers ABSENT — no `.mcp.json`) | — (no lane owns it in v1.0.0) | Revisit trigger: an MCP server surface lands in the sandbox or a mounted product MCP path exists; then add an NST family via COVERAGE-MATRIX §5 (MINOR bump) |

**Plane-level summary:** the Workflow Platform's contract/durable/trigger/
distribution/forge/app/execution-contract crates have library-level evidence
(green cargo transcripts, adversarial batteries) — but **no plane above the
library line has product-path evidence**, because the mount (Family A) does
not exist at this base. The teaching compiler — the plane the north-star
claim most depends on — has compile-level evidence only.

---

## 2. ENVIRONMENT coverage matrix

Environments (north-star README "environment universality"; architecture §6:
BROWSER / COMPUTER / TERMINAL / API / TOOL / MCP / HUMAN), at base `2d37cc0`.

| # | Environment | Status | Evidence pointer | Owning benchmark lane | What remains untested |
|---|---|---|---|---|---|
| E1 | Browser — SiteBuild (construction, 4101) | validated-with-evidence (fixture business behavior, human-path driving) | VWO-004-report.md: `construction-daily-progress`, `construction-procurement-approval`, `construction-safety-incident` (12/12 business paths passed; teaching blocked — Family A); VWO-007 `adv-session-loss-restart`; VWO-008 `adv-approval-race`; evidence dirs `docs/validation/evidence/VWO-004/<scenario>/` etc. | product-path exercise: NST-B lane (D001 keyboard variant) | Product-path browser steps (teaching/compiling/running workflows against it) |
| E2 | Browser — ForgeOps (software, 4102) | validated-with-evidence (same layer) | VWO-004: `software-pr-triage`, `software-incident-response`, `software-engineering-onboarding`; VWO-007 `adv-provider-failure`; VWO-008 `adv-prompt-injection`, `adv-duplicate-triggers` | NST-B (table/pagination/upload patterns: B003/B004) | Same as E1; plus richer table/pagination/upload patterns |
| E3 | Browser — RidePilot (rideshare, 4103) | validated-with-evidence (same layer) | VWO-004: `rideshare-driver-onboarding`, `rideshare-support-escalation`, `rideshare-surge-ops`; VWO-007/008 `adv-duplicate-triggers`, `adv-cancellation-race` | NST-B (live-map dynamic states: B002 class) | Same as E1; live-map dynamic-state interpretation |
| E4 | Browser — PressRoom (media, 4104) | validated-with-evidence (same layer) | VWO-004/005: `media-editorial-publishing`, `media-content-repurposing`, `media-breaking-news`; VWO-007/008 `adv-cancellation-race`, `adv-credential-exfiltration`; publication-terminality hole (Family F — RWO-004 proposed, NOT merged) | NST-B/NST-X (upload/rendition round-trips: B004, X007) | Same as E1; upload/rendition round-trips |
| E5 | Browser — FlowMart (marketplace, 4105) | validated-with-evidence (same layer, with open P1 holes) | VWO-005/006: `marketplace-create-and-sell`, `marketplace-fork-and-improve`, `marketplace-discover-install-configure`, `marketplace-upgrade-rollback`; VWO-009: `adv-version-confusion`, `adv-marketplace-entitlement-race`, `adv-environment-failure-rebinding`; families B/E/G/H open (RWO-002/003/007/008 proposed, NOT merged) | NST-B (first-class API steps: B009) | Same as E1; install-configure human path defective (Family B); first-class API steps |
| E6 | Browser — beyond the five fixtures (new/purpose-built web surfaces, multi-tab, downloads) | benchmark-pending | (none yet — lane defined) | NST-B (VWO-013): B001–B010 | Everything: multi-tab journeys, dynamic/validation-heavy forms, pagination sweeps, upload/download pipelines, session renewal, rich editors, redirect/transient-failure recovery, ambiguous UI |
| E7 | Native desktop GUI (Xvfb + xdotool + GUI-mode chromium; user-space GUI apps) | benchmark-pending (substrate probed and validated as PRESENT) | Substrate: VWO-001-report.md §1 (Xvfb live render, xdotool synthetic input, ffmpeg x11grab all verified); no desktop-GUI TASK has run | NST-D (VWO-012): D001–D010 | All task-level GUI work: launch/focus/switch, dialogs/menus, keyboard-only, mouse, clipboard, file pickers, document/spreadsheet editing, crash recovery |
| E8 | Terminal / CLI | validated-with-evidence (fixture terminal surface, human-path) — general terminal tasks pending | VWO-004 `software-incident-response` steps 3–4 (`deploy-console.js` login/`deployments`/`promote`) and VWO-007 `adv-provider-failure` (same console under 503); general-purpose shell work has NO task evidence | NST-T (VWO-014): T001–T010 | Pipelines, process inspection, file transformation, archives, git ops, build/test/lint, log analysis, CLI auth, checkpoint-resume |
| E9 | Filesystem / document manipulation | benchmark-pending (file API present; no document tasks) | VWO-001-report.md §1 (File API PRESENT, project root writable) | NST-T (T003/T005/T006/T008) + NST-D (D005/D006/D007/D008/D010) | File discovery/transformation, archives, document editing, file-picker/save dialogs |
| E10 | API / tool (first-class workflow steps) | benchmark-pending (API twins were post-hoc diagnostics only, per SCENARIO-CATALOG §2 binding rule 1) | Diagnostic usage evidence: VWO-004..009 post-hoc pulls (e.g., `/api/events` checks) across 34 runs; platform SDK present (VWO-001 §1, names only) | NST-B009/B010, NST-T004/T009/T010, NST-X002/X003/X005 | API steps as GOAL-CRITICAL workflow steps with bindings; platform-tool invocation |
| E11 | MCP | explicitly-out-of-scope-v1 | No MCP server config exists in the sandbox (VWO-001 §1: ABSENT) | — | Revisit when an MCP surface lands (then §5 procedure, MINOR bump) |
| E12 | Mixed / multi-environment (browser+terminal, browser+GUI+terminal+…) | benchmark-pending (one human-path mixed flow exists) | Human-path mixed browser+terminal: VWO-004 `software-incident-response` (browser steps + deploy-console terminal step, one HYBRID scenario — path passed, teaching blocked) | NST-X (VWO-015): X001–X008 | Product-level one-workflow-multi-environment execution; ≥3-modality tasks; long-running resumable across environments |
| E13 | E2B/Composio desktop proving ground (isolated multi-workspace pool) | explicitly-out-of-scope-v1 | VWO-001 §2 fallback record: integration ABSENT (no credentials; may not be invented); program §3 fallback governs | — | Revisit trigger: the connected E2B/Composio integration appears (capability_check.sh check 7 flips); then re-baseline desktop-lane fidelity |

---

## 3. INTERACTION coverage matrix

Interaction channels (north-star README "interaction universality": GUI,
browser, terminal, file, API/tool/MCP, human), at base `2d37cc0`. Note the
two-layer distinction: *channel exercised by a human worker* (wave-1/2
evidence) vs *channel exercised by the product's workflow execution* (none
yet — Family A).

| # | Interaction channel | Status | Evidence pointer | Owning benchmark lane | What remains untested |
|---|---|---|---|---|---|
| I1 | GUI (computer-use interaction: windows, keyboard/mouse injection) | benchmark-pending (substrate probed) | VWO-001 §1 (xdotool synthetic keystrokes verified received by an X client; Xvfb render verified) — no GUI interaction through the product | NST-D: all ten tasks | Product GUI action/observation; focus/stacking; dialogs; drag/drop |
| I2 | Browser (browser-use interaction) | validated-with-evidence (human-path channel: agent-browser driving across 34 runs) — product-step execution pending | VWO-004…009 reports (agent-browser 0.35.0 over Playwright chromium; every scenario's user-path evidence dirs); e.g., `docs/validation/evidence/VWO-004/construction-daily-progress/user-path/` | NST-B: all ten tasks | Product browser steps (navigate/act/observe as workflow actions); multi-tab ownership; downloads |
| I3 | Terminal (typed command interaction) | validated-with-evidence (fixture console channel) — general CLI pending | VWO-004 `software-incident-response` (deploy-console login/promote steps); VWO-007 `adv-provider-failure` (console under failure) | NST-T: all ten tasks | Product terminal steps; pipeline composition; process control; CLI credential bindings |
| I4 | File (direct file operations as interaction) | benchmark-pending | File API PRESENT (VWO-001 §1); file ops used by workers as test plumbing, never as product interaction | NST-T003/T005/T006/T008; NST-D005/D006/D007/D008 | Product file-op steps (create/read/write/move/delete documents) |
| I5 | API/tool/MCP interaction | benchmark-pending (API twins: post-hoc diagnostics only; MCP absent) | Post-hoc diagnostic usage across VWO-004…010; binding rule cited: SCENARIO-CATALOG.md §2 rule 1 | NST-B009/B010; NST-T004/T009; NST-X002/X003 | First-class API/tool calls inside workflows with explicit bindings; MCP: out-of-scope-v1 (E11) |
| I6 | Human interaction (approval/input as a step) | benchmark-pending (fixture approval gates + operator channel only; fidelity-limited) | Fixture-level gates: `construction-procurement-approval` (dual-approver conflict), `media-editorial-publishing` (editor approval), `adv-approval-race` (VWO-008, race repelled); human gate = operator chat (VWO-001 §2.1-6) | NST-X001/X002/X006; NST-H draw (≥2 human-in-the-loop per §7.3) | Product human-approval step semantics: gate, resume, timeout, escalation |

---

## 4. GENERALIZATION coverage matrix

Provenance classes (TASK-TAXONOMY TD-13; north-star README "generalization"),
at base `2d37cc0`.

| # | Provenance class | Status | Evidence pointer / owning lane | What remains untested |
|---|---|---|---|---|
| G1 | **Scripted** (validation-team-authored, known expected path) | validated-with-evidence (business-path layer; teaching layer blocked) | 34 scenario runs, VWO-004…009 — SCENARIO-ISSUE-MATRIX.md §1 (22 blocked at teaching/product layer · 10 passed (7 clean, 3 with recorded holes); 452 artifacts; teaching-mode totals DEMONSTRATE×11 / INSTRUCT×11 / HYBRID×12) | The product-teaching guarantee for even scripted scenarios (Family A); scripted tasks of the NEW benchmark lanes (NST-D/B/T/X scripted tasks remain pending until VWO-012..015 run them) |
| G2 | **Human-authored** (goal text only, no action script) | benchmark-pending | BENCHMARK-CATALOG v1.0.0 defines 9 human-authored tasks: D002, D007, D009, B002, B008, B010, T004, T007, X007 — executing WOs VWO-012/013/014/015 (VWO-013 requires ≥3 human-authored) | Whether a human-stated goal (no script) yields a working workflow through the product — the decisive mid-tier generalization test |
| G3 | **Held-out** (unseen until execution) | benchmark-pending (procedure defined; zero tasks enumerated BY DESIGN) | BENCHMARK-CATALOG.md §7 (selection procedure: ≥3 independent authors, sealed after VWO-015, stratified sampling grid, SHA-256 commitment ledger, anti-cheating rules) — executing WO: VWO-016 | The entire held-out class: unseen-goal success/recovery/adaptation rates by teaching mode; changed-state re-execution; human-intervention reasons; demonstrated-breadth vs unproven-capability distinction |

**Generalization honesty rule (binds all three rows):** passing scripted
scenarios never establishes universal capability (VWO-017 gate discipline;
VALIDATION-PROGRAM dependency-graph north_star_rule; north-star README claim
boundary). G1's "validated" label covers fixture business behavior only —
the strongest true statement at this base is: *scripted human paths behave
correctly in five fixture apps; the product path for teaching/executing them
is blocked by Family A (RWO-001 proposed, not merged).*

---

## 5. ADD-NEW-FAMILY PROCEDURE (how new task families are added WITHOUT changing workflow semantics)

**Guarantee (the VWO-011 acceptance demand):** catalog growth is a
DOCUMENTS-ONLY operation — no engine change, no runtime change, no fixture
change is required or permitted as part of adding a family. The catalog is
data the benchmark measures with, not code the product runs.

**Procedure (normative):**

1. **Propose the family.** A markdown proposal stating the recurring
   human-goal pattern (e.g., "calendar-driven multi-app reporting") and 1–3
   representative task sketches. Pattern is stated in human terms, never in
   engine/adapter terms (TASK-TAXONOMY §0 product-agnostic rule).
2. **Place on the taxonomy.** Map each representative task onto TD-01…TD-13
   (one value per dimension; `mixed` carries its set).
   - If every needed value exists → continue.
   - If a needed VALUE does not exist → STOP: that is a taxonomy value-set
     change = MAJOR benchmark bump + Tech Lead review (SCORING.md §8-1).
     The gap is recorded; the family waits. (This is the guard that keeps
     the grid honest instead of elastically redefining itself.)
3. **Assign lane + WO.** Primary interaction modality of the goal-critical
   steps picks the lane (D/B/T/X per BENCHMARK-CATALOG §2); held-out
   candidates go to the NST-H procedure instead (never enumerated).
4. **Author task records** per BENCHMARK-CATALOG §3: next-free NST IDs in
   the lane (IDs never renumbered or reused), goal in plain human terms,
   `[C1]`/`[C2]` criteria with GC/SUP marking, required environments/
   capabilities/resources, difficulty per §4 with justification, provenance,
   canonical teaching-mode binding chosen to keep the lane's mode table
   balanced (all three modes covered).
5. **Check lane sizing.** The lane must still satisfy its WO's coverage
   minimum (≥8 D/B/T; ≥6 X) including the new family's tasks.
6. **Version bump + changelog.** Additive tasks within existing semantics =
   MINOR bump (e.g., 1.0.0 → 1.1.0) with a changelog row. Run records
   already executed stay pinned to the version they ran under.
7. **No product/fixture changes ride along.** If the family needs a product
   capability that does not exist: record it in the task's
   required-capabilities field; execution will be BLOCKED → findings → a
   SEPARATE product work order (the catalog change and the product change
   never mix). If it needs a new fixture surface: a separate fixture WO
   under the harness owner (program §4 purpose-built-fixture policy).
   Fixtures and the catalog version independently.
8. **Update the ledger.** Extend/adjust coverage-matrix rows (this file) and
   re-run the SELF-CHECK consistency checks; record results.
9. **Reviewer checklist (what the Tech Lead verifies before accepting a
   family):**
   - [ ] every new task has all 13 taxonomy values, one each;
   - [ ] lane + executing WO unique and correct; IDs fresh, not reused;
   - [ ] criteria are `[C1]`/`[C2]`, never model-text;
   - [ ] difficulty matches the §4 scale (stressor count / triggers);
   - [ ] mode table per lane still covers DEMONSTRATE/INSTRUCT/HYBRID;
   - [ ] version bumped (MINOR) + changelog row + self-checks green;
   - [ ] NO diff outside `docs/validation/north-star/` (and the ledger)
     — no engine, runtime, fixture, or schema file touched.

**Precedent note:** this procedure is the same discipline the wave-0 catalog
used for scenarios (SCENARIO-CATALOG §3 "Adding a scenario"), lifted to the
north-star lane scale — reviewers already know how to audit it.

---

## 6. Reviewer quick-path (5-minute determination)

1. **What is validated?** Read §1/§2/§3/§4 rows with status
   `validated-with-evidence`; each cites a merged report + scenario ID or an
   evidence file path — spot-check two at the recorded base SHA.
2. **What is untested?** Every `benchmark-pending` row names its owning NST
   lane and executing WO; `explicitly-out-of-scope-v1` rows state the reason
   and revisit trigger. The one-line state of the world at this base:
   *contract/durable/distribution engine crates and five fixture apps have
   library/human-path evidence; every product-path computer-task lane
   (desktop, browser-beyond-fixtures, terminal/filesystem, API-first,
   cross-application, human-gate, held-out) is pending the mount (RWO-001)
   and the wave-4 executions.*
3. **How are families added?** §5 — documents-only, MINOR bump, IDs never
   reused, taxonomy value-set changes gated behind MAJOR bumps; reviewer
   checklist included.
