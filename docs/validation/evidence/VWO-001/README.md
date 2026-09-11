# VWO-001 Evidence Artifacts

Raw, re-runnable evidence captured during VWO-001 (2026-09-11 UTC).
Repository: `payswapdotorg/codex`, base SHA
`926c177a5515a1bf1316d7ef1bacb9b6d2d53611`, branch
`vwo-001/e2b-proving-ground`.

Policy: capability names, set/unset states, and endpoint reachability only.
No credentials, tokens, or identifiers with secret value appear in any file
under this directory.

| Artifact | What it proves |
|---|---|
| `environment-probes.txt` | full probe matrix: E2B/Composio absence (creds/CLI/config), network reachability, desktop/browser/terminal inventory, runtimes, platform API/tool surface |
| `cargo-check-family.txt` | launch verification: all 7 M4 workflow-family crates `cargo check` clean under pinned rustc 1.95.0 (cold run 4m15s, exit 0; incremental re-confirmation 1.83s, exit 0) — renamed from `.log` because the repository `.gitignore` excludes `*.log` |
| `desktop-pipeline-verification.txt` | single-invocation transcript: Xvfb bring-up → chromium GUI render → ffmpeg x11grab capture → PNG pixel census (GUI content rendered) → xev/xdotool input-injection proof (22 key events) |
| `capability-check-output.txt` | committed `capability_check.sh` default run: exit 0, required checks PASS, E2B fallback FAIL recorded |
| `capability-check-strict-output.txt` | `--strict` run: exit 1 — loud failure on the E2B fallback condition so full fidelity is never silently claimed |

Re-run any time:

```bash
bash docs/validation/proving-ground/capability_check.sh          # → capability-check-output.txt equivalent
bash docs/validation/proving-ground/capability_check.sh --strict # → strict output equivalent
```
