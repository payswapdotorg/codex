# VWO-001 Report — E2B Desktop Resource Proving Ground

**Work Order:** VWO-001 (Wave 0, `docs/validation/work-orders/VWO-001.md`)
**Worker:** VWO-001 Worker (validation-infrastructure specialist)
**Date/time window:** 2026-09-11 (UTC)
**Base branch:** `main`
**Base SHA:** `926c177a5515a1bf1316d7ef1bacb9b6d2d53611` (verified: `git rev-parse 926c177a5^{commit}` == `origin/main` HEAD at clone time)
**Head SHA:** recorded in the branch commit; see `git log vwo-001/e2b-proving-ground` after merge harvesting.
**Validation environment:** connected agent sandbox (the platform integration surface), NOT an E2B workspace — see §2.
**Deliverables:** `docs/validation/proving-ground/README.md`, `docs/validation/proving-ground/capability_check.sh`, this report, and raw evidence under `docs/validation/evidence/VWO-001/`.

**Status:** MERGED via PR (squash) — independently verified by the Tech Lead (see the addendum at the end of this report).

---

## 1. Objective and outcome

Provision and validate up to three isolated desktop-capable E2B workspaces
via the connected E2B/Composio integration when available, and establish a
reproducible fallback environment when it is not.

**Outcome:** the connected E2B/Composio integration is **not available** in
the agent sandbox (§2.1). The **fallback record** is therefore the governing
result (§2.2), with the existing agent sandbox established as the
reduced-fidelity proving ground: verified browser, terminal, file API,
network, pinned toolchain, buildable M4 workflow family, and a virtual
desktop substrate (Xvfb + capture + synthetic input injection). The
reproducible bootstrap, capability checks, reset procedure, and per-worker
steps are committed at `docs/validation/proving-ground/`.

## 2. Provisioning record — E2B/Composio availability (the decisive question)

### 2.1 Discovery probes (exact commands, results, no credential values)

**(a) credential env vars — set/unset state only:**

```bash
for v in E2B_API_KEY E2B_TEAM_ID E2B_DOMAIN E2B_API_URL E2B_ACCESS_TOKEN \
         COMPOSIO_API_KEY COMPOSIO_CLI_API_KEY COMPOSIO_BASE_URL E2B_TEMPLATE_ID; do
  [ -n "$(printenv $v)" ] && echo "$v: SET (value withheld)" || echo "$v: NOT SET"; done
```

Result: **all NOT SET** (E2B_API_KEY, E2B_ACCESS_TOKEN, E2B_TEAM_ID, E2B_TEMPLATE_ID,
COMPOSIO_API_KEY, COMPOSIO_CLI_API_KEY, COMPOSIO_BASE_URL, …). No credentials
are injected into this sandbox by the platform.

**(b) CLIs:**

```bash
for c in e2b composio e2b-cli composio-cli; do command -v $c || echo "$c: ABSENT"; done
```

Result: **all ABSENT**. No E2B or Composio CLI is installed.

**(c) platform config (existence only):** `~/.e2b`, `~/.composio`,
`~/.config/e2b`, `~/.config/composio`, `/home/z/my-project/.mcp.json`,
`/home/z/my-project/codex/.mcp.json`, `~/.mcp.json` — **all ABSENT**.

**(d) network reachability (10 s HTTPS probes):**

```bash
for host in api.e2b.dev api.composio.dev api.composio.com e2b.dev composio.dev; do
  curl -s -o /dev/null -m 10 -w "$host => HTTP %{http_code}\n" https://$host/; done
```

Result:

| Host | Result |
|---|---|
| `e2b.dev` | HTTP 200 — reachable |
| `api.e2b.dev` | HTTP 404 — reachable (TLS + HTTP respond; 404 on bare path is expected for an API root) |
| `composio.dev` | HTTP 200 — reachable |
| `backend.composio.dev` | HTTP 200 — reachable |
| `api.composio.dev` / `.com` / `.io` | DNS resolution failure (curl rc=6) |

**Conclusion:** the network path to E2B control planes exists, but with zero
credentials, zero CLI, zero platform config, the *connected integration* is
absent. Per the VWO-001 Forbidden list, no credentials may be invented or
placed into source, so **E2B workspace provisioning is not possible from this
sandbox**. This is a platform-supplied-integration absence, not a network
outage.

### 2.2 Fallback record (explicit, per VWO-001 acceptance)

- **Workspace/template identity:** none — no E2B objects exist. The proving
  ground identity of record is: *connected agent sandbox, single container
  per session* (Debian 13 userland, kernel 5.10.134, non-root uid 1001, one
  externally exposed port 3000 behind the platform gateway).
- **Reduced fidelity (recorded, not silent):**
  1. no isolated multi-workspace desktop pool — one sandbox per session; the
     three-worker program concurrency means three independent sessions, not
     three orchestrated desktops;
  2. no interactive human-observable desktop — no VNC binaries (`Xvnc`,
     `x11vnc`, `vncserver` all absent), no live X server at session start
     (`$DISPLAY` empty, `/tmp/.X11-unix` absent), no Wayland;
  3. input injection is user-space-only xdotool (no root/sudo — `sudo -n`
     fails), with no window manager, so focus must be driven explicitly;
  4. no E2B lifecycle operations (snapshot/pause/resume/fork);
  5. human-gate path = operator chat channel, not an in-workspace approval UI.
- Per `VALIDATION-PROGRAM.md` §3: *do not delay the program because E2B is
  unavailable* — waves proceed in this fallback; E2B should be retried if the
  platform ever connects it (`capability_check.sh` check 7 flips when it does).

## 3. Environment inventory (verified, exact evidence)

| Surface | Command | Result |
|---|---|---|
| terminal | `whoami; git --version; uname -a` | user `z`, git 2.47.3, Linux 5.10.134 x86_64 |
| file API | `ls /home/z/my-project/src/app/page.tsx` | present — project root is file-API-visible (Next.js platform app) |
| repo reachability | `git ls-remote …/codex HEAD` | `926c177a5515a1bf1316d7ef1bacb9b6d2d53611` |
| browser engine | `ls ~/.cache/ms-playwright` | `chromium-1200`, `chromium-1234`, `chromium_headless_shell-*`, `ffmpeg-1011` |
| headless render (live) | `chromium-1234 … --dump-dom 'data:text/html,<h1>PROVING-GROUND-RENDER</h1>'` | DOM returned containing `PROVING-GROUND-RENDER` (rc=0) |
| browser automation (live) | `agent-browser open https://example.com; agent-browser snapshot; agent-browser close` | title "Example Domain", a11y snapshot with refs (e1/e2) — `agent-browser 0.35.0` |
| virtual display (live) | `setsid Xvfb :99 -screen 0 1280x800x24 &` | display stayed up during the invocation; ffmpeg x11grab captured 1280x800 frame |
| GUI render into display (live) | chromium GUI-mode on `:99`, capture + PNG decode | **32.7 % non-black pixels — real window content rendered** (PNG 24,433 bytes, 1280x800) |
| input injection (live) | `xev` on `:99` + `xdotool key F13` / `xdotool type "PROBE"` | **22 KeyPress/KeyRelease events received, incl. keysym 0xffca F13** — synthetic input reaches X clients |
| xdotool acquisition | `apt-get download xdotool libxdo3 libxtst6 x11-utils xterm` + `dpkg-deb -x ~/.local/desktop-tools/` | works without root; `xdotool version 3.20160805.1` (needs `LD_LIBRARY_PATH` from the extraction — documented in README §3.3) |
| runtimes | `node --version; bun --version; python3 --version` | v24.19.0 / 1.3.14 / 3.12.14 |
| root/admin | `sudo -n true` | unavailable (password required) — hence user-space extraction strategy |
| platform API/tool surface | `rg z-ai-web-dev-sdk package.json` | present + installed in the platform project (name only) |
| MCP | config probe §2.1(c) | no MCP server configuration exposed |
| process persistence | `setsid sleep 200 &` in one invocation, `pgrep` in the next | **REAPED** — background processes do not survive between platform tool invocations even with setsid/nohup; all pipelines must be single-invocation |

## 4. Launch verification — M4 workflow family builds in this sandbox

Toolchain install (per the repo pin in `codex-rs/rust-toolchain.toml`):

```bash
curl https://sh.rustup.rs | sh -s -- -y --default-toolchain 1.95.0
rustup component add rust-src
rustc --version   # rustc 1.95.0 (59807616e 2026-04-14)
cargo --version   # cargo 1.95.0 (f2d3ce0bd 2026-03-21)
```

Family check (all seven M4 workflow crates):

```bash
cd /home/z/my-project/codex/codex-rs
cargo check -p codex-workflow-contracts -p codex-workflow-forge -p codex-workflow-app \
  -p codex-workflow-triggers -p codex-workflow-evolution \
  -p codex-workflow-distribution -p codex-workflow-durable
```

Result:

```
Checking codex-model-provider v0.0.0 (…/model-provider)
Checking codex-workflow-durable v0.0.0 (…/workflow-durable)
Checking codex-eval-compat v0.0.0 (…/eval-compat)
Checking codex-workflow-distribution v0.0.0 (…/workflow-distribution)
Checking codex-workflow-evolution v0.0.0 (…/workflow-evolution)
Finished `dev` profile [unoptimized + debuginfo] target(s) in 4m 15s
```

**exit code 0 — the M4 durable-control-plane stack (workflow-app /
workflow-durable / workflow-triggers and the full family) compiles clean
under the pinned toolchain in this sandbox.** This is the VWO-001 launch
verification: the current repository's workflow family is buildable in the
fallback proving ground, so workers can proceed to runtime launch.

The repository has no standalone workflow-app binary (workflow-app is a
library crate; its lifecycle/run/resume surfaces are exercised through the
Codex workspace). Application surfaces actually launched and verified inside
the proving ground this wave: the platform app surface on port 3000
(`HTTP 200`, dev.log clean) and the browser/automation surface (§3).

## 5. Capability checks — the re-runnable gate

Committed script: `docs/validation/proving-ground/capability_check.sh`
(9 checks; every check prints `[PASS]`/`[FAIL]`; required failures exit 1).

Recorded run on this environment (post-fix):

```
=== 1/9 terminal ===               [PASS] terminal executes commands …
=== 2/9 file-API ===               [PASS] project root visible and writable: /home/z/my-project
=== 3/9 repository reachability ===[PASS] git 2.47.3; codex reachable (HEAD 926c177a…)
=== 4/9 Rust toolchain ===          [PASS] rustc 1.95.0 + cargo 1.95.0 match pin 1.95.0
=== 5/9 browser availability ===   [PASS] agent-browser 0.35.0 + Playwright chromium-1234
=== 6/9 headless render (live) === [PASS] chromium headless rendered CAPCHECK-RENDER-MARKER
=== 7/9 E2B/Composio (fidelity) ===[FAIL] integration ABSENT — recorded fallback condition
=== 8/9 desktop (fidelity) ===     [PASS] Xvfb live on :137; ffmpeg capture OK;
                                    [PASS] xdotool input injection into :137
                                    [NOTE] VNC absent; level: virtual-display+capture+input
=== 9/9 human gate ===             [PASS] no session/chat ids leaked into env
RESULT: PASS — required capabilities verified (exit 0)
```

`--strict` run: `RESULT: STRICT FAIL` (exit 1) with exactly the E2B
fallback condition failing loudly — by design, so no one can mistake the
fallback for full fidelity.

The fail-loudly property was validated empirically: the first run caught a
real defect (user-space xdotool invoked without its `LD_LIBRARY_PATH`
failed the injection check); the script reported `[FAIL]` with the failing
check named, and the fix landed before commit.

## 6. Reset procedure (deterministic)

Recorded in `docs/validation/proving-ground/README.md` §4 — stop own
Xvfb/chromium instances by PID (never `pkill -x Xvfb` globally: other
workers may own live displays in this shared single-sandbox fallback),
remove `/tmp` artifacts, `git checkout -- .` in the clone, optional full
re-bootstrap (`rm -rf codex ~/.cargo ~/.rustup ~/.local/desktop-tools`) and
re-run §3.1–§3.7.

## 7. Acceptance-criteria mapping (VWO-001 packet → evidence)

| Packet requirement | Status | Evidence |
|---|---|---|
| Discover available E2B sandbox/template capabilities through the connected integration | DONE — discovery result: integration absent (no credentials/CLI/config; §2.1) | this report §2.1 |
| Prefer the desktop-capable template | N/A — no templates reachable without credentials; recorded, not invented | §2.1/§2.2 |
| Create isolated workspaces for validation workers | NOT POSSIBLE via E2B; fallback established: one connected sandbox per worker session | §2.2 |
| Clone the current repository at an exact verified SHA | DONE — `926c177a5515a1bf1316d7ef1bacb9b6d2d53611` verified == origin/main | report header, README §3.1 |
| Build/launch the actual Codex Universal runtime/application available in the repository | DONE — pinned toolchain installed; `cargo check` of all 7 workflow-family crates `Finished`, exit 0; platform app surface live on :3000 | §4 |
| Verify Browser Use, Computer Use, terminal, API/tool/MCP and human-gate paths the environment can support | DONE — browser (live render + automation), Computer-Use substrate (Xvfb + capture + verified input injection), terminal, file API, API/tool surface (platform SDK, name only); MCP absent; human gate = operator channel — each recorded with its status | §3, §5 |
| Capture workspace/template identity without recording credentials | DONE — identity of record: "connected agent sandbox, single container per session"; zero credentials recorded anywhere | §2.2 |
| Provide deterministic setup/reset instructions for workers | DONE — README §3 (bootstrap) and §4 (reset), re-runnable gate §3.7 | proving-ground/README.md |
| Explicitly record any desktop capability unavailable in the connected E2B account | DONE — the account itself is unconnected; unavailable: isolated multi-workspace pool, VNC/interactive desktop, WM, provider lifecycle | §2.2 |

Acceptance sentence: *A worker can enter the (fallback) proving ground,
launch the current Codex Universal application/runtime surfaces, interact
with browser and desktop capabilities, and reproduce the setup from
repository instructions — and the fallback record makes the reduced
fidelity explicit rather than silently claiming equivalence.* → satisfied.

## 8. Limitations, risks, deferred items

- **Limitations:** all fallback fidelity limits in §2.2; single-invocation
  process constraint (background reaping) shapes how workers script GUI
  pipelines; `cargo check` (not full `just test`) was the launch-verification
  bar per the Work Order design guidance.
- **Risks:** (1) platform sandbox capabilities may drift between sessions —
  mitigated by the committed `capability_check.sh` gate; (2) the fallback's
  human-gate fidelity is lower than an in-app gate, so Wave 1+ approval-path
  findings must be recorded as product findings; (3) if E2B is later
  connected, waves must re-evaluate which scenarios warrant re-running in the
  full proving ground (VWO-010 should decide from evidence).
- **Deferred:** E2B provisioning retry when the platform connects the
  integration (check 7 flips); full `just test` family sweep belongs to the
  owning M4/verification WOs, not VWO-001.
- **Security:** no credentials, tokens, or E2B identifiers recorded anywhere
  in this report, the README, the script, or evidence; only capability names
  and set/unset/reachability states. E2B is not a workflow semantic
  dependency; no provider-specific workflow concepts were created.

## 9. Worker conclusion

VWO-001 is **implemented in the fallback mode the packet explicitly
requires**: the E2B/Composio connected integration is absent from the
sandbox (recorded with exact probes), the existing agent sandbox is
established as the reduced-fidelity proving ground with a verified virtual
desktop substrate, and the reproducible bootstrap/gate/reset lives in the
repository at `docs/validation/proving-ground/`. Nothing was silently
claimed as full fidelity; the strict gate fails loudly exactly where
fidelity is reduced.

---

## Tech-Lead verification addendum (integration station, 2026-09-11)

- **Bundle acceptance:** strict base `926c177a5515a1bf1316d7ef1bacb9b6d2d53611` = origin/main at dispatch; single delivery commit `e4130f6d2`; scope exactly 9 files under `docs/validation/proving-ground/**`, `docs/validation/reports/VWO-001-report.md`, and `docs/validation/evidence/VWO-001/**` (+1,071 lines); zero codex-rs changes.
- **Capability gate re-run on the Tech Lead machine** (not a trust of the committed transcripts): `capability_check.sh` exit contract verified in all three modes — default shell without `~/.cargo/bin` on PATH → exit 1 with `RESULT: FAIL` (the fail-loudly contract works); with the pinned toolchain on PATH → exit 0 `RESULT: PASS — required capabilities verified`; `--strict` → exit 1 `RESULT: STRICT FAIL` (E2B/Composio absent + xdotool absent = the recorded fidelity gaps, correctly gated only under strict).
- **E2B/Composio absence independently corroborated:** the connected platform provides no E2B/Composio credentials, CLIs, or config to worker sandboxes — the fallback record is the governing result exactly as the packet requires ("failure to provision E2B must produce an explicit fallback record rather than silently claiming full fidelity").
- **No changes were made to the delivery** — zero integration fixes needed for this work order.
