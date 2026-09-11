# VWO-003 — Canonical Evidence Layout

**Established by:** VWO-003 (Wave 0, `docs/validation/work-orders/VWO-003.md`)
**Program:** `docs/validation/VALIDATION-PROGRAM.md` §9 (evidence requirements)
**Consumers:** every validation worker (VWO-004+) and the Tech Lead

This directory holds ALL raw validation evidence. It is the ONLY place raw
evidence lives (reports under `docs/validation/reports/` reference it by
path). Rules below are normative and enforced where marked [CHECKER].

---

## 1. Directory layout

```text
docs/validation/evidence/
└── <VWO-ID>/                        # one directory per Work Order run
    ├── <scenario-id>/               # one directory per catalog scenario
    │   ├── user-path/               # ONLY what the human saw on the product surface
    │   └── post-hoc/                # everything else (diagnostics)
    └── _infra/                      # reserved: non-scenario evidence for infra WOs
        ├── user-path/               #   (rarely used)
        └── post-hoc/                #   transcripts, self-tests, probes
```

- `<VWO-ID>`: the Work Order id (e.g. `VWO-004`). One worker may accumulate
  many scenario directories under its id.
- `<scenario-id>`: EXACTLY the scenario id from
  `docs/validation/scenarios/SCENARIO-CATALOG.md` (`[a-z0-9-]+`). Scenario
  evidence may never live outside a scenario directory. [CHECKER:
  validate-report.py --catalog]
- `_infra`: reserved pseudo-scenario for Work Orders that produce
  non-scenario evidence (VWO-003 harness self-tests, VWO-001-style
  environment probes). It is the ONLY non-catalog directory name allowed
  under `<VWO-ID>/`.
- **Grandfathered:** `VWO-001/` predates this layout and holds flat files
  (environment-level evidence, no scenario binding existed then). It is
  valid as-is; do not move it. Everything from wave 1 on follows this
  layout.

## 2. user-path vs post-hoc — the explicit separation (the core rule)

| Class | Definition | Examples |
|---|---|---|
| `user-path/` | artifacts of what the human SAW or DID on the observable product surface, captured during the run | page screenshots; DOM/accessibility snapshots; visible flash/confirmation text transcripts; terminal console transcripts (the ForgeOps console IS a product surface); the compiled-workflow proposal/review screen; instruction text the human submitted |
| `post-hoc/` | everything captured AFTER (or outside) the human path — diagnostics | `/api/...` JSON pulls; event-feed dumps; state checks; reset verifications; digests; logs; inferred-semantics exports; diff analyses; restart transcripts |

Normative rules:

1. **A file is user-path ONLY if a human looking at the product surface at
   that moment could have seen its content.** API JSON that the UI never
   renders is post-hoc even if captured during the run.
2. **Post-hoc inspection is permitted only after the normal user path has
   been attempted** (VALIDATION-PROGRAM.md §8). If you need diagnostics to
   understand a failure, that is allowed — but the artifact goes to
   `post-hoc/` and the report says the user path was attempted first.
3. **Never replace human-path testing with API calls.** The JSON API twins
   exist for diagnosis, not for skipping the UI. A report whose user-path
   directory is empty (with no recorded "surface missing" finding) is
   non-conforming. [CHECKER: validate-report.py warns when a scenario block
   references no user-path evidence]
4. Mixed content: one artifact, one class. If a screenshot shows a page AND
   you need the underlying JSON, that is two artifacts in two directories.

## 3. Naming convention

```text
<YYYYMMDDTHHMMSS>Z[-stepNN][-phase][-slug].<ext>
```

- `<YYYYMMDDTHHMMSS>Z`: UTC timestamp of CAPTURE (not of the step).
  Example: `20260912T143005Z`.
- `stepNN`: user-path artifacts carry the catalog entry's numbered step
  (`step01-login`, `step04-submit-report`). Post-hoc artifacts omit it.
- `phase`: for HYBRID runs, tag `instruct-phase` / `demo-phase` before the
  slug (TEACHING-MODE-MATRIX.md §5).
- `slug`: lowercase `[a-z0-9-]`, ≤ 40 chars, describing the artifact.
- `ext`: `.txt` (transcripts, JSON pulls, command output), `.png`
  (screenshots), `.md` (notes), `.json` (machine-readable pulls). **NEVER
  `.log`** — the repository `.gitignore` excludes `*.log` (learned by
  VWO-001); log-type content goes in `.txt`. [CHECKER: capture-evidence.sh
  renames `.log` → `.txt`]

Use `scripts/capture-evidence.sh` to produce conforming names automatically:

```bash
bash docs/validation/scripts/capture-evidence.sh VWO-004 construction-daily-progress \
     user-path  screenshot.png  step03-report-form
# → docs/validation/evidence/VWO-004/construction-daily-progress/user-path/
#   20260912T143005Z-step03-report-form.png
```

## 4. Size, type, and volume rules (keep the repository lean)

- Text-first: prefer `.txt` transcripts of visible page text over
  screenshots when the content is textual; a screenshot is better for
  layout/framing questions.
- PNG screenshots ≤ ~500 KiB each (downscale before capture if needed).
- Any single artifact > 1 MiB is refused by capture-evidence.sh unless
  `--big` is passed (justified in the report).
- No build artifacts, no databases, no node_modules, no videos. Trim event
  pulls with `?limit=`/`?type=` instead of dumping whole feeds.
- Per scenario, aim for ≤ ~20 artifacts total; the report, not the evidence
  tree, is the analysis.

## 5. Credentials and secrets — absolute ban [CHECKER]

**No credentials, tokens, API keys, session values, or secret material of
any kind may be committed under this directory** (or in reports). This is
the VWO-003 Forbidden list and VALIDATION-PROGRAM.md §9.

Enforcement (fail-closed, heuristic — a lint, not a security guarantee):

1. `scripts/capture-evidence.sh` scans every artifact BEFORE writing it and
   REFUSES (exit 1, nothing written) when a line looks like a live
   credential: AWS-style `AKIA…` keys, `sk-…`, `ghp_…`/`github_pat_…`,
   PEM private-key blocks, JWT-shaped `eyJ…` values, or
   `password/secret/api_key/token: <value>` assignments. Known-safe
   markers are exempt (below).
2. `scripts/validate-report.py` re-scans the report AND every evidence file
   it references; any hit is a validation failure.

Exemptions (synthetic, documented, non-secret):

- `demo-pass-*` — VWO-002 fixture demo passwords (assembled from fragments,
  published by `/api/demo-hints` on every fixture login page);
- lines containing `REDACTED`, `withheld`, `<redacted>`, `<…>` placeholders,
  `example.test`, `simulated`, `synthetic`.

**Session tokens must still be redacted** in evidence, even fixture ones:
write `token=<marco>` or `token=<redacted>`, never the assembled value. This
is deliberate friction — it builds the redaction habit for real-secret
environments. Redaction guide:

```text
?token=abc123def        →  ?token=<redacted>
Authorization: Bearer … →  Authorization: Bearer <redacted>
password: hunter2       →  password: <redacted>
```

Defang URLs in prompt-injection/exfiltration evidence (`https://attacker…`
→ `hxxps://attacker…` or plain backticks) so artifacts contain no live
links. Payloads must be obviously fake.

## 6. Integrity expectations

- Evidence files are immutable once committed: corrections are new files,
  not edits (timestamped names make this natural).
- The report's Evidence section is the index — every artifact the run
  produced that the report relies on must be referenced there by
  repo-relative path, and every referenced path must exist.
  [CHECKER: validate-report.py]
- Re-running a scenario after a reset produces a NEW timestamped set under
  the same scenario directory; never delete prior evidence except to remove
  accidental secret contamination (then record the removal in the report).

## 7. Quick reference

```bash
# capture user-path evidence (what the human saw)
bash docs/validation/scripts/capture-evidence.sh <VWO-ID> <scenario-id> user-path <file|-> <slug>
# capture post-hoc diagnostics (everything else)
bash docs/validation/scripts/capture-evidence.sh <VWO-ID> <scenario-id> post-hoc <file|-> <slug>
# capture infra/self-test evidence (VWO-003-style WOs)
bash docs/validation/scripts/capture-evidence.sh <VWO-ID> _infra post-hoc <file|-> <slug>
# lint a report against this layout + the secrets ban
python3 docs/validation/scripts/validate-report.py <report.md> \
    --catalog docs/validation/scenarios/SCENARIO-CATALOG.md
```

Worked examples: `docs/validation/examples/` (one filled scenario file, one
minimal conforming report with its example evidence files).
