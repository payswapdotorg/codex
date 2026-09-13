# VWO-011 — North-Star Benchmark Scoring Rules

**Work Order:** VWO-011 (Wave 4)
**Status:** CANONICAL — binds every benchmark task run executed by
VWO-012 … VWO-016 and every revalidation run after remediation merges.
**Governing sources:** `docs/validation/north-star/README.md` (evidence
discipline, claim boundary) · `docs/validation/VALIDATION-PROGRAM.md` §9
(evidence requirements incl. north-star additional fields), §5 (revalidation),
§10 (severity) · `docs/validation/reports/REPORT-SCHEMA.md` (report format
contract — benchmark run records align with its evidence fields) ·
`BENCHMARK-CATALOG.md` (task records, criteria, difficulty) ·
`TASK-TAXONOMY.md` (placement grammar).

---

## 1. Unit of scoring

The unit is the **task run**: one catalog task (NST ID), executed once
through the normal product path by one worker, at one exact repository SHA
and one benchmark_version. A task that the catalog requires to run twice
(adaptation-required: original + changed-but-equivalent state) is ONE run
with two execution phases; both phases must be evidenced and both count in
the criteria.

A task run is never scored from replays, logs of other tasks, or model
narrative — only from the evidence set defined in §7.

## 2. Criterion classes

Every catalog success criterion is tagged at authoring time:

- **`[C1]` machine-checkable** — decidable by a deterministic program:
  API/state pulls, file existence/contents/checksums, exit codes, event-log
  entries, counts, orderings, byte/image-header comparisons. The checker
  command or rule is recorded in the evidence.
- **`[C2]` inspector-checkable** — decidable by a human inspector against a
  pre-stated oracle: screenshots, transcripts, window-tree dumps. The
  oracle (what specifically must be visible) is written in the task record
  BEFORE the run; the inspector compares observation to oracle. The
  inspector may be the worker, but the oracle must pre-exist — an
  after-the-fact rationalization is not a `[C2]` pass.

**Forbidden as evidence (north-star README, verbatim rule): never treat
model text claiming success as execution evidence.** An LLM's "I have
submitted the form" is never a pass; the fixture state, event log, file, or
screenshot is. Model text MAY be recorded as diagnostic context, clearly
labeled, never as the pass basis.

## 3. Per-criterion pass/fail

Each criterion evaluates to exactly `pass`, `fail`, or `not-evaluated`
(with reason). No criterion is partially met. If a `[C1]` checker is
non-deterministic (e.g., live public-web content), the run must say so and
the criterion degrades to `[C2]` with an inspector oracle — recorded, never
silent.

## 4. Task outcomes

| Outcome | Definition |
|---|---|
| **PASS** | Every criterion (all `[C1]` and `[C2]`) passes in the required phases (including recovered end-state for recovery criteria and both states for adaptation criteria). |
| **PARTIAL** | All **goal-critical** criteria pass; ≤2 **supporting** criteria fail. (Goal-critical vs supporting is marked per criterion at authoring time; when unmarked, ALL criteria are goal-critical and PARTIAL is unavailable.) |
| **FAIL** | Any goal-critical criterion fails after all retries/recovery the task itself allows. |
| **BLOCKED** | A product surface the task requires does not exist (the wave-1/2 Family-A pattern: "step N not performed — surface missing"). BLOCKED is a distinct, honest outcome: it is an untested-not-passed result that feeds the remediation loop (a defect finding, severity per REPORT-SCHEMA §13), never a silent skip and never a pass. |

A task that requires human intervention the task did not semantically
require (TD-09) is scored FAIL on the affected criteria AND records the
intervention reason — intervention is a result, not a rescue (VWO-016
method step 7).

## 5. Partial credit policy

1. Criteria are partitioned **goal-critical (GC)** vs **supporting (SUP)**
   at authoring time (the catalog marks GC criteria; unmarked ⇒ GC).
2. PASS = all criteria pass. PARTIAL = all GC pass and ≤2 SUP fail.
   FAIL = any GC fails. BLOCKED = surface missing before the goal is
   achievable.
3. Recovery/adaptation criteria are ALWAYS goal-critical: a task that
   "completes" only because the injected failure never landed, or the
   changed-state re-run never happened, is NOT a pass for a
   recovery/adaptation-required task — the stressor must be evidenced as
   actually exercised (e.g., the 503 in the switch ledger, the kill -9
   transcript).
4. **Lane coverage credit:** only PASS counts toward a coverage-matrix row
   transition (benchmark-pending → validated-with-evidence). PARTIAL rows
   stay benchmark-pending with a note of the failed SUP criteria; FAIL
   rows stay pending and open findings; BLOCKED rows stay pending and open
   remediation work.
5. **Aggregation:** per-lane and per-WO reporting states N tasks run with
   X PASS / Y PARTIAL / Z FAIL / W BLOCKED, plus per-difficulty-level
   breakdown (L1–L4). A lane's "coverage claim" (BENCHMARK-CATALOG §8
   minimums) requires: minimum task count executed, all rows PASS, and all
   taxonomy cells the lane claims exercised with evidence.

## 6. Evidence required per task run (the record)

Every task-run record carries ALL of the following — aligned with
REPORT-SCHEMA.md §2 field rules and the north-star README evidence
discipline, plus the program §9 north-star additional fields, plus the
benchmark's own fields:

```text
# identity
exact repository SHA (40-hex, verified)
benchmark_version (from BENCHMARK-CATALOG.md)
nst-id / scenario ID (catalog slug; lane)
executing Work Order (VWO-012…016) + worker identity
date/time window (UTC)

# task placement
task_taxonomy_dimensions (the 13-tuple actually observed — vs the catalog
  binding; deviations recorded, never silent)
difficulty (catalog value; disagreement recorded)
scripted_or_held_out (provenance as executed)
teaching_mode (canonical or recorded non-canonical deviation)

# the goal and the run
human goal (verbatim from the catalog task record)
environment/workspace identity (substrates, fixture apps, ports, personas)
applications used (names + instance identity)
capabilities inferred / resources inferred (what the workflow required
  and how it was bound — including credential-binding identity when the
  task crosses TD-10; never the credential itself)
workflow/version identity (workflow name, immutable version, source
  revision, definition digest, dependency-lock identity — NONE-with-reason
  where the product does not yet expose them; never fabricated)

# expected vs actual
expected outcome (from the catalog criteria)
actual outcome (exactly what happened, verbatim errors included)
criterion-by-criterion results (pass/fail/not-evaluated + checker/oracle)

# computer-side result evidence (C1/C2 artifacts, paths under
# docs/validation/evidence/<VWO-ID>/<nst-id>/user-path|post-hoc/)
evidence of the actual computer-side result — state pulls, files,
events, screenshots, transcripts (user-path artifacts for what a human
could see; post-hoc for diagnostics AFTER the user path was attempted)

# north-star additional fields (program §9)
original_goal_state
changed_semantically_equivalent_state   (adaptation tasks: both snapshots)
adaptation_or_recovery_result
human_intervention_reason              (none, or the semantic reason)
cross_environment_count                (environments actually exercised)
application_count
generalization_result                  (held-out lane: per VWO-016)

# failure accounting
failure classification — exactly one of:
  missing-capability | binding/resource-issue | workflow-semantic-defect |
  model-inference-issue | environment-specific-limitation
  (VWO-012 acceptance taxonomy; BLOCKED runs classify as
  missing-capability by default, overridable with evidence)
findings opened (family/IDs, severity P0–P3 per REPORT-SCHEMA §13)
product friction (non-blocking usability observations — recorded, they
  are first-class benchmark output, not noise)
```

Evidence path convention follows the scenario catalog: user-path artifacts
for human-observable steps, post-hoc for diagnostics; secrets never in
reports or evidence (REPORT-SCHEMA §9; `evidence/README.md`).

## 7. Revalidation rule (post-remediation)

1. **Trigger:** any accepted remediation merge (RWO-001…017 or later fix
   WO) that touches a surface a benchmark lane exercised — engine mount,
   fixture ops, adapters, distribution, teaching surfaces.
2. **Scope:** every task run whose evidence cites the changed surface, at
   minimum: the tasks of the affected lane whose required capabilities or
   applications the fix touched, PLUS the catalog scenarios of the
   wave-1/2 program that the fix's owning RWO revalidates
   (VALIDATION-PROGRAM.md §5 Revalidation; VWO-010's revalidation
   discipline). A defect is closed only when: fix merged, original
   failure reproduces as fixed, no regression, architectural invariant
   intact.
3. **Execution:** re-run at the post-merge SHA, recording the new SHA and
   the same benchmark_version (or the new one if the catalog version
   bumped in between — then the re-run is a fresh run under the new
   version, and the old run's row is annotated `superseded-by <run-id>`).
4. **Outcome discipline:** prior PASS does not carry over when the owning
   surface changed — the row reverts to benchmark-pending until the
   re-run lands. A remediation that changes task semantics (criteria no
   longer honest) is a catalog MAJOR bump instead (§9).

## 8. No-regression rule (benchmark integrity)

The catalog is versioned (`BENCHMARK-CATALOG.md` §1) and this rule is the
guard:

1. **MAJOR bump (semantics change — old runs not comparable):** changing
   any success criterion, taxonomy value set, difficulty scale
   definitions, lane/WO binding of an existing task, canonical teaching
   mode of an existing task, or removing/renumbering tasks.
2. **MINOR bump (additive):** new tasks within existing semantics (the
   ADD-NEW-FAMILY PROCEDURE, COVERAGE-MATRIX.md §5), held-out ledger
   sealing, new evidence-link annotations.
3. **PATCH bump:** editorial fixes, erratum-register entries (placement
   value typos, wording), non-semantic clarifications. Errata are
   recorded in the register — never silently edited in place.
4. **Run records pin the version.** Comparing outcomes across versions
   requires re-execution; no cross-version aggregation is valid.
5. **History is immutable.** Superseded tasks keep their IDs with a
   `deprecated:` marker; IDs are never reused or renumbered.
6. **The catalog never bends to the product.** If the product cannot
   achieve a task, the outcome is BLOCKED/FAIL with findings — narrowing
   criteria to match current surfaces is a MAJOR bump requiring Tech Lead
   review and explicit justification (forbidden: silent narrowing).

## 9. Verdict discipline (what scores may NOT claim)

- Passing tasks prove coverage of those tasks and their taxonomy cells —
  nothing more. No universality claim may be derived from scripted or
  human-authored passes (VWO-011 Forbidden list; north-star README claim
  boundary: directly demonstrated / strongly generalized from held-out /
  untested / impossible-without-capabilities / product-blocked are
  SEPARATE verdict classes).
- The north-star verdict belongs to VWO-017 alone; VWO-012…016 report
  lane results, not the verdict.
- A finite benchmark is not mathematical proof of the task space
  (VALIDATION-PROGRAM.md §12 final paragraph; TASK-TAXONOMY.md §7).
- Reporting must include per-dimension coverage status from
  COVERAGE-MATRIX.md so "what remains untested" is as prominent as the
  pass count.
