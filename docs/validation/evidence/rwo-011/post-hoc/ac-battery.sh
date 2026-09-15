#!/usr/bin/env bash
# RWO-011 acceptance battery — live run against the fixture ports.
# Prereq: run-all.sh --reset with the patched shared runtime.
set -uo pipefail
cd "$(dirname "$0")"

log() { printf '%s\n' "$*"; }
tok() {
  curl -s "http://localhost:$1/api/login" -H 'content-type: application/json' \
    -d "{\"username\":\"$2\",\"password\":\"demo-pass-$(echo "$2" | cut -d. -f1)\"}" \
    | sed -n 's/.*"token": "\([^"]*\)".*/\1/p'
}
codes() { for i in $(seq 1 "$1"); do curl -s -o /dev/null -w "%{http_code} " "$2" -H "authorization: Bearer $3" -H 'content-type: application/json' -d "$4" & done; wait; printf '\n'; }
feed() { curl -s "http://localhost:$1/api/events?type=request.rejected&limit=20" | python3 -c "
import json,sys
evs=json.load(sys.stdin).get('events',[])
print('  request.rejected count:', len(evs))
for e in evs: print('  -', e['id'], '| actor:', e['actor'], '|', e['summary'], '| simulated:', e.get('data',{}).get('simulated'))"; }

log "=== RWO-011 AC battery — $(date -u +%Y-%m-%dT%H:%M:%SZ) ==="
log ""
log "--- AC4: full ecosystem regression (verify-sweep) ---"
bash ../../../fixtures/verify-sweep.sh 2>&1 | tail -1
log ""

log "--- AC1: storm probe — 8 parallel identical driver applications (rideshare) ---"
curl -s -X POST http://localhost:4103/api/reset >/dev/null
codes 8 http://localhost:4103/api/intake/apply "" '{"name":"Storm Probe","email":"storm@probe.test","regionId":"rg-1","plate":"SP-777","make":"Toyota","model":"Corolla"}' | sed 's/^/  HTTP: /'
feed 4103
log ""

log "--- AC2: approval-race context (construction) ---"
curl -s -X POST http://localhost:4101/api/reset >/dev/null
TD=$(tok 4101 dana.reyes); TM=$(tok 4101 marco.silva)
log "  losing race — 2 parallel decides on prq-5003 (valid vendor c-1):"
codes 2 http://localhost:4101/api/procurement/decide "$TD" '{"requestId":"prq-5003","decision":"approve","note":"race probe"}' | sed 's/^/  HTTP: /'
log "  stale re-submission on decisioned prq-5003:"
curl -s -o /dev/null -w "  HTTP: %{http_code}\n" http://localhost:4101/api/procurement/decide -H "authorization: Bearer $TD" -H 'content-type: application/json' -d '{"requestId":"prq-5003","decision":"approve"}'
log "  permission-denied probe (marco.silva, no po:approve):"
curl -s -o /dev/null -w "  HTTP: %{http_code}\n" http://localhost:4101/api/procurement/decide -H "authorization: Bearer $TM" -H 'content-type: application/json' -d '{"requestId":"prq-5001","decision":"approve"}'
feed 4101
log ""

log "--- AC3: distribution-context blocks (marketplace) ---"
curl -s -X POST http://localhost:4105/api/reset >/dev/null
TP=$(tok 4105 petra.voss)
log "  503-refused upgrade (service_unavailable switch):"
curl -s -o /dev/null -w "  HTTP: %{http_code}\n" "http://localhost:4105/api/installs/upgrade?failure=service_unavailable" -H "authorization: Bearer $TP" -H 'content-type: application/json' -d '{"installId":"ins-0401","targetVersion":"2.4.0","expectedCurrentVersion":"2.3.1"}'
log "  stale_entitlement block (switch):"
curl -s -o /dev/null -w "  HTTP: %{http_code}\n" "http://localhost:4105/api/installs/upgrade?failure=stale_entitlement" -H "authorization: Bearer $TP" -H 'content-type: application/json' -d '{"installId":"ins-0401","targetVersion":"2.4.0","expectedCurrentVersion":"2.3.1"}'
log "  data_conflict (natural stale expectedCurrentVersion):"
curl -s -o /dev/null -w "  HTTP: %{http_code}\n" "http://localhost:4105/api/installs/upgrade" -H "authorization: Bearer $TP" -H 'content-type: application/json' -d '{"installId":"ins-0401","targetVersion":"2.4.0","expectedCurrentVersion":"1.0.0"}'
feed 4105
log ""

log "=== battery complete ==="
