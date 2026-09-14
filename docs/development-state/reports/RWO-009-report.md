# RWO-009 Completion Report (filed by Tech Lead)

**Work Order:** RWO-009 — Engine Distribution: Visibility Gate on the Upgrade Path + Ordering Guard Rider (VWO-010 Family I P1 + Family G engine half)
**Session:** chat.z.ai agents-tab, GLM-5.3 + Full-Stack, chat 6b8e8364-e593-4c1e-b303-524e9bf19958 (third dispatch; two prior turns died mid-stream at the ~50k-char mark — silent network deaths, work re-done fresh; pod ws-cc234585 active, harvested inline 06:0x UTC)
**Delivery:** git bundle RWO-009-delivery.bundle (29,091 bytes; head f56816eaac2ed19b3fb0ca285ca21843a7accfcc, exactly 1 commit over dispatch base 2d37cc0ef; harvested from the live pod via the in-page workspaces files API)

## Worker's Delivery (from pod worklog.md + evidence, verbatim structure)

- Base verified reachable: `git rev-parse 2d37cc0ef^{commit}` → VWO-010 synthesis merge (dispatch SHA precedence over the advanced origin/main).
- Fix scope implemented exactly as bounded:
  1. `evaluate_upgrade` (~445): candidate filter now runs `release_installable(&entry.metadata, entry.state, &entry.audience, &install.principal)` — the SAME visibility predicate install enforces, install's owner as requesting principal. Private releases invisible to foreign installers never surface as upgrade candidates (Family I / attack 6d).
  2. `decide_upgrade` (~503): approval path re-runs every install gate in the documented order — ordering guard (proposal.to semver < pin semver → typed `IllegalDowngrade`) → visibility (`release_installable` → `ReleaseNotVisible`) → integrity (`proposal.to.verify()`) → access/entitlement gates (Family G engine half / attack 12 + 6d smuggled shape).
  3. `error.rs`: new typed variant `IllegalDowngrade { workflow, expected_newer_than: SemanticVersion, got: SemanticVersion }` (additive, same crate the RWO grants).
- `memory_tests.rs` (new in-crate unit-test module, `#[path]` pattern): 6 tests — evaluate skips invisible (6d), all-invisible → None (6d), decide refuses smuggled invisible targets (6d), decide refuses older-than-pin (12), rejected downgrade still records a rejection (boundary), equal-semver not a downgrade (boundary).
- Verification in the pod (cargo absent — protocol followed): line-level before/after trace; python predicate-model transcription of the Rust control flow against identical scenario state (both base holes reproduced, both fixed paths refused, parity PASS); compile-level static review; e2e compatibility trace. Evidence: `docs/validation/evidence/rwo-009/` (6 artifacts).
- Honest deviations: attack 5b is Family N (→ RWO-016 per matrix line 231), outside this RWO — flagged, not closed; `port.rs` trait docs not updated (bounded-surface discipline, follow-up question); `evaluate_upgrade` still surfaces older releases as pure data, refusal lives on `decide_upgrade` (matches acceptance criterion 2).

## Tech Lead Independent Verification (real runs, local rustc 1.95.0 / node v24.19.0)

- `git bundle verify` → okay; exactly 1 commit over base; diff scope: 9 files +1444/−5 — workflow-distribution crate (granted) + rwo-009 evidence only; fixtures, RWO doc, six validation reports, REPORT-SCHEMA: byte-identical to base.
- **Real compile + `cargo test -p codex-workflow-distribution`**: 16 unit tests + 8 e2e tests ALL PASS (supersedes the worker's cargo-absent static protocol; the 6 new tests pass explicitly).
- Fixtures `run-all.sh --reset` → `verify-sweep.sh`: **59 passed, 0 failed** on the merged tree.
- Trial merge vs main @ f5bface7d: zero conflicts; merged-tree identity `db840b249` = squash-merge tree on origin/main (proven identical).
- Merged: PR #39 squash `255e59683`; branch ref deleted.

## Program State

RWO remediation 6/12 (001/002/003/004/008/009). Remaining: rwo-005/006/007/010/011/012 (prompts staged). VWO-001..011 complete (wave-4 north-star v1); VWO-012..016 lane dispatchable.
