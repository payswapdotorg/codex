# ForgeOps (software/Google-like family) — deterministic failure switches

Activate per-request by appending `?failure=<switch>` to any request **or** by sending header `X-Failure-Switch: <switch>`. The switch applies only to the request that carries it. Unknown names return HTTP 400 with the known list. Machine-readable copy: `GET /api/failures`. HTML copy: `/failures`.

| Switch | Applies to | Behavior | Observable symptom |
|---|---|---|---|
| `service_unavailable` | any state-changing request | the synthetic `ci-runner-pool` dependency is treated as down | HTTP 503 `{error:"service_unavailable", service:"ci-runner-pool", simulated:true}`; reads still work |
| `notification_failure` | PR open/approve/merge, promote, rollback, incident open/resolve | operation succeeds; in-app notification stored `failed` | HTTP 200 + `warning`; failed row in `/notifications`; `notification.failed` event |
| `missing_asset` | promote deployment, `GET /api/artifacts/:id` | release artifact treated as missing from registry | HTTP 404 `{error:"missing_asset", artifactId}` |
| `permission_denied` | any endpoint declaring a permission | permission check fails even for authorized users | HTTP 403 `{error:"permission_denied", simulated:true}`; natural: self-approval is blocked by policy; an intern cannot promote |
| `data_conflict` | merge, approve, re-run CI, promote, rollback, incident resolve, doc update | record treated as concurrently changed | HTTP 409 `{error:"data_conflict"}`; natural: merging an already-merged PR, editing a doc on a stale `expectedVersion` |
| `duplicate_event` | any state-changing request | request signature recorded + rejected as replay; explicit `idempotencyKey` replays rejected | HTTP 409 `{error:"duplicate_event"}`; natural: same reviewer approving twice, second PR from the same branch |
| `stale_entitlement` | promote deployment | production release entitlement treated as expired | HTTP 409 `{error:"stale_entitlement", entitlement:"release-license"}` (simulated-only in this family — the seed license is valid so the golden path works) |

## Fixture clock

The demo world believes "today" is `state.meta.today = 2026-09-12` (see `seed.js`). Event timestamps use real time.

## Terminal desktop console

`node deploy-console.js [--port 4102]` — interactive readline client over the same HTTP API (`help` lists commands; supports scripted piping). It is the family's desktop-capable surface; it is a client only and holds no state.
