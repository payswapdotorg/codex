# PressRoom (media family) — deterministic failure switches

Activate per-request by appending `?failure=<switch>` to any request **or** by sending header `X-Failure-Switch: <switch>`. The switch applies only to the request that carries it. Unknown names return HTTP 400 with the known list. Machine-readable copy: `GET /api/failures`. HTML copy: `/failures`.

| Switch | Applies to | Behavior | Observable symptom |
|---|---|---|---|
| `service_unavailable` | any state-changing request | the synthetic `media-asset-pipeline` dependency is treated as down | HTTP 503 `{error:"service_unavailable", service:"media-asset-pipeline", simulated:true}`; reads still work |
| `notification_failure` | submit-for-review, decision, publish, correction, social posting | operation succeeds; notification record / social delivery stored `failed` | HTTP 200 + `warning`; failed row in `/notifications`; `notification.failed` event; social post marked failed |
| `missing_asset` | attach asset, publish story | referenced asset / hero image treated as unresolvable | HTTP 404 `{error:"missing_asset"}`; natural case: publishing a story with no hero image attached |
| `permission_denied` | any endpoint declaring a permission | permission check fails even for authorized users | HTTP 403 `{error:"permission_denied", simulated:true}`; natural cases: writer approving own story (self-approval policy), writer publishing |
| `data_conflict` | edit, submit, decide, publish, correction, rendition, social publish | optimistic-concurrency mismatch / record concurrently changed | HTTP 409 `{error:"data_conflict", currentVersion}`; natural case: saving an edit from a stale `expectedVersion` |
| `duplicate_event` | any state-changing request | request signature recorded + rejected as replay; idempotencyKey replays rejected | HTTP 409 `{error:"duplicate_event"}`; natural cases: publishing twice, two posts for same story+network, duplicate story title |
| `stale_entitlement` | publish story to licensed channels | channel distribution license treated as expired; that channel job fails while valid channels still get the story | HTTP 200 + `warning`; failed distribution job `error:"stale_entitlement"`; natural case: publishing to `partner-app` (license expired 2026-06-01) |

## Fixture clock

The demo world believes "today" is `state.meta.today = 2026-09-12` (see `seed.js`). Event timestamps use real time.

## Golden human path (editorial publishing)

1. Writer signs in → `/desk` → creates a story draft → attaches a hero image → submits for review.
2. Editor signs in → opens the story → approves.
3. Editor-in-chief/section editor publishes to selected channels → distribution jobs sent; front page updated.
4. Social manager schedules/posts social updates; corrections re-distribute the story.
5. Failure paths: publish with no hero (`missing_asset`), publish to `partner-app` (`stale_entitlement`), approve own story (`permission_denied`), stale `expectedVersion` edit (`data_conflict`).
