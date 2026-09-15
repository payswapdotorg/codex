# Codex Universal — App Launch Program: Final Release Report

**Program:** docs/product-delivery/APP-LAUNCH-PROGRAM.md
**Work Orders:** APP-001 · APP-002 · APP-003 · APP-004 (all complete)
**Verdict:** `DOWNLOADABLE_AND_LAUNCHABLE`
**Date:** 2026-09-15

## 1. Exact release version

`0.1.0` — GitHub release tag **`rust-v0.1.0`**:
https://github.com/payswapdotorg/codex/releases/tag/rust-v0.1.0

## 2. Exact source SHA

Tag `rust-v0.1.0` → merge commit **23b7f6c27** (`Merge pull request #51` —
the APP-002 CI hotfix exporting `CODEX_REPO_ROOT`; its parent lineage contains
PR #50 `ff936ca5de`, the APP-002 build/packaging delivery, on top of
a277542d6 = the Tech Lead handoff commit). The tag-check CI job enforces
tag == `codex-rs/Cargo.toml` workspace version (0.1.0). The first tag point
(abdf332f7, on ff936ca5de) was superseded when its release run failed at the
package-archive step for the missing workflow env — evidence:
`evidence/app-002/ci-first-run-repo-root-failure.md`.

## 3. Release artifacts

| Artifact | Platform / Arch | Kind |
|---|---|---|
| `codex-package-x86_64-unknown-linux-musl.tar.gz` (+ `.tar.zst` twin) | Linux x86_64 (static musl) | primary package |
| `codex-package-aarch64-unknown-linux-musl.tar.gz` (+ `.tar.zst` twin) | Linux aarch64 (static musl) | primary package |
| `codex-app-server-package-x86_64-unknown-linux-musl.tar.gz` (+ `.tar.zst` twin) | Linux x86_64 | app-server package |
| `codex-app-server-package-aarch64-unknown-linux-musl.tar.gz` (+ `.tar.zst` twin) | Linux aarch64 | app-server package |
| `codex-package_SHA256SUMS` | all | checksum manifest |
| `install.sh` / `install.ps1` | — | installers |
| `config-schema.json` | — | config schema |
| `codex` (dotslash) | linux x86_64 / aarch64 | one-command install/update |

Package layout: `bin/codex`, `bin/codex-code-mode-host`, `codex-path/rg`,
`codex-resources/bwrap`. Binaries carry keyless sigstore bundles
(`*.sigstore`).

## 4. Platform / architecture matrix

| Platform | x86_64 | aarch64 |
|---|---|---|
| Linux (musl static) | ✅ built + fresh-machine verified | ✅ built (alternate artifact) |
| macOS | ❌ (no signing environment in this fork — see APP-002 §2) | ❌ |
| Windows | ❌ | ❌ |

The primary validation platform is Linux x86_64 (the product validation
environment); the aarch64 artifact is the reproducible alternate the
existing machinery makes practical.

## 5. Checksums

See `codex-package_SHA256SUMS` on the release page (authoritative), and
`docs/product-delivery/evidence/app-002/checksums.txt` for the recorded
copy. The APP-003 fresh-machine run verified the x86_64 archive checksum
against the published manifest before install (evidence:
`evidence/app-003/run-*/transcript.txt`, step 2).

## 6. Build commands (reproducible)

CI (authoritative, on the tag):

```bash
cargo build --target x86_64-unknown-linux-musl --release --timings \
  --bin codex --bin codex-code-mode-host --bin codex-responses-api-proxy
# (after a first --bin bwrap pass whose sha256 is embedded at build time)
```

Full matrix and pinned toolchain: `.github/workflows/rust-release-fork.yml`
(rust-toolchain 1.95.0; zig 0.14.0 musl cross path; rusty_v8 pinned
artifacts; ripgrep from the pinned public dotslash manifest).

## 7. Installation commands

Guided (documented user path):

```bash
curl -fsSL https://github.com/payswapdotorg/codex/releases/download/rust-v0.1.0/install.sh \
  | CODEX_INSTALLER_USE_RELEASES_OPENAI_COM=false \
    CODEX_INSTALL_GITHUB_REPO=payswapdotorg/codex \
    CODEX_NON_INTERACTIVE=true sh
```

Manual:

```bash
tar -xzf codex-package-x86_64-unknown-linux-musl.tar.gz
```

## 8. Launch commands

```bash
codex --version
codex                      # agent TUI (model login required)
codex workflow --help      # workflow/teaching surface (no login required)
```

## 9. Fresh-environment identity

APP-003 ran from an isolated HOME (`fresh-home`), a sanitized PATH with no
cargo/rust, no `~/.codex`, and no source checkout anywhere in the
workspace; the artifact was downloaded over HTTPS from the public GitHub
release URL. Full identity: `evidence/app-003/run-*/transcript.txt`
(machine arch/target, isolation assertions, timestamps).

## 10. Smoke-test evidence

- **APP-001 smoke** (`evidence/app-001/run-20260915T183801Z/`):
  **13 passed, 0 failed** against the release binary — identity, entry
  point, teach→publish, instance run to terminal `completed`, relaunch
  durability, app-server surface.
- **APP-003 fresh-machine run** (`evidence/app-003/run-20260915T185129Z/`):
  **22 passed, 0 failed** — download, checksum verify, install.sh install,
  launch, teach + publish, instance run to `completed`, close/relaunch
  with durable instances, actionable error on the agent surface with
  unusable model config while the workflow surface stayed functional, and
  the documented uninstall procedure (user data preserved).
  Environmental deviations (sandbox IP rate limit on the installer's single
  API call; provider region-block on the agent surface) are documented in
  the APP-003 report §4 and covered in USER-GUIDE §10.

## 11. Workflow performed (representative user path)

`weekly-report-pipeline` — taught from three natural-language instructions
in a fresh git workspace, published as an immutable version (sha256 version
id), executed through `codex workflow instance run` to terminal state
**Completed**, then re-read across a process restart via
`codex workflow instance list` / `instance get`.

## 12. Persistence / relaunch evidence

- Workflow control plane: `~/.codex/workflow/` — instances survive
  process exit and relaunch; orphaned `running` instances reconcile to
  `paused` with their persisted position (engine startup sweep).
- Installer layout: `~/.codex/packages/standalone/` with a `current`
  pointer; `~/.local/bin/codex` launches it.

## 13. Known limitations

- Linux-only artifacts (x86_64 + aarch64); macOS/Windows require signing
  environments this distribution channel does not have.
- No npm / winget / r2 / dev-website distribution layers.
- No bundled zsh resource in packages (rg and bwrap ARE bundled).
- The `workflow` command group is flagged `[experimental]` in CLI help.
- Agent surface requires provider login; no anonymous model access.

## 14. Acceptance gate (program §Acceptance gate)

- [x] a release artifact exists
- [x] it is downloadable from GitHub
- [x] a source checkout is not required
- [x] the artifact launches on the documented platform
- [x] the user can reach the actual Codex Universal workflow surface
- [x] one representative workflow is completed through the normal application path
- [x] the app can be closed and relaunched successfully
- [x] artifact identity and checksum are documented
- [x] fresh-environment evidence is committed
- [x] exact user documentation is committed (`docs/product-delivery/USER-GUIDE.md`)

**Explicit verdict: `DOWNLOADABLE_AND_LAUNCHABLE`**

## 15. State reconciliation

- `docs/product-delivery/APP-LAUNCH-PROGRAM.md` — program COMPLETE
  (all four APP Work Orders accepted).
- Wave-4 validation (VWO-012..015) already merged on main (PRs #46–#49,
  docs(state) 2c6669305). VWO-016 (held-out generalization) remains
  operator-gated on independently authored briefs per catalog §7 and is
  not part of the app-launch program.
