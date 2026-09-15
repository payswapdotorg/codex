# APP-002 evidence: first fork CI run — package-archive step failure & fix

- Run: https://github.com/payswapdotorg/codex/actions/runs/34993047796
  (tag `rust-v0.1.0`, workflow `rust-release-fork`, started 2026-09-15T16:08:09Z)
- Result: all four build jobs (x86_64/aarch64 × primary/app-server) failed at the
  `Build Codex package archive` step; every prior step succeeded on GitHub-hosted
  runners (toolchain 1.95.0, zig musl cross, rusty_v8 pinned artifacts, bwrap
  digest, `cargo build --release`, symbols+strip, cosign, staging).

## Failure signature

```
File "/home/runner/work/codex/codex/scripts/codex_package/targets.py", line 11, in <module>
    raise RuntimeError(
RuntimeError: CODEX_REPO_ROOT must point to the repository root; run `just assemble-codex-package` to set it automatically
##[error]Process completed with exit code 1.
```

## Root cause

Upstream `rust-release.yml` declares a workflow-level env:

```yaml
env:
  CODEX_REPO_ROOT: ${{ github.workspace }}
```

The fork workflow (`rust-release-fork.yml`) reused the step bodies verbatim but
omitted this workflow-level env, so `scripts/codex_package/targets.py` could not
resolve the repository root. The `justfile` equivalent is
`export CODEX_REPO_ROOT := justfile_directory()` — only the `just` recipe path
set it; the workflow path relied on the workflow-level env.

## Fix

Added the same workflow-level env block to `rust-release-fork.yml` (with a
comment pointing at this evidence). Tag `rust-v0.1.0` moved to the fix commit
and re-pushed; second run: see `ci-second-run-*.md`.

## Notes

- The full compile succeeded on hosted runners in both architectures
  (~35-40 min per job), validating the "CI is the only viable build path"
  decision recorded in the APP-001/APP-002 reports (sandbox disk/RAM cannot
  link the codex workspace).
- The cancelled sibling run 34993073286 (main-branch, build-only
  workflow_dispatch path) was cancelled to free hosted-runner quota.

## Attempt 2 (run 34997644588, after the fix)

- All build steps green on all four jobs including package archive — the
  CODEX_REPO_ROOT fix is confirmed end-to-end (aarch64 app-server job fully
  succeeded first, ~17:24 UTC).
- One job (x86_64 app-server) failed at `Cosign Linux artifacts` with a
  transient keyless-OIDC flake:
  `getting key from Fulcio: fetching ambient OIDC credentials: invalid character 'u' looking for beginning of value`
  (the OIDC token endpoint returned non-JSON — a GitHub infra blip; the same
  step succeeded in the other three jobs of the same run and in all four jobs
  of attempt 1).
- Remediation: full run re-run (attempt 2, `POST /actions/runs/{id}/rerun`).
