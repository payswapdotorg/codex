# Codex Universal — Validation Proving Ground

**Established by:** VWO-001 (Wave 0, `docs/validation/work-orders/VWO-001.md`)
**Mode:** **FALLBACK** (E2B/Composio connected integration not available — see §2)
**Program:** `docs/validation/VALIDATION-PROGRAM.md` §3 (proving-ground strategy)
**Proven base SHA:** `926c177a5515a1bf1316d7ef1bacb9b6d2d53611` (main @ dispatch of VWO-001)

This directory defines the reproducible environment every validation worker
(VWO-002 … VWO-010) enters to exercise Codex Universal as a product. It
records what the connected sandbox platform actually provisions, how to
bootstrap it deterministically, how to reset it, and exactly which fidelity
limitations apply because the preferred E2B desktop workspaces could not be
provisioned.

The single source of truth for re-verification is:

```bash
bash docs/validation/proving-ground/capability_check.sh          # required checks gate the exit code
bash docs/validation/proving-ground/capability_check.sh --strict # also gate E2B + desktop fidelity
```

Every check prints `[PASS]`/`[FAIL]`. Nothing passes silently.

---

## 1. Environment inventory — what the connected sandbox provides

Probed 2026-09-11 from inside the connected agent sandbox (the platform
integration surface). Exact probe commands and raw outputs are recorded in
`docs/validation/reports/VWO-001-report.md` and committed as raw transcripts
under `docs/validation/evidence/VWO-001/`.

| Surface | Status | Evidence (re-runnable) |
|---|---|---|
| Terminal | **PRESENT** | bash; `git --version` → 2.47.3; stdout/stderr plumbing verified |
| File API | **PRESENT** | project root `/home/z/my-project` visible + writable via the platform file API (Next.js app tree present) |
| Network | **PRESENT** | `git ls-remote https://github.com/payswapdotorg/codex HEAD` returns a SHA; GitHub HTTPS 200 |
| Browser engine | **PRESENT** | Playwright chromium `~/.cache/ms-playwright/chromium-1234/chrome-linux64/chrome` (plus chromium-1200, headless shells, ffmpeg); headless `--dump-dom` render verified |
| Browser automation CLI | **PRESENT** | `agent-browser` 0.35.0 at `/usr/local/bin/agent-browser`; live navigation + accessibility snapshot verified |
| Virtual desktop | **PRESENT (virtual)** | `Xvfb` (Debian xvfb 2:21.1.16) verified live at 1280x800x24; chromium GUI-mode rendered into the framebuffer (32.7 % non-black pixels); `ffmpeg -f x11grab` capture verified |
| Desktop input injection | **PRESENT (user-space)** | `xdotool` 3.20160805.1 acquired via `apt-get download` + `dpkg-deb -x` into `~/.local/desktop-tools` (no root); synthetic keystrokes verified received by an X client (`xev`, 22 key events incl. keysym F13) |
| Screen capture | **PRESENT** | `ffmpeg` (system) x11grab → PNG, decoded and verified non-trivial |
| Node/Python runtimes | **PRESENT** | node v24.19.0; bun 1.3.14; Python 3.12.14 |
| Rust toolchain | **INSTALLABLE + PINNED** | rustup installs pinned `1.95.0` (per `codex-rs/rust-toolchain.toml`); verified `rustc 1.95.0 (59807616e 2026-04-14)` |
| API/tool surface | **PRESENT (platform)** | platform SDK `z-ai-web-dev-sdk` installed in the sandbox project (name recorded only); platform skills system available to the agent (names only) |
| MCP servers | **ABSENT** | no `.mcp.json` / MCP server configuration exposed to the sandbox |
| Human gate | **OPERATOR CHANNEL** | the IM chat session between operator and agent session; no in-sandbox approval widget is exposed to workers |
| E2B/Composio connected integration | **ABSENT** | no `E2B_API_KEY`/`COMPOSIO_API_KEY` env vars, no `e2b`/`composio` CLIs, no `~/.e2b`/`~/.composio` configs (names/state only — values never recorded); `e2b.dev` and `api.e2b.dev` are network-reachable (TLS/HTTP respond) but no credentials exist to provision |
| Live X server on session start | **ABSENT** | `$DISPLAY` empty; no `/tmp/.X11-unix`; no Wayland |
| VNC / interactive remote desktop | **ABSENT** | no `Xvnc`, `x11vnc`, `vncserver`; no window manager; `sudo` unavailable (password required) |
| Root/admin | **ABSENT** | non-root user (uid 1001); packages can still be acquired via `apt-get download` + user-space extraction |

**Workspace identity.** There is no E2B workspace/template identity to
record. The proving ground is the connected agent sandbox itself: one
container per session, host kernel `5.10.134`, Debian 13 (trixie) userland,
single externally exposed port (3000) behind the platform gateway. This is a
*single sandbox per session*, not an isolated pool of three desktop
workspaces.

---

## 2. Fallback record (explicit, per VWO-001 and VALIDATION-PROGRAM.md §3)

**Preferred path NOT provisioned.** The preferred proving ground — up to
three isolated desktop-capable E2B workspaces created through the connected
Composio/E2B integration — could not be provisioned, because the connected
integration is not present in this sandbox: no E2B/Composio credentials are
injected (env vars unset), no CLIs are installed, and no platform-level
config exists. Reachability alone (TLS to `e2b.dev`/`api.e2b.dev` responds)
does not provision anything without credentials, and no credentials may be
invented or placed into source, per the VWO-001 Forbidden list.

**Fallback path (this environment).** Per `VALIDATION-PROGRAM.md` §3 and the
VWO-001 packet, the existing agent sandbox is the reduced-fidelity proving
ground, and the program proceeds (do not delay waves on E2B availability).
Retry E2B provisioning only when the platform actually connects the
integration; re-run `capability_check.sh` to detect that moment (check 7
flips from `[FAIL]` to a surface report).

### 2.1 Fidelity limitations of the fallback (all explicit)

1. **No isolated multi-workspace desktop pool.** One sandbox per session.
   The three-worker concurrency of the program therefore means three agent
   sessions, each with its own container — not three isolated desktops
   inside one orchestrated pool with shared E2B lifecycle operations.
2. **No interactive human-observable desktop.** No VNC and no live X server
   at session start. "Desktop" means a virtual framebuffer (Xvfb) observed
   via screenshots (`ffmpeg x11grab`) and accessibility snapshots, not a
   screen a human can watch or drive interactively.
3. **Input injection is user-space and synthetic.** `xdotool` must be
   acquired per-sandbox via `apt-get download` + `dpkg-deb -x` (no root, no
   `sudo`). There is no window manager, so focus/stacking must be driven
   explicitly with `xdotool windowfocus`/`key --window`.
4. **No provider lifecycle.** E2B snapshot/pause/resume/fork operations do
   not exist here. Restart/session-loss validation must be exercised against
   the Codex-repo processes themselves (kill/restart binaries, durable state
   on disk), not against a provider control plane.
5. **No E2B template identity to record** — the workspace identity of record
   is "connected agent sandbox, single container per session".
6. **Human-gate fidelity.** Approvals flow through the operator chat channel
   (the IM session), which is lower fidelity than an in-application human
   gate driven inside a desktop workspace. Workers must record approval-path
   friction as product findings rather than fabricating an in-sandbox gate.

### 2.2 What the fallback still genuinely provides

- real browser execution and automation (Browser-Use-class surface): verified headless render and `agent-browser` navigation;
- a virtual desktop with real GUI rendering, screen capture, and synthetic keyboard/mouse input (Computer-Use-class substrate at X11 level);
- real terminal + file API + git transport to the exact repository SHA;
- the pinned Rust toolchain and a buildable M4 workflow family (launch verification, §3.4);
- platform API/tool surface (SDK + agent skills; names only).

---

## 3. Bootstrap — exact, reproducible, per-worker

Run every step in a fresh agent sandbox session. All commands are recorded
as executed; a full transcript for the VWO-001 session is in
`docs/validation/reports/VWO-001-report.md`.

### 3.1 Clone the repository at a verified SHA

```bash
cd /home/z/my-project                      # file-API-visible project root
git clone --filter=blob:none https://github.com/payswapdotorg/codex
cd codex
git rev-parse 926c177a5515a1bf1316d7ef1bacb9b6d2d53611^{commit}   # verify the recorded base
git checkout -b <your-vwo-branch> 926c177a5515a1bf1316d7ef1bacb9b6d2d53611
# If the pinned SHA is unreachable, base on origin/main HEAD and RECORD the exact SHA you based on.
```

### 3.2 Install the pinned Rust toolchain

```bash
curl https://sh.rustup.rs | sh -s -- -y --default-toolchain 1.95.0
source "$HOME/.cargo/env"
rustup component add rust-src
rustc --version    # expect: rustc 1.95.0 (59807616e 2026-04-14)
cargo --version    # expect: cargo 1.95.0 (f2d3ce0bd 2026-03-21)
```

### 3.3 (Optional) acquire user-space desktop input tooling

No root is required:

```bash
mkdir -p ~/.local/desktop-tools && cd /tmp
apt-get download xdotool libxdo3 libxtst6 x11-utils xterm
for p in libxdo3 libxtst6 xdotool x11-utils xterm; do
  dpkg-deb -x /tmp/"$p"_*.deb ~/.local/desktop-tools/
done
export PATH="$HOME/.local/desktop-tools/usr/bin:$PATH"
export LD_LIBRARY_PATH="$HOME/.local/desktop-tools/usr/lib/x86_64-linux-gnu:$HOME/.local/desktop-tools/usr/lib"
xdotool --version   # expect: xdotool version 3.20160805.1
```

### 3.4 Launch verification — cargo check the M4 workflow family

This is the VWO-001 "does the current repo's workflow family build here"
gate (the M4 durable control-plane stack):

```bash
source "$HOME/.cargo/env"; cd /home/z/my-project/codex/codex-rs
cargo check \
  -p codex-workflow-contracts -p codex-workflow-forge -p codex-workflow-app \
  -p codex-workflow-triggers -p codex-workflow-evolution \
  -p codex-workflow-distribution -p codex-workflow-durable
# expected terminal state: "Finished" with zero errors (see VWO-001-report.md §5 for the recorded result)
```

### 3.5 Verify the browser surface

```bash
# headless engine render
CHROME=$(ls -d ~/.cache/ms-playwright/chromium-*/chrome-linux*/chrome | sort | tail -1)
"$CHROME" --headless --no-sandbox --disable-gpu \
  --dump-dom 'data:text/html,<h1>PROVING-GROUND-RENDER</h1>'
# platform automation CLI
agent-browser open https://example.com && agent-browser snapshot && agent-browser close
```

### 3.6 Bring up the virtual desktop (when a scenario needs GUI rendering)

```bash
setsid Xvfb :99 -screen 0 1280x800x24 >/tmp/xvfb.log 2>&1 < /dev/null &
sleep 2
# render a GUI app into the virtual display
CHROME=$(ls -d ~/.cache/ms-playwright/chromium-*/chrome-linux*/chrome | sort | tail -1)
DISPLAY=:99 "$CHROME" --no-sandbox --disable-gpu --disable-dev-shm-usage \
  --window-size=1280,800 --user-data-dir=/tmp/chrome-gui-profile '<url>'
# capture the framebuffer
ffmpeg -y -f x11grab -video_size 1280x800 -i :99 -frames:v 1 /tmp/frame.png
# inject input (after §3.3)
export PATH="$HOME/.local/desktop-tools/usr/bin:$PATH"
export LD_LIBRARY_PATH="$HOME/.local/desktop-tools/usr/lib/x86_64-linux-gnu:$HOME/.local/desktop-tools/usr/lib"
WID=$(DISPLAY=:99 xdotool search --name "<window title>" | head -1)
DISPLAY=:99 xdotool windowfocus --sync "$WID"
DISPLAY=:99 xdotool type --window "$WID" --delay 60 "text"
```

Notes: background processes are reaped between platform tool invocations —
**even with `setsid`/`nohup`** (verified: a detached `sleep` did not survive
the next invocation). Keep every Xvfb/app/capture pipeline inside a single
command invocation, or re-launch each step. There is no window manager;
always focus the target window explicitly. Long builds: run `cargo check`
in the foreground of one invocation; the on-disk target cache makes retries
incremental.

### 3.7 Run the capability check (the gate)

```bash
bash docs/validation/proving-ground/capability_check.sh
# exit 0 → required surface verified (fallback profile)
bash docs/validation/proving-ground/capability_check.sh --strict
# exit 1 → E2B/desktop fidelity still missing (expected in fallback; recorded, not silent)
```

---

## 4. Deterministic reset instructions

Reproducible teardown of everything bootstrap creates (per-sandbox):

```bash
# stop proving-ground processes (own PIDs only — never pkill Xvfb globally,
# other workers may own live displays in this shared fallback)
pkill -f 'chrome.*--user-data-dir=/tmp/chrome-gui-profile' 2>/dev/null
# kill your Xvfb instance by the PID you recorded at launch
# remove transient artifacts
rm -rf /tmp/chrome-gui-profile /tmp/xvfb*.log /tmp/*probe*.png /tmp/xev*.log \
       /tmp/cargo-check-*.log /tmp/*.deb
# reset the repository working tree (keeps clone, discards session edits)
cd /home/z/my-project/codex
git checkout -- . && git clean -fdx docs/validation/evidence 2>/dev/null
# full re-bootstrap from scratch (destroys the clone and toolchain):
#   rm -rf /home/z/my-project/codex ~/.cargo ~/.rustup ~/.local/desktop-tools
# then repeat §3.1–§3.7 exactly.
```

Evidence artifacts workers produce belong in `docs/validation/evidence/`
(not `/tmp`) before reset so they are committed and harvested.

---

## 5. Per-worker reproducibility steps (checklist)

1. Fresh agent session; confirm `/home/z/my-project` is file-API visible.
2. §3.1 clone + branch at the SHA recorded for your wave.
3. §3.2 toolchain install (pinned 1.95.0).
4. §3.4 `cargo check` the workflow family — must reach `Finished` with zero errors.
5. §3.5 browser verification — headless render + `agent-browser` roundtrip.
6. §3.3 + §3.6 when your scenario needs desktop rendering/input.
7. `bash docs/validation/proving-ground/capability_check.sh` — must exit 0.
8. Do your VWO work; record evidence under `docs/validation/evidence/`.
9. §4 reset; leave the working tree clean for the next worker.
10. Report per `docs/validation/worker-report-template.md`, quoting the exact
    SHA and the capability-check output.

---

## 6. Security and policy notes (binding on all workers)

- **No credentials or tokens** may be recorded in reports, code, evidence, or
  this directory. Capability *names* and set/unset state only. E2B/Composio
  identifiers never enter workflow semantics or source (VWO-001 Forbidden).
- E2B remains an **optional resource provider only** — never a workflow
  semantic dependency, never a second workflow engine, never
  provider-specific workflow concepts (TECH_LEAD_START_HERE.md §4 hard
  invariants, RESOURCE PROVIDER RULE).
- Capability is not authorization; external outputs stay untrusted
  (TECH_LEAD_START_HERE.md §13).
- The fallback record is **honest by construction**: absent capabilities are
  recorded as absent, never silently passed. `capability_check.sh --strict`
  exists precisely so nobody can mistake the fallback for full fidelity.
