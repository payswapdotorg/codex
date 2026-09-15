# North-Star Generalization Results — Provenance-Tier Record (VWO-017)

**Status:** CANONICAL record of the three generalization provenance tiers
(TASK-TAXONOMY TD-13) at evidence base **`9f5eaffff`**.
**Discipline:** SCORING §9 — no universality claim may be derived from
scripted or human-authored passes; held-out results are the only
generalization evidence class that can support a `NORTH_STAR_SUPPORTED`
verdict; absent that class the strongest available verdict is
`NORTH_STAR_PARTIALLY_SUPPORTED`.

---

## 1. Tier summary

| Tier | Provenance | Tasks | Result | Verdict contribution |
|---|---|---|---|---|
| G1 | Scripted (validation-team-authored, known expected path) | 29 | 29 PASS / 0 PARTIAL / 0 FAIL / 0 BLOCKED (38-run lane set incl. G2 below; per-lane: D 10/10, B 10/10, T 10/10, X 8/8) | Coverage only — no generalization claim |
| G2 | Human-authored (goal text only, no action script) | 9 (D002, D007, D009, B002, B008, B010, T004, T007, X007) | 9 PASS (each through the product path: teach from goal text → publish → run → settle; recovery/adaptation criteria goal-critical-passed where the task carries them) | Mid-tier generalization demonstrated: a human-stated goal, no script, yields a working workflow. Still author-visible (the authors saw the system being built) — one tier below held-out |
| G3 | Held-out (unseen until execution, independent authors) | 0 executed (procedure sealed, ≥12-task pool defined, zero enumerated by design) | **NOT EXERCISED — OPERATOR-GATED** | No evidence available in either direction; the `NORTH_STAR_SUPPORTED` requirement "held-out generalization" is unmet by absence of execution, not by observed failure |

## 2. G3 — the honest record

- **What was required** (BENCHMARK-CATALOG §7, binding): ≥12-task authored
  pool from ≥3 independent human authors including at least one author
  unfamiliar with the workflow contracts; stratified task-space sampling
  grid; SHA-256 commitment ledger; sealed after VWO-015; executed by
  VWO-016 through the product path across all three teaching modes with
  changed-state re-execution and human-intervention recording.
- **What happened**: the operator gate never opened within the program
  window — no independent-author briefs were delivered to the executor.
  The anti-cheating rule bars the executor (and the Tech Lead acting as
  executor) from authoring the held-out tasks, so the lane could not be
  self-served. Requests were posted in the operator channel (12:05, 19:15,
  19:35 UTC records) with the stratification requirements restated.
- **What is NOT claimed**: no success rate, no per-mode breakdown, no
  changed-state adaptation rate, no intervention-reason distribution. These
  statistics do not exist and were never fabricated or extrapolated from
  G1/G2.
- **What would close it**: operator-delivered briefs → VWO-016 execution →
  reconciliation into this record (NORTH-STAR-VALIDATION §6 documents the
  escalation path to `NORTH_STAR_SUPPORTED`).

## 3. Per-mode teaching results (G1+G2 — for what they legitimately show)

All three teaching modes were exercised in every lane (lane mode tables in
the VWO-012..015 reports). Within the authored tiers:

- **DEMONSTRATE** — recorded demonstrations compiled to executable
  workflows and reproduced the demonstrated outcomes (e.g. D-lane recorded
  interaction traces, B007 armed-failure demonstration).
- **INSTRUCT** — natural-language instruction sets (3–5 statements)
  compiled, were approved through the review gate, and ran to the
  instructed outcomes (e.g. APP-003's `weekly-report-pipeline` taught from
  three instructions by a credential-less fresh user; T-lane instruct
  tasks).
- **HYBRID** — demonstration + instruction mixes resolved into single
  workflows (e.g. X001 bulletin approval-publish, B001 two-tab journey).

These are *teaching-mechanism* results on authored goals. They demonstrate
that the teaching surface works across modes; they do not measure
generalization to unseen goals (that is G3, above).

## 4. Changed-state re-execution (adaptation) — authored-tier results

Where catalog tasks required adaptation (changed but semantically
equivalent state), both phases are evidenced and goal-critical-passed:
B001 (conditional install not-fired under changed demand data), X003
(tampered-control branch fail-closed), X004 (resume after kill -9),
T008 (wrong-edit → golden FAIL → revert → correct edit), X007 (rendition
set exact under drift). No held-out adaptation data exists (G3).

## 5. Human-intervention record

All 38 lane runs completed without unintended human intervention (TD-09
discipline: intervention is a result, not a rescue). Intended
human-in-the-loop steps (approvals) executed as designed with ordering
evidence (X001/X002/X006). No intervention-reason distribution exists for
held-out goals (G3).
