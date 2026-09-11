# VWO-003 — Canonical Teaching-Mode Matrix

**Program:** `docs/validation/VALIDATION-PROGRAM.md` §6 (teaching-mode coverage)
**Catalog:** `docs/validation/scenarios/SCENARIO-CATALOG.md`
**Established by:** VWO-003 (Wave 0)

This matrix is **normative**: a worker assigned a scenario and a teaching mode
reads the mode's cell below and knows their exact procedure, what the human
sees, and what evidence is recorded. Mode definitions are quoted from
VALIDATION-PROGRAM.md §6 and are not re-interpreted here.

Every worker must complete at least one workflow in **each** of the three
modes (VALIDATION-PROGRAM.md §6). The catalog assigns a canonical mode per
scenario (its `teaching-mode:` binding); running a scenario in a different
mode is allowed but must be stated in the report with a reason — the mode
mismatch is a validator warning (`scripts/validate-report.py --catalog
--strict` fails on it).

For adversarial catalog entries, the mode below is the mode used to
**establish** the workflow before the attack; the attack phase follows the
same evidence rules.

---

## 1. The matrix (mode × procedure)

| | DEMONSTRATE | INSTRUCT | HYBRID |
|---|---|---|---|
| **Worker does** | Performs the process in the application, step by step, with the teaching surface observing; never pre-builds internal workflow IR | Describes the desired process in natural language; then evaluates what the system inferred (semantics, dependencies, resources, conditions, approvals) | Gives initial instructions, THEN demonstrates critical steps/exceptions in the application; checks reconciliation |
| **Human sees** | Their own normal work, recorded: pages, forms, flashes, terminal output — plus a compiled workflow proposal at the end | Their instructions, plus the system's inferred workflow (steps/gates/triggers) presented for review — no application walk-through required | Their instructions AND a demonstration of the tricky parts, then a merged workflow that must visibly reconcile both |
| **Evidence recorded** | user-path: screenshots/transcripts of every performed step + the proposal; post-hoc: state/event pulls | user-path: the instruction text + the inferred-workflow review surface; post-hoc: inferred-semantics export, API/state checks | user-path: instruction text + demonstrated steps + the reconciliation view; post-hoc: diff between instructed and demonstrated semantics, state/event pulls |
| **Pass requires** | Observation captured the real steps, gates and side-effects; compiled workflow matches what was demonstrated | Inference matches the described semantics, dependencies, resources, conditions and approvals — deviations are findings | Neither input silently overrides the other; conflicts surfaced; merged workflow correct |

---

## 2. DEMONSTRATE — exact procedure

**Definition (VALIDATION-PROGRAM.md §6):** "Worker performs the process in
the application and lets the system observe it."

**When to use:** catalog scenarios whose `teaching-mode:` is `DEMONSTRATE`
(e.g. `construction-daily-progress`, `rideshare-driver-onboarding`,
`media-editorial-publishing`, `software-engineering-onboarding`,
`marketplace-discover-install-configure`, and the DEMONSTRATE adversarial
establishments).

**Procedure (normative, numbered):**

1. **Preflight.** `bash docs/validation/proving-ground/capability_check.sh`
   (exit 0), `cd docs/validation/fixtures && ./run-all.sh && cd ../../..`,
   `bash docs/validation/scripts/reset-scenario.sh <family>`; note the exact
   repository SHA (`git rev-parse HEAD`).
2. **Open the teaching surface.** Use whatever Codex Universal product
   surface exists for teaching-by-demonstration. If no such surface exists,
   STOP and record "teaching surface missing" as a report finding — do not
   fabricate one and do not pre-construct internal workflow IR
   (VALIDATION-PROGRAM.md §8).
3. **Perform the scenario's user path verbatim** in the fixture app
   (browser pages/forms; the ForgeOps terminal console where the scenario
   binds it). Do not take shortcuts through API twins.
4. **Capture as you go** (not afterwards): every step's observable outcome
   through
   `capture-evidence.sh <VWO-ID> <scenario-id> user-path <file|-> stepNN-<slug>`
   — stepNN must match the numbered step in the catalog entry.
5. **Request the compiled workflow** through the product surface; capture
   the proposal/review screen as user-path evidence (this is what the human
   sees of the inference).
6. **Evaluate the proposal** against the catalog entry's "Expected
   observable outcomes" — did observation capture the steps, the approval
   gates, the triggers, the side-effects (notifications, digests)? Every
   mismatch is a finding (P1 if a gate or side-effect is missing on a core
   path).
7. **Post-hoc diagnostics only now:** event pulls, state checks, reset
   verification via
   `capture-evidence.sh <VWO-ID> <scenario-id> post-hoc <file|-> <slug>`.
8. **Write the report** per REPORT-SCHEMA.md; validate with
   `scripts/validate-report.py … --catalog …`.

**What the human sees (record it):** the application as the persona would
use it — login, forms, confirmation flashes, lists, maps, terminal output —
and at the end a workflow the system claims it learned.

**Mode-specific failure characteristics to probe (why this mode exists):**

- observation MISSING a step, gate, or side-effect that was clearly
  performed (under-capture);
- observation ADDING semantics the human never performed (over-capture);
- mis-ordered steps;
- approval gates inferred as automatic steps (gate erosion);
- triggers/schedules invented where the human performed a manual action.

---

## 3. INSTRUCT — exact procedure

**Definition (VALIDATION-PROGRAM.md §6):** "Worker describes the desired
process and evaluates whether the system correctly infers semantics,
dependencies, resources, conditions, and approvals."

**When to use:** catalog scenarios whose `teaching-mode:` is `INSTRUCT`
(e.g. `construction-safety-incident`, `software-pr-triage`,
`rideshare-support-escalation`, `media-breaking-news`,
`marketplace-fork-and-improve`, `marketplace-upgrade-rollback`, and the
INSTRUCT adversarial establishments).

**Procedure (normative, numbered):**

1. **Preflight** (same as DEMONSTRATE step 1).
2. **Open the teaching surface** for instruction-based creation. Missing
   surface → record, do not fabricate.
3. **Write the instruction text** for the scenario (the catalog entry's
   INSTRUCT quotes give the canonical content, e.g. "foreman reports,
   safety officer investigates and closes within 48h"). Paste the exact
   instruction text into the report — it is user-path evidence of what the
   human gave the system.
4. **Submit the instructions** and capture the system's response — the
   inferred workflow review (steps, dependencies, resources, conditions,
   approvals, triggers) as user-path evidence.
5. **Evaluate the inference field by field:**
   - semantics: are the steps the described steps?
   - dependencies: were referenced resources/apps identified?
   - resources: capabilities and bindings inferred?
   - conditions: thresholds/validity gates (expired contract, missing doc)
     present?
   - approvals: are human gates preserved (not automated away)?
   - triggers/schedules: inferred where described (48h timer, monthly,
     event-driven) and NOT inferred where the human said manual?
   Every mismatch is a finding; classify per REPORT-SCHEMA severity.
6. **Verify against the live app where cheap:** perform one or two of the
   described steps in the fixture app to confirm the inferred gates behave
   (e.g. the foreman-closed-blocked flash) — captured as user-path
   evidence.
7. **Post-hoc diagnostics** (same rule: only after the human-path attempt).
8. **Write the report** and validate it (same as DEMONSTRATE step 8).

**What the human sees (record it):** their own words going in; a structured
workflow proposal coming back for review; the review surface is the product
surface — inference quality IS the product behavior under test.

**Mode-specific failure characteristics to probe:**

- missing approval gates (instruction said "code owner must approve", the
  inference automated it);
- hallucinated triggers or resources never mentioned;
- conditions dropped (amount thresholds, validity gates);
- dependency confusion (wrong app/resource bound);
- silent semantic changes when instructions are re-submitted with edits.

---

## 4. HYBRID — exact procedure

**Definition (VALIDATION-PROGRAM.md §6):** "Worker gives initial
instructions and demonstrates critical steps/exceptions." The worker MUST
verify that instruction and demonstration are reconciled rather than one
silently overriding the other (VALIDATION-PROGRAM.md §6, binding).

**When to use:** catalog scenarios whose `teaching-mode:` is `HYBRID`
(e.g. `construction-procurement-approval`, `software-incident-response`,
`rideshare-surge-ops`, `media-content-repurposing`,
`marketplace-create-and-sell`, and the HYBRID adversarial establishments).

**Procedure (normative, numbered):**

1. **Preflight** (same as DEMONSTRATE step 1).
2. **Open the teaching surface** supporting both instruction input and
   demonstration. Missing either half → record as a finding (the mode
   cannot be exercised fully — that is a product gap, P1 if no workaround).
3. **Phase 1 — instruct:** submit the initial instruction text (catalog
   entries quote canonical content, e.g. "approve PRs up to $50k when the
   vendor contract is valid; route to finance above"). Capture the
   instruction and the initial inference as user-path evidence
   (`…-instruct-phase`).
4. **Phase 2 — demonstrate the critical steps/exceptions named in the
   catalog entry** (e.g. the expired-contract rejection path; the terminal
   promote step; the duplicate-post exception). Perform them in the
   application exactly as in DEMONSTRATE; capture each step
   (`…-demo-phase`).
5. **Phase 3 — reconcile (the binding check):** request the merged workflow
   and capture the reconciliation view. Then verify explicitly:
   - every instruction-level rule still present after the demonstration
     (demonstration must not override instruction);
   - every demonstrated exception present as a real branch/gate
     (instruction must not override demonstration);
   - conflicts between the two are SURFACED for the human, not silently
     resolved.
   Record the reconciliation verdict in the report; a silent override is a
   P1 (or P0 if it erases an approval gate).
6. **Post-hoc diagnostics** (only after phases 1–3).
7. **Write the report** and validate it (same as DEMONSTRATE step 8).

**What the human sees (record it):** their instructions, their
demonstration, and a merged workflow that visibly explains how both were
combined — including any conflict the system detected.

**Mode-specific failure characteristics to probe:**

- demonstration silently overriding instructions (or vice versa);
- conflicts resolved silently in either direction;
- mixed-modality steps (browser + terminal) dropped or mangled in the
  merge;
- exceptions demonstrated but generalized into the golden path (or the
  reverse: golden path contaminated by the exception).

---

## 5. Evidence requirements per mode (summary table)

| Mode | user-path/ (what the human saw) | post-hoc/ (diagnostics) |
|---|---|---|
| DEMONSTRATE | one artifact per numbered user-path step (`stepNN-` prefix); compiled-workflow proposal/review screen | event pulls, state checks, reset verification, inferred-semantics export |
| INSTRUCT | exact instruction text; inferred-workflow review surface; any live-app verification steps | inferred-semantics export diff vs instruction, state/event checks, reset verification |
| HYBRID | phase-tagged artifacts: `…-instruct-phase`, `…-demo-phase` (with `stepNN-` for demonstrated steps), reconciliation view | instruction-vs-demonstration semantic diff, state/event checks, reset verification |

All artifacts are captured through
`docs/validation/scripts/capture-evidence.sh` (timestamped names, secret
refusal); naming and layout rules are in `docs/validation/evidence/README.md`;
report rules are in `docs/validation/reports/REPORT-SCHEMA.md`.

---

## 6. Mode assignment across the catalog (the third axis)

The catalog fixes one canonical mode per scenario (its `teaching-mode:`
binding; see SCENARIO-CATALOG.md §1). Counts: DEMONSTRATE ×8, INSTRUCT ×9,
HYBRID ×9 — each wave-1/2 owner can cover all three modes from its own
scenarios:

| Owner | DEMONSTRATE | INSTRUCT | HYBRID |
|---|---|---|---|
| VWO-004 | `construction-daily-progress`, `software-engineering-onboarding`, `rideshare-driver-onboarding`, `media-editorial-publishing` | `construction-safety-incident`, `software-pr-triage`, `rideshare-support-escalation`, `media-breaking-news` | `construction-procurement-approval`, `software-incident-response`, `rideshare-surge-ops`, `media-content-repurposing` |
| VWO-005 | (may borrow any catalog DEMONSTRATE scenario, e.g. `marketplace-discover-install-configure`) | `marketplace-fork-and-improve` | `marketplace-create-and-sell` |
| VWO-006 | `marketplace-discover-install-configure` | `marketplace-upgrade-rollback` | (may borrow any catalog HYBRID scenario, e.g. `marketplace-create-and-sell`) |
| VWO-007 | `adv-session-loss-restart`, `adv-cancellation-race` | `adv-provider-failure` | `adv-duplicate-triggers` |
| VWO-008 | `adv-approval-race` | `adv-prompt-injection` | `adv-credential-exfiltration` |
| VWO-009 | (may borrow, e.g. `marketplace-discover-install-configure`) | `adv-version-confusion` | `adv-environment-failure-rebinding`, `adv-marketplace-entitlement-race` |

VWO-005/006/009 each hold at least one INSTRUCT and one HYBRID (or DEMONSTRATE)
scenario natively; the "may borrow" cells exist because their Work Orders
mandate additional workflows in specific modes beyond the catalog anchors —
borrowing any catalog scenario in the required mode satisfies coverage
without inventing un-cataloged scenarios. Any run in a non-canonical mode
must be recorded as such (validator warning → strict failure if unexplained).
