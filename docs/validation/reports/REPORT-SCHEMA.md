# VWO-003 — Canonical Worker Report Schema

**Extends:** `docs/validation/worker-report-template.md` (verbatim template,
machine-checkable rules added)
**Established by:** VWO-003 (Wave 0)
**Enforced by:** `python3 docs/validation/scripts/validate-report.py <report.md>
[--catalog docs/validation/scenarios/SCENARIO-CATALOG.md] [--strict]`

This schema makes worker reports mechanically comparable across workers.
The worker report template remains the human-readable contract; this
document defines the normative field rules the Tech Lead (and the validator)
will hold every report to. The final acceptance question every finding must
be answerable against:

> **Can a normal person use Codex Universal end-to-end without understanding
> its internals?**

---

## 1. Report classes

| Class | Who writes it | Scenario blocks? |
|---|---|---|
| **Class A — scenario run report** | every worker who executes catalog scenarios (VWO-004 … VWO-009, and revalidation runs) | yes — one or more |
| **Class B — infrastructure Work-Order report** | wave-0 infrastructure and synthesis WOs (VWO-003 itself; VWO-010) | no — Identity-level checks only |

Class A reports are the normative form of `worker-report-template.md` and
MUST carry every field in §2–§12 below. Class B reports MUST carry the
Identity block (§2) with exact SHAs, any findings classified per §13, and
evidence pointers that exist; they omit the scenario-scoped sections (there
is no scenario run to report). VWO-001/VWO-002 reports predate this schema
and are grandfathered. The validator auto-detects the class (presence of a
`- Scenario id:` line ⇒ Class A).

A Class A report may contain MULTIPLE scenario blocks (one per catalog
scenario executed, in execution order). Global sections (Identity) appear
once, before the first scenario block.

## 2. Structure (machine-checked)

Heading order, per report (Class A shown; Class B stops after Identity):

```markdown
# <Title>

## Identity                    ← global, once
- Work Order: VWO-00n
- Worker persona: …
- Base branch: main
- Base SHA: <40-hex>
- Head SHA: <40-hex | branch-reference>
- Validation environment: …
- E2B template/workspace identity, if used: …   (optional)
- Codex Universal runtime/application SHA: <40-hex | NONE — reason>
- Date/time window: <UTC range>

## Scenario                    ← scenario block starts (repeat per scenario)
- Scenario id: <catalog slug>
- Industry: …
- Scenario: <title>
- Teaching mode: DEMONSTRATE | INSTRUCT | HYBRID
- User goal: …

## User Path
1. <numbered, ≥3 steps — human-observable steps actually performed>

## Expected
<non-empty>

## Actual
<non-empty — exactly what happened>

## Workflow Identity
- Workflow: …
- Version: …
- Source revision: …
- Definition digest: …
- Dependency-lock identity: …

## Runtime Binding
- Capabilities: …
- Resources: …
- Environments: …
- Approvals: …
- Triggers/schedules: …

## Evidence
- docs/validation/evidence/<VWO-ID>/<scenario-id>/user-path/<file>
- docs/validation/evidence/<VWO-ID>/<scenario-id>/post-hoc/<file>

## Failure / Recovery
- Failure injected: …
- Recovery attempted: …
- Restart/session-loss behavior: …
- Final outcome: …

## Product / UX Friction
<non-empty — may be "none observed" + why>

## Security / Architecture Findings
- Finding: …
- Severity: P0 | P1 | P2 | P3 | (or the literal line "- No findings")
- Reproduction: …
- Root cause: …
- Frozen invariant affected: …

## Recommendation
- FIX NOW | NEW WORK ORDER | DEFER
- Proposed owner: …
- Verification required: …

## Worker Conclusion
<non-empty — must state passed / failed / blocked>
```

Heading set and order are validated exactly. Field lines (`- Field:`) are
required in every section where listed; `E2B template/workspace identity`
is optional ("if used"). Multiple findings repeat the whole
`Finding/Severity/Reproduction/Root cause/Frozen invariant affected` group.

## 3. Identity fields (exact SHA discipline — non-negotiable)

- **Base SHA** — the exact commit the worker branched from; 40 hex chars.
  Verify with `git rev-parse <sha>^{commit}` before starting.
- **Head SHA** — the exact commit containing the worker's evidence/report.
  Because a commit cannot contain its own hash, the accepted forms are:
  (a) the 40-hex SHA of the head commit (from `git rev-parse` after
  committing, written via a follow-up note/commit or by the Tech Lead), or
  (b) an explicit branch reference: `see git rev-parse <branch> (single
  commit <short-sha> on base <base-short>)`. Form (b) records intent
  without fabricating a hash. [CHECKER: 40-hex passes; branch-reference
  passes with a warning; anything else fails]
- **Codex Universal runtime/application SHA** — the exact SHA of the
  application/runtime build under test. If the runtime surface was absent
  in this environment (fallback proving ground), record `NONE — <reason>`;
  never invent a SHA. [CHECKER: 40-hex passes; `NONE — reason` warns; else
  fails]
- **Base branch, Validation environment, Date/time window (UTC), Work
  Order, Worker persona** — free text, non-empty. The date/time window MUST
  be UTC (the program timezone of record).

## 4. Scenario block fields

- **Scenario id** — MUST be a catalog slug (`[a-z0-9-]+`). With
  `--catalog`, the validator checks it exists in
  `docs/validation/scenarios/SCENARIO-CATALOG.md`. Non-catalog scenarios
  are non-conforming: add them to the catalog first.
- **Teaching mode** — exactly one of `DEMONSTRATE`, `INSTRUCT`, `HYBRID`.
  With `--catalog`, a mode that differs from the catalog binding is a
  warning (strict mode: failure) — record the reason in the report.
- **Industry, Scenario, User goal** — non-empty.

## 5. User Path

Numbered list, at least 3 steps, each a human-observable step ACTUALLY
performed (browser/terminal surface). Internal API calls are not user-path
steps. If the path was cut short by a missing surface, the last step must
say so ("step 6 not performed — teaching surface missing, see Actual").

## 6. Expected vs Actual

Both non-empty. Expected states what a human should see (per the catalog
entry). Actual states exactly what happened — including verbatim error
text where relevant. Findings cite the delta between these.

## 7. Workflow Identity (per scenario block)

- **Workflow** — name/id of the workflow under test.
- **Version** — its immutable version at test time.
- **Source revision** — pinned source revision the workflow executes.
- **Definition digest** — content digest of the workflow definition.
- **Dependency-lock identity** — identity of the pinned dependency lock.
Where the product surface does not yet expose a field, record
`NONE — surface missing` and classify the gap as a finding (missing
product surface, typically P1/P2) — never fabricate values.

## 8. Runtime Binding (per scenario block)

- **Capabilities** — capabilities the workflow requires/was granted.
- **Resources** — bound resources (apps, accounts, data).
- **Environments** — execution environments and their identity.
- **Approvals** — approval gates exercised and their outcomes.
- **Triggers/schedules** — trigger/schedule semantics exercised.

## 9. Evidence pointers (per scenario block)

- Bullet list of repo-relative paths under
  `docs/validation/evidence/` (Class A) — each path MUST exist in the
  repository. [CHECKER]
- Every user-path artifact referenced must live in the scenario's
  `user-path/` directory; post-hoc in `post-hoc/` (layout:
  `docs/validation/evidence/README.md`).
- A scenario block with NO user-path evidence pointer is a warning —
  either the run had no observable surface (then say so in Actual) or the
  human path was skipped (non-conforming, fix the run).
- Class B reports: point at `docs/validation/evidence/<VWO-ID>/_infra/**`
  (or explicitly state no evidence).
- Exception: the worked example's pointers live under
  `docs/validation/examples/evidence/` (an example-only location the
  validator accepts so the example stays conforming); REAL reports use
  `docs/validation/evidence/` only.
- Never place secrets in reports. [CHECKER: secrets scan]

## 10. Failure / Recovery

All four sub-fields present (fill with `not attempted — <reason>` where a
scenario had no injected failure). Adversarial scenarios must fill these
with the actual injected failure and recovery result.

## 11. Recommendation

Exactly one disposition keyword per finding group: `FIX NOW`, `NEW WORK
ORDER`, or `DEFER`. P0/P1 findings may never be `DEFER`
(VALIDATION-PROGRAM.md §10) — the validator flags that combination as a
failure.

## 12. Worker Conclusion

Non-empty; states `passed`, `failed`, or `blocked` (by a missing product
surface) for the scenario. [CHECKER: keyword presence]

## 13. Severity vocabulary (normative definitions)

| Level | Definition | Program §10 mapping |
|---|---|---|
| **P0** | security violation / data loss / product-blocking defect on a core human path | core product blocked; security/authorization failure; workflow authority violation |
| **P1** | core-path defect WITH a workaround, OR blocking defect on a secondary path | major functionality/reliability defect |
| **P2** | degraded usability / friction / non-blocking incorrectness | meaningful but non-blocking product defect |
| **P3** | polish / cosmetic / documentation gap | polish/documentation issue |

Binding rules:

1. Every finding's `Severity:` line carries exactly one of `P0 P1 P2 P3`
   (no compound values). [CHECKER]
2. The finding text MUST tie back to the acceptance question: state which
   step a normal person could not complete, or which internal they would be
   forced to understand. A finding that cannot cite its failing question is
   incomplete — rewrite it until it can.
3. P0/P1 ⇒ `Recommendation` must be `FIX NOW` or `NEW WORK ORDER` (never
   DEFER), and the report must propose an owner + verification.
   [CHECKER: P0/P1 + DEFER = failure]
4. Zero-findings scenario blocks carry the literal line `- No findings`
   under Security / Architecture Findings. [CHECKER]

## 14. Validation workflow

```bash
# after writing the report (from the repository root):
python3 docs/validation/scripts/validate-report.py \
    docs/validation/reports/VWO-004-report.md \
    --catalog docs/validation/scenarios/SCENARIO-CATALOG.md
# exit 0 = conforming (warnings allowed); 1 = violations; 2 = usage error
# add --strict to fail on warnings (SHA placeholders, mode mismatches,
# missing user-path evidence pointers)
```

Checks performed: heading set and order; required field lines; SHA
field format (40-hex / NONE-reason / branch-reference); scenario id
legality and (with `--catalog`) catalog existence + mode match; teaching
mode legality; ≥3 numbered user-path steps; severity legality; P0/P1-DEFER
combination ban; evidence path existence and evidence/ prefix; conclusion
keyword; secrets scan across the report and every referenced evidence
file.

A worked Class A example: `docs/validation/examples/VWO-XXX-example-report.md`
(with example evidence files under
`docs/validation/examples/evidence/VWO-XXX/`). The VWO-003 report itself is
the Class B example: `docs/validation/reports/VWO-003-report.md`.
