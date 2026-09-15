# RWO-007 — Upgrade Means Forward: Ordering + Downgrade + Rollback Direction Semantics: Completion Report

**Status:** COMPLETE — branch `rwo-007/upgrade-direction-semantics`
**Base:** main @ c5e7f2647 (post-RWO-006)
**Work order:** `docs/validation/work-orders/RWO-007.md` (Family G product half + Family M rider)
**Scope:** single fixture file + contract docs; no Rust, no protocol, no engine changes
(the Family G engine half — decide_upgrade ordering — is RWO-009's rider; surfaces disjoint)

## What was built

FlowMart's version-movement semantics are now honest at the product layer:

1. **`upgradeInstall`** — a segment-wise numeric semver comparison (`cmpSemver`)
   between `targetVersion` and the install's current version. An older target is
   refused with HTTP 409 `implicit_downgrade_refused` (naming `currentVersion`
   and `targetVersion`, pointing the user at the rollback op) before any
   mutation: pin, history, and events stay unchanged. The smallest fix per the
   work order: refuse (no labeled-downgrade flag).
2. **`rollbackInstall`** — the rollback target is now the `fromVersion` of the
   most recent `upgraded` history entry (the version the pin came from),
   replacing the old "any entry whose toVersion differs from the current pin"
   selection that could pick a NEWER version and move the pin forward labeled
   "rolled back". No prior upgrade → the existing "no prior version"
   `data_conflict` refusal, unchanged.
3. **`FAILURES.md`** — both contracts documented in a new
   "Upgrade/rollback direction contract" section.

## Acceptance criteria → evidence (all real runs; `docs/validation/evidence/rwo-007/`)

1. Upgrade 1.3.0-pin → 1.2.0 target: **409 `implicit_downgrade_refused`**,
   state byte-identical after the refusal (version 1.3.0, no new history entry).
2. Legit upgrade 1.2.0 → 1.3.0 then rollback: **lands on 1.2.0** (the upgrade's
   fromVersion; final history `rolled_back 1.3.0 → 1.2.0`, backward move).
3. Same-version upgrade → `duplicate_event` and stale `expectedCurrentVersion`
   → `data_conflict` both unchanged; the full ecosystem sweep —
   `run-all.sh --reset` + `verify-sweep.sh` — passes **59/59** (golden path,
   all failure switches, all five fixture apps).

## Lineage

Tech-Lead direct execution (single-file bounded scope; no worker dispatch
needed). Verification battery run live against the fixture ports.
