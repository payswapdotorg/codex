# VWO-011 — Self-Check Record

**Work Order:** VWO-011 (Wave 4)
**Purpose:** machine-verified consistency of the four benchmark artifacts
(`TASK-TAXONOMY.md`, `BENCHMARK-CATALOG.md` v1.0.0, `SCORING.md`,
`COVERAGE-MATRIX.md`) against each other and against the repository at base
SHA `2d37cc0ef7d075e877f370790f82ac9c70df26a4`. This file records the
checks, the exact commands, and the results observed at authoring time
(2026-09-13, UTC). Re-runnable by any reviewer from the repository root at
the same base.

---

## Check 1 — every taxonomy dimension has ≥1 catalog task (or explicit out-of-scope-v1 note with reason)

**Command (conceptual):** parse every `taxonomy:` binding block in
BENCHMARK-CATALOG.md; assert each of the 13 dimension keys appears in ≥1 of
the 38 task blocks; strip trailing `#` comments before comparing values;
compare the used value-set per dimension against the closed value sets
declared in TASK-TAXONOMY §3.

**Result: PASS.**

- 13/13 dimensions each appear in all 38 task blocks (every task carries a
  complete 13-tuple — one value per dimension, composition rule intact).
- Value-level coverage per dimension (comment-stripped):
  - TD-01: all 6 values used (`mixed` sets enumerated in every case).
  - TD-02: all 7 values used. TD-03: all 3. TD-04: 2/2. TD-05: 2/2
    (`parallel-fanout` via NST-X006). TD-06: 2/2. TD-07: all 4. TD-08: all
    4. TD-09: 2/2. TD-10: all 4. TD-11: all 4. TD-12: 2/2
    (`destructive-irreversible` via NST-D006, NST-X001).
  - TD-13: `scripted` (29 tasks) and `human-authored` (9 tasks) used;
    `held-out` is **deliberately not used by any enumerated task** — the
    held-out class is procedure-only by design (BENCHMARK-CATALOG §7) with
    the explicit note in §9's TD-13 row. This satisfies the "≥1 task OR
    explicit out-of-scope-v1 note with reason" rule for that value.
- Value-level explicit gap recorded (not a dimension gap): MCP-specific
  behavior inside `api-tool-mcp` — out-of-scope-v1 with reason + revisit
  trigger (BENCHMARK-CATALOG §9 note; COVERAGE-MATRIX row A16/E11).

## Check 2 — every catalog task maps to exactly one lane + executing WO

**Command:** `nst-id: (NST-[DBTXH]\d+)\nlane: (\S+)\nexecuting-wo: (VWO-\d+)`
over the catalog; compare against the lane→WO table (D→VWO-012,
B→VWO-013, T→VWO-014, X→VWO-015, H→VWO-016-procedure-only); assert
uniqueness of task IDs; assert zero uncommented `nst-id: NST-H###` records.

**Result: PASS.**

- 38 task records; 38 unique IDs; lane/WO mapping consistent for all 38
  (D:10→VWO-012, B:10→VWO-013, T:10→VWO-014, X:8→VWO-015).
- Uncommented NST-H task records: **0** (the only `NST-H001` string in the
  catalog is inside the commented-out ledger-format example in §7.5 — a
  format illustration, not a task record; the held-out lane stays
  procedure-only).
- Sizing: D=10 (≥8 required), B=10 (≥8), T=10 (≥8), X=8 (≥6) — all lanes
  exceed their WO's coverage minimum.

## Check 3 — every validated-with-evidence row cites a report that exists at the base

**Command:** extract every report filename, evidence file path, and
scenario slug cited in COVERAGE-MATRIX.md; verify each with
`git cat-file -e HEAD:<path>` at base `2d37cc0`; verify every cited
scenario slug exists in
`docs/validation/scenarios/SCENARIO-CATALOG.md` at the same base; verify
every `VWO-NNN` reference resolves to an existing work-order and/or report
file.

**Result: PASS.** All citations resolve at base `2d37cc0`:

- Reports (exist at `HEAD:docs/validation/reports/`): `VWO-001-report.md`,
  `VWO-004-report.md`, `VWO-007-report.md`, `VWO-008-report.md`,
  `VWO-009-report.md`, `SCENARIO-ISSUE-MATRIX.md`. (VWO-005/VWO-006 rows
  are cited through their work-order + evidence artifacts below; VWO-010 is
  cited through its synthesis documents SCENARIO-ISSUE-MATRIX.md /
  FINAL-HUMAN-WORKFLOW-VALIDATION.md, which exist.)
- Evidence files (exist at `HEAD:`):
  `docs/validation/evidence/vwo-010/_infra/post-hoc/20260912T104009Z-engine-mount-absence-probe.txt`,
  `docs/validation/evidence/vwo-010/_infra/post-hoc/20260912T104048Z-fork-improvement-trigger-surface-absence.txt`,
  `docs/validation/evidence/VWO-005/_infra/post-hoc/20260911T221920Z-m4-workflow-family-cargo-check.txt`,
  `docs/validation/evidence/VWO-005/_infra/post-hoc/20260911T222529Z-product-surface-dependency-probe.txt`.
- Scenario slugs: all 26 canonical catalog scenario IDs cited by the
  matrices exist in SCENARIO-CATALOG.md at base (verified by slug set
  equality — construction×3, software×3, rideshare×3, media×3,
  marketplace×4, adversarial×10).
- VWO references: every `VWO-001…017` mention in the matrices resolves to
  an existing work-order file `docs/validation/work-orders/VWO-NNN.md`
  and/or merged report.
- Scenario-run evidence directories cited by pattern
  (`docs/validation/evidence/VWO-004/<scenario>/` etc.): the per-VWO
  scenario subdirectories for all 34 runs exist at base (verified during
  authoring via `git ls-tree`; consolidated in SCENARIO-ISSUE-MATRIX §1).

## Check 4 — forbidden-list compliance (VWO-011 Forbidden)

**Result: PASS — each clause addressed by design:**

1. *"Do not claim universal computer capability merely because existing
   scenarios pass."* — No artifact claims it. SCORING.md §9 makes the
   verdict discipline explicit: passing tasks prove coverage of those tasks
   and their taxonomy cells only; the north-star verdict belongs to
   VWO-017; coverage rows carry "what remains untested" as a mandatory
   column; COVERAGE-MATRIX §4 states the strongest true claim at this base
   (scripted human paths behave in five fixture apps; the product path is
   blocked by Family A).
2. *"Do not narrow the north star to browser automation."* — Desktop
   (NST-D ×10), terminal/filesystem/developer (NST-T ×10), and
   cross-application (NST-X ×8) lanes are first-class, same ID scheme,
   same taxonomy, same scoring; COVERAGE-MATRIX rows E7/E8/E9/I1/I3/I4 are
   benchmark-pending, not dropped; the only out-of-scope-v1 items (MCP,
   E2B pool) carry reasons + revisit triggers.
3. *"Do not add a second workflow engine or benchmark-specific runtime."* —
   All four artifacts are documents; the ADD-NEW-FAMILY PROCEDURE
   (COVERAGE-MATRIX §5) is documents-only and step 7 explicitly routes
   product/fixture needs to separate work orders; purpose-built fixtures
   are application tooling per program §4, never a workflow runtime; no
   new executable code ships in this work order.
4. *"Do not hard-code a finite list as the definition of all possible
   computer tasks."* — TASK-TAXONOMY §0 states the claim boundary and
   cites `docs/validation/north-star/README.md` (a finite suite cannot
   prove an infinite task space) and VALIDATION-PROGRAM.md §12; the
   taxonomy is declared a measurement grid, not the task space; unmodeled
   placements force explicit Tech-Lead adjudication (MAJOR bump), never
   silent rounding; BENCHMARK-CATALOG §11 and SCORING.md §9 repeat the
   boundary.

## Check 5 — teaching-mode coverage per lane (VWO-012..015 worker-pool requirement)

**Result: PASS.** Each of the four executable lanes covers all three modes
from its own task set: D (DEMO 3 / INSTRUCT 3 / HYBRID 4), B (3/3/4),
T (3/3/4), X (1/3/4); the held-out draw rotates modes per §7.3.

## Check 6 — internal scale consistency (difficulty)

**Result: PASS.** Every task's `difficulty` field was reconciled against
the §4 stressor/hard-stressor/frontier-trigger scale before the
establishing commit (final distribution L1×6, L2×7, L3×16, L4×9); each
task carries a one-line justification tied to the scale. No placement
errata remain open at v1.0.0 (BENCHMARK-CATALOG §9 register statement).

## Re-run instructions

The checks above are reproducible with the parsing approach recorded here
(regex over the `nst-id`/`lane`/`executing-wo`/`taxonomy:` binding blocks
with comment stripping; `git cat-file -e HEAD:<path>` at base `2d37cc0`
for citations; slug-set comparison against
`docs/validation/scenarios/SCENARIO-CATALOG.md`). A reviewer can reproduce
each result in minutes from the repository root.
