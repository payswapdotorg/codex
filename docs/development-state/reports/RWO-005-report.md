# RWO-005 Completion Report (filed by Tech Lead)

**Work Order:** RWO-005 — Fork + Lineage Product Surface, mounted via RWO-001 (VWO-010 synthesis; Family C — VWO-005 F4 P1 + the fork half of VWO-009 F6)
**Session:** local Task-tool worker (flight 1, uncommitted hand-off after context death) + Tech-Lead completion on the same worktree — the chat.z.ai dispatch path was suspended this cycle by an operator-pod reset (lesson 118: local worktree workers are the reset-proof alternative, lesson 120)
**Dispatched:** 2026-09-14 18:24 UTC (worker flight 1) · **Completed:** 2026-09-14 21:30 UTC · **Base:** main @ 9441a823f576a53591a6ce3a96a5ed47b6e9ca85 (branch rwo-005/fork-lineage-surface)

## Lineage

- Worker flight 1 (Task-tool subagent, general-purpose): the full mount implementation — protocol types + method registration, app-server handler delegating to the frozen distribution port's `fork`, CLI subcommand + lineage rendering, tests at every layer (13 modified files, +997/−6). Died at 18:36 UTC mid-implementation (context death, 34 min file-silence; no commit).
- Tech-Lead completion (same worktree, audit-first per lesson 120): audited every inherited file line-by-line against the work order, then completed the delivery — the missing precomputed-export regeneration (python replication of the ts-rs 11.1.0 + schemars 0.8.22 generator output, RWO-001 lesson-118 precedent, format rules re-derived from the vendored tree and the actual generator sources), the evidence set, and this report.

## Implementation surface (exactly the bounded Fix scope)

1. **One protocol method** — `WorkflowFork => "workflow/fork"` (experimental, registered in macro order after `workflow/publish`): `WorkflowForkParams` (version_id, fork_repository, semantic_version?, commit_sha?, owner?, license?, attribution) and `WorkflowForkResponse` (full release identity + `lineage: WorkflowForkLineage` + carried `attribution`), with the nested `WorkflowForkAttribution` / `WorkflowForkLineage` types registered in `EXPERIMENTAL_CLIENT_METHOD_DEPENDENCY_TYPES` (export.rs) so they cannot leak into the stable fixture tree.
2. **One CLI subcommand** — `codex workflow fork <workflow>@<version-id> --as <new-repo> [--semver …] [--commit-sha …] [--owner …] [--license …] [--attribution "Name <contact>" …] [--json]`: forks a published version into a new sealed release and renders the result; `--json` emits the full response (lineage + attribution).
3. **Lineage on inspection** — the fork response and the CLI rendering carry forked-from workflow/version-id/semantic-version/repository, the upstream definition and dependency-lock digests, the anchoring commit, and the carried attribution. (Attribution CONTENT verification is RWO-016, out of scope here.)
4. **Engine semantics reused as-is** — the app-server handler seeds a fresh engine marketplace from the durable version store, delegates to the frozen `DistributionPort::fork`, mirrors the derived sealed release into the durable store (refusing an already-published identity), and pins the upgrade policy. No frozen-crate edits.

## Acceptance criteria mapping

1. *Fork through the surface → NEW immutable release with upstream pinned; re-publishing the same identity refused* — app-server tests `fork_produces_a_new_immutable_release_with_pinned_lineage` (workflow_tests.rs:608) and `reforking_the_same_identity_is_refused` (:667); JSON-RPC end-to-end `fork_publishes_a_new_immutable_release_with_lineage_over_jsonrpc` (tests/suite/v2/workflow.rs:369).
2. *Lineage visible in listing/detail* — `fork_response_serializes_lineage_and_attribution` (protocol workflow_tests.rs:317) + the CLI rendering probes (after-surface-presence.txt).
3. *Empty attribution on a fork refused (engine already enforces)* — the pre-existing, unchanged engine test `forks_must_carry_upstream_attribution` (workflow-distribution/src/metadata_tests.rs:140, PASSED in the local engine run) + mount-side early validation `fork_requires_carried_attribution` (workflow_tests.rs:702).

## Verification

- `cargo test -p codex-workflow-forge -p codex-workflow-distribution` (work-order command 1, REAL local runs, rustc 1.95.0 workspace pin): **forge 44 passed; distribution 16 + 8 (e2e) passed; 0 failed** — evidence/post-hoc/engine-verification.txt.
- `cargo check -p codex-app-server-protocol -p codex-app-server -p codex-cli`: **clean, 8m26s** — evidence/post-hoc/build-verification.txt.
- `cargo test -p codex-app-server-protocol -p codex-cli` (work-order command 2): **runs on the PR's GitHub Actions CI** — this pod's 4 GiB cgroup cannot link the protocol lib-test binary (kernel OOM at rustc anon-rss 2.7 GB, reproduced 4× with lld/-j1/debug-stripped; disk likewise insufficient for three target generations). The repo's rust-ci workflow on 2-core/7G+ runners is the authoritative judge; the merge gate is CI green, including `experimental_precomputed_exports_match_generated` byte/value-verifying the regenerated export.
- Precomputed export: regenerated (see evidence/post-hoc/precomputed-export-regeneration.txt) — structural invariants verified locally (single arm after publish, resolving refs both namespaces, hoisted-vs-inline definitions matching the WorkflowPublish* shape, alphabetical required arrays, paragraph-joined descriptions byte-checked against CommandExecParams).

## Deviations (honest)

1. The chat.z.ai worker session did not deliver this work order: the morning worker (59f8ad9e) died disk-starved at the cli-test phase with its pod subsequently reset; the afternoon dispatch (566698fc) fired at 14:54 UTC but died mid-recon; the final dispatch (f53b79e8, 15:38 UTC) remains queued server-side while the operator pod itself was reset at 17:34 UTC. The local Task-tool worker (lesson 120) carried the implementation instead; its flight died at context overflow and the Tech Lead completed the delivery on the same worktree (audit-first, completion-not-rewrite).
2. Work-order command 2 was not run to completion locally (environment-constrained, documented above and in build-verification.txt); the PR's CI is the verification of record for those suites.
3. The precomputed export was regenerated by python replication rather than the real generator (the generator lives in the test binary that cannot link here); byte/value-exactness is judged by the same CI test that judges the real generator's output.

=== RWO-005 COMPLETION REPORT ===
- base branch: main
- base SHA: 9441a823f576a53591a6ce3a96a5ed47b6e9ca85
- head SHA: (this commit — see `git rev-parse HEAD` on rwo-005/fork-lineage-surface after the delivery commits)
- tests: forge 44 passed / 0 failed; distribution 16+8 passed / 0 failed (local, real); protocol + app-server + cli suites via PR CI (gate: green)
- evidence: docs/validation/evidence/rwo-005/
- report: docs/development-state/reports/RWO-005-report.md
- deviations: 3 (all documented above)
=== END RWO-005 COMPLETION REPORT ===
