# MWO-001 — Durable Control-Plane Stores

**Status:** READY
**Depends on:** none (parallel with MWO-002)
**Program:** M4 durable-control-plane remediation (architect finding; see `TECH_LEAD_START_HERE.md` §16 and `docs/validation/prompts/01-final-completion-tech-lead-orchestration.md`)
**Architecture:** 0.2.0

## Objective

Provide **durable implementations** of the existing control-plane store ports so workflow state, evidence, triggers, and installations survive process restart. Today every port has an in-memory double only (`workflow-app/src/memory.rs`, `workflow-triggers/src/memory.rs`); the M4 audit found zero persistence code in the workspace. This Work Order implements the ports, **not** new semantics.

## Authorized change surface

- ONE new leaf crate `codex-rs/workflow-durable` (package `codex-workflow-durable`, lib `codex_workflow_durable`) plus exactly one members line appended to `codex-rs/Cargo.toml` members (after `"resource-providers"`).
- **Zero edits to existing crates.** The ports stay exactly where they are.

## Ports to implement (verbatim interfaces)

From `codex-workflow-app` (`src/port.rs`):
- `WorkflowVersionStore` (publish/load; re-verify integrity on load)
- `WorkflowInstanceStore` (create/save/load)
- `EvidenceStore` (store/promote; locator + digest minting parity with the in-memory double)

From `codex-workflow-triggers` (`src/port.rs`):
- `TriggerLedger` (accept/settle dedupe)
- `InstallationStore` (save/load configurations)
- `InstanceControl` (resume: Paused → Running only)

## Durability requirements

- **std-only:** no new external dependencies beyond existing workspace deps (serde, serde_json, thiserror are fine; rusqlite/sqlx are forbidden). File-backed durability: append-only JSONL journals + atomic snapshot files; writes go temp-file → fsync → rename; critical records fsync before returning.
- A builder/facade (`DurableStores` over a root directory) plus individual store structs so hosts adopt piecemeal.
- Deterministic, content-stable formats: reloading must produce records that serialize identically (round-trip parity with the in-memory doubles' observable behavior).
- Load-time integrity: sealed versions are re-verified (`verify_integrity`) on load; a corrupt journal tail is a deterministic error or a documented, tested truncation policy — never a silent wrong state.
- The ports take `&mut self`, so single-writer serialization is the host's contract; document it. No locking beyond what correctness requires.
- Credentials/secrets NEVER appear in stored state (scan your fixtures; runtime-assemble any fake).

## Crash/restart tests (the core deliverable)

- **Drop-and-reload:** construct stores at a temp root, drive each port exactly like the existing in-memory doubles' tests do, DROP every struct, re-open from the same root, assert state identical (including trigger dedupe still refusing double-fires, evidence locators/digests stable, installations loadable, instance statuses preserved).
- **Partial-write tolerance:** truncate the last journal line; assert the documented policy (deterministic error or skip-with-test) — never silent corruption.
- **E2E:** run a `workflow-app` lifecycle scenario + a `workflow-triggers` plane scenario with the durable stores wired in place of the in-memory doubles (dev-deps), then re-open and assert the persisted control-plane state matches the in-memory run's final state.

## Verification

`cargo test -p codex-workflow-durable`; `cargo clippy -p codex-workflow-durable --all-targets -- -D warnings`; `cargo fmt -p codex-workflow-durable -- --check`; regressions `cargo test -p codex-workflow-app -p codex-workflow-triggers`. Report at `docs/development-state/reports/MWO-001-report.md` with the standard completion format.

## Forbidden

- Editing any existing crate (the diff must be: new crate + one members line).
- New external dependencies.
- Contract reshaping, new semantics, host integration beyond the stores, a second engine, background threads, network.

## Completion report

Standard format (base/head SHAs, files, tests, acceptance mapping, limitations, deferred, risks) plus explicit evidence that state survives drop-and-reload for each of the six ports.
