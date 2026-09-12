# FINAL — Human-Workflow Validation Report (DRAFT — remediation loop pending)

**Program:** Codex Universal Autonomous Human-Workflow Validation (docs/validation/VALIDATION-PROGRAM.md)
**Synthesis Work Order:** VWO-010 (Wave 3)
**Report status:** DRAFT. This report consolidates waves 0–2 (VWO-001..VWO-009,
all MERGED). The production-readiness verdict is **explicitly deferred** to the
remediation loop (§13): canonical P0/P1 findings are open, reproduced, and
scheduled via RWO-001..RWO-009; no verdict is rendered until the required
RWO merges and revalidations land.

## Identity

- Work Order: VWO-010 (Wave 3 — validation synthesis, remediation planning, final report draft)
- Worker persona: Validation Synthesis Analyst (senior) — documents and verifications only; implements nothing
- Base branch: main
- Base SHA: 0f89393c60403e423a91cffc3055af07e31a0c0b
- Head SHA: see git rev-parse vwo-010/validation-synthesis — single delivery commit on base 0f89393 (a commit cannot embed its own hash; exact SHA recorded in the delivery bundle and the completion message)
- Validation environment: sandbox clone of github.com/payswapdotorg/codex; VWO-002 fixtures `run-all.sh --reset` (five apps healthy on 127.0.0.1:4101–4105; canonical verify-sweep 59/59 — `docs/validation/evidence/vwo-010/_infra/post-hoc/20260912T103350Z-environment-and-base-identity.txt`); agent-browser 0.35.0 (Playwright chromium, headless); curl API twins; node v24.19.0, git 2.47.3; cargo/rustc ABSENT — engine tests not re-run here (static probes + merged reports' cargo transcripts cited)
- E2B template/workspace identity, if used: N/A — E2B/Composio integration absent (VWO-001 fallback record governs)
- Codex Universal runtime/application SHA: NONE — no user-facing Codex Universal runtime/application build exists in this repository state (Family A; the engine crates are library-only)
- Date/time window: 2026-09-12T10:30Z–2026-09-12T11:05Z (UTC) — synthesis window; consolidated report windows are in §2

## 1. Exact repository identity

- Final repository SHA (this synthesis): **0f89393c60403e423a91cffc3055af07e31a0c0b**
  (main HEAD; VWO-009 merge is the direct parent). The VWO-010 delivery commit
  (this report + matrix + RWOs + reproduction evidence) sits on branch
  `vwo-010/validation-synthesis` on top of that SHA; its exact hash is recorded
  in the delivery bundle and the VWO-010 completion message.
- Per-report bases (verified in-repo): VWO-004/005/006/007 @ 0d314fd09cec70d000f33c81286edcd8cbf72501 ·
  VWO-008 @ d198a00f0fe330b512e025d63cb3707e64ec5f9c · VWO-009 @ 7736a42e57a48c67d97b40350001e759e78661ae.
  All six merged before 0f89393 (PRs #29/#30/#31 et al. in the log).

## 2. Exact validation environment identities

| WO | Environment (as recorded by the worker) | Evidence artifacts |
|---|---|---|
| VWO-004 | sandbox clone; VWO-002 fixtures 4101–4105 (`run-all.sh`, verify-sweep 59/59); agent-browser 0.35.0 (Playwright chromium, headless); deploy-console.js piped terminal; capability_check exit 1 (rust absent, VWO-001-owned) + 2 fidelity failures (E2B/Composio absent, xdotool absent — VWO-001 fallback governs) | 161 |
| VWO-005 | sandbox fallback proving ground (VWO-001 §2); node v24.19.0, bun 1.3.14, python 3.12.14, rustc 1.95.0 (pinned, installed mid-run); agent-browser 0.35.0; fixtures 4101–4105; verify-sweep 59/59; report lint PASS | 62 |
| VWO-006 | sandbox fallback; node v24.19.0, git 2.47.3, python 3.12; agent-browser 0.35.0 + Playwright chromium; rust toolchain 1.95.0 installed in-run; canonical reset+sweep 59/59 (a non-canonical re-sweep without reseed produced 46/13 — recorded as procedure note, not a defect) | 36 |
| VWO-007 | sandbox fallback; fixtures 4101–4105; agent-browser 0.35.0, curl, deploy-console.js; cargo 1.95.0 — engine crates attacked as libraries (`cargo test`); kill -9/restart process control | 75 |
| VWO-008 | sandbox fallback; five-app fixtures; agent-browser 0.35.0 (up to 5 concurrent sessions), curl, deploy-console; cargo 1.95.0 + standalone adversarial probe crate (path deps, no codex-rs modification) | 62 |
| VWO-009 | sandbox fallback; FlowMart 4105; agent-browser 0.35.0, curl; cargo 1.95.0 + standalone probe crate at /home/z/vwo009-probe (path deps); capability_check final PASS; verify-sweep 59/59 after seed reset | 56 |
| VWO-010 (this synthesis) | sandbox clone @ 0f89393; fixtures `run-all.sh --reset` (all five healthy); canonical verify-sweep **59 passed / 0 failed**; agent-browser 0.35.0 (human-path reproduction); curl API twins (post-hoc diagnostics); **cargo/rustc ABSENT** — engine tests NOT re-run here (static probes + merged reports' cargo transcripts cited; recorded as an honest limitation) | 8 |

E2B/Composio desktop proving ground: ABSENT throughout (VWO-001 fallback
record governs; recorded fidelity limitation in every report). No secrets were
used or introduced; all fixture personas are documented demo-world accounts.

## 3. Scenario coverage matrix (summary — full matrix in SCENARIO-ISSUE-MATRIX.md)

| WO | Persona | Scenarios | Modes | Verdicts | Artifact dirs |
|---|---|---|---|---|---|
| VWO-004 | enterprise-ops real-user (construction/software/rideshare/media) | 12 | DEMO×4 INSTRUCT×4 HYBRID×4 | 12× **blocked** (business paths passed; teaching step unperformable — Family A) | `evidence/VWO-004/<scenario>/` |
| VWO-005 | workflow creator / marketplace seller | 6 | DEMO×2 INSTRUCT×3 HYBRID×1 | 6× **blocked** (marketplace half passed in full; teach/fork/improve halves unreachable) | `evidence/VWO-005/<scenario>/` |
| VWO-006 | workflow consumer / automation operator | 3 | DEMO×1 INSTRUCT×1 HYBRID×1 | 3× **blocked** (store paths passed; consumer product surfaces absent/defective) | `evidence/VWO-006/<scenario>/` |
| VWO-007 | persistence/restart/recovery/cancellation adversary | 4 | DEMO×2 INSTRUCT×1 HYBRID×1 | 4× **passed** (all attacks REPELLED; F1 mount gap recorded) | `evidence/VWO-007/<scenario>/` |
| VWO-008 | security/authorization/trust-boundary adversary | 6 | INSTRUCT×1 DEMO×2 HYBRID×3 | 6× **passed** (10/10 attack classes exercised; two P1 write-path holes found during attacks) | `evidence/VWO-008/<scenario>/` |
| VWO-009 | versioning/marketplace/distribution adversary | 3 | HYBRID×2 INSTRUCT×1 | 1× passed, 2× **passed-with-holes** (12 attack classes: 7 REPELLED both-layers, 2 HELD engine, 3 HOLES F4/F9/F10) | `evidence/VWO-009/<scenario>/` |
| **Total** | | **34** | DEMO×11 INSTRUCT×11 HYBRID×12 | 22 blocked at the teaching/product layer · 10 passed (7 cleanly, 3 with recorded holes) · **452 artifacts** | |

**Business-path result:** every fixture-level business behavior catalogued in
all 34 scenarios passed as specified (fail-closed semantics, gates, recovery,
digest stability). Every BLOCKED verdict traces to the same root: the Codex
Universal product surface for workflows does not exist (Family A), plus the
bounded fixture defects listed in §5.

## 4. Teaching-mode comparison (VALIDATION-PROGRAM.md §6)

- **Coverage:** all three modes exercised in every wave-1 WO (per program
  requirement) and across wave-2; totals DEMO×11 / INSTRUCT×11 / HYBRID×12.
- **Mode-level results:** indistinguishable at the product layer — every mode's
  teaching step failed by the SAME absence (Family A: no surface accepts
  instructions, observes demonstrations, or reconciles hybrid instruct+demo).
  The modes could not expose different failure characteristics because the
  teaching surface itself is absent. INSTRUCT/HYBRID reconciliation
  (§6 "verify that instruction and demonstration are reconciled") is
  unverifiable — there is no compile surface to reconcile against.
- **Fixture-layer contrast (what could be observed):** the modes differ only in
  harness phrasing (instruct-phase instruction texts vs. demo-phase replays);
  all catalogued gates held in every mode. The differential-failure exposure
  the program wanted (§6 "prefer workflows where the modes expose different
  failure characteristics") remains UNTESTED at the product layer pending
  RWO-001.

## 5. Issue inventory (canonical families — deduplicated)

P0 ×1, P1 ×8 (open, reproduced, RWOs proposed), P2 ×8 families, P3 remainder —
full per-finding dispositions in SCENARIO-ISSUE-MATRIX.md §2–§3.

| Family | Sev | One-line | Confirmations (local ids) | Reproduced @0f89393 | Remediation |
|---|---|---|---|---|---|
| A | **P0** | workflow/teaching engine mounted in NO user-facing surface | 6 (VWO-004 F1; VWO-005 F1; VWO-006 F-1; VWO-007 F1; VWO-008 F1; VWO-009 F1) | YES (static, 7 probes) | RWO-001 |
| B | P1 | FlowMart configure form silently drops JSON input | 2 (VWO-005 F2; VWO-006 F-2) | YES (browser + API twin) | RWO-002 |
| C | P1 | fork surface + lineage data model absent | VWO-005 F4 (+VWO-009 F6 half) | YES (surface absence) | RWO-005 (needs RWO-001) |
| D | P1 | improvement-candidate + approval-before-publish absent | VWO-005 F5 | YES (zero dependents) | RWO-006 (needs RWO-001) |
| E | P1 | cross-tenant install writes incl. pin moves | 3 (VWO-005 F6; VWO-008 F4; VWO-009 F8) | YES (3 probes ok:true on foreign org) | RWO-003 |
| F | P1 | PressRoom publication terminality UI-only (edit + publish guards) | VWO-008 F3 | YES (both attacks + UI oracle) | RWO-004 |
| G | P1 | implicit downgrade via the upgrade op (product AND engine) | VWO-009 F4 | product YES / engine static (cargo absent) | RWO-007 + RWO-009 rider |
| H | P1 | entitlement revocation bricks org+package; renewal cannot restore | VWO-009 F7 | YES (revoke→grant→retry blocked) | RWO-008 |
| I | P1 | engine upgrade path skips the visibility gate (private-release leak) | VWO-009 F10 | static (code path) — dynamic pending cargo env | RWO-009 |
| J | P2 | repelled/blocked ops leave no audit events | 3 (VWO-007 F2; VWO-008 F6; VWO-009 F3) | incidental re-confirmation | RWO-011 |
| K | P2 | evidence-checker line-scoped exemption | VWO-008 F5 | source-verified (not re-exploited) | RWO-012 |
| L | P2/P3 | configure accepts identity-impersonating keys | 2 (VWO-009 F2; VWO-006 F-6) | YES (smuggling probe) | RWO-002 rider |
| M | P2 | rollback direction confusion after downgrade | VWO-009 F5 | YES (rolled "back" forward) | RWO-007 rider |
| N | P2 | fork attribution content unverified (engine) | VWO-009 F9 | cited (engine probe HOLE) | RWO-016 (named follow-up) |
| Policy | P2 | post-publication commercial-policy config absent | VWO-005 F3 | by absence | RWO-013 (named follow-up) |
| Consumer | P2 | pre-install compatibility contract; schedule/webhook/monitoring/uninstall absent | VWO-006 F-3/F-4 | by absence | RWO-014/RWO-015 (named follow-ups) |
| Drift | P2/P3 | catalog/seed/fixture drift (6 items + install-form bindings) | VWO-004 F2/F3/F4/F5/F6; VWO-005 F7; VWO-006 F-5 | seed/ops sources verified | RWO-010 (bindings → RWO-015) |
| Email-gap | P3 | no email-receiving surface (attack vector) | VWO-008 F2 | by absence | accepted limitation (documented) |
| Scope-gap | P3 | no product surface for dev-refs/locks/scope transitions | VWO-009 F6 | by absence | RWO-017 (named, DEFER) |

## 6. Root causes (P0/P1 — full analysis in the matrix §2 and RWO files)

1. **A (P0):** library-first landing. Implementation WOs (WO-009/010/011,
   MWO-001..003) delivered the nine engine crates with zero user-facing
   mounts; WO-010's required outcome "expose workflow lifecycle through the
   existing app-server surfaces" never received its follow-up. The engine's
   own tests are green (contracts 90, execution-contracts 112, forge 44,
   distribution 18, durable 46, app 36, triggers 23 — merged-report
   transcripts); the defect is integration, not semantics. Owning layer:
   app-server-protocol/cli mount seam (RWO-001).
2. **B:** shared runtime parses urlencoded forms into STRING values;
   `configureInstall` requires `typeof object` and silently substitutes `{}`
   (ops-install.js:107). Owning layer: fixture harness (RWO-002).
3. **C/D:** the forge/distribution fork port and the evolution
   candidate/approval port exist but have no product mount and no fixture
   surface (evolution has ZERO dependents). Owning layer: mount seam + port
   exposure (RWO-005/RWO-006).
4. **E:** install ops resolve by id only; the actor-org comparison exists on
   the read path but not the write path (ops-install.js configure/upgrade/
   rollback). Owning layer: fixture ops (RWO-003).
5. **F:** `editStory` checks author/editor permission but no story status;
   `publishStory` never calls the existing `checkVersion` helper
   (media/ops.js:46/159). Owning layer: fixture ops (RWO-004).
6. **G:** no ordering comparison between target and pin anywhere on the
   upgrade path (fixture `upgradeInstall`; engine `decide_upgrade` checks
   from==installed but not to>from). Owning layers: fixture ops (RWO-007) +
   distribution engine (RWO-009 rider).
7. **H:** `entitlementFor` is first-match over an append-only list; revocation
   is terminal and shadows later grants (ops-install.js:11-13). Owning layer:
   fixture ops (RWO-008).
8. **I:** the upgrade path never consults `release_installable` — the gate
   `install` enforces (memory.rs evaluate/decide/run_gates). Owning layer:
   distribution engine (RWO-009).

## 7. Remediation Work Orders (ALL merges PENDING)

One RWO per canonical P0/P1 family (+ bounded P2 fix-now batches). Status of
every RWO: **proposed** — the Tech Lead reviews, amends, dispatches (up to
three remediation workers concurrently per program §5 Wave 3).

| RWO | Family | Scope (one line) | Merge status |
|---|---|---|---|
| RWO-001 | A (P0) | mount the workflow/teaching control plane (app-server protocol methods + CLI subcommand) | **pending RWO-001 merge** |
| RWO-002 | B+L (P1) | configure op: JSON.parse form input + reserved-key deny-list | **pending RWO-002 merge** |
| RWO-003 | E (P1) | actor-org guard on configure/upgrade/rollback | **pending RWO-003 merge** |
| RWO-004 | F (P1) | PressRoom status gate + publish expectedVersion | **pending RWO-004 merge** |
| RWO-005 | C (P1) | fork + lineage surface via the mount (needs RWO-001) | **pending RWO-005 merge** |
| RWO-006 | D (P1) | improvement-candidate + approval-before-publish surface (needs RWO-001) | **pending RWO-006 merge** |
| RWO-007 | G+M (P1) | FlowMart upgrade ordering + rollback direction | **pending RWO-007 merge** |
| RWO-008 | H (P1) | newest-active entitlement resolution (renewal restores) | **pending RWO-008 merge** |
| RWO-009 | I+G-engine (P1) | visibility gate + ordering guard in distribution crate | **pending RWO-009 merge** |
| RWO-010 | Drift (P2) | catalog/seed/fixture alignment batch | **pending RWO-010 merge** |
| RWO-011 | J (P2) | rejection/conflict audit events in shared runtime | **pending RWO-011 merge** |
| RWO-012 | K (P2) | evidence-checker substring-scoped exemptions | **pending RWO-012 merge** |
| RWO-013..017 | follow-ups | commercial-policy surface; compatibility contract; consumer execution surfaces; fork attribution verification; branch/lock/scope roadmap (DEFER) | **named, not dispatched** |

**Revalidation requirement (program §5 Revalidation):** after each accepted
RWO, re-run the affected original scenarios; a defect closes only when the
fix is merged, the original failure reproduces as fixed, no regression
appears, and the architectural invariant holds. Representative re-runs
required before any verdict: one scenario per teaching mode (VWO-004 set),
the VWO-005 lifecycle legs owned by C/D/E, the VWO-006 consumer path, the
VWO-008 F3/F4 attacks, the VWO-009 holes F4/F7/F8/F10, and verify-sweep
59/59 at every step.

## 8. Security findings (VWO-008 + security-relevant items)

- 10/10 WO-required attack classes exercised; every attack on catalog-bound
  surfaces REPELLED: injected instructions inert (PR/doc/API/web), no
  credential propagation into outputs/logs/evidence/source (checker refused
  fail-closed twice), capability-without-authorization failed closed (7 API +
  terminal probes), stale approvals and grant replays repelled (engine
  forged-grant suites), dependency escalation repelled, single-winner approval
  races, cancellation races fail-closed, commercial gates fail-closed with
  entitlement ids named, digests byte-stable under every commercial change.
- **Open security defects (P1):** Family E cross-tenant install writes
  (server-side org guard absent — RWO-003) and Family F publication
  terminality UI-only (server-side guards absent — RWO-004). Family I
  private-release visibility leak (engine — RWO-009). Family G implicit
  downgrade (labeled-op integrity — RWO-007/009).
- **Open P2:** Family K evidence-checker bypass (RWO-012); Family J rejection
  audit invisibility (RWO-011 — auditability of repelled attacks); Family N
  fork attribution trustworthiness (engine, RWO-016 follow-up).
- Trust-boundary NOTE: every engine-side trust guarantee (grant forging,
  readiness ladder, resource requirements, no-credential bindings) is
  verified ONLY at library level — unobtainable by a normal person until
  Family A's mount lands (RWO-001).

## 9. Persistence/restart findings (VWO-007)

- ALL attacks repelled at fixture layer with engine parity at library layer:
  SIGKILL mid-workflow (state/positions/dedup-ledger survive; identical
  firstSeenAt across restarts), session loss (authority persists; recovery
  surface shows intact data, no blank slate), duplicate triggers (exactly one
  accept per distinct payload, 8-way storms), idempotency-key replay across
  restart (409 with identical first-seen timestamp), trigger settlement,
  version pinning across restart (install pin + immutable published digests),
  startup reconciliation (orphaned Running → paused, idempotent sweep),
  tampered snapshots fail integrity on load.
- Open items: Family A (the durable control plane is unmounted — a person
  cannot obtain these guarantees from Codex; RWO-001); Family J (repelled
  rejections leave no audit events — RWO-011); observation (not a finding):
  no explicit "instance recovered/resumed" indicator — the catalog accepts
  intact-data observation.

## 10. Marketplace findings (VWO-005/006/009)

- PROVEN (fixture + engine parity): immutable published identities
  (re-publish refused, tampered records fail digest recompute, re-seal = new
  identity); explicit pinned installs (moving refs refused, over-installs
  refused, pin moves only through explicit decisions); stale proposals
  conflict with the pin named; commercial gates fail closed with entitlement
  ids and never touch executable content; failed (503) upgrade leaves zero
  partial state with clean retry; digests byte-stable through grant/revoke/
  install/upgrade/rollback/configure (pk-0104@1.0.0; pk-102@2.3.1/2.3.2/2.4.0);
  auditor provenance verification MATCH; rebind moves binding values only
  (pin/digest/provenance immutable — incl. under deliberate semantic
  smuggling).
- **Open (P1):** Family E cross-tenant writes incl. pin moves (RWO-003);
  Family G implicit downgrade as "upgrade" (RWO-007/009); Family H renewal
  dead-end (RWO-008); Family I private-release leak on the upgrade path
  (engine, RWO-009). Families C/D absent lifecycle halves (RWO-005/006).
- **Open (P2/P3):** identity-impersonating config keys (RWO-002 rider);
  rollback direction confusion (RWO-007 rider); fork attribution content
  (RWO-016); commercial-policy surface (RWO-013); consumer compatibility
  contract + execution surfaces (RWO-014/015); dev-ref/lock/scope surfaces
  (RWO-017, DEFER).

## 11. UX/product friction (recorded, not findings unless flagged)

- The single largest product-friction item is Family A itself: every workflow
  lifecycle action a person attempts is absent — recorded 6× across the
  reports and never worked around.
- FlowMart configure silently reporting success while persisting nothing is
  "the worst kind of friction because it is silent" (VWO-006 F-2) — P1, RWO-002.
- VWO-008 friction observation: injection payloads render inline as plain
  escaped text with no untrusted-content marker — inert (correct), but a
  reviewer gets no visual separation between external content and system
  surfaces. Observation; revisit with the RWO-001 mount's review surface.
- VWO-007 observation: recovery state is inferred from intact data; no
  explicit resumed indicator (catalog accepts this).
- VWO-004 minor: rendition request control is row-scoped inside the assets
  table (mis-target hazard for keyboard/AT users); engagement placeholders
  0/0/0 immediately post-publish (cosmetic).
- VWO-009: blocked-upgrade errors name entitlement/status/action (good
  fail-closed UX); the renewal error keeps naming the OLD revoked entitlement
  (F7 root cause) — a person cannot reconcile it with the fresh grant.

## 12. Remaining limitations

1. **No user-facing workflow product surface exists** (Family A) — every
   teaching/lifecycle product guarantee is unverified pending RWO-001..006.
2. **Engine-side dynamic re-verification in VWO-010 was impossible** (no Rust
   toolchain in the synthesis sandbox): Families G-engine/I rest on static
   code-path analysis plus the VWO-009 probe transcripts; the Tech Lead
   independently compiles and runs everything (program dispatch contract).
3. E2B/Composio desktop proving ground absent throughout (VWO-001 fallback);
   desktop-capable surface = deploy-console.js terminal only.
4. Email-injection attack vector has no attackable surface (VWO-008 F2 —
   accepted limitation; nearest analogues exercised).
5. Product surfaces for dev-branch refs, dependency locks, listing-scope
   transitions, forks, private visibility are absent (VWO-009 F6 — engine
   coverage exists; fixture roadmap deferred).
6. Fixture-level P2 drift items (catalog/seed alignment) — RWO-010.
7. The north-star wave (VWO-011..017) has not started; generalization claims
   are not yet supported by evidence (blocked on this remediation loop).

## 13. Production-readiness verdict — **DEFERRED (explicitly)**

**No production-readiness verdict is rendered by this draft.** The evidence
does not yet support one. The verdict can be rendered only after ALL of the
following land and are revalidated per program §5:

1. **RWO-001 merged + revalidated** — the P0 mount: representative
   DEMONSTRATE/INSTRUCT/HYBRID scenarios re-run through the real surface
   (VWO-004 set), teaching/reconciliation verified, Workflow Identity fields
   visible. Without this, the core product claim ("create useful workflows
   rather than merely represent them" — program §12) is unmet.
2. **RWO-002, RWO-003, RWO-004, RWO-007, RWO-008, RWO-009 merged + their
   original failing scenarios reproduced as fixed** (VWO-005 F2 path,
   VWO-005/008/009 cross-tenant probes → 403, VWO-008 F3 attacks → refused,
   VWO-009 F4/F7/F10 attacks → refused/recovered), with verify-sweep 59/59
   and no regressions.
3. **RWO-005 and RWO-006 merged + the VWO-005 fork/improvement lifecycle legs
   re-run** through the mounted surfaces.
4. **P2/P3 dispositions executed or explicitly deferred by the Tech Lead**
   (RWO-010/011/012 fix-now batches; RWO-013..017 named follow-ups; accepted
   limitations documented — already recorded in SCENARIO-ISSUE-MATRIX.md §3).
5. **The final revalidation wave** re-runs representative scenarios per mode
   and updates the six Class A reports' revalidation sections (no report is
   rewritten — revalidation evidence appends under evidence/ and this report's
   §7 table flips each row from "pending RWO-00n merge" to the merged SHA).

Only then may the Tech Lead render the explicit verdict distinguishing
implemented, verified, production-ready, and deferred capabilities (VWO-010
acceptance). Rendering a verdict earlier would be unsupported by closed
evidence; per the program's completion gate (§12) P0/P1 closure is a
precondition.

## 14. What IS demonstrated today (interim, evidence-backed)

- The workflow engine family exists, compiles, and passes its own contract
  suites (identity/digest/dependency, lifecycle/legal transitions, durability/
  resume/reconciliation, triggers/dedup, distribution gates, evolution).
- The VWO-002 synthetic ecosystem behaves like a plausible product: 34
  scenarios' business paths, gates, fail-closed semantics, conflict recovery,
  idempotency, entitlement fail-closure, digest stability — all green
  (59/59 sweep; 452 artifacts).
- The adversarial surface repelled every catalogued attack at the
  fixture/engine layers with two bounded server-side write-path holes
  (Families E/F) and three engine holes (G-engine/I + attribution N), all
  reproduced and scheduled.
- The single root blocker (Family A) is an integration gap with a bounded,
  proposed mount (RWO-001) — not an engine-semantics failure.
