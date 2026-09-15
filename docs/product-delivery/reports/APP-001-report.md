# APP-001 — Launch Surface + User-Facing Workflow Entry Point (Completion Report)

**Status:** COMPLETE
**Work Order:** docs/product-delivery/work-orders/APP-001.md
**Base SHA:** a277542d6 (docs(product): add Tech Lead app launch handoff)
**Date:** 2026-09-15

## 1. Chosen client surface

The launch surface is the existing **`codex` CLI binary** from `codex-rs/cli`
(package `codex-cli`, binary `codex`), carrying two user-facing surfaces:

1. **The Codex Universal agent surface** — the interactive TUI (`codex`) and
   the non-interactive `codex exec`, both over the existing Codex runtime.
2. **The workflow/teaching surface** — the `codex workflow` command group
   (`codex-rs/cli/src/workflow_cmd.rs`): `teach`, `fork`, `improve
   {propose,validate,approve,reject,publish}`, and `instance
   {run,list,get,resume,cancel}`.

## 2. Why this is the smallest correct route (audit)

- The audit found the workflow plane **already mounted** on two existing
  surfaces with no new integration work required:
  - the CLI command group above, and
  - the app-server protocol (`codex-app-server-protocol` v2 `workflow/*`
    methods), served by `codex app-server` and implemented by the shared
  `WorkflowControlPlane` service (`codex-rs/app-server/src/workflow.rs`).
  Both surfaces route through the **same** control-plane service — the CLI
  is a thin arg-parser/printing shell over the protocol types
  (`rpc::WorkflowTeachStartParams` etc.). No second runtime, no second
  engine, no duplicated workflow semantics (verified: `workflow_cmd.rs`
  constructs only protocol types; zero engine logic in the client).
- The TUI exposes the agent experience but has no workflow subcommand
  surface; the workflow CLI is the real, complete, user-facing entry point
  for teach/compile/review/approve/publish/run/resume — the full
  workflow lifecycle, durable across invocations under
  `$CODEX_HOME/workflow/` (default `~/.codex/workflow/`).
- Teaching is **deterministic** (instruct/demonstrate statements are
  reconciled and compiled by the teaching compiler — no model call), so
  the workflow/teaching experience is reachable **without model
  credentials**. The agent surface (`codex` TUI / `codex exec`) requires
  model access and reports actionable errors when absent (validated in
  APP-003).
- A web/mobile/desktop-shell client would be a **new** presentation layer —
  explicitly rejected by the program (§2: "not required unless source
  evidence establishes that it is the smallest correct route").

## 3. Exact build and launch commands

From a repository checkout (development verification):

```bash
cd codex-rs
cargo build -p codex-cli --bin codex          # debug build (dev verification)
cargo build --release -p codex-cli --bin codex  # release build
./target/debug/codex --version                 # application identity
./target/debug/codex                           # TUI agent surface
./target/debug/codex workflow --help           # workflow/teaching entry point
```

Deterministic launch from the packaged artifact (APP-002):

```bash
bin/codex            # TUI agent surface
bin/codex workflow teach --mode instruct --name <name> --instruct "..." --yes
bin/codex workflow instance run <version-id>
```

## 4. Changed files

**None.** The audit (APP-001 §1–2) found the required user path fully
implemented at the base SHA; the only added artifacts are the smoke-test
harness and this report:

- `docs/product-delivery/evidence/app-001/app001-smoke.sh` (new)
- `docs/product-delivery/evidence/app-001/run-*/` (new; transcripts)
- `docs/product-delivery/reports/APP-001-report.md` (new)

The "client integration required to reach the real workflow path" was
already shipped by RWO-001 (CLI mounting) and the RWO-002 engine work; the
correct APP-001 action was to **verify the surface as-is**, not to add a
parallel path.

## 5. Smoke test

`app001-smoke.sh` runs against the **published release binary** (extracted
from `codex-package-x86_64-unknown-linux-musl.tar.gz`, tag `rust-v0.1.0`,
sha256-verified against the published manifest) with an isolated
`CODEX_HOME` and proves the six required behaviors:

1. application starts and identifies itself (`codex-cli 0.1.0`);
2. the workflow entry point is visible (`codex workflow --help` advertises
   teach + instance);
3. a real teach pipeline run: instruct ×3 → reconcile → compile → review →
   approve → **publish** (immutable version id emitted); the publish also
   pins a commit in the user's workspace git repo (observed in the run);
4. a representative workflow action end-to-end: `workflow instance run`
   reaches terminal state **completed** through the durable lifecycle
   (instantiate → walk → settle; serialized `"kind": "completed"`);
5. shutdown/relaunch durability: fresh process reads `instance list` /
   `instance get` from the durable control-plane root (1 instance
   survives);
6. the app-server protocol daemon surface is present (`codex app-server`).

Latest run: `evidence/app-001/run-20260915T183801Z/transcript.txt` —
**RESULT: 13 passed, 0 failed** against the rust-v0.1.0 release binary
(package sha256
`4a6e7e9e2ecfbe3d86925bb877ce7751c30cfbd3d901ebcf1544d2c7e69b54d1`).

Two harness corrections were made before the recorded run, both discovered
only when a real binary existed (the harness pre-dated any artifact):
`workflow instance run/list/get` print their response directly as
pretty multi-line JSON (not a `{"stage","response"}` envelope like
`teach`), and the terminal kind serializes lowercase `"completed"`. The
corrected parsing is part of the committed harness.

## 6. Known limitations

- The `workflow` command group is flagged `[experimental]` in `codex --help`
  (upstream wording); the surface itself is complete and tested.
- The TUI agent surface requires model credentials; the workflow/teaching
  surface does not. First-run credential behavior is validated in APP-003.
- `codex workflow teach` publishes pinned to the **current git repository's
  HEAD commit** (or explicit `--repo`/`--commit-sha`); the user path
  assumes a git-tracked project workspace (documented in APP-004).
