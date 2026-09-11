# RidePilot (ride-share family) — deterministic failure switches

Activate per-request by appending `?failure=<switch>` to any request **or** by sending header `X-Failure-Switch: <switch>`. The switch applies only to the request that carries it. Unknown names return HTTP 400 with the known list. Machine-readable copy: `GET /api/failures`. HTML copy: `/failures`.

| Switch | Applies to | Behavior | Observable symptom |
|---|---|---|---|
| `service_unavailable` | any state-changing request | the synthetic `dispatch-payout-gateway` dependency is treated as down | HTTP 503 `{error:"service_unavailable", service:"dispatch-payout-gateway", simulated:true}`; reads still work |
| `notification_failure` | intake, screening, escalation, surge broadcast, incident report/resolve | operation succeeds; notification/broadcast stored `failed` | HTTP 200 + `warning`; failed row in `/notifications`/broadcasts; `notification.failed` event |
| `missing_asset` | screening approval | a required onboarding document is treated as not on file | HTTP 404 `{error:"missing_asset", document:"insurance"}`; natural case: approving `da-0302` (Elena Petrova — insurance missing) |
| `permission_denied` | any endpoint declaring a permission | permission check fails even for authorized users | HTTP 403 `{error:"permission_denied", simulated:true}`; natural case: support agent screening, finance adjusting surge |
| `data_conflict` | screening, escalation, resolve, surge, payout approval, incident resolve | record treated as concurrently decided | HTTP 409 `{error:"data_conflict"}`; natural case: screening an already-decided application, approving a processed payout |
| `duplicate_event` | any state-changing request | request signature recorded + rejected as replay; idempotencyKey replays rejected | HTTP 409 `{error:"duplicate_event"}`; natural case: public intake submitting the same plate twice |
| `stale_entitlement` | driver activation, payout approval | regional driver permit/entitlement treated as expired | HTTP 409 `{error:"stale_entitlement"}`; natural cases: activating `da-0303` (permit expired 2026-08-01), approving `pay-0903` (Lena Marsh, permit ended 2026-07-01) |

## Fixture clock

The demo world believes "today" is `state.meta.today = 2026-09-12` (see `seed.js`). Event timestamps use real time.

## Golden human path (driver onboarding)

1. Public intake: `/intake` (no login) — applicant submits name/region/vehicle.
2. Ops manager signs in → `/onboarding` → screens the application (approve).
3. Ops manager activates the approved application → driver appears in the directory and on the region map; welcome broadcast recorded.
4. Failure path: screen with `?failure=missing_asset` (or approve `da-0302`), activate `da-0303` (naturally stale permit) or with `?failure=stale_entitlement`.
