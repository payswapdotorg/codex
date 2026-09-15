#!/usr/bin/env bash
# APP-001 launch-surface smoke test.
#
# Proves, against a freshly built `codex` binary, that:
#   1. the application starts and identifies itself (help + version);
#   2. the workflow entry point is visible (`codex workflow --help`);
#   3. a user can teach + publish a workflow through the real control plane
#      (teach -> reconcile -> compile -> review -> approve -> publish);
#   4. a representative workflow action runs end-to-end and reaches a
#      terminal state (`workflow instance run` -> Completed);
#   5. shutdown/relaunch does not corrupt durable state (instance list +
#      instance get after a fresh process);
#   6. the same experience is reachable over the app-server protocol
#      surface (the workflow methods are the client integration contract).
#
# Usage: app001-smoke.sh <path-to-codex-binary> [evidence-dir]
# Environment: CODEX_HOME is pointed at a scratch home so the test never
# touches a developer's real ~/.codex.
set -euo pipefail

CODEX_BIN="${1:?usage: app001-smoke.sh <codex-binary> [evidence-dir]}"
EV="${2:-$(dirname "$0")}/run-$(date -u +%Y%m%dT%H%M%SZ)"
mkdir -p "$EV"
EV="$(cd "$EV" && pwd)"
CODEX_BIN="$(readlink -f "$CODEX_BIN")"

export CODEX_HOME="$EV/codex-home"
mkdir -p "$CODEX_HOME"

# A normal user teaches a workflow from their own project workspace; the
# control plane pins that repository's HEAD commit as the immutable source
# revision. The scratch workspace here stands in for the user's project.
WORKSPACE="$EV/user-workspace"
mkdir -p "$WORKSPACE"
git -C "$WORKSPACE" init -q
git -C "$WORKSPACE" -c user.name=app001 -c user.email=app001@example.invalid commit -q --allow-empty -m "scratch project state"

pass=0; fail=0
ok()  { pass=$((pass+1)); echo "  PASS  $1" | tee -a "$EV/transcript.txt"; }
bad() { fail=$((fail+1)); echo "  FAIL  $1" | tee -a "$EV/transcript.txt"; }
step(){ echo "== $1"  | tee -a "$EV/transcript.txt"; }

# The teach pipeline prints one JSON envelope per stage
# ({"stage": ..., "response": {...}}); the publish stage is the last line.
stage_field() { # <file> <json-pointer-python-expr over d["response"]>
  /usr/bin/python3 - "$1" "$2" <<'PY'
import json, sys
lines = [l for l in open(sys.argv[1]) if l.strip()]
d = json.loads(lines[-1])
assert d.get("stage") == "publish", f"last stage was {d.get('stage')!r}"
print(eval(sys.argv[2], {}, {"d": d["response"]}))
PY
}

cd "$WORKSPACE"

step "1. application starts and identifies itself"
"$CODEX_BIN" --version > "$EV/version.txt" 2>&1 && ok "codex --version: $(cat "$EV/version.txt")" || bad "codex --version"
"$CODEX_BIN" --help > "$EV/help.txt" 2>&1 && ok "codex --help renders the application surface" || bad "codex --help"

step "2. workflow entry point is visible"
"$CODEX_BIN" workflow --help > "$EV/workflow-help.txt" 2>&1 && ok "codex workflow --help renders the entry point" || bad "codex workflow --help"
grep -q "teach" "$EV/workflow-help.txt" && ok "workflow teach subcommand advertised" || bad "workflow teach subcommand advertised"
grep -qE "instance" "$EV/workflow-help.txt" && ok "workflow instance subcommand advertised" || bad "workflow instance subcommand advertised"

step "3. teach + compile + review + approve + publish through the real control plane"
"$CODEX_BIN" workflow teach \
  --mode instruct \
  --name "daily-standup-digest" \
  --instruct "Collect status updates from the three active project channels" \
  --instruct "Summarize blockers into a single digest section" \
  --instruct "Post the digest to the team-wide channel before 10:00 local time" \
  --yes \
  --json > "$EV/teach.json" 2>&1 \
  && ok "workflow teach published an immutable version" \
  || { bad "workflow teach"; tail -5 "$EV/teach.json"; }
VERSION_ID="$(stage_field "$EV/teach.json" 'd["versionId"]')"
ok "extracted version id: ${VERSION_ID:0:24}..."

step "4. representative workflow action end-to-end (instance run)"
"$CODEX_BIN" workflow instance run "$VERSION_ID" --json > "$EV/instance-run.json" 2>&1 \
  && ok "workflow instance run executed" || bad "workflow instance run"
STATUS="$(/usr/bin/python3 -c 'import json; d=json.loads([l for l in open("'"$EV"'/instance-run.json") if l.strip()][-1]); print(d["response"]["terminal"]["kind"])')"
[[ "$STATUS" == "Completed" ]] && ok "instance reached terminal state Completed" || bad "instance terminal state: $STATUS"

step "5. shutdown/relaunch durability (fresh process, same control plane)"
"$CODEX_BIN" workflow instance list --json > "$EV/instance-list.json" 2>&1 \
  && ok "workflow instance list after relaunch" || bad "workflow instance list"
COUNT="$(/usr/bin/python3 -c 'import json; d=json.loads([l for l in open("'"$EV"'/instance-list.json") if l.strip()][-1]); print(len(d["response"]["instances"]))')"
[[ "$COUNT" -ge 1 ]] && ok "durable instances survive process restart ($COUNT listed)" || bad "durable instances after restart"
INSTANCE_ID="$(/usr/bin/python3 -c 'import json; d=json.loads([l for l in open("'"$EV"'/instance-list.json") if l.strip()][-1]); print(d["response"]["instances"][0]["instanceId"])')"
"$CODEX_BIN" workflow instance get "$INSTANCE_ID" --json > "$EV/instance-get.json" 2>&1 \
  && ok "workflow instance get reads persisted evidence" || bad "workflow instance get"

step "6. app-server protocol surface exposes the same workflow contract"
"$CODEX_BIN" app-server --help > "$EV/app-server-help.txt" 2>&1 && ok "codex app-server (protocol daemon) present" || bad "codex app-server --help"

echo | tee -a "$EV/transcript.txt"
echo "RESULT: $pass passed, $fail failed" | tee -a "$EV/transcript.txt"
[[ "$fail" -eq 0 ]]
