#!/usr/bin/env bash
# VWO-002 deterministic verification sweep — runs the seven canonical failure
# switches plus the golden path against every family and prints a compact
# pass/fail transcript. Run from docs/validation/fixtures with all apps up
# (./run-all.sh --reset). Used to produce the evidence in
# docs/validation/reports/VWO-002-report.md.
set -uo pipefail

PASS=0; FAIL=0
ok()   { PASS=$((PASS+1)); echo "  PASS  $1"; }
bad()  { FAIL=$((FAIL+1)); echo "  FAIL  $1"; }
# expect <name> <expected-http> <actual-http> [substring-in-body]
expect() {
  local name="$1" want="$2" got="$3" body="${4:-}" sub="${5:-}"
  # 2026-09-11 integration fix (Tech Lead): `echo | grep -q` under `set -o
  # pipefail` is a flaky TEST bug, not an app bug — grep -q exits on the first
  # match, echo takes SIGPIPE on large bodies, and pipefail reports 141 as a
  # FAIL even though the substring matched (observed: "missing '1.3.0'" under
  # a body visibly containing 1.3.0). Here-string grep: no pipe, no SIGPIPE,
  # identical BRE semantics.
  if [ "$want" = "$got" ] && { [ -z "$sub" ] || grep -q "$sub" <<< "$body"; }; then ok "$name (HTTP $got)"; else bad "$name (want HTTP $want, got $got${sub:+, missing '$sub'})"; echo "$body" | head -12 | sed 's/^/        /'; fi
}
login() { # login <port> <user>
  curl -s "http://localhost:$1/api/login" -H 'content-type: application/json' \
    -d "{\"username\":\"$2\",\"password\":\"demo-pass-$3\"}" | sed -n 's/.*"token": "\([^"]*\)".*/\1/p'
}
mut() { # mut <port> <token> <path> <json-body> [failure] -> "<body>\n<http-code>"
  local port="$1" tok="$2" path="$3" body="$4" f="${5:-}"
  local sep="?" q=""
  [ -n "$f" ] && { q="${sep}failure=$f"; sep="&"; }
  [ -n "$tok" ] && q="${q}${sep}token=${tok}"
  curl -s -w "\n%{http_code}" "http://localhost:${port}${path}${q}" -H 'content-type: application/json' -d "$body"
}

echo "== construction (SiteBuild, 4101) =="
T=$(login 4101 marco.silva marco)
body=$(mut 4101 "$T" /api/progress '{"projectId":"p-101","reportDate":"2026-09-14","percentComplete":66,"taskIds":["t-101c"],"photoIds":["a-101"],"notes":"sweep"}')
expect "golden: submit daily progress" 200 "${body##*$'\n'}" "$body" '"percentComplete": 66'
D=$(login 4101 dana.reyes dana)
for sw in service_unavailable notification_failure missing_asset permission_denied data_conflict duplicate_event stale_entitlement; do
  case $sw in
    service_unavailable) body=$(mut 4101 "$T" /api/progress '{"projectId":"p-101","reportDate":"2026-09-15"}' $sw); want=503; sub='service_unavailable';;
    notification_failure) body=$(mut 4101 "$T" /api/safety/report '{"projectId":"p-102","severity":"minor","category":"equipment","description":"x"}' $sw); want=200; sub='"warning"';;
    missing_asset) body=$(mut 4101 "$T" /api/progress '{"projectId":"p-101","reportDate":"2026-09-16","photoIds":["a-101"]}' $sw); want=404; sub='missing_asset';;
    permission_denied) body=$(mut 4101 "$T" /api/procurement/decide '{"requestId":"prq-5001","decision":"approve"}' $sw); want=403; sub='permission_denied';;
    data_conflict) body=$(mut 4101 "$D" /api/procurement/decide '{"requestId":"prq-5001","decision":"approve"}' $sw); want=409; sub='data_conflict';;
    duplicate_event) body=$(mut 4101 "$T" /api/progress '{"projectId":"p-101","reportDate":"2026-09-17"}' $sw); want=409; sub='duplicate_event';;
    stale_entitlement) body=$(mut 4101 "$D" /api/procurement/decide '{"requestId":"prq-5002","decision":"approve"}' $sw); want=409; sub='stale_entitlement';;
  esac
  expect "switch $sw" "$want" "${body##*$'\n'}" "$body" "$sub"
done
body=$(mut 4101 "$D" /api/procurement/decide '{"requestId":"prq-5001","decision":"approve"}')
expect "natural stale_entitlement (SteelCo contract expired)" 409 "${body##*$'\n'}" "$body" 'stale_entitlement'

echo "== software (ForgeOps, 4102) =="
A=$(login 4102 ari.klein ari); N=$(login 4102 noor.haddad noor); RJ=$(login 4102 raj.patel raj); ME=$(login 4102 mei.chen mei)
body=$(mut 4102 "$A" /api/prs/create '{"repoId":"r-1","title":"Golden path PR","branch":"ari/golden","additions":50,"deletions":5}')
expect "golden: open PR + auto-CI" 200 "${body##*$'\n'}" "$body" 'CI run'
body=$(mut 4102 "$N" /api/prs/approve '{"prId":"pr-0304","note":"ok"}')
expect "golden: code owner approves" 200 "${body##*$'\n'}" "$body" 'Approval recorded'
body=$(mut 4102 "$RJ" /api/prs/merge '{"prId":"pr-0304"}')
expect "golden: merge → staging deploy" 200 "${body##*$'\n'}" "$body" 'deployed to staging'
body=$(mut 4102 "$RJ" /api/deployments/promote '{"deploymentId":"dep-1104"}')
expect "golden: promote to production" 200 "${body##*$'\n'}" "$body" 'promoted to production'
for sw in service_unavailable notification_failure missing_asset permission_denied data_conflict duplicate_event stale_entitlement; do
  case $sw in
    service_unavailable) body=$(mut 4102 "$A" /api/issues/create '{"repoId":"r-1","title":"svc test issue","type":"bug"}' $sw); want=503; sub='service_unavailable';;
    notification_failure) body=$(mut 4102 "$ME" /api/incidents/open '{"serviceId":"s-3","severity":"sev3","title":"n","description":"d"}' $sw); want=200; sub='"warning"';;
    missing_asset) body=$(mut 4102 "$RJ" /api/deployments/promote '{"deploymentId":"dep-1101"}' $sw); want=404; sub='missing_asset';;
    permission_denied) body=$(mut 4102 "$RJ" /api/prs/approve '{"prId":"pr-0301"}' $sw); want=403; sub='permission_denied';;
    data_conflict) body=$(mut 4102 "$RJ" /api/prs/merge '{"prId":"pr-0303"}' $sw); want=409; sub='data_conflict';;
    duplicate_event) body=$(mut 4102 "$A" /api/issues/create '{"repoId":"r-1","title":"dup test","type":"bug"}' $sw); want=409; sub='duplicate_event';;
    stale_entitlement) body=$(mut 4102 "$RJ" /api/deployments/promote '{"deploymentId":"dep-1101"}' $sw); want=409; sub='stale_entitlement';;
  esac
  expect "switch $sw" "$want" "${body##*$'\n'}" "$body" "$sub"
done
body=$(mut 4102 "$A" /api/prs/approve '{"prId":"pr-0301"}')
expect "natural permission_denied (self-approval blocked)" 403 "${body##*$'\n'}" "$body" 'permission_denied'

echo "== rideshare (RidePilot, 4103) =="
body=$(mut 4103 "" /api/intake/apply '{"name":"Golden Driver","regionId":"rg-1","plate":"GLD-0001","make":"Kia","model":"Niro","year":2023}')
expect "golden: public intake (no auth)" 200 "${body##*$'\n'}" "$body" 'da-0304'
J=$(login 4103 jules.moreau jules)
body=$(mut 4103 "$J" /api/applications/screen '{"applicationId":"da-0304","decision":"approve","notes":"ok"}')
expect "golden: screening approval" 200 "${body##*$'\n'}" "$body" 'approved'
body=$(mut 4103 "$J" /api/drivers/activate '{"applicationId":"da-0304"}')
expect "golden: driver activation" 200 "${body##*$'\n'}" "$body" 'activated'
for sw in service_unavailable notification_failure missing_asset permission_denied data_conflict duplicate_event stale_entitlement; do
  case $sw in
    service_unavailable) body=$(mut 4103 "" /api/intake/apply '{"name":"SVC Driver","regionId":"rg-1","plate":"SVC-0001"}' $sw); want=503; sub='service_unavailable';;
    notification_failure) body=$(mut 4103 "$J" /api/regions/surge '{"regionId":"rg-2","level":2.5}' $sw); want=200; sub='broadcast FAILED';;
    missing_asset) body=$(mut 4103 "$J" /api/applications/screen '{"applicationId":"da-0301","decision":"approve"}' $sw); want=404; sub='missing_asset';;
    permission_denied) body=$(mut 4103 "$J" /api/payments/approve '{"paymentId":"pay-0901"}' $sw); want=403; sub='permission_denied';;
    data_conflict) body=$(mut 4103 "$J" /api/applications/screen '{"applicationId":"da-0303","decision":"approve"}' $sw); want=409; sub='data_conflict';;
    duplicate_event) body=$(mut 4103 "$J" /api/regions/surge '{"regionId":"rg-3","level":1.3}' $sw); want=409; sub='duplicate_event';;
    stale_entitlement) body=$(mut 4103 "$J" /api/drivers/activate '{"applicationId":"da-0303"}' $sw); want=409; sub='stale_entitlement';;
  esac
  expect "switch $sw" "$want" "${body##*$'\n'}" "$body" "$sub"
done
body=$(mut 4103 "$J" /api/drivers/activate '{"applicationId":"da-0303"}')
expect "natural stale_entitlement (da-0303 expired permit)" 409 "${body##*$'\n'}" "$body" 'stale_entitlement'
body=$(mut 4103 "" /api/intake/apply '{"name":"Dup Driver A","regionId":"rg-1","plate":"DUP-0002"}')
body=$(mut 4103 "" /api/intake/apply '{"name":"Dup Driver B","regionId":"rg-1","plate":"DUP-0002"}')
expect "natural duplicate_event (same plate twice)" 409 "${body##*$'\n'}" "$body" 'duplicate_event'

echo "== media (PressRoom, 4104) =="
H=$(login 4104 hana.kim hana); C=$(login 4104 camille.dubois camille)
body=$(mut 4104 "$H" /api/stories/create '{"title":"Golden path story","section":"news","dek":"d","body":"b"}')
expect "golden: writer creates draft" 200 "${body##*$'\n'}" "$body" 'st-0206'
body=$(mut 4104 "$H" /api/stories/attach '{"storyId":"st-0206","assetId":"a-0301","role":"hero"}')
expect "golden: attach hero image" 200 "${body##*$'\n'}" "$body" 'attached as hero'
body=$(mut 4104 "$H" /api/stories/submit '{"storyId":"st-0206"}')
expect "golden: submit for review" 200 "${body##*$'\n'}" "$body" 'submitted for review'
body=$(mut 4104 "$C" /api/stories/decide '{"storyId":"st-0206","decision":"approve"}')
expect "golden: editor approves" 200 "${body##*$'\n'}" "$body" 'approved for publication'
body=$(mut 4104 "$C" /api/stories/publish '{"storyId":"st-0206","channels":["ch-1","ch-2"]}')
expect "golden: publish to channels" 200 "${body##*$'\n'}" "$body" 'published'
for sw in service_unavailable notification_failure missing_asset permission_denied data_conflict duplicate_event stale_entitlement; do
  case $sw in
    service_unavailable) body=$(mut 4104 "$H" /api/stories/create '{"title":"svc story","section":"news"}' $sw); want=503; sub='service_unavailable';;
    notification_failure) body=$(mut 4104 "$H" /api/stories/submit '{"storyId":"st-0203"}' $sw); want=200; sub='"warning"';;
    missing_asset) body=$(mut 4104 "$C" /api/stories/publish '{"storyId":"st-0205","channels":["ch-1"]}' $sw); want=404; sub='missing_asset';;
    permission_denied) body=$(mut 4104 "$H" /api/stories/decide '{"storyId":"st-0202","decision":"approve"}' $sw); want=403; sub='permission_denied';;
    data_conflict) body=$(mut 4104 "$H" /api/stories/edit '{"storyId":"st-0203","expectedVersion":1,"body":"x"}' $sw); want=409; sub='data_conflict';;
    duplicate_event) body=$(mut 4104 "$C" /api/stories/publish '{"storyId":"st-0204","channels":["ch-1"]}' $sw); want=409; sub='duplicate_event';;
    stale_entitlement) body=$(mut 4104 "$C" /api/stories/publish '{"storyId":"st-0204","channels":["ch-5"]}' $sw); want=200; sub='stale license';;
  esac
  expect "switch $sw" "$want" "${body##*$'\n'}" "$body" "$sub"
done
body=$(mut 4104 "$C" /api/stories/publish '{"storyId":"st-0205","channels":["ch-1"]}')
expect "natural missing_asset (approved, no hero)" 404 "${body##*$'\n'}" "$body" 'missing_asset'

echo "== marketplace (FlowMart, 4105) =="
I=$(login 4105 iris.chen iris); V=$(login 4105 vera.osei vera); M=$(login 4105 mia.torres mia); RV=$(login 4105 ravi.gupta ravi)
body=$(mut 4105 "$I" /api/installs/install '{"packageId":"pk-103","expectedVersion":"1.2.0","config":{"zones":"A"}}')
expect "golden: Northwind installs (active entitlement)" 200 "${body##*$'\n'}" "$body" 'ins-0402'
body=$(mut 4105 "$V" /api/versions/publish '{"packageId":"pk-103","version":"1.3.0","changelog":"cold zones"}')
expect "golden: publisher releases 1.3.0" 200 "${body##*$'\n'}" "$body" '1.3.0'
body=$(mut 4105 "$I" /api/installs/upgrade '{"installId":"ins-0402","targetVersion":"1.3.0","expectedCurrentVersion":"1.2.0"}')
expect "golden: upgrade 1.2.0 → 1.3.0" 200 "${body##*$'\n'}" "$body" 'upgraded'
body=$(mut 4105 "$I" /api/installs/rollback '{"installId":"ins-0402"}')
expect "golden: rollback to 1.2.0" 200 "${body##*$'\n'}" "$body" 'rolled back'
body=$(mut 4105 "$M" /api/provenance/verify '{"packageId":"pk-101","version":"1.1.0"}')
expect "golden: provenance digest verifies" 200 "${body##*$'\n'}" "$body" 'MATCH\|matches'
for sw in service_unavailable notification_failure missing_asset permission_denied data_conflict duplicate_event stale_entitlement; do
  case $sw in
    service_unavailable) body=$(mut 4105 "$V" /api/listings/publish '{"name":"SVC Package","slug":"svc-package"}' $sw); want=503; sub='service_unavailable';;
    notification_failure) body=$(mut 4105 "$V" /api/versions/publish '{"packageId":"pk-102","version":"2.4.0","changelog":"x"}' $sw); want=200; sub='"warning"';;
    missing_asset) body=$(mut 4105 "$I" /api/installs/install '{"packageId":"pk-102","expectedVersion":"2.3.1"}' $sw); want=404; sub='missing_asset';;
    permission_denied) body=$(mut 4105 "$RV" /api/installs/install '{"packageId":"pk-102","expectedVersion":"2.3.1"}' $sw); want=403; sub='permission_denied';;
    data_conflict) body=$(mut 4105 "$I" /api/installs/install '{"packageId":"pk-102","expectedVersion":"2.2.0"}' $sw); want=409; sub='data_conflict';;
    duplicate_event) body=$(mut 4105 "$V" /api/listings/publish '{"name":"Dup","slug":"invoice-autofill"}' $sw); want=409; sub='duplicate_event';;
    stale_entitlement) body=$(mut 4105 "$I" /api/installs/install '{"packageId":"pk-101","expectedVersion":"1.1.0"}' $sw); want=409; sub='stale_entitlement';;
  esac
  expect "switch $sw" "$want" "${body##*$'\n'}" "$body" "$sub"
done
body=$(mut 4105 "$I" /api/installs/install '{"packageId":"pk-101","expectedVersion":"1.1.0"}')
expect "natural stale_entitlement (Northwind expired trial)" 409 "${body##*$'\n'}" "$body" 'stale_entitlement'

echo
echo "RESULT: $PASS passed, $FAIL failed"
exit $([ "$FAIL" -eq 0 ] && echo 0 || echo 1)
