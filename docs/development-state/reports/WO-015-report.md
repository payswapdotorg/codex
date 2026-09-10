# WO-015 Completion Report (harvested)

Source: agents-tab session e99edae1 (GLM-5.3 + Full-Stack), workspace-archive harvest + Tech-Lead reconstruction.
Worker branch wo-015/env-adapters @ c7f64c4d (base 67c0460, pre-WO-011); reconstructed onto main @ 55c80f66a, merged via PR #14 (2042503a4).
Independent verification: 25 unit + 7 E2E tests green, clippy -D warnings clean, fmt clean (toolchain 1.95.0).

---

=== WO-015 COMPLETION REPORT

Work Order ID: WO-015 — Additional Execution Environments (payswapdotorg/codex)

Base branch + SHA: origin/main @ 67c0460ae16836cac23e13e38f3bea3fcb4682e0 (verified: git rev-parse 67c0460ae^{commit} == clone HEAD at branch point; WO-010 merged, 10/15 WOs)

Head SHA: c7f64c4de304c45eefa86e8ac16dc81c45102aa3 on branch wo-015/env-adapters (commit: feat(env-adapters): WO-015 additional execution environments; not pushed — Tech Lead applies/merges; source tree fully preserved, codex-rs/target removed post-verification per sandbox-lean instruction)

Changed files / surfaces (20 files, +4,460, zero deletions):

codex-rs/Cargo.toml (+1: members line) — the only edit to any existing file
codex-rs/Cargo.lock (+19: new package entry; hand-written, cargo accepted verbatim, git diff confirms no rewrite)
New leaf crate codex-rs/env-adapters (lib codex_env_adapters): Cargo.toml, lib.rs, bridge.rs (host-bridge port + FailureKind mapping + clamped normalized failures), policy.rs (shared allow/deny AccessTable), evidence.rs (digest-bearing EvidenceJournal, deterministic locators, SHA-256 over canonical JSON), session.rs (ownership + takeover receipts, dispatch-owner displacement, re-prepare re-acquisition), error.rs, remote_desktop.rs + remote_desktop_adapter.rs, mobile_device.rs + mobile_device_adapter.rs, 6 unit-test modules, 1 E2E file (tests/e2e_env_adapters.rs, 1,522 lines)

Implementation summary:

Two new adapters over the frozen WO-005 EnvironmentAdapter seam, in the WO-006/WO-007 leaf-adapter shape (no second runtime, lazy init, evidence-not-authority, native approvals, no credentials cross boundaries):
RemoteDesktopEnvironmentAdapter — provides control_remote_desktop + control_desktop_app as compatible alternate; requires remote_desktop_session resource; host policy gate; bridge port (connect/execute/health).
MobileDeviceEnvironmentAdapter — control_mobile_device; mobile_device resource; device policy gate; attach/execute port.
Real host bridges out of scope per guidance: modeled as port + scripted in-memory test doubles (WO-006/007 precedent).
Recorded design deviation: ExecutionEnvironment::Mobile/RemoteDesktop enum variants are frozen-reserved in execution-contracts (is_reserved() gate + merged tests assert loud failure). Editing that gate would violate the stronger "Do NOT edit existing crates" constraint, so both adapters register behind the peer Computer class at BindingClass::Compatible — the compatible-alternate position in the existing fallback chain, consistent with the repo's own WO-016 advisory ("a remote desktop resource … is reached through the existing Computer Use semantic contract") and architecture §6. Reserved variants and their frozen tests are untouched; a unit test (reserved_environment_classes_still_fail_loudly) pins this.
Hybrid environments (browser/computer/remote/mobile) compose through the same workflow-app lifecycle/run ports — zero environment-specific orchestration branches. All actions pass native policy/sandbox gates; evidence + approvals identical to existing adapters.

Test commands and precise results (real cargo 1.95.0 = repo pin; re-run post-stall-recovery on fresh rebuilds of the committed tree):

cargo test -p codex-env-adapters → exit 0: 25 unit passed + 7 E2E passed, 0 failed (0.00s / 0.01s)
cargo clippy -p codex-env-adapters --all-targets -- -D warnings → exit 0 (3m 33s; workflow-app + browser-use-adapter compiled as E2E deps in same pass)
cargo fmt -p codex-env-adapters -- --check → exit 0 (only repo-wide pre-existing stable-channel imports_granularity warning)
cargo check -p codex-workflow-app → exit 0 (fresh target, 3m 38s; first attempt hit ENOSPC from the clippy build filling the disk — environmental, not code; green after cleanup)

Acceptance evidence (item by item):

Additional environment runs existing-compatible workflows through the same execution contract — e2e_mixed_run_binds_browser_computer_remote_and_mobile_in_one_registry: WO-006 browser + WO-007 computer + remote-desktop + mobile adapters bound in ONE CapabilityRegistry through the same workflow-app run path (20-event sequence, 4 approvals, 5 observations, 4 traces, adapter evidence journals).
Adapter introduction requires no workflow-semantic change — diff touches zero lines of workflow-contracts / execution-contracts / workflow-app sources; adapters implement the pre-existing seam; new environments register via the same registry path (future envs need no semantic redesign).
Same capability/resource/observation/action/result/recovery/takeover model — full lifecycle per adapter (descriptor, resource binding, readiness DECLARED→BOUND→EXECUTING, observations journaled, recovery records, takeover displacing dispatch owner until re-prepare).
Failure/readiness diagnostics — e2e_unreachable_remote_bridge_blocks_the_plan_with_explicit_diagnostics: NotInstalled diagnostics + registry diagnose(); unreachable_bridges_probe_not_installed (both adapters).
Security/authorization — e2e_policy_scope_denies_the_new_environments (registry PolicyDenied), e2e_adapter_policy_denial_fails_actions_and_escalates (host-policy denial, zero bridge calls); credentials never cross the bridge port.
Bridge loss / recovery — e2e_desktop_bridge_loss_rebinds_to_the_remote_desktop_environment (native desktop loss → FallbackRecord{PreferredNotReady} → rebind to remote-desktop env with Recovery{Rebind} + fresh approval); e2e_transient_mobile_bridge_loss_recovers_by_retry (Recovery{Retry, Recovered} + fresh approval).
No mandatory new environment — ordinary_codex_behavior_is_untouched_without_a_workflow: NoActiveWorkflow, zero activity.

Compatibility impact: With no active workflow, behavior is provably unchanged (no-op test + green cargo check -p codex-workflow-app). The workspace gains one leaf crate + one members line; no existing crate binary/API surface modified.

Known limitations:

Real RDP/VNC/ADB host bridges are out of scope (port + scripted doubles only) — per work-order scope.
Reserved enum 