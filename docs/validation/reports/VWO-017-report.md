# VWO-017 — Universal Computer Automation North-Star Gate (Final Synthesis Report)

## Identity
- Work Order: VWO-017
- Worker persona: Tech Lead local delivery (same mode as the wave-4 lanes — recorded per the RWO registry precedent)
- Base branch: main
- Base SHA: 9f5eaffffea2f940290c884c0996edfca47d2061
- Head SHA: see git rev-parse vwo-017/north-star-synthesis (single commit on base 9f5eaffff)
- Validation environment: n/a (synthesis work order — no new task runs; reconciles merged evidence only)
- Codex Universal runtime/application SHA: NONE — synthesis report (no new task runs); the product-path lane evidence (VWO-012..015) ran through the workflow surfaces mounted at the evidence base, and the delivered runtime artifact is release rust-v0.1.0 (tag 23b7f6c27; x86_64 package sha256 4a6e7e9e2ecfbe3d86925bb877ce7751c30cfbd3d901ebcf1544d2c7e69b54d1 — validated by APP-001 13/13 and APP-003 22/22)
- Date/time window: 2026-09-15 19:45–20:10 UTC
  (Evidence-base note: every cited report/evidence tree is reachable from the Base SHA above.)
- Dependency state: VWO-010 merged ✓ · VWO-011..015 merged ✓ · RWO-001..012 merged ✓ · APP-001..004 merged ✓ · **VWO-016 NOT EXECUTED (operator-gated — the decisive open dependency, documented in-range below)**

## Verdict

**`NORTH_STAR_PARTIALLY_SUPPORTED`**

Broad computer-task automation is demonstrated through the product path
across every environment modality the sandbox actually provides — desktop
GUI, browser (fixture and purpose-built surfaces), terminal/shell,
filesystem, first-class API steps, cross-environment composition — with all
three teaching modes, human-authored goals, adversarial recovery, and a
downloadable, fresh-machine-validated product. The held-out generalization
class (VWO-016) never executed because the operator gate on independently
authored briefs did not open within the program window (the executor is
barred from self-authoring by BENCHMARK-CATALOG §7); material generality is
therefore **unproven, not failed** — the exact definition of
`NORTH_STAR_PARTIALLY_SUPPORTED` (broad automation works; evidence coverage
leaves generality open).

## Required artifacts (created by this work order)

| Artifact | Path |
|---|---|
| Final gate report (the verdict + 8-class table + claim-boundary separation) | `docs/validation/north-star/NORTH-STAR-VALIDATION.md` |
| Reconciled coverage matrix (row transitions at 9f5eaffff) | `docs/validation/north-star/coverage-matrix.md` |
| Generalization results (G1/G2/G3 provenance-tier record) | `docs/validation/north-star/generalization-results.md` |

## The eight required verdict classes (summary — full table in NORTH-STAR-VALIDATION §1)

1. Semantic universality — **DEMONSTRATED** (one control plane, one IR, one durable root across all 38 lane runs; zero benchmark-specific engines)
2. Interaction universality — **DEMONSTRATED** (GUI/terminal/file/browser/API beyond browser-only)
3. Environment universality — **DEMONSTRATED** (X-lane 8/8: ≥3-modality, concurrent sessions, dual-surge, resumable cross-environment)
4. Capability/resource universality — **DEMONSTRATED** (task-level inference/binding; T009 credential binding; no provider-semantics leakage)
5. Teaching universality — **DEMONSTRATED** (DEMONSTRATE/INSTRUCT/HYBRID in every lane; 9/9 human-authored G2 goals)
6. Execution reliability — **DEMONSTRATED** (engine batteries + task-level armed-503/kill-9/stale-checkpoint recovery; exactly-once semantics)
7. Generalization (held-out) — **NOT EXERCISED — OPERATOR-GATED** (VWO-016; no fabricated statistics — generalization-results.md §2)
8. Product usability — **DEMONSTRATED** (APP-003 fresh-machine 22/22; release rust-v0.1.0 live, checksum-verified)

## Claim-boundary separation (summary — full in NORTH-STAR-VALIDATION §2)

- **Directly demonstrated**: substrate; five-app ecosystem human paths; engine adversarial batteries; all four scripted lanes 38/38 at benchmark v1.0.0 through the mounted product surfaces; recovery/adaptation at task level; product-level human-in-the-loop; delivered downloadable product.
- **Strongly generalized from held-out**: NONE — class unexercised.
- **Remains untested**: held-out class; MCP; E2B/Composio; macOS/Windows; real-model agent surface (egress region block).
- **Impossible without unavailable capabilities**: MCP (no server surface), E2B/Composio (credentials never available), provider model access from this egress.
- **Blocked by product limits (not architecture)**: F-B1 ForgeOps comment/release-thread surface (fixture class); 11 recorded environment drifts (all fixture/catalog/persona/route class, none engine).
- **Product defects**: zero workflow-semantic defects in 38 runs; fixture-scope findings + friction log recorded (stdin-by-design, installer rate-limit sensitivity, experimental flag).
- **Architecture defects**: none open (families A–M remediated by RWO-001..012 and re-proven; no new architecture family in wave-4). Untested ≠ defect: held-out and MCP remain evidence-silent.

## Honest limitations of this synthesis

- VWO-016 did not run; class-7 verdicts are absence-of-evidence records, never negative or positive claims (SCORING §9 discipline).
- The synthesis runs at the merged-evidence base 9f5eaffff; the release validation (APP-001/003) ran against the rust-v0.1.0 artifact built from tag-point 23b7f6c27 (ancestor of the base).
- A finite benchmark is not mathematical proof of the task space (VALIDATION-PROGRAM §12; TASK-TAXONOMY §7); the verdict is an empirical determination, as the work order requires.

## Escalation path (NORTH-STAR-VALIDATION §6)

Operator-delivered held-out briefs (catalog §7 procedure) → VWO-016
execution → reconciliation. If held-out results match the authored-tier
performance with no new architecture family, the verdict escalates to
`NORTH_STAR_SUPPORTED` (with MCP/E2B/platform rows remaining explicitly
bounded).
