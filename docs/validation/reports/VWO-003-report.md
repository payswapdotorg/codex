# VWO-003 Report — Validation Harness, Scenario Catalog, and Evidence

## Identity

- Work Order: VWO-003 (Wave 0, `docs/validation/work-orders/VWO-003.md`)
- Worker persona: VWO-003 Worker (validation-harness specialist)
- Base branch: main
- Base SHA: e2a5e968581e4feb60b754a848824d7c05a583a9
- Head SHA: see git rev-parse vwo-003/validation-harness — single delivery commit on top of base e2a5e968581e4feb60b754a848824d7c05a583a9 (exact full SHA recorded in the delivery bundle and the completion message; a commit cannot embed its own hash)
- Validation environment: agent sandbox fallback proving ground (VWO-001 record; node v24.19.0, bun 1.3.14, python 3.12.14; browser + terminal + file API verified by capability_check.sh per VWO-001)
- E2B template/workspace identity, if used: N/A — E2B/Composio integration absent (VWO-001 §2 fallback record governs)
- Codex Universal runtime/application SHA: NONE — documentation + schema + light-scripts Work Order: no Codex Universal runtime build or launch belongs to VWO-003 scope (zero codex-rs changes; the repo's buildability in this sandbox is VWO-001's record, `evidence/VWO-001/cargo-check-family.txt`)
- Date/time window: 2026-09-11T17:55Z–2026-09-11T18:10Z (UTC)

---

## 1. Objective and outcome

Create repository-native scenario definitions, evidence schemas, reporting
conventions, and reusable validation helpers so all workers produce
comparable evidence without bypassing product surfaces.

**Outcome: implemented and verified.** The canonical scenario catalog
(26 scenarios: 16 industry + 10 adversarial, all bound to VWO-002 fixtures),
the normative teaching-mode matrix, the evidence layout with the
user-path/post-hoc separation, the report schema with severity vocabulary,
one worked scenario example + one conforming example report with example
evidence, and three dependency-free helpers (scenario reset, evidence
capture with secrets refusal, report validation) are committed under
`docs/validation/`. Everything composes — nothing duplicates — VWO-001's
proving ground and VWO-002's fixture ecosystem.

## 2. Deliverables (changed files/surfaces)

All paths relative to the repository root; zero changes to `codex-rs/**`,
zero changes to `docs/validation/fixtures/**` (VWO-002's surface),
`docs/validation/proving-ground/**` (VWO-001's surface), prompts, program
spec, work orders, or `worker-report-template.md`:

- `docs/validation/scenarios/SCENARIO-CATALOG.md` — canonical catalog: §1
  selection table (select-and-run in minutes), §2 runbook, §3 entry format
  (machine-checkable binding block), §4 16 industry scenarios, §5 10
  adversarial scenarios, §6 coverage map to prompt-02 / program §7.
- `docs/validation/scenarios/TEACHING-MODE-MATRIX.md` — DEMONSTRATE /
  INSTRUCT / HYBRID × (worker does / human sees / evidence recorded), plus
  normative numbered procedures per mode and the owner×mode assignment
  table.
- `docs/validation/evidence/README.md` — canonical evidence layout
  (`evidence/<VWO-ID>/<scenario-id>/{user-path,post-hoc}/`), naming
  convention (UTC timestamps, stepNN, phase tags, no `.log`), size rules,
  credentials ban with checker enforcement, redaction guide, VWO-001
  grandparenting + `_infra` convention.
- `docs/validation/reports/REPORT-SCHEMA.md` — normative report schema
  extending `worker-report-template.md` (Class A scenario reports / Class B
  infra reports; exact SHA discipline; P0–P3 severity definitions; findings
  must cite the failing acceptance question; validator usage).
- `docs/validation/examples/scenario-construction-daily-progress.md` — ONE
  filled example scenario file (format disambiguation).
- `docs/validation/examples/VWO-XXX-example-report.md` — ONE minimal
  example report (conforming Class A, passes the validator against the real
  catalog).
- `docs/validation/examples/evidence/VWO-XXX/construction-daily-progress/{user-path,post-hoc}/*` —
  three example evidence files the example report references (paths exist;
  validator-clean).
- `docs/validation/scripts/reset-scenario.sh` — restart + reseed ONE
  fixture app (composes run-all.sh; `--all`/`--stop-all` delegate to it).
- `docs/validation/scripts/capture-evidence.sh` — timestamped copy into
  the correct `user-path/` or `post-hoc/` directory with fail-closed
  secrets refusal.
- `docs/validation/scripts/validate-report.py` — report schema lint
  (sections, SHAs, severities, evidence paths, catalog ids, secrets scan).
- `docs/validation/reports/VWO-003-report.md` — this report.
- `docs/validation/evidence/VWO-003/_infra/post-hoc/*` — three self-test
  evidence artifacts (live harness run, secrets-refusal demo, verification
  transcript).

## 3. Design decisions (recorded, not silent)

1. **Catalog = registry, not runtime.** Scenario entries are definitions +
   bindings; nothing in the harness schedules, executes, or orchestrates
   workflows (Forbidden list). The runbook (catalog §2) composes
   `capability_check.sh` (VWO-001), `run-all.sh`/`verify-sweep.sh` (VWO-002)
   and the VWO-003 helpers by path reference only.
2. **26 scenarios, not the minimum.** The WO required ≥1 per industry family
   + ≥3 adversarial. The catalog covers all five industry families (3–4
   scenarios each, mirroring program §7's scenario families exactly) AND
   all TEN adversarial items from prompt-02's list, so the catalog↔prompt
   cross-check is total (10/10, no gaps) rather than partial.
3. **Report classes.** The WO's report schema is scenario-run-shaped, but
   VWO-001/VWO-002 (already Tech-Lead-verified) wrote infra-shaped reports.
   REPORT-SCHEMA.md therefore defines Class A (scenario runs — the full
   normative template; mandatory for VWO-004+) and Class B (infra/synthesis
   WOs — Identity rigor + severity legality + evidence pointers; VWO-003
   itself is the Class B example). VWO-001/002 reports are grandfathered.
4. **Secrets discipline extends to fixture demo tokens.** The checker
   allows `demo-pass-*` (published synthetic passwords) but still demands
   redaction of session-token values (`token=<redacted>`) — deliberate
   friction to build the redaction habit for real-secret environments
   (evidence/README.md §5).
5. **`*.log` is banned in evidence** — the repository `.gitignore` excludes
   it (VWO-001 learned this); transcripts use `.txt` and
   capture-evidence.sh renames any `.log` source.

## 4. Implementation summary

- The **catalog** anchors every scenario id to a VWO-002 fixture app
  (family dir + port + terminal surface where present, e.g.
  `software/deploy-console.js` for the two terminal-bound scenarios) with a
  machine-parseable yaml binding block per entry. Each entry defines all
  ten required fields: id, persona (named seeded user), user goal, teaching
  mode, bound app, user-path steps (human-observable UI steps incl. the
  Codex Universal product-surface operations, with "missing surface →
  record, don't fabricate" as a binding rule), expected observable
  outcomes, failure switches + recovery expectations (switch names AND
  natural seed cases with exact ids: prq-5001, da-0302, da-0303, st-0205,
  ent-0502, ins-0401, …), evidence to capture (user-path vs post-hoc), and
  approvals/triggers exercised. Primary-owner assignment maps wave-1/2 Work
  Orders (VWO-004…009) onto the catalog so every worker owns a
  mode-complete scenario set.
- The **teaching-mode matrix** is normative per cell: definitions quoted
  from program §6; numbered procedure, what the human sees, evidence per
  mode, pass criteria, and mode-specific failure characteristics
  (under/over-capture, gate erosion for DEMONSTRATE; inference fidelity for
  INSTRUCT; silent-override reconciliation for HYBRID).
- The **evidence layout** fixes `user-path/` = only what the human saw on
  the product surface, `post-hoc/` = diagnostics, with a 4-rule normative
  separation, UTC-timestamp naming, size/type rules, and the
  checker-enforced credentials ban.
- The **report schema** extends the verbatim template with machine-checkable
  rules and the normative severity vocabulary (P0 security/data-loss/
  core-path blocking; P1 core-path-with-workaround or secondary-path
  blocking; P2 degraded usability/non-blocking incorrectness; P3 polish/
  cosmetic/doc gap), the P0/P1-never-DEFER rule, and the requirement that
  findings cite the failing acceptance question.
- The **helpers** are dependency-free (bash + python3 std-lib; no new
  packages, no node_modules, no codex-rs): reset-scenario.sh mirrors
  run-all.sh's app map/pid/log conventions (verified against it),
  capture-evidence.sh scans before writing (fail closed), validate-report.py
  implements the §14 check list.

## 5. Verification — commands and exact results

Full transcript captured at
`docs/validation/evidence/VWO-003/_infra/post-hoc/20260911T180915Z-verification-transcript.txt`;
live harness run at `20260911T180656Z-harness-self-test.txt`; refusal demo
at `20260911T180730Z-secrets-refusal-demo.txt`. Summary (all from the
sandbox, base e2a5e968581e4feb60b754a848824d7c05a583a9):

1. **Script syntax (the WO's required static verification):**
   - `bash -n docs/validation/scripts/reset-scenario.sh` → exit 0 (OK)
   - `bash -n docs/validation/scripts/capture-evidence.sh` → exit 0 (OK)
   - `python3 -m py_compile docs/validation/scripts/validate-report.py` → exit 0 (OK)
   - `node --check` → **N/A**: VWO-003 adds no `.js` files (helpers are
     bash + python3; 0 modified `.js` files in the delivery).
2. **Validator on the worked example (with catalog):**
   `python3 docs/validation/scripts/validate-report.py
   docs/validation/examples/VWO-XXX-example-report.md --catalog
   docs/validation/scenarios/SCENARIO-CATALOG.md` →
   `RESULT: PASS — Class A, 1 scenario block(s), 1 warning(s)`, exit 0.
   The one warning is the example's deliberate `NONE —` runtime SHA
   placeholder (honest by design; strict mode fails on it).
3. **Negative tests (corrupted copies must FAIL, all exit 1):**
   C1 illegal severity `P9` → exit 1 · C2 P1 finding DEFERred → exit 1 ·
   C3 dangling evidence pointer → exit 1 · C4 user path with <3 numbered
   steps → exit 1 · C5 scenario id not in catalog → exit 1 · C6 secret in
   a referenced evidence file → `[FAIL] …leak.txt:1: secret-looking
   content…` exit 1 (scratch contamination dir removed, never committed).
4. **Catalog↔fixture cross-check (no dangling bindings):** parsed the 26
   yaml binding blocks (the §3 format template excluded): 26/26 `app-dir`
   directories exist; ports match `fixtures/run-all.sh`'s APPS map exactly;
   the two non-`none` terminal surfaces exist
   (`software/deploy-console.js`); owners/modes/industries all legal;
   scenario ids unique. → OK.
5. **Catalog↔prompt-02 cross-check (recorded):** industry families present
   with counts {construction 3, software 3, rideshare 3, media 3,
   marketplace 4}; adversarial entries 10, prompt-02's nine adversarial
   items (restart/session-loss, duplicate triggers, provider/environment
   failure, prompt injection, credential exfiltration, authorization
   races, version confusion, cancellation races, marketplace entitlement
   races — program §7's ten cross-cutting items) covered **10/10, missing:
   none**; teaching-mode coverage DEMONSTRATE ×8 / INSTRUCT ×9 / HYBRID ×9;
   the §1 selection table is consistent with the binding blocks (26 rows,
   industries, modes).
6. **Live harness self-test (new-worker test, single invocation):**
   `reset-scenario.sh construction` handled a stale pid, started the app,
   passed the health check, reset to seed (`world.reset` event recorded);
   `healthz` returns the exact app identity the catalog binds
   (construction / SiteBuild Field Suite / 4101); `capture-evidence.sh`
   wrote the canonical timestamped path
   `…/VWO-003/_infra/post-hoc/20260911T180656Z-harness-self-test.txt`;
   the contaminated-capture demo exited 1 with NOTHING written (fail
   closed); `--stop` handled both a live pid and a missing pid file.
7. **Checker self-consistency (notable):** the first capture attempt of the
   verification transcript was itself REFUSED — the transcript quoted the
   C6 fake AKIA key verbatim, and capture-evidence.sh has no self-exemption.
   The quoted value was redacted (`AKIA<example-redacted>`) and re-captured;
   the refusal is noted inside the committed transcript. Fail-closed works
   on its own evidence.

## 6. Required-outcomes evidence (packet → artifacts)

| Required outcome | Evidence |
|---|---|
| Canonical scenario catalog covering construction, software, ride-share, media, marketplace, and adversarial scenarios | `docs/validation/scenarios/SCENARIO-CATALOG.md` — 26 entries (16 industry + 10 adversarial); cross-checks in §5 above |
| Canonical teaching-mode matrix for DEMONSTRATE, INSTRUCT, HYBRID | `docs/validation/scenarios/TEACHING-MODE-MATRIX.md` (normative procedures + owner×mode table; catalog mode coverage 8/9/9) |
| Worker report schema with exact repository/environment SHA identity | `docs/validation/reports/REPORT-SCHEMA.md` §§2–3 (40-hex discipline, NONE-with-reason, branch-reference form); enforced by validate-report.py |
| Evidence directory layout under `docs/validation/evidence/` and reports under `docs/validation/reports/` | `docs/validation/evidence/README.md` §§1–3; live layout `evidence/VWO-003/_infra/post-hoc/` (3 artifacts); reports/ gains REPORT-SCHEMA.md + this report |
| Optional lightweight helpers/scripts for workspace setup, scenario reset, timestamped evidence, and report validation | `docs/validation/scripts/{reset-scenario.sh,capture-evidence.sh,validate-report.py}` — dependency-free; runbook composition in catalog §2 |
| Explicit distinction between user-path evidence and post-hoc diagnostic evidence | `evidence/README.md` §2 (4 normative rules incl. "API JSON the UI never renders is post-hoc"); enforced per-capture by capture-evidence.sh's class argument + validator warning on user-path-less scenario blocks |
| Severity vocabulary P0/P1/P2/P3 | `REPORT-SCHEMA.md` §13 (normative definitions + program §10 mapping + P0/P1-never-DEFER rule); validator checks legality (negative test C1) |

## 7. Acceptance evidence — the new-worker test

*"A new worker can select a scenario, understand its user persona/goal/
teaching mode, record exact steps and outcomes, attach diagnostics safely,
and produce a report that the Tech Lead can compare across workers."*
Mechanical support demonstrated:

1. **Select a scenario:** catalog §1 selection table (id, industry, mode,
   app, port, terminal surface, owner) + §2 runbook; live:
   `reset-scenario.sh construction` → healthy, seeded (self-test
   transcript, exit 0; stale-pid path exercised).
2. **Understand persona/goal/mode:** each catalog entry carries Persona
   (named seeded user + role), User goal, Teaching mode; the mode's exact
   procedure is TEACHING-MODE-MATRIX.md §§2–4; the fully-annotated
   worked example is `examples/scenario-construction-daily-progress.md`.
3. **Record exact steps and outcomes:** catalog entries carry numbered
   human-observable user-path steps + expected observable outcomes;
   capture-evidence.sh stamps `stepNN-` prefixed user-path artifacts
   (demonstrated: `20260911T180656Z-harness-self-test.txt` capture);
   validate-report.py enforces ≥3 numbered steps + Expected vs Actual
   (negative test C4).
4. **Attach diagnostics safely:** post-hoc class separation enforced per
   capture; secrets refusal demonstrated twice (contaminated file → exit
   1, nothing written; the transcript self-refusal in §5.7); redaction
   guide + allowlist in evidence/README.md §5.
5. **Produce a comparable report:** REPORT-SCHEMA.md fixes the structure;
   validate-report.py makes conformance mechanical (example report →
   `RESULT: PASS — Class A …` exit 0; six negative tests all exit 1); the
   severity vocabulary + acceptance-question citation rule make findings
   comparable across workers; the example report shows the exact shape.

## 8. Forbidden-list compliance

- **Treating the harness as a workflow runtime:** NONE — no scheduling,
  no triggers, no execution, no state machines anywhere in the delivery;
  the helpers move files and lint text. The catalog's binding rules point
  workers at product surfaces, never at harness internals.
- **Replacing human-path testing with internal API calls:** NONE — catalog
  user-path steps are browser/terminal observable steps; evidence/README §2
  makes API twins post-hoc-only; the validator warns on scenario blocks
  without user-path evidence; the live self-test used /healthz and
  /api/events only as harness self-diagnostics (post-hoc, labeled).
- **Storing credentials or secrets in evidence:** NONE — fail-closed
  checker on capture + validator re-scan of reports and referenced
  evidence; demonstrated by the refusal demo and the transcript
  self-refusal; all demo credential material in committed docs is
  placeholder/redacted (`token=<redacted>`, `demo-pass-*` documented as
  synthetic).
- **Mutating production semantic contracts merely to make tests pass:**
  NONE — zero `codex-rs/**` changes, zero fixture changes, zero changes to
  VALIDATION-PROGRAM.md / prompts / work orders / template; `git
  diff --stat` (see §10) touches only `docs/validation/{scenarios,evidence,
  reports,examples,scripts}/**` new files.

## 9. Security / Architecture Findings

- Finding: the secrets checker is a heuristic lint, not a security
  guarantee — documented false-negative space: bare `?token=` values (only
  `access[_-]?token`/`auth[_-]?token`/`authorization`/`bearer` key shapes
  are matched), non-text artifacts (.png etc. are not content-scanned),
  and novel credential shapes. Mitigated by the documented normative rule
  (redact all tokens) and by validator+capture double-scanning. A normal
  person is unaffected (internal tooling).
- Severity: P3
- Reproduction: capture a file containing `?token=abcdef123456` — the
  scanner does not flag the bare token key form (documented in
  scripts/capture-evidence.sh header and evidence/README.md §5).
- Root cause: deliberate heuristic scope to keep the checker
  dependency-free and false-positive-tolerant.
- Frozen invariant affected: none.

## 10. Verification of delivery scope

`git status --porcelain` before commit: only the files listed in §2 (all
new paths under `docs/validation/`). No fixture/proving-ground/program
files touched. No build artifacts; total new-file volume is small (text
only). Delivery bundle:
`git bundle create /home/z/my-project/vwo-003-delivery.bundle
e2a5e968581e4feb60b754a848824d7c05a583a9..vwo-003/validation-harness`.

## 11. Recommendation

- NEW WORK ORDER (owning: wave-1 dispatch) — VWO-004/005/006 are unblocked
  by this delivery per `validation-dependency-graph.json` (all three depend
  on VWO-001+002+003; 001 and 002 are MERGED, this closes wave 0).
- Proposed owner: Tech Lead (dispatch) → wave-1 workers (execution)
- Verification required: re-run the §5 checks on the merged commit; wave-1
  workers must produce Class A reports passing validate-report.py.

## 12. Known limitations

1. The catalog binds scenarios to the CURRENT VWO-002 seed ids (prq-5001,
   da-0302/0303, st-0205, ent-0502, ins-0401 …); seed changes in future
   VWO-002 revisions require a catalog refresh (same risk class VWO-002
   recorded for its own report).
2. Teaching-surface steps are named generically ("the product's teaching
   surface") because the Codex Universal product UI is whatever wave-1
   workers find on current main — the catalog's binding rule (record
   missing surfaces, never fabricate) is the honest handling; the example
   report demonstrates exactly this recording.
3. The validator's markdown parsing is line-based (headings/field-bullets);
   exotic formatting inside sections could confuse it — mitigated by the
   worked example + negative tests pinning the supported shapes.
4. `reset-scenario.sh`'s port map is duplicated from run-all.sh (with a
   documented MUST-mirror comment and a cross-check in §5.4 that verified
   they agree at delivery time); VWO-002 port changes require a one-line
   update here.

## 13. Deferred items + owning WO

- Live scenario RUNS against the Codex Universal product surface — owned
  by VWO-004…VWO-009 (wave 1/2); the harness only defines and enforces.
- Dependency-graph status flip for VWO-003 (blocked→merged) and VWO-004…
  006 (blocked→ready) — owned by the Tech Lead at merge/harvest time.
- Re-evaluation of scenario fidelity if E2B becomes connected (VWO-001
  check 7 flips) — owned by VWO-010 (synthesis) per VWO-001's risk note.

## 14. Risks

- **Workers bypassing the human path anyway.** The harness warns (missing
  user-path pointers) but a determined worker could fabricate "what the
  human saw" text. Mitigation: Tech Lead cross-checks user-path artifacts
  against fixture event feeds; the validator's warnings surface in every
  run.
- **Catalog↔product-surface drift:** if the product's teaching/compile
  surfaces land AFTER wave-1 dispatch, catalog step 6 phrasing ("teaching
  surface") stays valid (generic), but expected-outcome wording may need a
  refresh — small, owned by each scenario's primary owner via catalog
  additions/edits (format fixed in catalog §3).
- **Single-invocation constraint** (background reaping) may confuse
  workers used to long-lived servers; the runbook and proving-ground README
  §3.6 state it; reset-scenario.sh is safe to re-run (stale-pid handling
  proven live).

## 15. Worker Conclusion

VWO-003 is **implemented and verified**: the canonical catalog (26
scenarios, all fixture bindings live-checked, prompt-02 coverage total),
the normative teaching-mode matrix, the evidence layout with explicit
user-path/post-hoc separation and an enforced credentials ban, the report
schema with exact-SHA discipline and the P0–P3 severity vocabulary, the
worked examples, and the three dependency-free helpers — with static
verification (bash -n / py_compile), a passing validator run on the
example, six failing negative tests, two live cross-checks, and a live
harness self-test all recorded as committed evidence. Wave 0 is complete
with this delivery; VWO-004…006 become dispatchable.
