#!/usr/bin/env bash
# capability_check.sh — VWO-001 proving-ground environment re-verification
#
# Purpose: any validation worker can re-run this script to verify what the
# connected sandbox platform actually provides for the Codex Universal
# human-workflow validation program. It checks the environment that was
# inventoried in docs/validation/proving-ground/README.md.
#
# Design rules (VWO-001):
#   * Every check prints an explicit [PASS]/[FAIL] verdict — never silent.
#   * A failed REQUIRED check fails the whole script with exit code 1.
#   * Fidelity checks (E2B/Composio integration, live desktop/VNC) print loud
#     FAIL verdicts and shape the FIDELITY PROFILE, but only gate the exit
#     code when --strict is passed, because their absence is the *expected,
#     recorded* fallback condition (VALIDATION-PROGRAM.md §3).
#   * Credential checks report capability NAMES and set/unset state ONLY.
#     Values are never printed, echoed, logged, or stored.
#
# Usage:
#   bash docs/validation/proving-ground/capability_check.sh [--strict]
#
# Exit codes:
#   0 — all required checks passed (fallback profile is acceptable)
#   1 — at least one required check failed, or --strict and any check failed

set -u

STRICT=0
[ "${1:-}" = "--strict" ] && STRICT=1

REQUIRED_FAILURES=0
STRICT_FAILURES=0

pass() { echo "[PASS] $1"; }
fail() { echo "[FAIL] $1  <== FAILING CHECK: $2"; }
note() { echo "[NOTE] $1"; }

hdr() { echo; echo "=== $1 ==="; }

# ---------------------------------------------------------------------------
# 1. REQUIRED — terminal works (command execution + stdout plumbing)
# ---------------------------------------------------------------------------
hdr "1/9 terminal"
if MARKER="capcheck-$RANDOM" OUT="$(echo "$MARKER")" && [ "$OUT" = "$MARKER" ]; then
  pass "terminal executes commands and returns stdout (${SHELL:-shell})"
else
  fail "terminal command roundtrip" "shell stdout/stderr plumbing broken"
  REQUIRED_FAILURES=$((REQUIRED_FAILURES + 1))
fi

# ---------------------------------------------------------------------------
# 2. REQUIRED — file API visibility (project root present and writable)
# ---------------------------------------------------------------------------
hdr "2/9 file-API (project root)"
PROJ_ROOT="${PROVING_GROUND_ROOT:-/home/z/my-project}"
if [ -d "$PROJ_ROOT" ] && touch "$PROJ_ROOT/.capcheck-write-probe" 2>/dev/null; then
  rm -f "$PROJ_ROOT/.capcheck-write-probe"
  pass "project root visible and writable: $PROJ_ROOT"
else
  fail "project root not writable or absent: $PROJ_ROOT" "file API surface"
  REQUIRED_FAILURES=$((REQUIRED_FAILURES + 1))
fi

# ---------------------------------------------------------------------------
# 3. REQUIRED — repository reachable (git transport to GitHub)
# ---------------------------------------------------------------------------
hdr "3/9 repository reachability"
if command -v git >/dev/null 2>&1; then
  REMOTE_SHA="$(git ls-remote https://github.com/payswapdotorg/codex HEAD 2>/dev/null | cut -f1)"
  if [ -n "$REMOTE_SHA" ]; then
    pass "git present ($(git --version)) and payswapdotorg/codex reachable (HEAD $REMOTE_SHA)"
  else
    fail "git ls-remote could not reach https://github.com/payswapdotorg/codex" "network/GitHub access"
    REQUIRED_FAILURES=$((REQUIRED_FAILURES + 1))
  fi
else
  fail "git CLI not found" "git installation"
  REQUIRED_FAILURES=$((REQUIRED_FAILURES + 1))
fi

# ---------------------------------------------------------------------------
# 4. REQUIRED — Rust toolchain present at the pinned channel
# ---------------------------------------------------------------------------
hdr "4/9 Rust toolchain (pinned by codex-rs/rust-toolchain.toml)"
PINNED="1.95.0"
if command -v cargo >/dev/null 2>&1; then
  RUSTC_VER="$(rustc --version 2>/dev/null | awk '{print $2}')"
  if [ "$RUSTC_VER" = "$PINNED" ]; then
    pass "rustc $RUSTC_VER + cargo $(cargo --version | awk '{print $2}') match pin $PINNED"
  else
    fail "rustc is '$RUSTC_VER' but repo pin is '$PINNED'" "pinned toolchain mismatch (run: curl https://sh.rustup.rs | sh -s -- -y --default-toolchain $PINNED)"
    REQUIRED_FAILURES=$((REQUIRED_FAILURES + 1))
  fi
else
  fail "cargo not on PATH" "Rust toolchain absent (run: curl https://sh.rustup.rs | sh -s -- -y --default-toolchain $PINNED)"
  REQUIRED_FAILURES=$((REQUIRED_FAILURES + 1))
fi

# ---------------------------------------------------------------------------
# 5. REQUIRED — browser present (platform CLI and/or Playwright chromium)
# ---------------------------------------------------------------------------
hdr "5/9 browser availability"
BROWSER_FOUND=0
if command -v agent-browser >/dev/null 2>&1; then
  AB_VER="$(agent-browser --version 2>/dev/null | head -1)"
  pass "platform browser automation CLI present: agent-browser ${AB_VER:-<version unqueried>}"
  BROWSER_FOUND=1
else
  note "agent-browser CLI not on PATH (not fatal if Playwright chromium present)"
fi
CHROME_BIN=""
for d in "$HOME"/.cache/ms-playwright/chromium-*/chrome-linux*/chrome; do
  [ -x "$d" ] && CHROME_BIN="$d"
done
if [ -n "$CHROME_BIN" ]; then
  pass "Playwright chromium present: $CHROME_BIN"
  BROWSER_FOUND=1
else
  note "no Playwright chromium under ~/.cache/ms-playwright"
fi
if [ "$BROWSER_FOUND" -eq 0 ]; then
  fail "no browser engine found (neither agent-browser nor Playwright chromium)" "browser engine"
  REQUIRED_FAILURES=$((REQUIRED_FAILURES + 1))
fi

# ---------------------------------------------------------------------------
# 6. REQUIRED — headless browser actually renders (live verification)
# ---------------------------------------------------------------------------
hdr "6/9 headless render (live)"
if [ -n "$CHROME_BIN" ]; then
  RENDER_OUT="$("$CHROME_BIN" --headless --no-sandbox --disable-gpu --dump-dom \
    'data:text/html,<title>capcheck</title><h1>CAPCHECK-RENDER-MARKER</h1>' 2>/dev/null | tail -1)"
  if echo "$RENDER_OUT" | grep -q "CAPCHECK-RENDER-MARKER"; then
    pass "chromium headless rendered CAPCHECK-RENDER-MARKER into DOM"
  else
    fail "chromium headless did not render the probe page" "headless render"
    REQUIRED_FAILURES=$((REQUIRED_FAILURES + 1))
  fi
elif command -v agent-browser >/dev/null 2>&1; then
  if timeout 90 agent-browser open 'data:text/html,<h1>CAPCHECK-RENDER-MARKER</h1>' >/dev/null 2>&1 \
     && timeout 60 agent-browser snapshot 2>/dev/null | grep -q "CAPCHECK-RENDER-MARKER"; then
    pass "agent-browser rendered CAPCHECK-RENDER-MARKER (snapshot matched)"
    timeout 30 agent-browser close >/dev/null 2>&1
  else
    fail "agent-browser could not render/snapshot the probe page" "headless render"
    REQUIRED_FAILURES=$((REQUIRED_FAILURES + 1))
  fi
else
  fail "no browser engine available to attempt a render" "headless render"
  REQUIRED_FAILURES=$((REQUIRED_FAILURES + 1))
fi

# ---------------------------------------------------------------------------
# 7. FIDELITY — E2B/Composio connected integration (expected ABSENT = fallback)
#    Names and set/unset state ONLY. Values are never printed.
# ---------------------------------------------------------------------------
hdr "7/9 E2B/Composio connected integration (fidelity check)"
E2B_SURFACE=0
for v in E2B_API_KEY E2B_ACCESS_TOKEN E2B_TEAM_ID; do
  if [ -n "$(printenv "$v")" ]; then
    pass "credential env var $v is SET (value withheld by policy)"
    E2B_SURFACE=1
  else
    note "credential env var $v not set"
  fi
done
for c in e2b composio; do
  if command -v "$c" >/dev/null 2>&1; then
    pass "CLI '$c' present"
    E2B_SURFACE=1
  else
    note "CLI '$c' absent"
  fi
done
for p in "$HOME/.e2b" "$HOME/.composio"; do
  [ -e "$p" ] && { pass "config dir $p exists"; E2B_SURFACE=1; }
done
if [ "$E2B_SURFACE" -eq 1 ]; then
  note "some E2B/Composio surface detected — full provisioning must be re-verified by hand"
  STRICT_FAILURES=$((STRICT_FAILURES + 1)) # partial surface still needs manual confirmation
else
  echo "[FAIL] E2B/Composio connected integration ABSENT — no credentials, no CLI, no config."
  echo "       This is the recorded VWO-001 fallback condition; see README.md §2 (Fidelity Limitations)."
  STRICT_FAILURES=$((STRICT_FAILURES + 1))
fi

# ---------------------------------------------------------------------------
# 8. FIDELITY — desktop capability (virtual display, capture, input injection)
# ---------------------------------------------------------------------------
hdr "8/9 desktop capability (fidelity check)"
DESKTOP_LEVEL="none"
if command -v Xvfb >/dev/null 2>&1; then
  pass "Xvfb present (virtual X framebuffer)"
  DESKTOP_LEVEL="virtual-display"
  FFMPEG_BIN="$(command -v ffmpeg || true)"
  XDOTOOL=""
  for cand in "$(command -v xdotool || true)" \
              "$HOME/.local/desktop-tools/usr/bin/xdotool"; do
    [ -n "$cand" ] && [ -x "$cand" ] && XDOTOOL="$cand"
  done
  # Live test: start a private Xvfb on a dedicated display, capture one
  # frame, then kill ONLY our own process (never pkill Xvfb globally — other
  # workers may own live displays in this shared single-sandbox fallback).
  XVFB_DISPLAY="${CAPCHECK_DISPLAY:-:137}"
  XVFB_LOG="$(mktemp)"
  CAPTURE_PNG="$(mktemp --suffix=.png)"
  Xvfb "$XVFB_DISPLAY" -screen 0 1280x800x24 >"$XVFB_LOG" 2>&1 &
  XVFB_PID=$!
  sleep 2
  if kill -0 "$XVFB_PID" 2>/dev/null; then
    if [ -n "$FFMPEG_BIN" ]; then
      if ffmpeg -y -loglevel error -f x11grab -video_size 1280x800 -i "$XVFB_DISPLAY" \
           -frames:v 1 "$CAPTURE_PNG" >/dev/null 2>&1 \
         && [ -s "$CAPTURE_PNG" ]; then
        pass "virtual display $XVFB_DISPLAY live; ffmpeg x11grab captured a frame ($(stat -c%s "$CAPTURE_PNG") bytes)"
        DESKTOP_LEVEL="virtual-display+capture"
      else
        fail "ffmpeg x11grab could not capture the virtual display" "desktop capture pipeline"
        STRICT_FAILURES=$((STRICT_FAILURES + 1))
      fi
    else
      fail "ffmpeg absent — virtual display cannot be captured" "desktop capture pipeline"
      STRICT_FAILURES=$((STRICT_FAILURES + 1))
    fi
    if [ -n "$XDOTOOL" ]; then
      # user-space xdotool needs its extracted shared libraries on the loader path
      XDO_LIBDIR="$HOME/.local/desktop-tools/usr/lib/x86_64-linux-gnu:$HOME/.local/desktop-tools/usr/lib"
      if [ "$XDOTOOL" = "$HOME/.local/desktop-tools/usr/bin/xdotool" ] \
         && [ -d "$HOME/.local/desktop-tools/usr/lib" ]; then
        export LD_LIBRARY_PATH="${LD_LIBRARY_PATH:+$LD_LIBRARY_PATH:}$XDO_LIBDIR"
      fi
      if DISPLAY="$XVFB_DISPLAY" "$XDOTOOL" key F13 >/dev/null 2>&1; then
        pass "input injection works (xdotool synthetic keystroke accepted by $XVFB_DISPLAY)"
        DESKTOP_LEVEL="virtual-display+capture+input"
      else
        fail "xdotool present but could not inject input into $XVFB_DISPLAY (missing LD_LIBRARY_PATH for user-space build?)" "desktop input injection"
        STRICT_FAILURES=$((STRICT_FAILURES + 1))
      fi
    else
      fail "xdotool absent (system and user-space) — no synthetic input injection" \
        "desktop input injection (see README.md §3.6 for user-space acquisition)"
      STRICT_FAILURES=$((STRICT_FAILURES + 1))
    fi
  else
    fail "Xvfb $XVFB_DISPLAY did not stay running (already in use? set CAPCHECK_DISPLAY)" "virtual display bring-up"
    STRICT_FAILURES=$((STRICT_FAILURES + 1))
  fi
  kill "$XVFB_PID" >/dev/null 2>&1
  wait "$XVFB_PID" >/dev/null 2>&1
  rm -f "$CAPTURE_PNG" "$XVFB_LOG"
else
  fail "Xvfb absent — no virtual desktop capability in this sandbox" "desktop capability"
  STRICT_FAILURES=$((STRICT_FAILURES + 1))
fi
for c in Xvnc x11vnc vncserver; do
  if command -v "$c" >/dev/null 2>&1; then
    pass "VNC server binary '$c' present — interactive desktop may be reachable"
  else
    note "VNC server binary '$c' absent (no interactive human-observable desktop)"
  fi
done
note "desktop capability level: $DESKTOP_LEVEL"

# ---------------------------------------------------------------------------
# 9. FIDELITY — human gate / operator channel observable from this sandbox
# ---------------------------------------------------------------------------
hdr "9/9 human gate (operator channel)"
# The connected platform routes operator communication through the IM chat
# session that spawned this agent; there is no in-sandbox widget to probe.
# We verify only that the session trace metadata is NOT leaked into the env.
if [ -z "$(printenv ZAI_SESSION_ID 2>/dev/null)" ] && [ -z "$(printenv ZAI_CHAT_ID 2>/dev/null)" ]; then
  pass "no session/chat identifiers exported into worker env (human gate stays in the operator channel)"
  note "human gate = operator chat session (IM gateway); documented in README.md §1"
else
  note "session identifiers present in env (names only; values never printed)"
fi

# ---------------------------------------------------------------------------
# Summary
# ---------------------------------------------------------------------------
hdr "FIDELITY PROFILE"
echo "required failures : $REQUIRED_FAILURES"
echo "fidelity failures : $STRICT_FAILURES (strict gates exit code only with --strict)"
if [ "$REQUIRED_FAILURES" -gt 0 ]; then
  echo
  echo "RESULT: FAIL — required proving-ground capabilities are missing; fix them before Wave 1+ dispatch."
  exit 1
fi
if [ "$STRICT" -eq 1 ] && [ "$STRICT_FAILURES" -gt 0 ]; then
  echo
  echo "RESULT: STRICT FAIL — fallback fidelity limitations present (E2B/desktop); recorded in README.md §2."
  exit 1
fi
echo
echo "RESULT: PASS — required capabilities verified."
case $DESKTOP_LEVEL in
  virtual-display+capture+input) echo "Fidelity: fallback with virtual-desktop substrate.";;
  *) echo "Fidelity: fallback (see README.md §2).";;
esac
exit 0
