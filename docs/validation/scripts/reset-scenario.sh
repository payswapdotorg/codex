#!/usr/bin/env bash
# VWO-003 helper — restart + reseed ONE VWO-002 fixture app between scenarios.
#
# Composes VWO-002's launcher (does not duplicate it): --all / --stop-all
# delegate directly to docs/validation/fixtures/run-all.sh; single-family
# resets reuse run-all.sh's app map, PID-file and log conventions
# (fixtures/.run/<family>.{pid,log}).
#
# Usage:
#   reset-scenario.sh <family>              # restart <family> and reseed it
#   reset-scenario.sh <family> --port <n>   # ... on a non-default port
#   reset-scenario.sh --stop <family>       # stop only <family>
#   reset-scenario.sh --all                 # delegate: fixtures/run-all.sh --reset
#   reset-scenario.sh --stop-all            # delegate: fixtures/run-all.sh --stop
#
# Families: construction software rideshare media marketplace
# (default ports 4101-4105 — MUST mirror run-all.sh's APPS map).
#
# This is a convenience wrapper, NOT a workflow runtime: it holds no state,
# knows nothing about workflows, and never touches codex-rs. It never resets
# apps you did not name (shared-sandbox safety).
set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "$0")" && pwd)
FIXTURES=$(cd "$SCRIPT_DIR/../fixtures" && pwd)
RUN_DIR="$FIXTURES/.run"

default_port() {
  case "$1" in
    construction) echo 4101 ;;
    software)     echo 4102 ;;
    rideshare)    echo 4103 ;;
    media)        echo 4104 ;;
    marketplace)  echo 4105 ;;
    *)            echo "" ;;
  esac
}

die() { echo "reset-scenario: $*" >&2; exit 2; }

[ -d "$FIXTURES" ] || die "fixtures directory not found at $FIXTURES (is this run from the codex repo?)"
command -v curl >/dev/null 2>&1 || die "curl is required"

MODE="restart"
FAMILY=""
PORT=""

while [ $# -gt 0 ]; do
  case "$1" in
    --all)      MODE="all" ;;
    --stop-all) MODE="stop-all" ;;
    --stop)     MODE="stop" ;;
    --port)     shift; [ $# -gt 0 ] || die "--port needs a value"; PORT="$1" ;;
    -h|--help)  sed -n '2,20p' "$0"; exit 0 ;;
    *)          [ -z "$FAMILY" ] || die "unexpected extra argument: $1"; FAMILY="$1" ;;
  esac
  shift
done

case "$MODE" in
  all)
    echo "reset-scenario: delegating to fixtures/run-all.sh --reset"
    exec bash "$FIXTURES/run-all.sh" --reset
    ;;
  stop-all)
    echo "reset-scenario: delegating to fixtures/run-all.sh --stop"
    exec bash "$FIXTURES/run-all.sh" --stop
    ;;
esac

[ -n "$FAMILY" ] || die "usage: reset-scenario.sh <family>|--all [--port N]|--stop <family>|--stop-all (see --help)"
[ -d "$FIXTURES/$FAMILY" ] || die "unknown family '$FAMILY' (no fixture at $FIXTURES/$FAMILY); known: construction software rideshare media marketplace"

if [ "$MODE" = "stop" ]; then
  if [ -f "$RUN_DIR/$FAMILY.pid" ]; then
    pid=$(cat "$RUN_DIR/$FAMILY.pid")
    if kill "$pid" 2>/dev/null; then echo "stopped $FAMILY (pid $pid)"; else echo "$FAMILY not running (stale pid $pid)"; fi
    rm -f "$RUN_DIR/$FAMILY.pid"
  else
    echo "$FAMILY not running (no pid file)"
  fi
  exit 0
fi

[ -n "$PORT" ] || PORT=$(default_port "$FAMILY")
[ -n "$PORT" ] || die "no default port for family '$FAMILY'"

mkdir -p "$RUN_DIR"

# Stop any existing instance of THIS family only (never other families).
if [ -f "$RUN_DIR/$FAMILY.pid" ]; then
  pid=$(cat "$RUN_DIR/$FAMILY.pid")
  if kill "$pid" 2>/dev/null; then echo "stopped previous $FAMILY (pid $pid)"; else echo "previous $FAMILY not running (stale pid $pid)"; fi
  rm -f "$RUN_DIR/$FAMILY.pid"
fi

# Same runner preference and log/pid conventions as run-all.sh. run-all.sh
# cds into fixtures/ before launching; do the same so the relative
# <family>/server.js path resolves regardless of the caller's cwd.
RUNNER=node
command -v node >/dev/null 2>&1 || RUNNER=bun
cd "$FIXTURES"
nohup "$RUNNER" "$FAMILY/server.js" --port "$PORT" > "$RUN_DIR/$FAMILY.log" 2>&1 &
echo $! > "$RUN_DIR/$FAMILY.pid"
echo "started $FAMILY on http://localhost:$PORT/ (pid $(cat "$RUN_DIR/$FAMILY.pid"))"

# Health check (max ~10s, same cadence as run-all.sh).
ok=""
for _ in $(seq 1 20); do
  if curl -sf "http://localhost:$PORT/healthz" >/dev/null 2>&1; then ok=1; break; fi
  sleep 0.5
done
[ -n "$ok" ] || { echo "FAILED health check: $FAMILY on port $PORT (see $RUN_DIR/$FAMILY.log)" >&2; exit 1; }
echo "healthy: $FAMILY -> http://localhost:$PORT/"

# Reseed to deterministic VWO-002 seed state.
curl -sf -X POST "http://localhost:$PORT/api/reset" >/dev/null && echo "reset: $FAMILY (seed state restored, world.reset event recorded)"
echo "scenario-ready: $FAMILY — sign in at http://localhost:$PORT/login (personas on the page and at /api/demo-hints)"
