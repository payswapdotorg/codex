# RWO-002 Completion Report (filed by Tech Lead)

**Work Order:** RWO-002 — FlowMart Configure Op: Parse Form JSON + Reject Identity-Impersonating Keys (VWO-010 Family B P1 + Family L rider)
**Session:** chat.z.ai agents-tab, GLM-5.3 + Full-Stack, chat 0e47bfd4-9784-458f-aa04-f700fca8bb52 (assault-ladder fresh session after three rate-limit-voided predecessors)
**Dispatched:** 2026-09-14 00:57 UTC · **Completed:** 2026-09-14 01:48 UTC (~51 min, clean run, zero stall-recovery cycles)
**Delivery:** git bundle RWO-002-delivery.bundle (16,879 bytes; head 97b2eaae86684460e5ee4f4337a95096d9bdfdf6, exactly 1 commit over dispatch base 2d37cc0ef; harvested from the live pod via workspaces files API)

## Worker's Completion Report (verbatim)

=== RWO-002 COMPLETION REPORT ===

Work Order ID: RWO-002
base branch + base SHA: main @ 2d37cc0ef7d075e877f370790f82ac9c70df26a4 (verified with git rev-parse 2d37cc0ef7d075e877f370790f82ac9c70df26a4^{commit}; reachable — branch created ON that SHA. Note: origin/main had moved to cebc6be44 after dispatch (RWO-001 merged); the work order's named base was used, not moved main.)
head SHA (your branch, after your commit): 97b2eaae86684460e5ee4f4337a95096d9bdfdf6 (branch rwo-002/flowmart-configure-contract, exactly one commit, NOT pushed)
changed files/surfaces:
docs/validation/fixtures/marketplace/ops-install.js (the RWO Fix scope): new shared configPatch(b, op) helper replacing both silent typeof b.config === 'object' && b.config ? b.config : {} intakes — in configureInstall and the install op's config intake (installPackage)
docs/validation/fixtures/marketplace/FAILURES.md + docs/validation/fixtures/README.md (Fix scope item 3: "Configure-config contract" documentation)
docs/validation/evidence/rwo-002/ (4 new evidence artifacts, canonical layout post-hoc/ + _infra/)
Frozen surfaces byte-identical (git diff vs base empty): work-orders/, the six validation reports, REPORT-SCHEMA, scenario catalog; no engine crates touched
implementation summary: String config values (the urlencoded textarea — the human path) are now JSON.parsed; parse failure or a non-object value (scalar/array/null) → HTTP 400 invalid_json naming field config — never a silent {} success. Reserved-key deny-list → HTTP 400 reserved_config_key naming the offending keys: version, digest, targetVersion, manifest, packageId, expectedVersion, any __-prefixed key — applied to BOTH ops. Rejected requests mutate nothing (no history entry, no event); success remains observable (flash/message names keys, Config cell shows them, history entry configKeys lists them, install.configured event carries data.keys).
verification commands and exact results:
bash docs/validation/fixtures/run-all.sh --reset → five apps healthy (construction 4101, software 4102, rideshare 4103, media 4104, marketplace 4105; all reset)
bash docs/validation/fixtures/verify-sweep.sh → RESULT: 59 passed, 0 failed (also run at the unmodified base tree via git stash: 59/59 — no regression; verify-sweep.sh itself NOT modified, no harness alignment needed)
node --check docs/validation/fixtures/marketplace/ops-install.js → clean
VWO-010 Family B/L evidence flow reproduced at base, then fixed on branch via one identical 12-probe script (form twin w/ session cookie + API twins): form string config base 303 ok=…configured (no keys) config unchanged → branch 303 ok=…configured (opsChannel) config persisted; invalid JSON base 200 silent → branch 400 invalid_json; reserved keys base 200 merged (version/digest/targetVersion/manifest) → branch 400 reserved_config_key; __smuggled base 200 merged → branch 400; object twin 200 both; install intake same pattern (string persisted / reserved 400 / invalid 400 / object unchanged); install.configured events 8 at base → 4 on branch (rejected requests emit nothing)
acceptance-criteria evidence (all in docs/validation/evidence/rwo-002/post-hoc/20260914T014613Z-acceptance-criteria.txt, citing the before/after + sweep artifacts):
Browser path (real agent-browser 0.35.0 run: login iris.chen → catalog → Install 1.2.0 → ins-0402 → paste {"opsChannel":"east"} → Apply config): flash (role=status) "Install ins-0402 configured (opsChannel)."; Config cell {"opsChannel":"east"}; history entry configKeys:["opsChannel"] + event data.keys=["opsChannel"] — VERIFIED
Invalid JSON {"opsChannel": east → role=alert flash "…[invalid_json]"; config unchanged; exactly the same 2 history entries and 1 event (no configured entry, no event) — VERIFIED
{"version":"9.9.9"} → HTTP 400 reserved_config_key (API twin literal; full 4-key battery with reservedKeys listed in the JSON body); config unchanged — VERIFIED
API twin object JSON 200 merged on configure AND install (plus 59/59 sweep) — VERIFIED, no regression
rollback note: Single-file fixture change (+2 doc files + evidence): git revert 97b2eaae8 (or revert the commit on the branch) restores base behavior. No state migration — config records keep whatever they already contain (per the RWO).
known limitations / honest deviations:
Absent-config-field decision (documented contract): b.config === undefined still means "no keys to merge" (success, "configured (no keys)"). Deliberate: the /ui/installs/install HTML form sends no config field at all, so absent-tolerance is required for the install form; the RWO text ties "never a silent {} success" to parse failure. Documented in FAILURES.md. Empty-string/null/non-object values DO 400.
History-table rendering question for the Tech Lead: criterion 1's "history entry lists the key" is satisfied at the data level (the configured history ENTRY created by the in-scope configureInstall carries configKeys:["opsChannel"], verifiable via /api/installs); the /installs history TABLE row (ui.js:199, outside the RWO's named fix scope) renders action/versions/by/at and does not render the key list. I did NOT edit ui.js (bounded scope); flagging whether a one-line ui.js addition is wanted.
Form-path errors surface as the runtime's standard 303 redirect + err flash "<message> [<code>]" (runtime error handler, unchanged — not part of the fix scope); the literal HTTP 400 codes are on the JSON API twins, which is where the RWO's "HTTP 400" wording is verified.
No Rust toolchain in this sandbox (which cargo empty) — irrelevant to this JS-only work order; no engine crates touched, no engine-side claims made. All verification results are real command outputs (node v24.19.0, bash 5.2.37, curl 8.14.1, agent-browser 0.35.0).
Base sweep was captured via a temporary git stash cycle (fixtures restarted from the unmodified tree), then the fix was restored — recorded in the evidence _infra file.
delivery: git bundle create /home/z/my-project/RWO-002-delivery.bundle 2d37cc0ef7d075e877f370790f82ac9c70df26a4..rwo-002/flowmart-configure-contract → created (16,879 bytes; git bundle verify + list-heads confirm exactly one commit: 97b2eaae8). Sandbox working tree kept (fixtures stopped, state gitignored); no push performed.

=== END RWO-002 COMPLETION REPORT ===

## Tech-Lead Independent Verification (2026-09-14 01:5x UTC)

**Bundle integrity:** `git bundle verify` → "is okay", requires 2d37cc0ef, contains exactly refs/heads/rwo-002/flowmart-configure-contract @ 97b2eaae8 (sha1). Byte size 16,879 matches the report. Fetched into the canonical clone as rwo-002-incoming; `git log` confirms one commit, author Z User, 2026-09-14 01:48:10 +0000.

**Diff scope audit:** `git diff --stat 2d37cc0ef rwo-002-incoming` → 7 files, +851/−3 — exactly ops-install.js (46 lines), FAILURES.md (14), fixtures README (2), and 4 evidence artifacts; zero changes under work-orders/, reports/, scenarios/ (frozen check: 0 matches); zero engine/crate changes.

**Sweep (real run, node v24.19.0 local):** worktree checkout of the branch → `run-all.sh --reset` (five apps healthy) → `verify-sweep.sh` → RESULT: 59 passed, 0 failed — matches the worker's claim; no harness alignment was needed (verify-sweep.sh untouched by the diff).

**Acceptance probes (independent, not the worker's script):** against marketplace :4105 as iris.chen on ins-0402 —
- P1 Family B core: POST /api/installs/configure with config as STRING `'{"opsChannel":"east"}'` → HTTP 200 "configured (opsChannel)", install.config = {zones:"A", opsChannel:"east"}, history entry configKeys:["opsChannel"] — the string is parsed and merged (base behavior: silent {} "no keys").
- P2 Family L core: config {"opsChannel":"east","version":"9.9.9","digest":"deadbeef","__smuggled":1} → HTTP 400 reserved_config_key, reservedKeys: ["version","digest","__smuggled"], install config unpolluted.
- P3 invalid JSON string → 400 invalid_json; P4 scalar config 42 → 400 invalid_json ("not a number").
- P5 valid object → 200, merged (golden unchanged).
- P6 absent config field → 200 "configured (no keys)" — the documented install-form contract.
- Mutation audit: exactly two configured history entries after P1+P5; the three rejected probes (P2–P4) produced zero history entries and zero events — "rejected requests mutate nothing" confirmed.

**Trial merge + merged-main identity:** merge-base(main, branch) = 2d37cc0ef; `git merge --no-ff` into a trial off main @ cebc6be44 → zero conflicts, tree e679a2ca9; sweep on the merged tree → 59/59. PR #36 squash-merged as 1f1b7c67e with tree e679a2ca9 — byte-identical to the locally verified trial tree. Branch ref deleted.

**Disposition of the worker's flagged questions:**
1. *ui.js history-table key rendering* — accepted as out-of-scope (bounded-scope discipline honored; the data-level contract is fully verifiable via /api/installs). Deferred to a UI-polish pass if wanted; not a security surface.
2. *Absent-config = "no keys"* — accepted; it is the documented install-form contract (the HTML form sends no config field), and every present-but-invalid value 400s.
3. *Form-path 303 + flash vs literal HTTP 400* — accepted; the 400 contract is on the API twins, the form twin surfaces the same codes via err-flash `[code]`, consistent with the shared runtime error handler.

**Result:** RWO-002 MERGED via PR #36 (squash 1f1b7c67e). 4/12 RWOs closed (001, 002, 003, 004). Remaining: rwo-005/006/010/011/012 undispatched, rwo-007 parked, rwo-008 + vwo-011 completions awaiting pod-wake harvest, rwo-009 queued.
