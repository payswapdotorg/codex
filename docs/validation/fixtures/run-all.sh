#!/usr/bin/env bash
# VWO-002 fixture ecosystem launcher — starts all five synthetic apps.
#
# Usage:
#   ./run-all.sh                 start all five apps (background, logs in .run/)
#   ./run-all.sh --reset         start all five apps and reset each to seed state
#   ./run-all.sh --stop          stop all five apps
#
# Runtime: node (or bun — both work). No external dependencies.
# Apps: construction 4101 · software 4102 · rideshare 4103 · media 4104 · marketplace 4105
set -euo pipefail
cd "$(dirname "$0")"

RUN_DIR=.run
mkdir -p "$RUN_DIR"

declare -A APPS=(
  [construction]=4101
  [software]=4102
  [rideshare]=4103
  [media]=4104
  [marketplace]=4105
)

if [ "${1:-}" = "--stop" ]; then
  for fam in "${!APPS[@]}"; do
    if [ -f "$RUN_DIR/$fam.pid" ]; then
      pid=$(cat "$RUN_DIR/$fam.pid")
      if kill "$pid" 2>/dev/null; then echo "stopped $fam (pid $pid)"; else echo "$fam not running (stale pid $pid)"; fi
      rm -f "$RUN_DIR/$fam.pid"
    fi
  done
  exit 0
fi

RUNNER=node
command -v node >/dev/null 2>&1 || RUNNER=bun

for fam in "${!APPS[@]}"; do
  port=${APPS[$fam]}
  nohup "$RUNNER" "$fam/server.js" --port "$port" > "$RUN_DIR/$fam.log" 2>&1 &
  echo $! > "$RUN_DIR/$fam.pid"
  echo "started $fam on http://localhost:$port/ (pid $(cat "$RUN_DIR/$fam.pid"))"
done

# Wait for health checks (max ~10s per app).
for fam in "${!APPS[@]}"; do
  port=${APPS[$fam]}
  ok=""
  for _ in $(seq 1 20); do
    if curl -sf "http://localhost:$port/healthz" >/dev/null 2>&1; then ok=1; break; fi
    sleep 0.5
  done
  if [ -n "$ok" ]; then echo "healthy: $fam -> http://localhost:$port/"; else echo "FAILED health check: $fam (see $RUN_DIR/$fam.log)"; exit 1; fi
done

if [ "${1:-}" = "--reset" ]; then
  for fam in "${!APPS[@]}"; do
    port=${APPS[$fam]}
    curl -sf -X POST "http://localhost:$port/api/reset" >/dev/null && echo "reset: $fam"
  done
fi

echo
echo "All five VWO-002 fixture apps are running:"
echo "  construction (SiteBuild)   http://localhost:4101/"
echo "  software    (ForgeOps)     http://localhost:4102/   terminal console: (cd software && node deploy-console.js)"
echo "  rideshare   (RidePilot)    http://localhost:4103/"
echo "  media       (PressRoom)    http://localhost:4104/"
echo "  marketplace (FlowMart)     http://localhost:4105/"
echo "Demo personas: GET http://localhost:<port>/api/demo-hints  (also on each /login page)"
echo "Stop with: ./run-all.sh --stop"
