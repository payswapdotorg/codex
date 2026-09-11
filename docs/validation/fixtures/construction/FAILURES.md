# SiteBuild (construction family) — deterministic failure switches

Activate per-request by appending `?failure=<switch>` to any request **or** by sending header `X-Failure-Switch: <switch>`. The switch applies only to the request that carries it. Unknown names return HTTP 400 with the known list. Machine-readable copy: `GET /api/failures`. HTML copy: `/failures`.

| Switch | Applies to | Behavior | Observable symptom |
|---|---|---|---|
| `service_unavailable` | any state-changing request (API or form) | the synthetic `site-photo-store` dependency is treated as down; the mutation is rejected before touching state | HTTP 503 `{error:"service_unavailable", service:"site-photo-store", simulated:true}`; GET reads still work |
| `notification_failure` | progress submit, PR decision, invoice approval, incident report/close | the operation succeeds but the in-app notification is stored with `status:"failed"` | HTTP 200 with `warning` field; row in `/notifications` shows `failed`; `notification.failed` event in `/events` |
| `missing_asset` | `POST /api/progress`, `POST /ui/progress`, `GET /api/assets/:id` | a referenced photo asset is treated as unresolvable (dangling reference) | HTTP 404 `{error:"missing_asset", assetId}`; natural case: submitting an unknown photo id behaves identically |
| `permission_denied` | any endpoint declaring a permission | the permission check fails even for a user who would normally pass | HTTP 403 `{error:"permission_denied", required, simulated:true}`; natural case: foreman calling a `po:approve` endpoint |
| `data_conflict` | PR decision, invoice approval, incident close | the record is treated as already decisioned by someone else | HTTP 409 `{error:"data_conflict", currentStatus}`; natural case: deciding the same purchase request twice |
| `duplicate_event` | any state-changing request | request signature is recorded and rejected as a replay; explicit `idempotencyKey` replays are always rejected | HTTP 409 `{error:"duplicate_event", key}`; natural case: a second progress report for the same project+date |
| `stale_entitlement` | PR approval (`decidePurchaseRequest`) | the vendor contract is treated as expired | HTTP 409 `{error:"stale_entitlement", vendorId, contractValidUntil}`; natural case: approving `prq-5001` (SteelCo contract expired 2026-08-01 under the fixture clock) |

## Fixture clock

The demo world believes "today" is `state.meta.today = 2026-09-12` (see `seed.js`), so seed dates are deterministic regardless of the real wall clock. Event timestamps use real time.

## Example driven paths

```bash
BASE=http://localhost:4101
# golden path — foreman submits daily progress (API twin of the browser form)
TOKEN=$(curl -s $BASE/api/login -H 'content-type: application/json' \
  -d '{"username":"marco.silva","password":"<from /api/demo-hints>"}' | jq -r .token)
curl -s "$BASE/api/progress?token=$TOKEN" -H 'content-type: application/json' \
  -d '{"projectId":"p-101","reportDate":"2026-09-13","percentComplete":65,"taskIds":["t-101c"],"photoIds":["a-101"],"notes":"Formwork on 13 started."}'
# failure path — same submit with the notification switch
curl -s "$BASE/api/progress?failure=notification_failure&token=$TOKEN" ... # 200 + warning + notification.failed event
```
