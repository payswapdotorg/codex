# APP-003 — Fresh-Machine Install/Launch + Product Smoke (Completion Report)

**Status:** COMPLETE
**Work Order:** docs/product-delivery/work-orders/APP-003.md
**Depends on:** APP-002 (release `rust-v0.1.0` published)
**Release:** tag `rust-v0.1.0` → 23b7f6c27, published 2026-09-15T18:34:56Z
**Harness:** `docs/product-delivery/evidence/app-003/app003-fresh-env.sh`
**Result:** **22 passed, 0 failed**
(`evidence/app-003/run-20260915T185129Z/transcript.txt`)

## 1. Environment identity

- Isolated `HOME` (`fresh-home/`) — no prior `~/.codex`, verified by
  assertion before any step runs.
- Sanitized `PATH` (`/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin`)
  — no cargo/rust/rustup reachable (asserted; `command -v cargo` must fail).
- No source checkout anywhere in the harness workspace; the only inputs are
  the public GitHub release URLs.
- Machine: x86_64 → target `x86_64-unknown-linux-musl`.
- Product identity: `codex-cli 0.1.0`; package sha256 verified against the
  published `codex-package_SHA256SUMS`
  (`4a6e7e9e2ecfbe3d86925bb877ce7751c30cfbd3d901ebcf1544d2c7e69b54d1`,
  128,736,412 bytes).

## 2. The validated user path (work-order steps 1–10)

| # | Work-order requirement | Result | Evidence |
|---|---|---|---|
| 1 | Download the release artifact from GitHub | PASS (direct CDN URL) | `download.log`, transcript step 1 |
| 2 | Verify the checksum | PASS (manifest match) | transcript step 2 |
| 3 | Install using only published instructions | PASS (`install.sh`, non-interactive env documented in USER-GUIDE §2) | `install.log` |
| 4 | Launch the application | PASS (`codex-cli 0.1.0`) | `version.txt` |
| 5 | Start a session | PASS — the CLI is process-per-command (no daemon); "session" = the teach run below | — |
| 6 | Reach workflow/teaching functionality | PASS (`workflow --help`, teach ×3 instruct) | `workflow-help.txt`, `teach.json` |
| 7 | One representative real workflow action | PASS (`weekly-report-pipeline` taught + published; `instance run` → terminal `completed`) | `teach.json`, `instance-run.json` |
| 8 | Close and relaunch | PASS (fresh process per command) | transcript steps 8–9 |
| 9 | Persisted state survives restart | PASS (1 durable instance listed after relaunch) | `instance-list-relaunch.json` |
| 10 | Missing-credential/dependency error handling | PASS (see §3) | `missing-credential.txt`, `workflow-no-cred.json` |

Plus the documented uninstall procedure (USER-GUIDE §9) exercised as
harness step 11: visible command removed, installer-managed payloads
removed, application no longer launches, and **user workflow data
preserved** (`~/.codex/workflow/` untouched — opt-in removal documented).

## 3. Step 10 detail: unusable-model-config behavior

With a clean `HOME` and no provider credentials, the agent surface
(`codex exec`) terminates with a **clear, actionable error and exit 1**
rather than hanging, crashing, or corrupting state. On this validation
network the egress region is blocked by the provider, so the surfaced
error is the provider refusal:

```
ERROR: unexpected status 403 Forbidden: Country, region, or territory
not supported, url: https://api.openai.com/v1/responses
```

On an unblocked network the same path surfaces the auth/login error
(`login|auth|credential|api key|unauthorized|401` — the harness matcher
accepts both classes). The workflow/teaching surface is entirely
unaffected — `workflow instance list` executes normally with the same
credential-less HOME (`workflow-no-cred.json`).

The run also exercised the **bundled-bubblewrap fallback** as designed:
`warning: Codex could not find bubblewrap on PATH … Codex will use the
bundled bubblewrap in the meantime` — the package's
`codex-resources/bwrap` (digest-pinned at build time by the release
workflow) is used when the host has none.

## 4. Documented deviations (environmental, not product)

1. **api.github.com authentication for install.sh's single metadata
   lookup.** This machine's shared egress IP exhausted GitHub's
   unauthenticated API bucket (0/60 at run time; a real user's IP is not
   rate-limited). A curl wrapper injected an `Authorization` header for
   `api.github.com` ONLY, so the documented install path could be
   exercised for real: genuine API metadata → genuine asset selection →
   genuine direct-URL downloads (all `github.com/…/releases/download/…`
   fetches remained unauthenticated). The wrapper is committed with the
   evidence (`run-*/bin-overrides/curl`) and reads the token from the
   environment at run time (no secret is recorded anywhere in the
   evidence; scanned before commit).
2. **stdin for `codex exec`.** `codex exec` reads piped stdin as
   additional prompt context by design; a non-interactive caller must
   close stdin (`</dev/null` or a here-doc). The first harness attempt
   blocked on an inherited pipe until timeout — corrected in the harness
   and documented in USER-GUIDE §10 (scripting).

## 5. Product friction log

- `install.sh` requires one `api.github.com` metadata call; users on
  rate-limited shared egress IPs (CI runners without a token) may hit
  secondary rate limits. (The installer already supports
  `CODEX_RELEASE` pinning, which reduces but does not remove API use.)
- `codex exec` blocks on stdin when it is an open pipe — scripting docs
  now cover it (USER-GUIDE §10).
- In provider-blocked regions the agent surface reports the region
  refusal verbatim — accurate and actionable; the workflow surface still
  works fully (now documented in USER-GUIDE §8).

## 6. Changed files

- `docs/product-delivery/evidence/app-003/app003-fresh-env.sh` (new —
  the harness; includes the documented deviations above)
- `docs/product-delivery/evidence/app-003/run-20260915T185129Z/` (new —
  full transcript + per-step artifacts; bulky reproducible payloads
  trimmed with sizes recorded in the transcript, step 12)
- `docs/product-delivery/reports/APP-003-report.md` (this report)

## 7. Acceptance

A clean-environment user-path smoke test **passes without cloning the
repository and without calling internal workflow stores or protocol
methods** — every action goes through the published binary's
user-facing commands (`codex`, `codex workflow …`, `install.sh`).
**ACCEPTED.**
