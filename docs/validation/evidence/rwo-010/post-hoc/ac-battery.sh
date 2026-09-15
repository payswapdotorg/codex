#!/usr/bin/env bash
# RWO-010 acceptance battery — live run against the fixture ports.
# Captured post-hoc on a fresh `run-all.sh --reset` seed state.
set -uo pipefail
cd "$(dirname "$0")"

log() { printf '%s\n' "$*"; }
tok() { # tok <port> <user>
  curl -s "http://localhost:$1/api/login" -H 'content-type: application/json' \
    -d "{\"username\":\"$2\",\"password\":\"demo-pass-$(echo "$2" | cut -d. -f1)\"}" \
    | sed -n 's/.*"token": "\([^"]*\)".*/\1/p'
}

log "=== RWO-010 AC battery — $(date -u +%Y-%m-%dT%H:%M:%SZ) ==="
log ""
log "--- AC1: full ecosystem regression (verify-sweep) ---"
bash ../../../fixtures/verify-sweep.sh 2>&1 | tail -1
log ""

log "--- AC2a: procurement approve->PO on the second OPEN PRQ (VWO-004 F2) ---"
T=$(tok 4101 dana.reyes)
curl -s -w "\n%{http_code}" http://localhost:4101/api/procurement/decide \
  -H "authorization: Bearer $T" -H 'content-type: application/json' \
  -d '{"requestId":"prq-5003","decision":"approve","note":"RWO-010 AC battery"}' \
  | python3 -c "import sys; d=sys.stdin.read(); body,_,code=d.rpartition(chr(10)); import json; j=json.loads(body); print('HTTP',code,'|',j['message'])"
log ""

log "--- AC2b: support-agent resolution (VWO-004 F3) ---"
T=$(tok 4103 ana.silva)
curl -s -w "\n%{http_code}" http://localhost:4103/api/tickets/resolve \
  -H "authorization: Bearer $T" -H 'content-type: application/json' \
  -d '{"ticketId":"tk-0702","resolution":"Refund issued and driver notified — RWO-010 AC battery"}' \
  | python3 -c "import sys; d=sys.stdin.read(); body,_,code=d.rpartition(chr(10)); import json; j=json.loads(body); print('HTTP',code,'|',j['message'],'| final status:',j['ticket']['status'])"
log ""

log "--- AC2c: rendition missing_asset switch arms and fires (VWO-004 F4) ---"
T=$(tok 4104 omar.bhatt)
curl -s -w "\n%{http_code}" "http://localhost:4104/api/assets/rendition?failure=missing_asset" \
  -H "authorization: Bearer $T" -H 'content-type: application/json' \
  -d '{"assetId":"a-0301","name":"hero-4k"}' \
  | python3 -c "import sys; d=sys.stdin.read(); body,_,code=d.rpartition(chr(10)); import json; j=json.loads(body); print('HTTP',code,'| error:',j['error'],'| simulated:',j.get('simulated'))"
log "control (switch off, same asset):"
curl -s -w "\n%{http_code}" "http://localhost:4104/api/assets/rendition" \
  -H "authorization: Bearer $T" -H 'content-type: application/json' \
  -d '{"assetId":"a-0301","name":"hero-4k"}' \
  | python3 -c "import sys; d=sys.stdin.read(); body,_,code=d.rpartition(chr(10)); import json; j=json.loads(body); print('HTTP',code,'|',j['message'])"
log ""

log "--- AC2f: hero attach on APPROVED story (VWO-005 F7, smallest fix) ---"
T=$(tok 4104 hana.kim)
curl -s -w "\n%{http_code}" http://localhost:4104/api/stories/attach \
  -H "authorization: Bearer $T" -H 'content-type: application/json' \
  -d '{"storyId":"st-0205","assetId":"a-0301","role":"hero"}' \
  | python3 -c "import sys; d=sys.stdin.read(); body,_,code=d.rpartition(chr(10)); import json; j=json.loads(body); print('HTTP',code,'|',j['message'],'| status stays:',j['story']['status'])"
log ""

log "--- AC2d: breaking-news publishes to the push fast channel (VWO-004 F6) ---"
T=$(tok 4104 diego.morales)
curl -s -w "\n%{http_code}" http://localhost:4104/api/stories/publish \
  -H "authorization: Bearer $T" -H 'content-type: application/json' \
  -d '{"storyId":"st-0205","expectedVersion":1,"channels":["ch-1","ch-6"]}' \
  | python3 -c "import sys; d=sys.stdin.read(); body,_,code=d.rpartition(chr(10)); import json; j=json.loads(body); jobs=j.get('jobs') or j.get('distributionJobs') or []; print('HTTP',code,'|',j.get('message'), '| jobs:', [(x['channelId'],x['status']) for x in jobs])"
log ""

log "--- AC2e: follow-up-note step name matches surface (VWO-004 F5) ---"
log "static catalog wording change; verified in commit diff (SCENARIO-CATALOG.md step 3 rename)."
log ""
log "=== battery complete ==="
