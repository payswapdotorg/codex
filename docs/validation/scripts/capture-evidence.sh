#!/usr/bin/env bash
# VWO-003 helper — capture an artifact into the canonical evidence tree with
# a UTC timestamp name, REFUSING secret-looking content (fail closed).
#
# Usage:
#   capture-evidence.sh <VWO-ID> <scenario-id> <user-path|post-hoc> <source> [slug]
#   capture-evidence.sh <VWO-ID> <scenario-id> <user-path|post-hoc> <source> [slug] --step N
#   capture-evidence.sh <VWO-ID> <scenario-id> <user-path|post-hoc> <source> [slug] --big
#
#   <source>  a file path, or '-' to read stdin (e.g. pipe a command transcript)
#   <slug>    lowercase-hyphen label (<=40 chars); defaults to the source
#             filename without extension, sanitized; REQUIRED for stdin
#   --step N  prefix the name with stepNN- (user-path steps; TEACHING-MODE-
#             MATRIX.md §5 also suggests -instruct-phase/-demo-phase tags —
#             just put them in the slug)
#   --big     allow artifacts larger than 1 MiB (justify in the report)
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
# capture with exit 1 and NOTHING is written. Known-safe markers are exempt
# (demo-pass- fixture passwords, REDACTED/withheld/placeholder/… forms,
# example.test hosts). This is a heuristic lint, not a security guarantee —
# see evidence/README.md §5 for the redaction guide.
set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "$0")" && pwd)
REPO_ROOT=$(cd "$SCRIPT_DIR/../../.." && pwd)
EVIDENCE_ROOT="$REPO_ROOT/docs/validation/evidence"

die() { echo "capture-evidence: $*" >&2; exit 2; }
refuse() { echo "capture-evidence: REFUSED — $*" >&2; exit 1; }

VWO=""; SCEN=""; CLASS=""; SRC=""; SLUG=""; STEP=""; BIG=0
while [ $# -gt 0 ]; do
  case "$1" in
    --step) shift; [ $# -gt 0 ] || die "--step needs a number"; STEP="$1" ;;
    --big)  BIG=1 ;;
    -h|--help) sed -n '2,30p' "$0"; exit 0 ;;
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

[ -n "$VWO" ] && [ -n "$SCEN" ] && [ -n "$CLASS" ] && [ -n "$SRC" ] || die "usage: capture-evidence.sh <VWO-ID> <scenario-id> <user-path|post-hoc> <source> [slug] [--step N] [--big]"
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

# ---- Secrets scan (fail closed; same patterns as validate-report.py) ----
SECRETS_RE='(AKIA[0-9A-Z]{16})|(sk-[A-Za-z0-9_-]{16,})|(ghp_[A-Za-z0-9]{20,})|(github_pat_[A-Za-z0-9_]{20,})|(-----BEGIN [A-Z ]*PRIVATE KEY-----)|(eyJ[A-Za-z0-9_-]{8,}\.[A-Za-z0-9_-]{8,})|((password|passwd|secret|api[_-]?key|apikey|access[_-]?token|auth[_-]?token|authorization|bearer)["'"'"'[:space:]]*[:=][[:space:]]*["'"'"']?[A-Za-z0-9_@#$%^&*+.-]{8})'
ALLOW_RE='(demo-pass-|redacted|withheld|with-held|placeholder|xxxx|example\.test|not-a-secret|<[^>]*>|simulated|synthetic)'

if [ "$BYTES" -gt 0 ]; then
  HITS=$(grep -Ein "$SECRETS_RE" "$TMP" 2>/dev/null || true)
  if [ -n "$HITS" ]; then
    VIOLATIONS=""
    while IFS= read -r line; do
      [ -n "$line" ] || continue
      if ! printf '%s' "$line" | grep -Eqi "$ALLOW_RE"; then
        VIOLATIONS="$VIOLATIONS
$line"
      fi
    done <<EOF
$HITS
EOF
    if [ -n "$VIOLATIONS" ]; then
      echo "capture-evidence: REFUSED — secret-looking content detected (credential patterns)." >&2
      echo "Redact these lines (evidence/README.md §5: token=<redacted>, password: <redacted>) and re-capture:" >&2
      printf '%s\n' "$VIOLATIONS" | head -10 | cut -c1-160 >&2
      exit 1
    fi
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
