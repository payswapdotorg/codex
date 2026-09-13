# VWO-011 — Universal Computer Task Taxonomy

**Work Order:** VWO-011 (Wave 4 — north-star task taxonomy and coverage benchmark)
**Status:** CANONICAL — the benchmark catalog (`BENCHMARK-CATALOG.md`), scoring rules
(`SCORING.md`) and coverage matrices (`COVERAGE-MATRIX.md`) classify every task
against THIS document. VWO-012 … VWO-016 must place their executed tasks on these
dimensions; deviations are recorded, never silent.
**Governing sources (this document composes, never replaces):**
`docs/validation/north-star/README.md` (test model, coverage dimensions, evidence
discipline, claim boundary) · `docs/validation/VALIDATION-PROGRAM.md` §7
(universal computer-task coverage) · `docs/validation/work-orders/VWO-011.md`
(Required task dimensions — the acceptance contract for this taxonomy).

---

## 0. Purpose and claim boundary

This taxonomy turns the north-star claim — *automate anything a human can do on a
computer, subject to capabilities, authorization, resources, interfaces and
environment access actually available* — into a classification instrument.

It is **not** a definition of all computer tasks. A finite value set over finite
dimensions is a measurement grid, not the task space itself; per the north-star
README, a finite test suite can demonstrate breadth and uncover limits but cannot
prove that every conceivable task is solvable. New task families enter the
benchmark through the ADD-NEW-FAMILY PROCEDURE (`COVERAGE-MATRIX.md` §5), which
never changes workflow semantics. Where a task genuinely needs a value this
taxonomy does not yet model, the placement procedure (§3) forces an explicit
`unmodeled` exception that the Tech Lead must adjudicate — the gap is recorded,
never silently rounded to the nearest existing value.

The taxonomy is deliberately **product-agnostic**: it classifies *tasks as a human
would describe them*, not implementation strategy. The same task placement is
valid regardless of which engine, adapter or teaching mode executes it.

---

## 1. Dimension set (overview)

The VWO-011 Required-task-dimensions list names seven axes — environment modality,
interaction modality, statefulness, application boundaries, data shape, human
involvement, failure/adaptation requirements — and twenty bullet values that must
all be placeable. Six of those bullets are compound (parallel tasks, conditional
branching, uploads/downloads, destructive/irreversible operations, tasks with no
predefined workflow, authentication/authorization), so this taxonomy decomposes
the seven axes into **thirteen fine-grained dimensions** grouped under the seven
axis names. One value per dimension is the composition rule (§2); the mapping
table (§4) shows that every required bullet lands on exactly one dimension.

| Group (axis) | Dim | Name | Values (short) |
|---|---|---|---|
| A — environment modality | TD-01 | environment modality | `browser` `desktop-gui` `terminal-cli` `filesystem` `api-tool-mcp` `mixed` |
| B — interaction modality | TD-02 | interaction modality | `gui` `browser-ui` `terminal` `file-op` `api-tool-call` `human` `mixed` |
| C — statefulness & structure | TD-03 | statefulness | `stateless` `stateful-session` `long-running-resumable` |
| | TD-04 | control structure | `linear` `conditional-branching` |
| | TD-05 | concurrency | `sequential` `parallel-fanout` |
| D — application boundary | TD-06 | application boundary | `single-application` `cross-application` |
| E — data shape | TD-07 | data shape | `structured` `unstructured-text` `unstructured-visual-media` `mixed` |
| | TD-08 | transfer operations | `none` `upload` `download` `upload+download` |
| F — human involvement | TD-09 | human involvement | `unattended` `human-in-the-loop` |
| | TD-10 | credential boundary | `none` `authentication` `authorization` `authentication+authorization` |
| G — failure & adaptation | TD-11 | failure/adaptation requirement | `happy-path-only` `recovery-required` `adaptation-required` `recovery+adaptation` |
| | TD-12 | reversibility | `reversible` `destructive-irreversible` |
| H — workflow provenance | TD-13 | workflow provenance | `scripted` `human-authored` `held-out` |

TD-13 (provenance) is added beyond the seven prose axes because the required
bullet "tasks with no predefined workflow supplied to the agent" and VWO-016's
held-out method both need a first-class dimension; it is also the dimension the
north-star README's *generalization* coverage dimension measures.

---

## 2. Composition rule (normative)

> **A benchmark task = exactly one value on each of the thirteen dimensions**
> (a 13-tuple). Multi-valued realities are represented by the explicit `mixed`
> value, which MUST carry an enumerated set of the concrete values involved
> (minimum two, each named).

Consequences, all binding:

1. **No dimension may be left blank.** If a value genuinely cannot be determined
   at authoring time (held-out tasks are classified *after* execution), the
   placeholder is `pending-placement` and the task may not be counted in any
   coverage row until placed.
2. **`mixed` is not a wildcard.** `TD-01: mixed {browser, terminal}` and
   `TD-01: mixed {desktop-gui, filesystem, api-tool-mcp}` are different
   classifications; the set is part of the classification and is what coverage
   matrices aggregate.
3. **One lane per task.** The primary environment (the environment hosting the
   goal-critical steps) determines the benchmark lane and the executing Work
   Order; see `BENCHMARK-CATALOG.md` §2. A task whose goal-critical steps are
   genuinely balanced across environments is classified `mixed` on TD-01 and
   owned by the cross-application lane (NST-X / VWO-015).
4. **Classification is task-level, not step-level.** Steps may be recorded with
   their own environment/interaction for evidence purposes, but the task's
   dimension values describe the whole goal.
5. **Values are closed per benchmark version.** Adding a value to any
   dimension's value set changes measurement semantics → MAJOR benchmark version
   bump (see `SCORING.md` §8). Adding tasks within existing values is a MINOR
   bump.

---

## 3. Dimension definitions (normative)

Each dimension: rigorous definition → closed value set → placement procedure
(how a concrete task is placed).

### TD-01 — Environment modality (group A)

**Definition.** The class(es) of computer environment in which the task's
goal-critical steps physically execute, classified by where the world-state the
task mutates or observes actually lives — not by which tool the agent prefers.

**Values.**

- `browser` — steps execute against web-application state reached through a
  browser engine (pages, forms, DOM state, web sessions).
- `desktop-gui` — steps execute against OS-window-graph state of native
  applications (windows, dialogs, menus, native apps rendered on a display
  server; in the fallback proving ground: Xvfb virtual framebuffer, VWO-001 §1).
- `terminal-cli` — steps execute as shell commands / CLI programs against
  process, environment and stdout/stderr state.
- `filesystem` — steps execute against filesystem state (files, directories,
  documents) as the primary world being mutated, independent of the access tool.
- `api-tool-mcp` — steps execute as direct programmatic calls (HTTP APIs,
  platform tools, MCP servers) where the world-state is the remote service's.
- `mixed {set}` — goal-critical steps span ≥2 of the above; the set MUST be
  enumerated.

**Placement.** Ask: "if I snapshot the world the human cares about before and
after, where does it live?" A browser form that writes a file via a save dialog
is `mixed {browser, filesystem}`; a shell pipeline that reads a web API is
`mixed {terminal-cli, api-tool-mcp}` only if the API state (not merely the
command) is goal-critical, otherwise `terminal-cli` with a supporting step.

### TD-02 — Interaction modality (group B)

**Definition.** The action channel(s) through which the executor acts and
observes, as distinct from where state lives (TD-01). Mirrors the north-star
README interaction universality axis: GUI, browser, terminal, file, API/tool/MCP
and human interaction must be representable.

**Values.**

- `gui` — point/click/type/keyboard against rendered application UI (desktop
  windows; includes GUI-mode browser windows when the *window graph* — focus,
  stacking, dialogs — is what matters).
- `browser-ui` — page-level web interaction (navigate, click DOM, fill forms,
  read rendered pages, tabs).
- `terminal` — typed command interaction with a shell/CLI.
- `file-op` — direct file operations (create/read/write/move/delete documents).
- `api-tool-call` — programmatic calls to APIs/tools/MCP servers.
- `human` — structured interaction with a human (approval, input, choice) as an
  action channel.
- `mixed {set}` — ≥2 of the above, enumerated.

**Placement.** Ask: "how does the executor act at each goal-critical step?"
TD-01 and TD-02 are orthogonal: filesystem navigation through a GUI file manager
is `TD-01: filesystem` + `TD-02: gui`; the same goal by shell is
`TD-02: terminal`. The benchmark deliberately contains such pairs —
environment/interaction orthogonality is itself under test.

### TD-03 — Statefulness (group C)

**Definition.** The durability and lifetime requirements of task state across
steps, sessions and restarts.

**Values.**

- `stateless` — each step is independent; no carry-over needed; a rerun of any
  step is harmless.
- `stateful-session` — intermediate results must persist across steps within one
  working session (login sessions, multi-step wizards, carried data).
- `long-running-resumable` — the task outlives a session; it must resume
  correctly after restart/session-loss with idempotency (durable state, triggers
  — the VWO-007 class of guarantees, now at task level).

**Placement.** Ask: "if the process dies mid-task and restarts, what must be
true?" Nothing → `stateless`; the session must be re-established but the goal
re-executes from the start cleanly → `stateful-session`; execution must continue
from recorded progress without duplicate side-effects → `long-running-resumable`.

### TD-04 — Control structure (group C)

**Definition.** The shape of the task's decision logic.

**Values.** `linear` (fixed step sequence); `conditional-branching` (the step
path depends on observed state — thresholds, gates, validation outcomes, error
paths that change what to do next, not merely retry).

**Placement.** Ask: "can two legitimate runs of this task take different step
paths?" No → `linear`. Yes (e.g., approve under $50k, escalate above; story
publishes to licensed channels only) → `conditional-branching`. Plain
retry-the-same-step is recovery (TD-11), not branching.

### TD-05 — Concurrency (group C)

**Definition.** Whether the task's goal-critical steps must overlap in time or
may be strictly ordered.

**Values.** `sequential` (any serialization is acceptable); `parallel-fanout`
(at least two goal-critical steps must run concurrently — parallel batch work,
concurrent multi-actor operations, racing producers).

**Placement.** Ask: "does the goal still hold if every step is strictly
serialized?" Yes → `sequential`. If serialization defeats the goal (deadline
fan-out, concurrency races that must be handled as they occur, true parallel
data collection) → `parallel-fanout`. Performance-only parallelism (faster but
serializable) stays `sequential`; record the performance note separately.

### TD-06 — Application boundary (group D)

**Definition.** Whether the goal is achieved inside one application's authority
boundary or requires crossing between applications (distinct products,
instances, tenants, or origin-isolated web apps).

**Values.** `single-application` (all goal-critical state changes belong to one
application/instance); `cross-application` (goal-critical state spans ≥2
applications — browser app + desktop app, two web apps on different origins,
web app + terminal service).

**Placement.** Ask: "must data/authority cross from one application to another
for the goal to hold?" Crossing only for diagnostics (post-hoc evidence pulls)
does not count. Different origins on different localhost ports count as
different applications (the fixture apps are separate origins by design).

### TD-07 — Data shape (group E)

**Definition.** The shape of the data the task must consume or produce to
satisfy the goal.

**Values.** `structured` (tables, forms, JSON/CSV, database-shaped records);
`unstructured-text` (prose, documents, logs-as-text, email bodies);
`unstructured-visual-media` (images, screenshots, video, rendered visual state
requiring looking); `mixed {set}` (≥2, enumerated).

**Placement.** Classify by the data whose correctness the success criterion
checks. A pipeline reading JSON and writing a formatted report the criterion
inspects as text is `mixed {structured, unstructured-text}`.

### TD-08 — Transfer operations (group E)

**Definition.** Required movement of data across the local/remote boundary via
the user-visible transfer primitives (the VWO-011 bullet "uploads/downloads").

**Values.** `none`; `upload` (local→remote via a file-picker/form/API
multipart); `download` (remote→local producing a file the later steps or the
criterion depend on); `upload+download`.

**Placement.** Ask: "does the goal require a file to cross the boundary through
a user-level transfer action (not merely an API call returning JSON)?" Clipboard
copy/paste is TD-02 (`gui`), not TD-08; API JSON responses are
`api-tool-call`, not `download`. TD-08 fires when a *file artifact* is the
carrier.

### TD-09 — Human involvement (group F)

**Definition.** Whether a human must participate **during execution** for the
goal to be met (as opposed to authoring, which is TD-13).

**Values.** `unattended` (no human needed after launch); `human-in-the-loop`
(the workflow must legitimately obtain an approval, decision, or input mid-run,
and the goal is not achievable without it).

**Placement.** Ask: "if every human is unreachable while it runs, can it
finish?" Yes → `unattended`. No — a gate, judgment call, or credential handover
is semantically required → `human-in-the-loop`. Human intervention caused by
defects is a failure classification (SCORING.md §4/§6), never a TD-09 value.

### TD-10 — Credential boundary (group F)

**Definition.** The trust-boundary requirements the task imposes: proving
identity and holding rights.

**Values.** `none` (public/anonymous); `authentication` (must prove identity —
login, session, token); `authorization` (must hold a permission/entitlement —
role, license, scope); `authentication+authorization` (both).

**Placement.** Ask: "what does the environment check before the goal-critical
writes are allowed?" Login-only → `authentication`; logged-in-but-role-gated
(approval rights, entitlement) → `authorization` or the combination. The
credential *acquisition* path (approved resource bindings) is a capability
requirement recorded per task in the catalog, not a dimension value.

### TD-11 — Failure/adaptation requirement (group G)

**Definition.** What the task DEMANDS of the executor when the world does not
cooperate (the VWO-011 bullet "recovery/adaptation"; the north-star test model's
recovery/adaptation and changed-but-equivalent-state stages).

**Values.** `happy-path-only` (success requires only the sunny-day sequence);
`recovery-required` (a controlled failure is part of the task — injected error,
transient outage, crash/restart — and the goal must still be met by recovering);
`adaptation-required` (the task must run a second time on changed but
semantically equivalent state and still succeed — different IDs, dates, record
orderings, equivalent UI layout); `recovery+adaptation` (both).

**Placement.** Authored into the task by design: the catalog states which
failure/adaptation is exercised and the success criteria include the
recovered/adapted end state. A task that merely *permits* recovery without
requiring it is `happy-path-only`; VWO-015/VWO-016 tasks carry the stronger
values by construction.

### TD-12 — Reversibility (group G)

**Definition.** The blast-radius class of the task's goal-critical operations.

**Values.** `reversible` (every operation can be undone or reset to an
equivalent prior state); `destructive-irreversible` (at least one goal-critical
operation destroys state that cannot be reconstructed — deletes, publishes
immutably, sends externally, spends). The value drives guard requirements:
destructive-irreversible tasks MUST include a confirmation/gate criterion.

**Placement.** Ask: "if the executor performs the destructive step wrongly, can
the prior world be restored?" Fixture demo worlds can be reseeded, but the
*semantic* irreversibility is judged in-world: publishing a story that external
channels consumed is irreversible in-world even though the fixture can reseed.

### TD-13 — Workflow provenance (group H)

**Definition.** Who authored the task and what the executor could know in
advance — the generalization axis (VWO-016's unseen-goal method; the VWO-011
bullet "tasks with no predefined workflow supplied to the agent").

**Values.** `scripted` (validation-team-authored with a known expected action
path — the wave-1/2 catalog model); `human-authored` (authored from a human
goal statement by a person, with NO action script or expected step sequence
supplied — the VWO-013 "authored from a human goal" class); `held-out` (authored
by independent authors, sealed and unseen by implementers/executors until
execution time — NST-H selection procedure, `BENCHMARK-CATALOG.md` §7).

**Placement.** Determined by authoring process, not content. The anti-cheating
rules of VWO-016 bind: no expected action sequence peeking, no workflow IR, no
benchmark-specific planner for any provenance value.

---

## 4. Required-bullet coverage map (VWO-011 → TD)

Every bullet of the VWO-011 "Required task dimensions" list, and where it lives:

| # | VWO-011 required bullet | Dimension(s) | Value(s) |
|---|---|---|---|
| 1 | browser | TD-01 | `browser` (interaction face: TD-02 `browser-ui`) |
| 2 | desktop GUI | TD-01 | `desktop-gui` (interaction face: TD-02 `gui`) |
| 3 | terminal/CLI | TD-01 | `terminal-cli` (interaction face: TD-02 `terminal`) |
| 4 | filesystem/document manipulation | TD-01 | `filesystem` (interaction face: TD-02 `file-op`) |
| 5 | API/tool/MCP | TD-01 | `api-tool-mcp` (interaction face: TD-02 `api-tool-call`) |
| 6 | human-in-the-loop | TD-09 (+ TD-02 `human`) | `human-in-the-loop` |
| 7 | mixed/multi-environment | TD-01 | `mixed` |
| 8 | single-application | TD-06 | `single-application` |
| 9 | cross-application | TD-06 | `cross-application` |
| 10 | structured data | TD-07 | `structured` |
| 11 | unstructured text | TD-07 | `unstructured-text` |
| 12 | unstructured visual/media data | TD-07 | `unstructured-visual-media` |
| 13 | authentication/authorization | TD-10 | all four values |
| 14 | uploads/downloads | TD-08 | `upload` / `download` / `upload+download` |
| 15 | long-running/stateful tasks | TD-03 | `stateful-session`, `long-running-resumable` |
| 16 | parallel tasks | TD-05 | `parallel-fanout` |
| 17 | conditional/branching tasks | TD-04 | `conditional-branching` |
| 18 | recovery/adaptation | TD-11 | `recovery-required`, `adaptation-required`, `recovery+adaptation` |
| 19 | destructive/irreversible operations | TD-12 | `destructive-irreversible` |
| 20 | tasks with no predefined workflow supplied to the agent | TD-13 | `human-authored`, `held-out` |

All twenty bullets are placeable; none is left unmapped. (Self-check #1 in
`SELF-CHECK-VWO-011.md` verifies the catalog exercises each dimension.)

---

## 5. Canonical serialization (machine-checkable)

Every benchmark task record carries this binding block (consumed by the
self-checks and by the coverage matrices' aggregation):

```yaml
nst-id: NST-<lane-letter><3 digits>
lane: desktop | browser | terminal | cross-application | held-out
executing-wo: VWO-012 | VWO-013 | VWO-014 | VWO-015 | VWO-016
taxonomy:
  td-01-environment-modality: <value>          # mixed ⇒ mixed {a, b, …}
  td-02-interaction-modality: <value>
  td-03-statefulness: <value>
  td-04-control-structure: <value>
  td-05-concurrency: <value>
  td-06-application-boundary: <value>
  td-07-data-shape: <value>                    # mixed ⇒ mixed {a, b}
  td-08-transfer-operations: <value>
  td-09-human-involvement: <value>
  td-10-credential-boundary: <value>
  td-11-failure-adaptation: <value>
  td-12-reversibility: <value>
  td-13-workflow-provenance: <value>
```

An `unmodeled` value is never valid in a committed record — a placement gap is
an issue for the Tech Lead and a MAJOR version bump, not a silent value.

---

## 6. Placement worked examples

1. **Wave-2's `adv-session-loss-restart`** (SIGKILL mid-workflow, restart,
   idempotent replay) placed on the taxonomy: TD-01 `browser` (fixture world),
   TD-02 `browser-ui`, TD-03 `long-running-resumable`, TD-04 `linear`,
   TD-05 `sequential`, TD-06 `single-application`, TD-07 `structured`,
   TD-08 `none`, TD-09 `unattended`, TD-10 `authentication+authorization`,
   TD-11 `recovery-required`, TD-12 `reversible`, TD-13 `scripted`.
   This is the classification grammar the wave-1/2 evidence already speaks.
2. **A hypothetical "fill the site report twice, second day different data"**:
   TD-11 `adaptation-required` (changed-but-equivalent state), everything else
   as above — the north-star README's decisive second-run test.
3. **"Export the incident list from the browser, summarize it in the terminal,
   and have a manager approve the summary in the app before it is posted"**:
   TD-01 `mixed {browser, terminal-cli, api-tool-mcp}`, TD-02 `mixed
   {browser-ui, terminal, api-tool-call, human}`, TD-06 `cross-application`,
   TD-09 `human-in-the-loop`, TD-13 per authoring process.

---

## 7. What this taxonomy does NOT claim

- It does not enumerate "all computer tasks" — it is a measurement grid over a
  task space the north-star README explicitly treats as unbounded (claim
  boundary: `docs/validation/north-star/README.md` §Universal claim boundary;
  VALIDATION-PROGRAM.md §12 final paragraph).
- It does not encode difficulty (difficulty is a benchmark-catalog composition
  metric over these dimensions — `BENCHMARK-CATALOG.md` §4).
- It does not encode product capabilities; a task's required capabilities are
  recorded per task in the catalog and may legitimately exceed what the product
  currently mounts (blocked outcomes are honest results, never reasons to
  narrow the task).
- It changes only through the ADD-NEW-FAMILY PROCEDURE / benchmark version
  discipline (`COVERAGE-MATRIX.md` §5, `SCORING.md` §8) — never ad hoc.
