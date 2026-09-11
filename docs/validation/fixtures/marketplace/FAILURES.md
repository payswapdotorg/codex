# FlowMart (marketplace family) — deterministic failure switches

Activate per-request by appending `?failure=<switch>` to any request **or** by sending header `X-Failure-Switch: <switch>`. The switch applies only to the request that carries it. Unknown names return HTTP 400 with the known list. Machine-readable copy: `GET /api/failures`. HTML copy: `/failures`.

| Switch | Applies to | Behavior | Observable symptom |
|---|---|---|---|
| `service_unavailable` | any state-changing request | the synthetic `registry-blob-store` dependency is treated as down | HTTP 503 `{error:"service_unavailable", service:"registry-blob-store", simulated:true}`; catalog/search reads still work |
| `notification_failure` | listing/version publish, install, upgrade, rollback, entitlement grant/revoke, review | operation succeeds; in-app notification stored `failed` | HTTP 200 + `warning`; failed row in `/notifications`; `notification.failed` event |
| `missing_asset` | package install | package archive treated as missing from the registry blob store | HTTP 404 `{error:"missing_asset", version}` |
| `permission_denied` | any endpoint declaring a permission | permission check fails even for authorized users | HTTP 403 `{error:"permission_denied", simulated:true}`; natural cases: org member (non-admin) installing, buyer publishing listings, publisher granting entitlements |
| `data_conflict` | version publish, install (`expectedVersion`), upgrade (`expectedCurrentVersion`), rollback, entitlement revoke, provenance verify | optimistic-concurrency mismatch / concurrent change / digest mismatch | HTTP 409 `{error:"data_conflict", currentVersion?}`; natural cases: installing an out-of-date `expectedVersion`, re-revoking an entitlement |
| `duplicate_event` | any state-changing request | request signature recorded + rejected as replay; idempotencyKey replays rejected | HTTP 409 `{error:"duplicate_event"}`; natural cases: duplicate listing slug, duplicate version, second install for the same org, second review from the same org |
| `stale_entitlement` | install, upgrade, rollback, configure | entitlement treated as expired/revoked; commercial operations blocked | HTTP 409 `{error:"stale_entitlement", entitlementId, validUntil}`; natural cases: Northwind Logistics installing `pk-101` (trial expired 2026-08-01); any operation on an install whose entitlement was revoked |

## Scope boundary (VWO-002 forbidden list)

FlowMart deliberately contains **no workflow semantics and no execution engine**. Packages are opaque artifacts: a manifest, a sha256 digest computed at publish time, and a provenance record (source repo, commit, builder, signer). Install/upgrade/rollback are *commercial records*, not executions. Workflow definition and execution belong to Codex Universal — testing those against this fixture is out of VWO-002's scope by design.

## Fixture clock

The demo world believes "today" is `state.meta.today = 2026-09-12` (see `seed.js`). Event timestamps use real time.

## Golden human path (discover → install → configure → upgrade → rollback → verify)

1. Buyer admin (iris.chen) signs in → `/catalog` → searches "triage" → opens the listing.
2. Installs `support-triage-assistant` (Acme already has an entitlement; Northwind would auto-grant a trial).
3. Configures the install → publisher releases a new version → buyer upgrades → buyer rolls back.
4. Auditor (mia.torres) verifies provenance → digest matches.
5. Failure paths: Northwind installing `invoice-autofill` (`stale_entitlement` — expired trial), org member installing (`permission_denied`), install with stale `expectedVersion` (`data_conflict`), same slug twice (`duplicate_event`).
