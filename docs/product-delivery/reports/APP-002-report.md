# APP-002 — Reproducible Packaging + GitHub Download Artifact (Completion Report)

**Status:** COMPLETE
**Work Order:** docs/product-delivery/work-orders/APP-002.md
**Depends on:** APP-001 (accepted)
**Release tag:** rust-v0.1.0
**Source SHA:** (see §3 — the merge commit carrying this report and the release workflow)
**Date:** 2026-09-15

## 1. What was packaged

The APP-001 launch surface (the `codex` CLI binary package) is published as
GitHub release artifacts built by `.github/workflows/rust-release-fork.yml`
on the `rust-v0.1.0` tag:

| Artifact | Platform | Arch | Launch path |
|---|---|---|---|
| `codex-package-x86_64-unknown-linux-musl.tar.gz` / `.tar.zst` | Linux (musl, static) | x86_64 | `bin/codex` |
| `codex-package-aarch64-unknown-linux-musl.tar.gz` / `.tar.zst` | Linux (musl, static) | aarch64 | `bin/codex` |
| `codex-app-server-package-x86_64-unknown-linux-musl.tar.gz` / `.tar.zst` | Linux (musl, static) | x86_64 | `bin/codex-app-server` |
| `codex-app-server-package-aarch64-unknown-linux-musl.tar.gz` / `.tar.zst` | Linux (musl, static) | aarch64 | `bin/codex-app-server` |
| `codex-package_SHA256SUMS` | all | — | checksum manifest |
| `install.sh` / `install.ps1` | — | — | installer scripts |
| `config-schema.json` | — | — | config schema |
| `codex` (dotslash executable) | linux-x86_64 / linux-aarch64 | — | one-command install/update |

Package layout (from `scripts/codex_package/layout.py`, unchanged):
`bin/codex`, `bin/codex-code-mode-host`, `codex-path/rg`,
`codex-resources/bwrap` — the workflow/teaching surface needs no model
credentials, so the package is fully usable post-unpack.

## 2. Why the upstream release workflow was adapted (required evidence)

APP-002 forbids duplicating release machinery **without evidence that the
existing machinery cannot satisfy the gate**. The evidence:

- `rust-release.yml` Linux build jobs run on OpenAI's self-hosted runners
  (`codex-linux-x64-xl` / `codex-linux-arm64`), which do not exist in this
  fork — the jobs would queue forever.
- The macOS chain (`sign-macos-binaries` → `package-macos` →
  `sign-macos-dmg` → `finalize-macos`) requires the protected `codesigning`
  GitHub environment backed by Azure Key Vault secrets
  (`AKV_CODESIGN_*`), which do not exist in this fork.
- `publish-npm`, `publish-r2-assets`, `winget`, and `deploy-dev-website`
  require org-owned publishing identities (npm trusted publishing, R2
  secrets, winget repo, website deploy key) that this fork does not have.

The adaptation therefore **reuses the upstream build and packaging steps
verbatim** (same toolchain pin, same zig/musl cross path, same
rusty_v8 pinned-artifact verification, same bwrap digest embedding, same
symbol/strip script, same keyless cosign sigstore signing, same
`build-codex-package-archive.sh` packaging, same checksum-manifest and
release-publish steps, same dotslash publisher) on the platform matrix the
fork can build reproducibly: GitHub-hosted `ubuntu-latest` (x86_64) and
`ubuntu-24.04-arm` (aarch64) runners.

## 3. Identity, build and launch commands

- **Version:** `0.1.0` (`codex-rs/Cargo.toml` `[workspace.package]`; the
  tag-check job enforces tag == Cargo.toml version).
- **Exact build command (CI, per target):**
  `cargo build --target <target> --release --timings --bin codex --bin codex-code-mode-host --bin codex-responses-api-proxy` (+ prior `--bin bwrap` for the embedded digest).
- **Exact launch command (user):** unpack the archive, then
  `./bin/codex --version` · `./bin/codex` · `./bin/codex workflow --help`.
- **One-command install (Linux):**
  `curl -fsSL <release>/install.sh | CODEX_INSTALLER_USE_RELEASES_OPENAI_COM=false CODEX_INSTALL_GITHUB_REPO=payswapdotorg/codex sh`
  (the installer now honors `CODEX_INSTALL_GITHUB_REPO`, default
  `openai/codex` — additive, upstream-compatible).
- **Checksum verification:**
  `sha256sum -c codex-package_SHA256SUMS` (or compare against the
  checksums printed below).

## 4. Checksums (from the published release)

Recorded from the release assets at publication time (release published
2026-09-15T18:34:56Z by run 34997644588, conclusion **success**; 34
assets). The published manifest:

```
cac80d76f1d7e16f271739a3770c2c7ed820d4057bd56795495d0dcde4a8be0d  codex-app-server-package-aarch64-unknown-linux-musl.tar.gz
d168477423e8d1dc293f5ac132a3c7068c45c08df4712595c24e691e1dfd7695  codex-package-aarch64-unknown-linux-musl.tar.gz
dc1bd2651e4bbc72a6de0f184bc17dd82792db5d53ff208f14f9a0962407eaa8  codex-app-server-package-x86_64-unknown-linux-musl.tar.gz
4a6e7e9e2ecfbe3d86925bb877ce7751c30cfbd3d901ebcf1544d2c7e69b54d1  codex-package-x86_64-unknown-linux-musl.tar.gz
```

Captured evidence: `evidence/app-002/release-assets.json` (full API
record), `asset-list.tsv` (34 assets with sizes + URLs), `checksums.txt`
(the manifest as published), `verified-x86_64-sha256.txt` +
`capture-receipt.md` (the downloaded x86_64 package verified byte-identical
to the manifest: 128,736,412 bytes,
sha256 `4a6e7e9e…69b54d1`).

## 5. Changed files

- `.github/workflows/rust-release-fork.yml` (new — the fork release path;
  amended by PR #51 to export the upstream workflow-level
  `CODEX_REPO_ROOT` env — see §8)
- `.github/dotslash-config-fork.json` (new — dotslash outputs for the two
  Linux platforms the fork publishes)
- `codex-rs/Cargo.toml` (workspace version `0.0.0` → `0.1.0`)
- `codex-rs/Cargo.lock` (version sync)
- `scripts/install/install.sh` (`CODEX_INSTALL_GITHUB_REPO` override)
- `.github/workflows/rust-release.yml` + `.github/workflows/postmerge-ci.yml`
  (disabled — self-hosted-runner startup_failure noise; reversible)
- `docs/product-delivery/reports/APP-002-report.md` (this report)
- `docs/product-delivery/evidence/app-002/` (release evidence)

## 6. User gate check (pre-APP-003)

- [x] release artifact exists on GitHub (tag `rust-v0.1.0`)
- [x] downloadable from GitHub
- [x] no source checkout required (tarball + install.sh both work standalone)
- [x] starts from a clean user workspace (validated in APP-003 from a
      fresh environment — see the APP-003 report)

## 7. Known limitations

- Linux only (x86_64 + aarch64, musl static). macOS/Windows artifacts
  require the org signing environments (§2).
- No npm packages, winget manifest, r2 mirrors, or dev-website deploy.
- Package archives ship without the bundled zsh resource (the pinned
  `codex-zsh` release asset is not published in this fork; `rg` and `bwrap`
  resources ARE bundled — `rg` from the pinned public dotslash manifest,
  `bwrap` built and digest-pinned by the workflow).
- Binaries are sigstore-signed (keyless OIDC) but not Apple-notarized
  (N/A on Linux) — verify via `codex-package_SHA256SUMS`.

## 8. First release run: failure and hotfix (audit trail)

The first tag run (34993047796, tag point abdf332f7 on ff936ca5de)
compiled all four build jobs fully green on GitHub-hosted runners —
toolchain, zig musl cross, rusty_v8 pinned artifacts, bwrap digest, cargo
build, symbols, cosign, staging — and then failed at the `Build Codex
package archive` step: the fork workflow had reused the upstream step
bodies but dropped the upstream workflow-level
`env: CODEX_REPO_ROOT: ${{ github.workspace }}`, which
`scripts/codex_package/targets.py` requires. Fix delivered via PR #51
(merge 23b7f6c27), tag `rust-v0.1.0` re-pointed there, release re-run
(34997644588). Full evidence: `evidence/app-002/ci-first-run-repo-root-failure.md`.
Positive signal: the failure itself proves the entire compile/packaging
chain runs on GitHub-hosted runners — only the missing env var stood
between the tag and the published release.

**Outcome:** the re-run (34997644588) completed **success** at
18:34:56 UTC — all four build jobs green through package-archive,
checksum manifest, keyless cosign, release publish, and dotslash; the
release carries 34 assets (see §4). The published artifacts were then
validated end-to-end by APP-001 (13/13 against the release binary) and
APP-003 (22/22 fresh-machine user path).
