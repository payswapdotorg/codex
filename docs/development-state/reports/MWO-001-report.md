# MWO-001 — Durable Control-Plane Stores: Worker Completion Report

**Work Order:** MWO-001 (M4 durable-control-plane remediation program)
**Program authority:** `TECH_LEAD_START_HERE.md` §16; `docs/validation/prompts/01-final-completion-tech-lead-orchestration.md`
**Architecture:** 0.2.0 (frozen; no divergence)
**Status:** Implemented, verified, committed. NOT PUSHED — the Tech Lead applies, independently verifies, pushes, and merges.

=== MWO-001 COMPLETION REPORT ===

## 1. Base SHA & Head SHA

- **Base:** `7b7dfa0cb2499a68d145fe15193ec0e786dd4284` — verified reachable via `git rev-parse 7b7dfa0cb^{commit}`; it is the clone's `origin/main` HEAD and the direct parent of the implementation commit. No fallback was needed.
- **Implementation commit (head of the code):** `6d1d07a77b21b2f505485d68d4a4136c7b25199f` on branch `mwo-001/workflow-durable`, message exactly `feat(workflow-durable): MWO-001 durable control-plane stores`, 27 files, +4,558 lines, 0 deletions.
- **Report commit:** the commit adding this file (branch tip after it).
- **SOURCE BUNDLE disposition:** the anonymous clone succeeded and the live tree was reachable, so **CLONE WINS**. Every bundled signature was reconciled against the live tree; the bundle's `port.rs`/`memory.rs`/contract files matched the clone verbatim at the base commit, and the bundle's `workflow-triggers/src/memory.rs` (signatures only) was completed from the live file. No bundle-vs-tree conflicts arose.

## 2. Changed Files (27 files, +4,558 lines, 0 deletions)

| File | Lines | Purpose |
| --- | --- | --- |
| `codex-rs/Cargo.toml` | +1 | single members line `"workflow-durable",` appended after `"resource-providers"` |
| `codex-rs/Cargo.lock` | +16 | generated entry for the new crate (no new external dependencies) |
| `codex-rs/workflow-durable/Cargo.toml` | +27 | crate manifest: edition/license/version.workspace, `[lints] workspace = true`, `[lib] doctest = false`, no BUILD.bazel |
| `src/lib.rs` | +138 | crate docs (role, stack diagram, is/is-not, module map, single-writer + torn-tail + integrity policies), `#![deny(missing_docs)]`, module wiring, re-exports |
| `src/error.rs` | +119 | `DurableStoreError` + documented projections into the frozen port error enums |
| `src/atomic.rs` / `atomic_tests.rs` | +106 / +48 | atomic snapshot writes (temp → fsync → rename, parent-dir fsync on Unix) |
| `src/journal.rs` / `journal_tests.rs` | +162 / +188 | append-only JSONL journals (write_all line+\n, fsync per record) + torn-tail load/repair policy |
| `src/version_store.rs` / `version_store_tests.rs` | +130 / +114 | `WorkflowVersionStore` over an atomic snapshot; `verify_integrity` on every load |
| `src/instance_store.rs` / `instance_store_tests.rs` | +135 / +127 | `WorkflowInstanceStore` over an atomic snapshot; interior-mutable `put` core for the control composition |
| `src/evidence_store.rs` / `evidence_store_tests.rs` | +208 / +172 | `EvidenceStore` over a JSONL journal; locator/digest minting parity with the double |
| `src/trigger_ledger.rs` / `trigger_ledger_tests.rs` | +356 / +439 | `TriggerLedger` over a JSONL journal (accept/settle replay, `evt-<n>` allocation, await derivation) |
| `src/installation_store.rs` / `installation_store_tests.rs` | +151 / +139 | `InstallationStore` over a snapshot (configurations) + journal (rebind audits) |
| `src/instance_control.rs` / `instance_control_tests.rs` | +134 / +196 | `InstanceControl` over the shared durable instance store + resume-directive journal |
| `src/facade.rs` / `facade_tests.rs` | +99 / +291 | `DurableStores` builder/facade over one root directory (canonical file layout, associated consts) |
| `src/testutil.rs` | +159 | cfg(test) shared fixtures (scratch roots, sealed versions, records) |
| `tests/common/mod.rs` | +350 | E2E scaffolding: in-memory baseline harness + durable harness over the same planes |
| `tests/e2e_durable_lifecycle.rs` | +212 | workflow-app lifecycle scenario: durable stores boxed in place of the doubles, drop-and-reload, baseline parity |
| `tests/e2e_durable_triggers.rs` | +341 | workflow-triggers plane scenario: install → fire → pause → duplicate no-op → resume → retry, drop-and-reload, dedupe across restart, baseline parity |

No existing crate was edited — the diff against base is purely additive (verified via `git show --stat`): one members line, the generated lock entry, and the new crate.

## 3. Implementation Summary

A new leaf crate `codex-rs/workflow-durable` (package `codex-workflow-durable`, lib `codex_workflow_durable`) that provides **std-only, file-backed durability** for the six existing control-plane store ports — the ports are implemented, not reshaped:

- **Storage layout under one root** (`DurableStores::open(root)`): atomic snapshots for current-state records (`versions.json`, `instances.json`, `installations.json`) and append-only JSONL journals for event-shaped state (`evidence.jsonl`, `triggers.jsonl`, `install-audits.jsonl`, `resume-directives.jsonl`). Every store is also constructible individually for piecemeal host adoption.
- **Atomic snapshot writes:** serialize the whole ordered map deterministically (BTreeMap, compact JSON), write to `<file>.tmp`, fsync the temp file, rename over the target (the rename is the commit point), fsync the parent directory on Unix. Crash leaves either the previous or the new snapshot — never partial.
- **Journals:** each append writes the full JSON line plus terminating newline with one `write_all` and fsyncs before the port method returns, so a record is either fully committed or absent.
- **Torn-tail policy (documented + tested):** at open, bytes after the last newline are a torn append and are dropped; a final line that fails to parse is the least-committed write and is dropped; the healthy prefix is atomically rewritten so later appends can never land behind uncommitted bytes. A parse failure in any *committed* (earlier) line, invalid UTF-8 in the committed region, a replayed duplicate acceptance, or a settlement of an unknown event id is structural corruption: deterministic `DurableStoreError::Corrupt` at construction — never a silent wrong state.
- **Load-time integrity:** every `WorkflowVersionStore::load` re-runs `WorkflowVersion::verify_integrity`; a tampered record surfaces as `WorkflowAppError::VersionIntegrity` and never executes.
- **Minting parity with the in-memory doubles:** evidence locators are `workflow-app/<kind-slug>/<sequence>` with `ContentDigest::of` digests (the double's exact minting contract, slug table replicated verbatim); ledger event ids are `evt-<n>` with the counter recovered from the highest committed acceptance; `promote` validates a digest and persists nothing, exactly like the double.
- **Single-writer contract (documented in the crate docs):** the ports take `&mut self`, so the host serializes writers exactly as with the doubles. The stores are shared-state *handles* (clones observe the same backing files) with an internal mutex only because sharing is what lets a host audit a store while the lifecycle/trigger plane owns the boxed port — the same shape as the in-memory doubles. No cross-process locking, no background threads, one writer per root is the host's contract.
- **Error projection (documented in `error.rs`):** the frozen error enums predate durability and have no i/o variant and may not be edited, so store failures project explicitly: i/o and serialization failures → `WorkflowAppError::Serialization` (context-decorated, via serde's public `serde_json::Error::custom`), surfacing on the trigger side through the existing `From<WorkflowAppError>` projection; identity failures reuse the existing variants verbatim (`InstanceAlreadyExists`, `InstanceUnavailable`, `IllegalResume`, `UnknownTriggerEvent`); integrity failures → `VersionIntegrity { version, reason }`.
- **All-or-nothing writes:** every port write mutates in-memory state, flushes, and rolls the in-memory state back on flush failure, so observable state always matches the durable bytes; `InstanceControl::resume` persists the `Paused → Running` transition before journaling the directive and rolls back if journaling fails.

## 4. Verification Results (real toolchain)

No Rust toolchain was present on the machine; a minimal stable toolchain was installed via rustup (precedent: the WO-011 worker did the same) — **cargo/rustc 1.98.1, rustfmt 1.9.0, clippy 0.1.98**. All commands ran from `codex-rs/` with `CARGO_PROFILE_DEV_DEBUG=0` (disk hygiene; affects debuginfo only):

- `cargo test -p codex-workflow-durable` → **38/38 green** (36 unit tests + 1 lifecycle E2E + 1 triggers E2E; 0 failed, 0 ignored).
- `cargo clippy -p codex-workflow-durable --all-targets -- -D warnings` → **clean** (`Finished`, zero warnings/errors).
- `cargo fmt -p codex-workflow-durable -- --check` → **exit 0** (only the pre-existing repo-wide notice that `imports_granularity` in rustfmt.toml is nightly-only on stable — not a failure).
- Regressions: `cargo test -p codex-workflow-app -p codex-workflow-triggers` → **37/37 green** (workflow-app: 3 lib + 11 E2E; workflow-triggers: 9 lib + 14 E2E; 0 failed).
- `codex-rs/target/` removed after results were recorded; the work tree is exactly the two commits described above.

## 5. Acceptance Criteria — Evidence per Packet Bullet

| Requirement | Evidence |
| --- | --- |
| `WorkflowVersionStore` (publish/load; re-verify integrity on load) | `version_store.rs` (snapshot + `verify_integrity` on every load); `version_store_tests::{publish_and_load_match_the_in_memory_double, state_survives_drop_and_reload, tampered_snapshots_fail_integrity_on_load, unparseable_snapshots_fail_deterministically_at_open}` |
| `WorkflowInstanceStore` (create/save/load) | `instance_store.rs`; `instance_store_tests::{create_save_load_match_the_in_memory_double (incl. duplicate rejection parity), statuses_and_evidence_survive_drop_and_reload, missing_instances_load_as_none}` |
| `EvidenceStore` (store/promote; locator + digest minting parity) | `evidence_store.rs` (slug table replicated verbatim); `evidence_store_tests::{locator_and_digest_minting_match_the_in_memory_double, promote_matches_the_in_memory_double_without_persisting, locators_and_digests_survive_drop_and_reload, journal_bytes_are_content_stable_across_reload}` |
| `TriggerLedger` (accept/settle dedupe) | `trigger_ledger.rs`; `trigger_ledger_tests::{acceptance_dedupe_and_settlement_match_the_in_memory_ledger, await_registration_and_resolution_match_the_in_memory_ledger, dedupe_and_settlements_survive_drop_and_reload, record_snapshots_expose_the_full_audit_shape}` |
| `InstallationStore` (save/load configurations) | `installation_store.rs`; `installation_store_tests::{save_load_list_and_audits_match_the_in_memory_store, configurations_and_audits_survive_drop_and_reload, uninstalled_workflows_load_as_none}` |
| `InstanceControl` (resume: Paused → Running only) | `instance_control.rs`; `instance_control_tests::{resume_matches_the_in_memory_control_plane, only_paused_instances_resume, resumed_status_and_directives_survive_drop_and_reload}` |
| std-only; no new external dependencies | `Cargo.toml`: normal deps = codex-workflow-app, codex-workflow-contracts, codex-workflow-triggers, serde, serde_json, thiserror; dev-deps = pretty_assertions, tokio, plus codex-execution-contracts + codex-workflow-forge (existing workspace crates, needed to construct `LifecycleDeps`/`TriggerDeps` in the E2E — same pattern as the env-adapters template). No rusqlite/sqlx; zero new registry dependencies (Cargo.lock gains only the crate's own package block). |
| Journals + atomic snapshots; temp → fsync → rename; critical records fsynced before returning | `atomic.rs`, `journal.rs`; `atomic_tests::snapshot_writes_replace_content_and_leave_no_temporary`, `journal_tests::journal_round_trips_every_appended_record` |
| Builder/facade + individual stores | `facade.rs` (`DurableStores` + canonical file-name consts) and each store's independent `open` constructor |
| Deterministic, content-stable formats; round-trip parity | BTreeMap snapshots + compact JSON; `version_store_tests` (byte-identical re-publish), `evidence_store_tests::journal_bytes_are_content_stable_across_reload` |
| Load-time integrity; corrupt journal tail deterministic or documented skip | `journal.rs` torn-tail policy (module docs); `journal_tests::{torn_tail_bytes_are_dropped_and_repaired, whole_file_without_newline_is_a_torn_tail, final_line_that_fails_to_parse_is_dropped, committed_line_corruption_is_a_deterministic_error, invalid_utf8_in_the_committed_region_is_a_deterministic_error}`; ledger-level: `trigger_ledger_tests::{truncated_tail_settlement_is_skipped_deterministically, committed_ledger_corruption_is_a_deterministic_error, duplicate_acceptance_in_the_journal_is_a_deterministic_error}` |
| Single-writer `&mut self` contract documented; no locking beyond correctness | `lib.rs` "Single-writer contract" durability policy section; shared-state handles mirror the doubles |
| Credentials never in stored state | `facade_tests::stored_files_carry_no_credential_shapes` (runtime-assembled markers scanned over every stored file and reloaded records); all fixtures are attribution labels/opaque ids by construction |
| **Drop-and-reload** (the core deliverable) | Per-port reload tests above; facade-level `facade_tests::the_full_control_plane_survives_drop_and_reload` (all six stores at one root); E2E: `e2e_durable_lifecycle::durable_lifecycle_matches_the_in_memory_baseline_and_survives_restart`, `e2e_durable_triggers::durable_trigger_plane_matches_the_baseline_and_dedupe_survives_restart` — **including trigger dedupe still refusing double-fires after restart, evidence locators/digests stable, instance statuses preserved, installations loadable** |
| Partial-write tolerance | `journal_tests::torn_tail_bytes_are_dropped_and_repaired` (chopped final line: dropped, file repaired, subsequent appends clean); `trigger_ledger_tests::truncated_tail_settlement_is_skipped_deterministically` (torn settlement skipped, never half-applied; dedupe still holds) |
| E2E: workflow-app lifecycle + workflow-triggers plane scenarios with durable stores in place of the doubles, then re-open and match | `tests/e2e_durable_lifecycle.rs` (select → instantiate → run → verify + evidence path, in-memory baseline parity, drop-and-reload); `tests/e2e_durable_triggers.rs` (install → fire → pause → duplicate no-op → resume → retry, baseline parity of outcome shapes/settlements/statuses, drop-and-reload, post-restart dedupe + fresh event id allocation) |
| Forbidden: editing existing crates / new external deps / contract reshaping / second engine / threads / network | Diff is new crate + one members line (+ generated lock entry); ports implemented verbatim; no threads, no network, no engine logic — stores only |

## 6. Compatibility Impact

Purely additive leaf crate: one workspace members line + generated Cargo.lock entry; zero edits to any existing crate's source, tests, or semantics. All 37 regression tests in the port-owning crates pass unchanged. Ordinary Codex behavior is untouched — nothing executes unless a host opens a store and hands it to a lifecycle or trigger plane.

## 7. Known Limitations

1. **Error-projection fidelity:** the frozen enums have no i/o variant, so durable i/o failures surface as `WorkflowAppError::Serialization` (trigger side via the existing `App` projection) with full context in the message rather than a dedicated variant. Documented in `error.rs`; a future Work Order that may edit those crates could add an i/o variant.
2. **Snapshot write amplification:** versions/instances/installations rewrite their whole (ordered, compact) snapshot per write — O(state) per operation. Correct and simple for control-plane volumes; a compaction/journaling hybrid is a future optimization, not a semantic change.
3. **`InstanceControl::resume` rollback-of-rollback:** if journaling a directive fails and the subsequent status rollback flush also fails, in-memory state may transiently diverge from disk until the next reload (the snapshot always holds the last flushed state). The path is documented in the code; it requires two consecutive disk failures.
4. **No cross-process coordination:** one writer per root directory is the documented host contract; concurrent processes on one root are outside the contract (as with any single-writer file store).
5. **Directory fsync** is performed on Unix only (platform allowance); on other platforms the rename remains atomic but its durability across power loss depends on the OS.
6. **The approval plane stays non-durable** in the E2E composition: `InMemoryApprovalSource` is hard-wired to the in-memory evidence store type, and approval durability is outside MWO-001's six ports.

## 8. Deferred Items

- **Cross-plane durability of the in-process forge `InstallRegistry`** (the E2E demonstrates host re-hydration by re-installing over the re-opened durable stores): a dedicated host-side mirror is MWO-003 territory (restart reconciliation).
- **Resume continuation** (walking a resumed instance from its resumable run state) — MWO-003.
- **Legal-transition enforcement beyond `Paused → Running` and cancellation** — MWO-002.
- **Compaction of journals / snapshot+journal hybrids for the three snapshot stores** — future optimization Work Order if control-plane volumes demand it.

## 9. Risks

1. **Tech Lead compiles independently:** all verification here is real (rustup 1.98.1; commands and results above), but the Tech Lead's toolchain/lock resolution may differ cosmetically (e.g. thiserror 2.0.18 lock entry). The lock diff is only the new package block.
2. **`serde_json::Error::custom` projection:** public serde API, stable across serde_json 1.x, but the projected i/o errors classify as serde "data" category rather than i/o — cosmetic; messages carry full context.
3. **Torn-tail skip policy** drops a parse-failing *final* journal line by design (documented + tested). A host wanting repair telemetry must compare file sizes across opens; no logging dependency was added deliberately.
4. **Bazel lockfile:** per repo AGENTS.md, Cargo changes normally require `just bazel-lock-update`; no Bazel/just exists in this sandbox and the workflow leaf crates have no BUILD.bazel or MODULE.bazel entries (verified — WO-012's merge set the same precedent: Cargo.lock + members line only). Flagging for the Tech Lead's CI run.

## 10. Artifacts

- Branch `mwo-001/workflow-durable` at `/home/z/my-project/codex` (base `7b7dfa0cb`, implementation commit `6d1d07a77`, report commit on top).
- Git bundle: `/home/z/my-project/mwo-001-delivery.bundle` (`7b7dfa0cb..mwo-001/workflow-durable`).
- Sandbox working tree retained for harvest via the file API; `codex-rs/target/` removed after results were recorded.
