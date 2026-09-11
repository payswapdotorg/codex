#!/usr/bin/env python3
"""VWO-003 helper — schema lint for validation worker reports.

Usage:
  validate-report.py <report.md> [--catalog SCENARIO-CATALOG.md] [--strict]
                     [--repo-root PATH]

Checks (see docs/validation/reports/REPORT-SCHEMA.md, the normative source):
  - heading set and order (Class A: Identity + one or more full scenario
    blocks; Class B: Identity only — auto-detected);
  - required Identity fields, incl. SHA formats (40-hex / NONE-reason /
    branch-reference);
  - scenario block fields: Scenario id, Industry, Scenario, Teaching mode,
    User goal;
  - scenario id exists in the catalog (with --catalog) and teaching mode
    matches the catalog binding (mismatch = warning; failure under --strict);
  - at least 3 numbered user-path steps;
  - Workflow Identity and Runtime Binding field lines present;
  - evidence pointers: repo-relative, must exist, under
    docs/validation/evidence/ (or the example-only
    docs/validation/examples/evidence/ location); Class A scenario blocks
    should reference at least one user-path artifact (warning if not);
  - severity values legal (P0/P1/P2/P3); P0/P1 findings may not be DEFERred;
  - Recommendation carries a disposition keyword (Class A);
  - Worker Conclusion states passed/failed/blocked (warning);
  - secrets scan across the report and every referenced text evidence file
    (same patterns as capture-evidence.sh; fail closed on hits).

Exit codes: 0 = conforming (warnings allowed); 1 = violations (or warnings
under --strict); 2 = usage error. Dependency-free: Python 3 std-lib only.
"""
import re
import sys
import os
import argparse

IDENTITY_FIELDS = [
    "Work Order",
    "Worker persona",
    "Base branch",
    "Base SHA",
    "Head SHA",
    "Validation environment",
    "Codex Universal runtime/application SHA",
    "Date/time window",
]
SHA_FIELDS = ["Base SHA", "Head SHA", "Codex Universal runtime/application SHA"]
SCENARIO_FIELDS = ["Scenario id", "Industry", "Scenario", "Teaching mode", "User goal"]
WORKFLOW_IDENTITY_FIELDS = [
    "Workflow",
    "Version",
    "Source revision",
    "Definition digest",
    "Dependency-lock identity",
]
RUNTIME_BINDING_FIELDS = [
    "Capabilities",
    "Resources",
    "Environments",
    "Approvals",
    "Triggers/schedules",
]
SCENARIO_ORDER = [
    "Scenario",
    "User Path",
    "Expected",
    "Actual",
    "Workflow Identity",
    "Runtime Binding",
    "Evidence",
    "Failure / Recovery",
    "Product / UX Friction",
    "Security / Architecture Findings",
    "Recommendation",
    "Worker Conclusion",
]
MODES = {"DEMONSTRATE", "INSTRUCT", "HYBRID"}
SEVERITIES = {"P0", "P1", "P2", "P3"}
DISPOSITIONS = ("FIX NOW", "NEW WORK ORDER", "DEFER")
EVIDENCE_PREFIXES = (
    "docs/validation/evidence/",
    "docs/validation/examples/evidence/",  # example-only location
)
TEXT_EXTS = (".txt", ".md", ".json", ".csv", ".sh", ".py", ".js")

SHA40_RE = re.compile(r"^[0-9a-f]{40}$")
SLUG_RE = re.compile(r"^[a-z0-9][a-z0-9-]*$")
FIELD_RE = re.compile(r"^\s*-\s+([^:]+):\s*(.*)$")
NUMBERED_RE = re.compile(r"^\s*\d+\.\s+\S")
PATH_RE = re.compile(r"[\w./-]+/[\w./-]+")

# Secrets scan — MUST stay in sync with capture-evidence.sh (fail-closed
# heuristic; known-safe markers below are exempt). A lint, not a security
# guarantee (docs/validation/evidence/README.md §5).
SECRETS_RE = re.compile(
    r"(AKIA[0-9A-Z]{16})"
    r"|(sk-[A-Za-z0-9_-]{16,})"
    r"|(ghp_[A-Za-z0-9]{20,})"
    r"|(github_pat_[A-Za-z0-9_]{20,})"
    r"|(-----BEGIN [A-Z ]*PRIVATE KEY-----)"
    r"|(eyJ[A-Za-z0-9_-]{8,}\.[A-Za-z0-9_-]{8,})"
    r"|((?:password|passwd|secret|api[_-]?key|apikey|access[_-]?token"
    r"|auth[_-]?token|authorization|bearer)[\"'\\\s]*[:=]\s*[\"']?"
    r"[A-Za-z0-9_@#$%^&*+.-]{8})",
    re.IGNORECASE,
)
ALLOW_RE = re.compile(
    r"(demo-pass-|redacted|withheld|with-held|placeholder|xxxx"
    r"|example\.test|not-a-secret|<[^>]*>|simulated|synthetic)",
    re.IGNORECASE,
)


class Report:
    def __init__(self, text):
        self.lines = text.splitlines()
        self.sections = []  # (heading, [body lines])
        cur_head, cur_body = None, []
        for line in self.lines:
            m = re.match(r"^##\s+(.+?)\s*$", line)
            if m:
                if cur_head is not None:
                    self.sections.append((cur_head, cur_body))
                cur_head, cur_body = m.group(1), []
            elif cur_head is not None:
                cur_body.append(line)
        if cur_head is not None:
            self.sections.append((cur_head, cur_body))

    def find(self, heading):
        out = []
        for h, body in self.sections:
            if h == heading:
                out.append(body)
        return out

    def first(self, heading):
        for h, body in self.sections:
            if h == heading:
                return body
        return None


def check_sha_field(field, value):
    """Return (level, message): 'ok' | 'warn' | 'fail'."""
    v = value.strip()
    if SHA40_RE.match(v):
        return "ok", ""
    upper = v.upper()
    if upper.startswith("NONE") or upper.startswith("N/A") or "SEE GIT" in upper or "BRANCH" in upper:
        return "warn", "recorded without a 40-hex SHA (explicit placeholder — acceptable only with the stated reason)"
    return "fail", "not a 40-hex SHA (and not an explicit NONE/branch reference)"


def parse_fields(body):
    fields = {}
    for line in body:
        m = FIELD_RE.match(line)
        if m:
            name, val = m.group(1).strip(), m.group(2).strip()
            if name not in fields:
                fields[name] = val
    return fields


def field_list(body):
    out = []
    for line in body:
        m = FIELD_RE.match(line)
        if m:
            out.append((m.group(1).strip(), m.group(2).strip()))
    return out


def nonempty(body):
    return any(l.strip() and not l.strip().startswith("-") for l in body) or any(
        l.strip() for l in body
    )


def scan_secrets(path, rel):
    hits = []
    if os.path.splitext(path)[1].lower() not in TEXT_EXTS:
        return hits
    try:
        with open(path, "r", encoding="utf-8", errors="ignore") as f:
            for i, line in enumerate(f, 1):
                for m in SECRETS_RE.finditer(line):
                    if not ALLOW_RE.search(line):
                        hits.append(f"{rel}:{i}: secret-looking content: {line.strip()[:120]}")
                        break
    except OSError:
        hits.append(f"{rel}: could not read file for secrets scan")
    return hits


def parse_catalog(catalog_path):
    ids = {}
    in_yaml = False
    cur = {}
    with open(catalog_path, "r", encoding="utf-8") as f:
        for line in f:
            s = line.strip()
            if s.startswith("```yaml"):
                in_yaml, cur = True, {}
                continue
            if in_yaml and s.startswith("```"):
                if cur.get("scenario-id"):
                    ids[cur["scenario-id"]] = cur.get("teaching-mode", "")
                in_yaml = False
                continue
            if in_yaml and ":" in s:
                k, _, v = s.partition(":")
                cur[k.strip()] = v.strip()
    return ids


def main():
    ap = argparse.ArgumentParser(description="VWO-003 report schema lint")
    ap.add_argument("report")
    ap.add_argument("--catalog")
    ap.add_argument("--strict", action="store_true")
    ap.add_argument("--repo-root")
    args = ap.parse_args()

    script_dir = os.path.dirname(os.path.abspath(__file__))
    repo_root = os.path.abspath(
        args.repo_root or os.path.join(script_dir, "..", "..", "..")
    )

    try:
        with open(args.report, "r", encoding="utf-8") as f:
            report = Report(f.read())
    except OSError as e:
        print(f"validate-report: cannot read {args.report}: {e}", file=sys.stderr)
        return 2

    violations, warnings = [], []

    def fail(msg):
        violations.append(msg)

    def warn(msg):
        warnings.append(msg)

    # ---- class detection & heading structure ----
    scenario_blocks = report.find("Scenario")
    class_a = bool(scenario_blocks)
    heads = [h for h, _ in report.sections]
    identity_blocks = report.find("Identity")

    if not identity_blocks:
        fail("missing '## Identity' section")
    if len(identity_blocks) > 1:
        fail("'## Identity' section appears more than once")
    if identity_blocks and heads and heads[0] != "Identity":
        fail(f"'## Identity' must be the first section (found '## {heads[0]}')")

    if class_a:
        # headings after Identity must be exact repetitions of SCENARIO_ORDER
        rest = heads[1:] if heads and heads[0] == "Identity" else heads
        n = len(SCENARIO_ORDER)
        if len(rest) % n != 0 or len(rest) == 0:
            fail(
                f"scenario-block headings must be whole repetitions of the "
                f"{n}-section block (found {len(rest)} sections after Identity)"
            )
        else:
            for b in range(len(rest) // n):
                got = rest[b * n:(b + 1) * n]
                if got != SCENARIO_ORDER:
                    want = SCENARIO_ORDER
                    for i, (g, w) in enumerate(zip(got, want)):
                        if g != w:
                            fail(
                                f"scenario block {b + 1}, section {i + 1}: "
                                f"expected '## {w}', found '## {g}'"
                            )
                            break

    # ---- Identity fields ----
    if identity_blocks:
        f = parse_fields(identity_blocks[0])
        for name in IDENTITY_FIELDS:
            if name not in f or not f[name]:
                fail(f"Identity: missing field '- {name}:'")
        for name in SHA_FIELDS:
            if name in f and f[name]:
                level, why = check_sha_field(name, f[name])
                if level == "fail":
                    fail(f"Identity: '{name}': {why} (value: {f[name][:60]})")
                elif level == "warn":
                    warn(f"Identity: '{name}' {why} (value: {f[name][:60]})")

    # ---- scenario blocks (Class A) ----
    catalog = {}
    if args.catalog:
        try:
            catalog = parse_catalog(args.catalog)
        except OSError as e:
            print(f"validate-report: cannot read catalog {args.catalog}: {e}", file=sys.stderr)
            return 2

    evidence_files = []

    if class_a:
        for idx, body in enumerate(scenario_blocks, 1):
            tag = f"scenario block {idx}"
            f = parse_fields(body)
            for name in SCENARIO_FIELDS:
                if name not in f or not f[name]:
                    fail(f"{tag}: missing field '- {name}:'")
            sid = f.get("Scenario id", "")
            if sid and not SLUG_RE.match(sid):
                fail(f"{tag}: Scenario id '{sid}' is not a lowercase slug")
            if sid and catalog:
                if sid not in catalog:
                    fail(f"{tag}: Scenario id '{sid}' not found in catalog {args.catalog}")
                elif f.get("Teaching mode") and catalog[sid] and f["Teaching mode"] != catalog[sid]:
                    warn(
                        f"{tag}: teaching mode '{f['Teaching mode']}' differs from catalog "
                        f"binding '{catalog[sid]}' for {sid} — record the reason in the report"
                    )
            mode = f.get("Teaching mode", "")
            if mode and mode not in MODES:
                fail(f"{tag}: Teaching mode '{mode}' not one of DEMONSTRATE/INSTRUCT/HYBRID")

            # sections within this block
            sec = {}
            start = heads.index("Identity") + 1 if heads and heads[0] == "Identity" else 0
            b = idx - 1
            for i, name in enumerate(SCENARIO_ORDER):
                gi = start + b * len(SCENARIO_ORDER) + i
                if gi < len(report.sections):
                    sec[name] = report.sections[gi][1]

            up = sec.get("User Path", [])
            steps = [l for l in up if NUMBERED_RE.match(l)]
            if len(steps) < 3:
                fail(f"{tag}: User Path has {len(steps)} numbered steps (minimum 3)")
            for name in ("Expected", "Actual", "Product / UX Friction", "Worker Conclusion"):
                if not nonempty(sec.get(name, [])):
                    fail(f"{tag}: section '{name}' is empty")

            wf = parse_fields(sec.get("Workflow Identity", []))
            for name in WORKFLOW_IDENTITY_FIELDS:
                if name not in wf or not wf[name]:
                    fail(f"{tag}: Workflow Identity: missing field '- {name}:'")
            rb = parse_fields(sec.get("Runtime Binding", []))
            for name in RUNTIME_BINDING_FIELDS:
                if name not in rb or not rb[name]:
                    fail(f"{tag}: Runtime Binding: missing field '- {name}:'")

            # evidence pointers: the first path-like token of each bullet
            ev = sec.get("Evidence", [])
            ptrs = []
            for line in ev:
                m = re.match(r"^\s*-\s+(.+)$", line)
                if not m:
                    continue
                found = PATH_RE.findall(m.group(1))
                if found:
                    ptrs.append(found[0])
            if not ptrs:
                fail(f"{tag}: Evidence section has no repo-relative evidence pointers")
            has_userpath = any("/user-path/" in p for p in ptrs)
            if not has_userpath:
                warn(f"{tag}: no user-path/ evidence pointer — human path skipped or surface missing (say which)")
            for p in ptrs:
                if not p.startswith(EVIDENCE_PREFIXES):
                    fail(f"{tag}: evidence pointer '{p}' is not under docs/validation/evidence/")
                abspath = os.path.join(repo_root, p)
                if not os.path.exists(abspath):
                    fail(f"{tag}: evidence pointer '{p}' does not exist in the repository")
                else:
                    evidence_files.append((abspath, p))

            # findings
            fnd = sec.get("Security / Architecture Findings", [])
            sevs = [v for k, v in field_list(fnd) if k == "Severity"]
            if sevs:
                for s in sevs:
                    if s not in SEVERITIES:
                        fail(f"{tag}: illegal severity value '{s}' (must be P0/P1/P2/P3)")
            else:
                if not any("No findings" in l for l in fnd):
                    fail(
                        f"{tag}: Security / Architecture Findings has neither a "
                        f"'- Severity: P…' line nor the literal '- No findings'"
                    )
            rec = " ".join(sec.get("Recommendation", []))
            if not any(d in rec for d in DISPOSITIONS):
                fail(f"{tag}: Recommendation lacks a disposition keyword (FIX NOW / NEW WORK ORDER / DEFER)")
            has_p01 = any(s in ("P0", "P1") for s in sevs)
            if has_p01 and "DEFER" in rec and "FIX NOW" not in rec and "NEW WORK ORDER" not in rec:
                fail(f"{tag}: P0/P1 finding(s) DEFERred — not allowed (REPORT-SCHEMA.md §13)")

            concl = " ".join(sec.get("Worker Conclusion", [])).lower()
            if not any(k in concl for k in ("passed", "failed", "blocked")):
                warn(f"{tag}: Worker Conclusion does not state passed/failed/blocked")

    else:
        # Class B: bullets referencing docs/ paths anywhere in the report —
        # evidence pointers (under docs/validation/evidence/) must exist and
        # are secrets-scanned; other referenced docs/ paths must exist
        # (broken-pointer check) but may be deliverable references.
        for h, body in report.sections:
            for line in body:
                m = re.match(r"^\s*-\s+(.+)$", line)
                if not m:
                    continue
                found = PATH_RE.findall(m.group(1))
                if not found:
                    continue
                p = found[0]
                if not p.startswith("docs/"):
                    continue
                abspath = os.path.join(repo_root, p)
                if not os.path.exists(abspath):
                    fail(f"referenced path '{p}' does not exist in the repository")
                elif p.startswith("docs/validation/evidence/"):
                    evidence_files.append((abspath, p))
        for h, body in report.sections:
            for k, v in field_list(body):
                if k == "Severity" and v and v not in SEVERITIES:
                    fail(f"illegal severity value '{v}' (must be P0/P1/P2/P3)")

    # ---- secrets scan (report + referenced text evidence) ----
    rel_report = os.path.relpath(args.report, repo_root)
    hits = scan_secrets(args.report, rel_report)
    for abspath, rel in evidence_files:
        hits.extend(scan_secrets(abspath, rel))
    for h in hits:
        fail(h)

    # ---- verdict ----
    for w in warnings:
        print(f"[WARN] {w}")
    for v in violations:
        print(f"[FAIL] {v}")
    n_blocks = len(scenario_blocks) if class_a else 0
    label = f"Class {'A' if class_a else 'B'}"
    if violations or (args.strict and warnings):
        print(
            f"RESULT: FAIL — {label}, {n_blocks} scenario block(s), "
            f"{len(violations)} violation(s), {len(warnings)} warning(s)"
        )
        return 1
    print(
        f"RESULT: PASS — {label}, {n_blocks} scenario block(s), "
        f"{len(warnings)} warning(s)"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
