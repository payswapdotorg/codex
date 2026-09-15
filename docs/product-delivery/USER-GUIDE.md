# Codex Universal — User Guide (Release 0.1.0)

This guide is for **users**. No Rust, no repository checkout, no build steps.

Codex Universal 0.1.0 is a downloadable command-line application with two
surfaces:

- the **agent surface** — an interactive TUI (`codex`) and a non-interactive
  runner (`codex exec`) powered by the Codex runtime (requires model access);
- the **workflow/teaching surface** — `codex workflow`: teach a workflow from
  instructions and/or demonstrations, publish it as an immutable version,
  fork published workflows, propose governed improvements, and run durable
  instances that survive restarts (works **without** any model credentials).

## 1. What to download

**Prerequisites:** a Linux system (x86_64 or aarch64); a shell; `curl` or
`wget` plus `tar` and `sha256sum` (present on virtually every Linux
distribution) for download and verification; `git` (only for the workflow
teaching surface, which pins the current project's commit — any git-tracked
workspace works). No Rust toolchain, no repository checkout, no build steps.

Go to the release page:

```
https://github.com/payswapdotorg/codex/releases/tag/rust-v0.1.0
```

Pick the archive for your machine:

| Your machine | File |
|---|---|
| Linux, 64-bit Intel/AMD (most PCs/servers) | `codex-package-x86_64-unknown-linux-musl.tar.gz` |
| Linux, ARM64 (e.g. Graviton/Raspberry Pi 5 class) | `codex-package-aarch64-unknown-linux-musl.tar.gz` |

Also download `codex-package_SHA256SUMS` (checksum manifest) and, if you
prefer the guided installer, `install.sh`.

> The release also carries size-optimized `.tar.zst` twins of each archive;
> they unpack to the identical content but are **not** listed in the checksum
> manifest — prefer the `.tar.gz` archives if you want to verify the download.

## 2. How to verify it

```bash
sha256sum -c codex-package_SHA256SUMS   # after downloading into the same dir
```

The checksum manifest lists every published archive; the line for your
file must verify.

## 3. How to install

### Option A — guided installer (recommended)

```bash
curl -fsSL https://github.com/payswapdotorg/codex/releases/download/rust-v0.1.0/install.sh \
  | CODEX_INSTALLER_USE_RELEASES_OPENAI_COM=false \
    CODEX_INSTALL_GITHUB_REPO=payswapdotorg/codex \
    CODEX_NON_INTERACTIVE=true sh
```

This installs to `~/.local/bin/codex` (make sure `~/.local/bin` is on your
`PATH`; the installer prints instructions when it is not).

### Option B — manual unpack

```bash
tar -xzf codex-package-x86_64-unknown-linux-musl.tar.gz   # creates bin/, codex-path/, codex-resources/
./bin/codex --version
```

You can move the unpacked directory anywhere; keep `bin/`, `codex-path/`
and `codex-resources/` together.

## 4. How to launch

```bash
codex            # interactive TUI (agent surface)
codex --version  # application identity
codex workflow --help   # the workflow/teaching surface
```

(For Option B installs, `./bin/codex` instead of `codex`.)

## 5. How to configure model access

The agent surface talks to a model provider. Sign in with:

```bash
codex login            # browser-based sign-in (ChatGPT account), or
codex login --api-key  # paste an API key instead
```

Credentials are stored under `~/.codex/` and never in your project.

**You do not need model access to use the workflow surface** (teach,
publish, fork, improve, run instances) — teaching compiles your statements
deterministically.

## 6. How to start a workflow (first run)

Workflows are taught from your project directory (a git repository — the
published version pins the exact commit):

```bash
cd ~/my-project                       # any git-tracked workspace
codex workflow teach \
  --mode instruct \
  --name "my-first-workflow" \
  --instruct "Step one of what should happen" \
  --instruct "Step two of what should happen" \
  --instruct "Step three of what should happen" \
  --yes
```

Codex opens a teaching session, records your instructions, reconciles the
trajectory, compiles it, prints a review, and publishes an immutable
version. The output ends with:

```
Run it with: codex workflow instance run <version-id>
```

Run it:

```bash
codex workflow instance run <version-id>      # executes to completion
codex workflow instance list                 # every durable instance
codex workflow instance get <instance-id>    # one instance + evidence
codex workflow instance resume <instance-id> # resume a paused instance
codex workflow instance cancel <instance-id> "reason"
```

Other lifecycle commands:

```bash
codex workflow fork <workflow>@<version-id> --as <your-repo-url> --carry "Your Name"
codex workflow improve propose <version-id>     # from recorded evidence
codex workflow improve validate <candidate-id> --version <semver>
codex workflow improve approve <candidate-id>   # the gate publication requires
codex workflow improve publish <candidate-id> --tag <release-tag>
```

## 7. Restart / relaunch behavior

- The workflow control plane persists under `~/.codex/workflow/`. Close the
  app any time; `codex workflow instance list` reads the same durable state
  on the next launch.
- Instances interrupted mid-run are listed as `paused` (never silently
  `running`) and can be resumed.

## 8. Update policy

Releases are immutable GitHub release tags (`rust-vX.Y.Z`), each carrying a
`codex-package_SHA256SUMS` manifest. To update, re-run the installer with
the new version:

```bash
CODEX_RELEASE=0.2.0 CODEX_INSTALLER_USE_RELEASES_OPENAI_COM=false \
CODEX_INSTALL_GITHUB_REPO=payswapdotorg/codex CODEX_NON_INTERACTIVE=true sh install.sh
```

Published workflow versions are immutable and pinned to the source commit
they were taught from; updating the app never rewrites them.

## 9. Uninstall

```bash
rm -f ~/.local/bin/codex ~/.local/bin/codex-code-mode-host
rm -rf ~/.codex/packages/standalone     # installer-managed payloads
# optionally, to remove ALL local state (workflows, sessions, credentials):
rm -rf ~/.codex
```

Manual installs: delete the unpacked directory.

## 10. Troubleshooting first runs

| Symptom | Meaning / fix |
|---|---|
| `codex: command not found` | `~/.local/bin` is not on your `PATH` — add `export PATH="$HOME/.local/bin:$PATH"` to your shell profile. |
| Login/auth error from `codex` or `codex exec` | The agent surface needs model access — run `codex login` (or `codex login --api-key`). The workflow surface needs no login. |
| `403 Forbidden: Country, region, or territory not supported` | Your network egress region is blocked by the model provider. The agent surface cannot run from that network; the workflow/teaching surface (`codex workflow …`) works fully without model access. |
| `codex exec` seems to hang with `Reading additional input from stdin...` | `codex exec` reads piped stdin as extra prompt context by design. In scripts, close stdin: `codex exec "prompt" < /dev/null`, or pipe your context deliberately: `cat notes.md | codex exec "summarize"`. |
| `Could not fetch GitHub release metadata … rate limited` (install.sh) | GitHub's API rate limit for your IP is exhausted (shared/CI egress IPs). Wait for the window to reset, or install manually: download the package + `codex-package_SHA256SUMS` from the release page, verify, and unpack (§2–§3). |
| `refusing to publish without approval in non-interactive mode` | Pass `--yes` to approve the publish yourself, or run in a terminal and answer the prompt. |
| `resolve the current repository's HEAD commit` failed | Teach from inside a git repository, or pass `--repo <url> --commit-sha <40-hex-sha>` explicitly. |
| `zstd: command not needed` | This release's checksum-covered archives are `.tar.gz`; plain `tar -xzf` unpacks them. (`.tar.zst` twins exist but are not in the checksum manifest.) |
| Checksum mismatch | Re-download; verify against `codex-package_SHA256SUMS` from the release page. Never run a binary that fails verification. |
| `warning: Codex could not find bubblewrap on PATH` | Harmless — the app falls back to the bundled, build-time digest-pinned bubblewrap from `codex-resources/`. |

## 11. Supported / unsupported in this release

**Supported (this release):**

- Linux x86_64 and aarch64 (static musl binaries — no runtime library
  dependencies beyond the kernel);
- the full `codex workflow` lifecycle: teach (instruct / demonstrate /
  hybrid), compile, review, approve, publish, fork, governed improvement,
  durable instance run/list/get/resume/cancel;
- durable restart semantics for the workflow control plane;
- the agent TUI and `codex exec` with provider sign-in.

**Not in this release:**

- macOS and Windows binaries (no signing environment in this distribution
  channel yet);
- npm packages, winget, and mirror distribution;
- bundled zsh integration resource;
- marketplace/payment functionality (explicitly out of scope for the
  launch surface).
