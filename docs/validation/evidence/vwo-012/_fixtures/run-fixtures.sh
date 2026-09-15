#!/usr/bin/env bash
# run-fixtures.sh — VWO-012 purpose-built GUI fixture suite launcher.
# Usage: run-fixtures.sh [--stop] [--seed]
# Ports: notes-editor 4201 · grid-sheet 4202 · pix-bench 4203 · files-app 4204
set -euo pipefail
cd "$(dirname "$0")"

WS="${NSTD_WS:-/tmp/nstd-workspace}"

if [ "${1:-}" = "--seed" ]; then
  bash seed-workspace.sh "$WS"
  exit 0
fi
if [ "${1:-}" = "--stop" ]; then
  for pidfile in .run/*.pid; do
    [ -f "$pidfile" ] || continue
    pid=$(cat "$pidfile"); kill "$pid" 2>/dev/null && echo "stopped $(basename "$pidfile" .pid) ($pid)" || true
    rm -f "$pidfile"
  done
  exit 0
fi

mkdir -p .run
declare -A APPS=([notes-editor]=4201 [grid-sheet]=4202 [pix-bench]=4203 [files-app]=4204)
for app in notes-editor grid-sheet pix-bench files-app; do
  port=${APPS[$app]}
  if curl -sf "http://127.0.0.1:$port/" >/dev/null 2>&1; then echo "$app already up on :$port"; continue; fi
  NSTD_WS="$WS" nohup node "$app/server.js" "$port" > ".run/$app.log" 2>&1 &
  echo $! > ".run/$app.pid"
  echo "started $app on http://127.0.0.1:$port/ (pid $(cat ".run/$app.pid"))"
done
for app in notes-editor grid-sheet pix-bench files-app; do
  port=${APPS[$app]}
  ok=""
  for _ in $(seq 1 20); do
    if curl -sf "http://127.0.0.1:$port/" >/dev/null 2>&1; then ok=1; break; fi
    sleep 0.5
  done
  [ -n "$ok" ] && echo "healthy: $app :$port" || { echo "FAILED: $app :$port (see .run/$app.log)"; exit 1; }
done
echo "fixture suite up (workspace: $WS)"
