# RWO-012 — Evidence Checker: Substring-Scoped Exemption Markers: Completion Report

**Status:** COMPLETE — branch `rwo-012/substring-scoped-exemptions`
**Base:** main @ 29d554c2e (post-RWO-011)
**Work order:** `docs/validation/work-orders/RWO-012.md` (Family K; closes VWO-008 F5)
**Scope:** the evidence checker script + one documented evidence redaction — no Rust, no protocol, no zst, no fixtures

## What was built

1. **Substring-scoped exemptions** (`capture-evidence.sh`). The line-scoped
   bypass is closed: a line containing BOTH a credential-shaped match AND an
   exemption marker is no longer exempted wholesale. The new
   `line_has_violation` core evaluates each `SECRETS_RE` match individually:
   a match is exempt iff **(1)** the match text itself carries a known-safe
   marker (`demo-pass-…` fixture passwords, `REDACTED…` placeholders,
   `<redacted>` forms), or **(2)** the match sits inside a single
   whitespace-delimited segment that carries a marker (e.g. a defanged
   `…example.test` URL with an embedded example key). A marker elsewhere on
   the line — the VWO-008 F5 bypass — no longer exempts a raw assignment.
2. **`{8}` → `{8,}`** on the assignment value in `SECRETS_RE`. The old
   exactly-8 capture truncated values mid-marker (`demo-pas`), which would
   have broken legitimate fixture-password artifacts under substring
   scoping. Detection semantics are unchanged (both forms require ≥ 8 value
   chars); only the match extent differs, so markers inside values are now
   visible to rule (1).
3. **`--selftest` mode** (added, per the WO): the three WO cases plus the
   AC2 bypass shape and four regression guards — 9 tests, all green:
   (a) marker + fake AKIA key on one line → REFUSE; (b) only the defanged
   URL → ALLOW; (c) redacted placeholder alone → ALLOW; (c′) redacted value
   form → ALLOW; (d) the VWO-008 F5 original bypass shape (AWS-example key +
   password assignment + defanged host) → REFUSE; (e) fixture demo password
   (JSON form) → ALLOW; (f) defanged URL with embedded example key → ALLOW;
   (g) multi-token assignment with marker value → ALLOW; (h) raw assignment +
   unrelated marker token → REFUSE.
4. **`--scan` mode** (added): re-scan existing files (a path, a directory
   tree, or a stdin path list) with the same rules, no capture — the WO's
   re-scan command now exists.
5. **Documented evidence redaction.** The corpus re-scan caught exactly one
   artifact — VWO-008's own `adv-prompt-injection` step-07 transcript, whose
   injected attack cell contains literal fake `api_key:`/`password:`
   assignments (20-char value, no marker) that the old line-scoped exemption
   waved through via the defanged host. This is the F5 residual surviving in
   the committed corpus. Redacted per the VWO-008 documented-removal
   precedent: the two assignments became `<redacted RWO-012 re-scan>` forms;
   the attack-instruction text (the evidentiary point) is preserved.

## Acceptance criteria → evidence (all real runs; `docs/validation/evidence/rwo-012/`)

1. The three self-test cases behave as specified — `--selftest`: **9 passed,
   0 failed** (`post-hoc/selftest-and-rescan.txt`).
2. VWO-008's original bypass payload shape is refused fail-closed: a live
   capture attempt of the shape exits **1** with `REFUSED — secret-looking
   content` and the target directory is byte-identical before/after
   (9 files = 9 files — nothing written). Proof captured in
   `post-hoc/selftest-and-rescan.txt`.
3. The committed corpus passes the re-scan: `--scan` over the full evidence
   tree — **525 files checked, 0 violations** (after the one documented
   redaction; the pre-redaction scan caught exactly that artifact and
   nothing else — no false positives on legitimate content: demo-pass
   artifacts, defanged URLs, redacted forms all pass).

## Residual risk (documented, out of WO scope)

`docs/validation/scripts/validate-report.py` line ~189 carries the same
line-scoped exemption shape (`ALLOW_RE.search(line)` on the whole line). The
WO bounds this fix to `capture-evidence.sh`; the Python twin should receive
the same substring-scoping treatment in a follow-up (its corpus — the merged
reports — is the same shape this checker now guards).

## Lineage

Tech-Lead direct execution (single-script bounded scope). Probe-driven: the
first selftest run exposed the `{8}` truncation via a failing multi-token
fixture-password case, which drove the `{8,}` fix before any corpus scan.
