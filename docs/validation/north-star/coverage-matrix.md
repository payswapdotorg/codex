# North-Star Coverage Matrix — Reconciled (VWO-017)

**Status:** CANONICAL RECONCILIATION of `COVERAGE-MATRIX.md` (the VWO-011
running ledger, written at base `2d37cc0ef` before the mount and the
wave-4 executions) against the merged evidence at **`9f5eaffff`**
(VWO-001..015 + RWO-001..012 + APP-001..004 all merged; VWO-016 not
executed — operator-gated).
**Rules applied:** SCORING §5-4 (only PASS transitions a row);
§9 (what remains untested is as prominent as the pass count). Row IDs and
meanings are inherited unchanged from the VWO-011 matrices — this file
records the *transition state*, it does not redefine the grid.

---

## 1. ARCHITECTURE matrix — reconciled

| # | Plane / crate | Was (base 2d37cc0) | Now (at 9f5eaffff) | Transition evidence |
|---|---|---|---|---|
| A1 | workflow-contracts | validated (library) | **validated (product path)** | Every wave-4 lane ran on the contract surface (38/38 PASS); enforcement exercised through the mounted CLI/app-server (RWO-001), incl. forged-grant/foreign-binding suites still green post-mount. |
| A2 | workflow-forge | validated (library) | **validated (product path)** | Teach/compile/review/approve/publish executed in all 38 task runs; fork/lineage product surface exercised (RWO-005; NST marketplace tasks B009, X003). |
| A3 | workflow-app | validated (library) | **validated (product path)** | All lanes composed apps through it (task-level composition evidence in each lane report). |
| A4 | teaching-compiler | compile-level ONLY — "the single most important untested plane" | **validated (product path, all three modes)** | DEMONSTRATE/INSTRUCT/HYBRID all exercised per-lane (lane mode tables, VWO-012..015); the deterministic compile pipeline produced executable workflows from natural-language instruct and recorded demonstration in every lane. The base-state Family-A blocker on this plane is closed. |
| A5 | workflow-durable | validated (library + adversarial) | **validated (product path, task level)** | X004 long-running resumable across kill -9 (checkpoint resume, exactly-one summary); T010 exactly-once + stale-checkpoint supersede; APP-003 close/relaunch durability from a fresh install. |
| A6 | workflow-triggers | validated (library) | **validated (product path)** | Trigger-driven tasks executed (B006 renewal, X004; lane trigger-intent records). |
| A7 | workflow-distribution | validated (library; 3 HOLES open at base) | **validated (product path; holes closed + re-proven)** | RWO-007/009 closed families G/I; product-path install/upgrade/rollback with UI evidence: B009 control branch, X003 digest-gated install + tampered-control, X008 stale-entitlement fail-closed + last-good pin. |
| A8 | workflow-evolution | benchmark-pending (zero dependents, no surface) | **validated (product path)** | RWO-006 mounted the improvement/approval surface; X001 approval-before-publish ordering + armed-503 recovery exercised the lifecycle end-to-end. |
| A9 | execution-contracts | validated (library) | **validated (product path)** | Real adapter execution under contracts across D/B/T/X lanes; T009 credential binding path (redacted). |
| A10 | browser-use-adapter | benchmark-pending | **validated (task level)** | NST-B lane 10/10 incl. multi-tab ownership, downloads, uploads, rich editors, session renewal, dynamic forms (VWO-013 report + evidence tree). |
| A11 | computer-use-adapter | benchmark-pending | **validated (task level)** | NST-D lane 10/10 incl. dialogs, keyboard-only, mouse, clipboard, file pickers, document/spreadsheet editing, crash recovery (VWO-012). |
| A12 | env-adapters / resource-providers | benchmark-pending | **validated (task level)** | Capability inference + binding proposals in every teach; T009 CLI credential binding; provider-semantics isolation held (no leakage findings). |
| A13 | runtime mount (CLI/TUI/app-server/exec) | benchmark-pending — Family A, "the prerequisite for every product-path task" | **validated (product path + fresh machine)** | RWO-001 mounted the workflow family on the CLI + app-server protocol v2; every lane ran through it; APP-001 verified the surface against the release binary (13/13) and APP-003 from a fresh install (22/22). |
| A14 | Evidence plane (product-native) | benchmark-pending (harness only) | **validated (product-attached)** | Every task run's evidence set recorded through the product's own audit/event ledgers (evidence.jsonl, install-audits.jsonl, run-positions) captured in lane evidence trees + APP-001 codex-home/workflow artifacts. |
| A15 | Human-gate plane | benchmark-pending (operator-chat only) | **validated (product path)** | X001/X002/X006 human-in-the-loop at product level (approval gates inside the workflow lifecycle with ordering evidence); fixture-level gates from wave-1/2 remain as supporting evidence. |
| A16 | MCP modality | explicitly-out-of-scope-v1 | **unchanged — out-of-scope-v1** | No MCP server surface exists in the environment (revisit trigger as recorded). |

## 2. ENVIRONMENT matrix — reconciled

| # | Environment | Was | Now | Transition evidence |
|---|---|---|---|---|
| E1–E5 | the five fixture apps (SiteBuild/ForgeOps/RidePilot/PressRoom/FlowMart) | validated (human-path business behavior) | **validated (product path)** | Wave-4 lanes re-drove the fixture apps through the product path (browser + terminal + API surfaces; e.g. T-lane git/API work, X-lane cross-app joins). |
| E6 | browser beyond the five fixtures | benchmark-pending | **validated** | NST-B purpose-built surfaces (VendorGate, TeamPad, MarketplaceDigests, ForgeExport) + B001–B010 patterns: multi-tab, dynamic/validation forms, pagination sweeps, uploads/downloads, session renewal, rich editors, transient-failure recovery. |
| E7 | native desktop GUI | benchmark-pending (substrate probed) | **validated** | NST-D lane 10/10 (full GUI task inventory exercised). |
| E8 | terminal / CLI | fixture-console only | **validated** | NST-T lane 10/10 (pipelines, process inspection, SIGSTOP, file transformation, archives, git, build/test/lint, log analysis, CLI auth, checkpoint-resume). |
| E9 | filesystem / documents | benchmark-pending | **validated** | T003/T005/T006/T008 + D005/D006/D007/D008/D010 (discovery, transformation, archives, document editing, pickers/save dialogs). |
| E10 | API/tool as first-class steps | benchmark-pending (post-hoc diagnostics only) | **validated** | B009/B010, T004/T009/T010, X002/X003/X005 — API calls as goal-critical workflow steps with explicit bindings. |
| E11 | MCP | out-of-scope-v1 | **unchanged — out-of-scope-v1** | (as recorded). |
| E12 | mixed / multi-environment | benchmark-pending (one human-path flow) | **validated** | NST-X lane 8/8: ≥3-modality tasks, concurrent sessions, dual-surge fanout, long-running resumable across environments. |
| E13 | E2B/Composio proving ground | out-of-scope-v1 | **unchanged — out-of-scope-v1** (user-space fallback validated; 7 fidelity substitutions recorded, VWO-012) |

## 3. INTERACTION matrix — reconciled

| # | Channel | Was | Now | Transition evidence |
|---|---|---|---|---|
| I1 | GUI | benchmark-pending (substrate) | **validated (product path)** | NST-D 10/10 through computer-use interaction. |
| I2 | Browser | human-path channel only | **validated (product path)** | NST-B 10/10; browser steps as workflow actions incl. multi-tab ownership + downloads. |
| I3 | Terminal | fixture console only | **validated (product path)** | NST-T 10/10; pipeline composition, process control, credential bindings. |
| I4 | File | benchmark-pending | **validated (product path)** | T/D file-op tasks as product steps. |
| I5 | API/tool | benchmark-pending | **validated (product path)** | First-class API steps with explicit bindings (E10 tasks). MCP remains out-of-scope-v1. |
| I6 | Human | fixture gates + operator channel | **validated (product path)** | X001/X002/X006 approval/input semantics at product level (gate, resume, ordering). |

## 4. GENERALIZATION matrix — reconciled

| # | Provenance class | Was | Now | Evidence |
|---|---|---|---|---|
| G1 | Scripted | validated (business-path layer) | **validated (product path, all lanes)** | 29 scripted catalog tasks PASS through the product (D/B/T/X lanes). |
| G2 | Human-authored (goal only) | benchmark-pending | **validated — 9/9 PASS** | D002, D007, D009, B002, B008, B010, T004, T007, X007 (lane reports). |
| G3 | Held-out | benchmark-pending (procedure sealed) | **NOT EXERCISED — OPERATOR-GATED** | No independent-author briefs delivered within the program window; the executor is barred from self-authoring (catalog §7). Row stays `benchmark-pending` with this note — see `generalization-results.md`. |

**Generalization honesty rule (unchanged, binds this reconciliation):**
G1+G2 passes never establish universal capability. The strongest true
statement at `9f5eaffff`: *every task class the benchmark enumerated across
desktop GUI, browser, terminal/filesystem, API-first and cross-application
work — scripted and human-authored — was executed successfully through the
product path with environment-only evidence; the held-out class was never
opened, so generalization beyond authored goals is unproven, not failed.*

## 5. Not-carried / explicitly-bounded rows (no transition, by design)

A16 (MCP), E11 (MCP), E13 (E2B/Composio) — out-of-scope-v1 with recorded
revisit triggers; macOS/Windows platforms — release channel limitation
(APP-002 §2, org signing environments unavailable); real-model agent
surface — egress region block (validated for actionable error behavior
only). None of these rows may be transitioned by future runs without the
recorded triggers firing.
