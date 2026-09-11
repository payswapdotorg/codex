# WO-016 Report — Execution Resource Provider Plane

**Work Order:** WO-016 (docs/work-orders/WO-016-E2B-RESOURCE-PROVIDER.md)
**Status:** MERGED via PR #18 (squash `6bbe5eb45801ba8b700eeb598e2e058b65c33f08`)
**Activation:** Post-roadmap extension packet, activated per its own scheduling rule — the core roadmap reached the provider/resource extension boundary (15/15 merged at `528741a67`), and the Tech Lead audited the live architecture first (worklog WO-016-AUDIT).

## Delivery

- **Worker session:** chat `b3c65437` (agents tab, GLM-5.3 + Full-Stack), dispatched by the Tech Lead with the wave-6 prompt (112,151 chars: verbatim WO packet + 16-file source bundle + design guidance from the audit).
- **Worker delivery:** git bundle `wo-016-delivery.bundle` at the sandbox's file-API-visible root; harvested via the workspaces files API (57,623 bytes, `git bundle verify` OK).
- **Worker base:** main @ `528741a67` (verified equal to origin/main at dispatch). **Worker head:** `bb822f113`.
- **Change surface:** exactly one members line (`"resource-providers"` appended after `"workflow-distribution"`) + the new leaf crate `codex-rs/resource-providers` (package `codex-resource-providers`, lib `codex_resource_providers`) + an additive `Cargo.lock` package entry. **Zero edits to existing crates.**
- **Final merged shape:** 24 files, +5,717 lines (11 src modules + 9 sibling test modules + Cargo.toml + E2E suite + workspace members line + lock entry).

## Independent verification (Tech Lead, toolchain 1.95.0)

The worker's sandbox had no Rust toolchain (static verification only), so the Tech Lead compiled and ran everything:

- `cargo test -p codex-resource-providers`: **30 unit + 8 E2E = 38/38 passed**
- `cargo clippy -p codex-resource-providers --all-targets -- -D warnings`: **clean**
- `cargo fmt -p codex-resource-providers -- --check`: **clean**
- Regressions (pure-leaf proof): `codex-execution-contracts` **112/112**, `codex-workflow-contracts` **86/86** — both green on the merged main.
- Re-verified on the squash-merge result (`6bbe5eb45`, re-parented onto the architect's docs-only `7e3285bcc`): trio green.

### Tech-Lead integration fixes (applied to the worker's delivery before merge)

The worker's static-only verification missed compile and logic errors the real toolchain exposed:

1. `model.rs`: the private `use codex_execution_contracts::ResourceId` became a **public re-export** (four modules resolve `crate::model::ResourceId`).
2. `provider.rs`: **`Display` impl for `LifecycleOp`** (thiserror's `Unsupported` variant formats the operation; delegates to the existing `name()`).
3. `local_containers.rs`: named lifetime on `require_owned` (returned reference borrows `inner`, not `self`).
4. `error.rs` **logic bug**: `clamp_message` truncated to `FAILURE_MESSAGE_MAX_BYTES` and then appended the 14-byte marker, emitting a 4,110-byte message that `ExecutionFailure::new` (hard 4,096 limit) would reject. The clamp now reserves the marker inside the bound.
5. `local_containers.rs` **logic bug**: the fixture declared lifecycle `create` (`.with_create()`) but its `Create` match arm unconditionally returned `Unsupported` — the shared conformance harness correctly caught the contradiction. The arm now honors the declaration (re-provision with a new opaque id, mirroring the sandbox fixture's recovery path).
6. Test-code fixes: `denined`→`denied` typo, `String` label key, `*resource_type` deref; two unused imports removed; a clippy `type_complexity` table given a named alias; a `redundant_clone` dropped; rustfmt applied.

## Implementation surface

A provider-neutral execution resource provider plane beneath the frozen WO-005 `ResourceBinding` boundary:

- **`model.rs`** — `ProviderId` (contract token rules), `ResourceClass` (Sandbox/RemoteDesktop/PersistentWorkspace/GpuCompute), bounded/validated `ResourceSpec` (+`NetworkPolicy`), credential-free `ResourceHandle`, `ResourceStatus`, `ResourceSnapshot`.
- **`descriptor.rs`** — capability declarations: `ResourceLifecycleSupport` (create/pause_resume/snapshot/fork/resize/network_policy/gpu), `ResourceQuota`, health, regions, cost. Capability is declaration only — never authorization.
- **`provider.rs`** — the port: `ExecutionResourceProvider` (sync, `Send + Sync`, host-owns-the-runtime per the WO-015 bridge convention): `descriptor/probe/create/status/release`, ONE enum-dispatched `lifecycle(handle, LifecycleOp) -> LifecycleOutcome`, required `evidence_records()` harvesting. Unsupported ops return explicit `Unsupported` — never silent no-ops, never provider-specific method names.
- **`error.rs`** — `ResourceProviderError` wrapping contract errors + `Unsupported/ResourceLost/CapacityExhausted/InvalidSpec/...`, mapped onto the contract `FailureKind` family (`failure_kind()`, `rebind_advisable()`, `normalized_failure()`).
- **`evidence.rs`** — canonical-JSON SHA-256 journal, deterministic `codex-resource-providers/<provider>/<seq>` locators, kinds reused from WO-003 `EvidenceKind`.
- **`bind.rs`** — `bind/bind_as` minting plain WO-005 `ResourceBinding` records (logical snake_case type, opaque id, class-derived peer holder: sandbox/compute→Terminal, remote-desktop→Computer).
- **`registry.rs`** — host-wiring `ProviderRegistry` + `ProviderCredentials::EnvKey` (name-only references, model-provider-info precedent; resolution is host-owned via the secrets crate).
- **`conformance.rs`** — the shared provider-neutral conformance harness (identical ops + assertions against any fixture, declaration-driven expectations, explicit `Unsupported` asserted for undeclared ops).
- **Two materially different fixtures:** `sandbox_cloud.rs` (E2B-shaped: ephemeral, snapshot→fork, pause/resume, network-policy input, capacity ceiling, outage fault, latency, cost) and `local_containers.rs` (local/container-shaped: persistent workspace, resize, container-loss fault, over-quota rejection, no fork/snapshot/pause).
- **`tests/e2e_resource_providers.rs`** — 8 E2E tests through workflow-app: provider-bound resource need into `InstantiateRequest.resources`; execution through the existing `EnvironmentAdapter` dispatch; remote-desktop through the WO-015 Computer-peer adapter; provider loss → `EnvironmentLost`→`Unavailable`→`RecoveryStrategy::Rebind` with version/integrity identity unchanged (registry and run levels); conformance parity across both fixtures; provider-neutral binding (no provider types in workflow contracts); no-op default (nothing changes when no provider is configured).

## Acceptance-criteria evidence (12/12)

1. Generic resource need, no provider identifiers in workflow semantics — E2E: workflow declares `ResourceRequirement{sandbox_compute}` (logical type only); provider-minted handle becomes a plain 3-key `ResourceBinding`; binding serialization asserted provider-free.
2. Two independent implementations, one universal contract — both fixtures pass the one shared `assert_provider_conformance` harness (unit + E2E `e2e_conformance_parity_across_both_fixtures`).
3. Compute resource executes through the existing execution abstraction — E2E: Terminal-env adapter requires `sandbox_compute`; binding attached through `CapabilityRegistry::bind_resource`/`bind`; step executes through `WorkflowLifecycle::run`.
4. Remote desktop through the existing Computer Use semantic contract — E2E: `RemoteDesktop`-class resource bound via `bind_as(..., remote_desktop_session)` under `Computer`, consumed by the WO-015 adapter.
5. Lifecycle actions stay resource semantics — `ResourceStatus` is provider-plane only; lifecycle transitions journal `EvidenceKind::Artifact` records distinct from workflow state; no lifecycle op mutates any workflow contract.
6. Long-running semantics stay with Codex/workflow orchestration — structurally absent from the port; docs state the rule; no second engine/runtime.
7. Provider failure through the existing recovery model, rebindable — E2E at run level (`FailureKind::Unavailable`→`Rebind`, `Recovered` event, new opaque resource id, `version_id`/integrity unchanged) and registry level (`EnvironmentLost`→`Unavailable`→resolve→replacement resource attached).
8. Credentials only through approved runtime configuration/reference mechanisms — `ProviderCredentials::EnvKey` name-only; E2E asserts the reference; no credential value anywhere (scan).
9. Version/source/dependency pinning independent of provider ids — E2E: `verify_integrity()` after rebinding; version/IR JSON contains no fixture/provider ids; digests recompute to pinned values.
10. Evidence distinguishes provider/resource facts from workflow semantic facts — provider-scoped locators; payloads asserted to carry no workflow identity and no credential markers.
11. Existing regressions remain green — zero edits to existing crates (diff proves it); execution-contracts 112/112 and workflow-contracts 86/86 re-run green; E2E no-op test re-proves the ordinary path.
12. No provider-specific types or semantics leak into universal workflow contracts — E2E binding-is-exactly-the-3-key-record assertion + the bind.rs class→(logical type, peer holder) mapping.

## Known limitations / deferrals

- The two implementations are in-process test fixtures (the WO's "conformance fixtures" path): no real E2B/Daytona/Modal/Kubernetes SDK is linked, and none may become a mandatory runtime dependency. Real-provider conformance evidence from current APIs/SDKs remains host-owned future work.
- `ProviderRegistry` is wiring-only; durable provider configuration, credential resolution via the secrets crate, and host-side concurrency bridging are host-owned.
- The M4 durable-control-plane gap (see the WO-012 state notes) is unchanged by this packet: provider resource lifecycle state remains in-memory in the fixtures and is never persisted as workflow state.

## Lineage

- Prompt: `replay2/scripts/build_wave6_prompts.py` (packet + 16-file source bundle at base `528741a67`)
- Worker: session `b3c65437`, base `528741a67`, worker head `bb822f113` (bundle)
- Tech-Lead integration: amended single commit `f800c5f02` (worker delivery + fixes + additive lock entry)
- Merge: PR #18 squash `6bbe5eb45` on main (after the architect's docs-only `7e3285bcc`)
