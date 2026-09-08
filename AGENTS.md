# Agent Operating Contract

## Project bootstrap

This fork has a project-level architecture and workflow contract. Before changing code, read `ARCHITECT_START_HERE.md`, then `docs/architecture/CODEX-UNIVERSAL-ARCHITECTURE.md`, `docs/architecture/CODEX-UNIVERSAL-LOCK.md`, and the current development-state/work-order files under `docs/development-state/` and `docs/work-orders/`.

The repository's existing Codex/Rust guidance below remains in force. The project-level documents above define the fork's additional architecture and authority rules.

## Rust/codex-rs

In the codex-rs folder where the rust code lives:

- Crate names are prefixed with `codex-`. For example, the `core` folder's crate is named `codex-core`
- When using format! and you can inline variables into {}, always do that.
- Install any commands the repo relies on (for example `just`, `rg`, or `cargo-insta`) if they aren't already available before running instructions here.
- Never add or modify any code related to `CODEX_SANDBOX_NETWORK_DISABLED_ENV_VAR` or `CODEX_SANDBOX_ENV_VAR`.
  - You operate in a sandbox where `CODEX_SANDBOX_NETWORK_DISABLED=1` will be set whenever you use the `shell` tool. Any existing code that uses `CODEX_SANDBOX_NETWORK_DISABLED_ENV_VAR` was authored with this fact in mind. It is often used to early exit out of tests that the author knew you would not be able to run given your sandbox limitations.
  - Similarly, when you spawn a process using Seatbelt (`/usr/bin/sandbox-exec`), `CODEX_SANDBOX=seatbelt` will be set on the child process. Integration tests that want to run Seatbelt themselves cannot be run under Seatbelt, so checks for `CODEX_SANDBOX=seatbelt` are also often used to early exit out of tests, as appropriate.
- Always collapse if statements per https://rust-lang.github.io/rust-clippy/master/index.html#collapsible_if
- Always inline format! args when possible per https://rust-lang.github.io/rust-clippy/master/index.html#uninlined_format_args
- Use method references over closures when possible per https://rust-lang.github.io/rust-clippy/master/index.html#redundant_closure_for_method_calls
- Avoid bool or ambiguous `Option` parameters that force callers to write hard-to-read code such as `foo(false)` or `bar(None)`. Prefer enums, named methods, newtypes, or other idiomatic Rust API shapes when they keep the callsite self-documenting.
- When you cannot make that API change and still need a small positional-literal callsite in Rust, follow the `argument_comment_lint` convention.
- When possible, make `match` statements exhaustive and avoid wildcard arms.
- Newly added traits should include doc comments that explain their role and how implementations are expected to use them.
- Discourage both `#[async_trait]` and `#[allow(async_fn_in_trait)]` in Rust traits. Prefer native RPITIT trait methods with explicit `Send` bounds on returned futures.
- Prefer comparing equality of entire objects over fields one by one in tests.
- Do not add tests for statically defined values.
- Do not add negative tests for logic that was removed.
- Do not add general product or user-facing documentation to the `docs/` folder except architecture/project-control documents explicitly authorized by the fork architecture.
- Prefer private modules and explicitly exported public crate API.
- If you change `ConfigToml` or nested config types, run `just write-config-schema` to update `codex-rs/core/config.schema.json`.
- When working with MCP tool calls, prefer existing connection-manager abstractions.
- If you change Rust dependencies (`Cargo.toml` or `Cargo.lock`), run `just bazel-lock-update` from the repo root and include the lockfile update.
- Bazel compile-time source access requires corresponding build metadata such as `compile_data`/`build_script_data`.
- Do not create one-use helper methods without a strong reason.
- Prefer `#[tracing::instrument(...)]` on function definitions for async tracing.
- Target Rust modules under 500 LoC excluding tests; strongly reconsider files approaching 800 LoC.
- Run `just fmt` in `codex-rs` after Rust changes.
- Do not run `cargo test` directly; use `just test`.
- Run project-specific tests after changes; full workspace tests are reserved for changes affecting common/core/protocol and should be explicitly coordinated.

## The codex-core crate

Resist adding new concepts to `codex-core`. Prefer an existing suitable crate or introduce a new focused crate when a new architectural concept would otherwise increase core coupling.

## Code review rules

Preserve incremental context construction, hard caps on context items, and explicit context-fragment types. Treat app-server APIs, raw response events, CLI parameters, config loading, and persisted rollouts as compatibility surfaces.

Features changing agent logic require integration coverage. User-visible TUI changes require snapshot coverage.

Large changes should be split into reviewable stages; do not routinely exceed roughly 800 changed lines.

## Fork-specific architecture rules

- Universal model/provider logic belongs behind provider-neutral contracts.
- Workflow semantics belong to the workflow plane, not the TUI, model adapter, browser driver, or tool implementation.
- Workflow execution reuses the Codex runtime; no second agent loop or workflow engine is permitted.
- External model/tool/browser/connector output is untrusted by default.
- Credentials are capability-scoped and never ordinary workflow content.
- Published workflow versions are immutable.
- Architecture changes require an Architecture Change Request; implementation agents may not silently redesign frozen architecture.

## Completion report

Every implementation report must include Work Order, base SHA, head SHA, changed surfaces, tests run/results, acceptance evidence, known limitations, exact artifacts, and any architecture divergence.