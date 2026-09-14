# VWO-011 — North-Star Benchmark Catalog

**Work Order:** VWO-011 (Wave 4)
**Status:** CANONICAL — VWO-012 … VWO-016 execute tasks from this catalog
(deviations recorded, never silent). The classification grammar is
`TASK-TAXONOMY.md` (TD-01 … TD-13); the measurement rules are `SCORING.md`;
the state of the world against this catalog is `COVERAGE-MATRIX.md`.
**Governing sources:** `docs/validation/north-star/README.md` ·
`docs/validation/VALIDATION-PROGRAM.md` §7 ·
`docs/validation/work-orders/VWO-011.md` (Required benchmark artifacts) ·
`docs/validation/work-orders/VWO-012.md` … `VWO-016.md` (executing lanes).

---

## 1. Benchmark version

```yaml
benchmark_version: 1.0.0
catalog_status: canonical
established_by: VWO-011
base_sha: 2d37cc0ef7d075e877f370790f82ac9c70df26a4   # main HEAD as of dispatch
```

Every executed task run MUST record `benchmark_version` (SCORING.md §6/§8).
Comparing runs across versions is invalid without re-execution.

### Changelog

| Version | Date (UTC) | Change | Author |
|---|---|---|---|
| 1.0.0 | 2026-09-13 | Initial catalog: taxonomy v1 (13 dimensions); 38 enumerated tasks — NST-D ×10, NST-B ×10, NST-T ×10, NST-X ×8 — plus the NST-H held-out selection procedure (procedure only; zero held-out tasks enumerated by design); difficulty scale L1–L4; lane→WO binding (VWO-012/013/014/015/016). | VWO-011 |

Version discipline (no-regression rule): see `SCORING.md` §8 — semantic
changes (success criteria, difficulty definitions, taxonomy value sets, mode
bindings, task removal) require a MAJOR bump; additive tasks within existing
semantics are MINOR; editorial fixes are PATCH. The catalog never rewrites
history: superseded tasks keep their ID with a `deprecated:` marker.

---

## 2. ID scheme and lane rules

**ID scheme.** `NST-<lane-letter><###>` — three digits, zero-padded, monotonic
per lane, never reused, never renumbered:

- `NST-D###` — desktop lane (primary interaction: native GUI) — executing WO **VWO-012**
- `NST-B###` — browser lane (primary interaction: browser UI) — executing WO **VWO-013**
- `NST-T###` — terminal/filesystem/developer lane (primary interaction: terminal, file, CLI, developer tooling; includes API-from-terminal) — executing WO **VWO-014**
- `NST-X###` — cross-application/multi-environment lane (goal-critical steps genuinely balanced across ≥2 environments) — executing WO **VWO-015**
- `NST-H###` — held-out lane (**selection procedure only, §7 — task IDs are reserved but NEVER enumerated in this catalog; enumeration before execution would defeat holding them out**)

**Lane placement rule** (normative; refines TASK-TAXONOMY.md §2 rule 3): the
lane is determined by (a) the primary interaction modality of the task's
goal-critical steps (TD-02 dominant value), and (b) which wave-4 Work Order's
required task families the task exercises. When goal-critical steps are
balanced across ≥2 environments with no dominant interaction surface, the task
is NST-X. Every task maps to exactly one lane and exactly one executing WO —
verifiable mechanically from the binding blocks (SELF-CHECK #2).

---

## 3. Task record format (normative, machine-checkable)

Every enumerated task carries a fenced binding block followed by required
prose fields. The binding block mirrors the scenario catalog's convention
(`docs/validation/scenarios/SCENARIO-CATALOG.md` §3):

```yaml
nst-id: NST-<lane><###>
lane: desktop | browser | terminal | cross-application | held-out
executing-wo: VWO-012 | VWO-013 | VWO-014 | VWO-015 | VWO-016
difficulty: L1 | L2 | L3 | L4
provenance: scripted | human-authored | held-out
teaching-mode: DEMONSTRATE | INSTRUCT | HYBRID      # canonical mode (see §11)
taxonomy:
  td-01-environment-modality: <value>
  td-02-interaction-modality: <value>
  td-03-statefulness: <value>
  td-04-control-structure: <value>
  td-05-concurrency: <value>
  td-06-application-boundary: <value>
  td-07-data-shape: <value>
  td-08-transfer-operations: <value>
  td-09-human-involvement: <value>
  td-10-credential-boundary: <value>
  td-11-failure-adaptation: <value>
  td-12-reversibility: <value>
  td-13-workflow-provenance: <value>                # equals `provenance`
```

Prose fields per task (all required):

- **Expected user goal** — plain human terms, as the person would state it.
- **Observable success criteria** — numbered; each tagged `[C1]`
  (machine-checkable) or `[C2]` (inspector-checkable with a pre-stated
  oracle). NEVER model text (SCORING.md §2).
- **Required environments/capabilities/resources** — substrates,
  applications/personas, product capabilities the workflow must bind.
- **Difficulty justification** — one line tying the level to the §4 definition.

---

## 4. Difficulty scale (definition + values)

Difficulty is a composition metric over taxonomy stressors, not a separate
opinion. **Stressor set S (10):** {conditional-branching (TD-04),
parallel-fanout (TD-05), cross-application (TD-06), mixed data shapes (TD-07
`mixed`), transfer operations (TD-08 ≠ none), human-in-the-loop (TD-09),
any credential boundary (TD-10 ≠ none), recovery or adaptation (TD-11 ≠
happy-path-only), destructive-irreversible (TD-12), long-running-resumable
(TD-03)}.

**Hard stressors** (each alone floors a task at L3): recovery-required,
adaptation-required, destructive-irreversible, and dynamic/ambiguous-UI
interpretation (waiting on changing rendered state, conditional reveals,
skeleton loaders).

**Frontier triggers** (each alone raises a task to L4): long-running-
resumable (TD-03), parallel-fanout (TD-05), TD-01 `mixed` set of size ≥3,
held-out provenance (TD-13).

| Level | Definition | Placement rule |
|---|---|---|
| **L1 Routine** | Zero stressors: single environment, linear, happy-path-only, no boundary crossings, no credentials. | \|S\| = 0 |
| **L2 Moderate** | Exactly one stressor. | \|S\| = 1 |
| **L3 Hard** | Two or three stressors, OR any single hard stressor. | \|S\| ∈ {2,3}, or a hard stressor present |
| **L4 Frontier** | Four or more stressors, OR any frontier trigger. | \|S\| ≥ 4, or a frontier trigger present |

Held-out tasks are always L4 (frontier trigger: held-out provenance).

---

## 5. Catalog-wide binding notes (apply to every task)

1. **Proving ground.** The fallback sandbox profile per VWO-001
   (`docs/validation/proving-ground/README.md` §1–2) is the environment of
   record: Xvfb virtual framebuffer + xdotool (user-space) + ffmpeg x11grab
   for the desktop lane; agent-browser 0.35.0 / Playwright chromium for the
   browser lane; bash + node/bun/python for the terminal lane; the five
   VWO-002 fixture apps on 127.0.0.1:4101–4105; operator chat channel as the
   human gate (reduced fidelity, recorded). E2B/Composio desktop pool:
   ABSENT — out-of-scope-v1 with the VWO-001 fallback record.
2. **Fixture personas/credentials.** Seeded demo personas per
   `docs/validation/scenarios/SCENARIO-CATALOG.md` (persona tables on login
   pages; demo hints via `/api/demo-hints`). No real credentials anywhere;
   secrets discipline per REPORT-SCHEMA.md §9 and `evidence/README.md`.
3. **Product-mount status (Family A).** At catalog-establishment base
   `2d37cc0`, the workflow/teaching engine is mounted in no user-facing
   surface (canonical Family A, SCENARIO-ISSUE-MATRIX.md §2). Catalog tasks
   define required PRODUCT capabilities (what a normal user must be able to
   teach/run). If the mount is still absent at execution time, the honest
   outcome is `BLOCKED — surface missing` (SCORING.md §4), which feeds the
   remediation loop (RWO-001). Tasks are NEVER narrowed to fit current
   surfaces.
4. **Purpose-built fixture policy.** Where a lane needs a surface no fixture
   provides (e.g., a rich web editor, a download endpoint, a GUI table app),
   the executing WO may build a small purpose-built fixture per
   VALIDATION-PROGRAM.md §4 ("These applications may be purpose-built for the
   proving ground"). A purpose-built fixture is application tooling, NEVER a
   workflow engine or benchmark runtime (VWO-011 Forbidden: no second
   workflow engine; VWO-014 acceptance: no validation-only shell orchestrator
   becoming a second runtime).
5. **Failure switches.** The seven canonical VWO-002 switches
   (`service_unavailable`, `notification_failure`, `missing_asset`,
   `permission_denied`, `data_conflict`, `duplicate_event`,
   `stale_entitlement`) arm controlled failures via `?failure=<switch>` /
   `X-Failure-Switch` header.
6. **Reseed discipline.** `reset-scenario.sh <family>` between tasks;
   deterministic fixture clock `state.meta.today = 2026-09-12`.
7. **Evidence discipline.** Every executed run records the full
   SCORING.md §6 field set; evidence lands under
   `docs/validation/evidence/<VWO-ID>/<nst-id>/user-path|post-hoc/`.

---

## 6. Lane catalogs

### 6.1 NST-D — native desktop GUI lane (executing WO: VWO-012; 10 tasks)

#### NST-D001 — Keyboard-only daily report entry

```yaml
nst-id: NST-D001
lane: desktop
executing-wo: VWO-012
difficulty: L2
provenance: scripted
teaching-mode: DEMONSTRATE
taxonomy:
  td-01-environment-modality: browser            # world state = SiteBuild app
  td-02-interaction-modality: gui                # X11-level keystrokes into the window
  td-03-statefulness: stateful-session
  td-04-control-structure: linear
  td-05-concurrency: sequential
  td-06-application-boundary: single-application
  td-07-data-shape: structured
  td-08-transfer-operations: none
  td-09-human-involvement: unattended
  td-10-credential-boundary: authentication
  td-11-failure-adaptation: happy-path-only
  td-12-reversibility: reversible
  td-13-workflow-provenance: scripted
```

- **Expected user goal:** "Log into the site portal as the foreman and file
  today's progress report for Harborview Tower — without touching the mouse
  once; keyboard only."
- **Observable success criteria:**
  1. `[C1]` `GET /api/progress?projectId=p-101` (post-hoc) returns a new
     report for the fixture day containing exactly the typed percent/tasks/
     notes values.
  2. `[C1]` Project percent on `GET /api/projects/p-101` moved to the typed
     value.
  3. `[C2]` A captured screenshot (ffmpeg x11grab) shows the post-submit
     confirmation flash and report row (oracle: the flash text naming the
     project and date).
  4. `[C2]` The xdotool/xev input transcript for the run contains key events
     and zero pointer motion/click events (oracle: event-type counts).
- **Required environments/capabilities/resources:** Xvfb display; GUI-mode
  chromium launched into the framebuffer; xdotool keyboard injection;
  SiteBuild (port 4101) persona `marco.silva` (password via
  `/api/demo-hints`); product capabilities: computer-use (keyboard), screen
  observation, credential binding for `marco.silva`.
- **Difficulty justification:** one stressor (credential boundary) → L2.

#### NST-D002 — Mouse-only hero-photo selection

```yaml
nst-id: NST-D002
lane: desktop
executing-wo: VWO-012
difficulty: L3
provenance: human-authored
teaching-mode: INSTRUCT
taxonomy:
  td-01-environment-modality: browser
  td-02-interaction-modality: gui
  td-03-statefulness: stateful-session
  td-04-control-structure: linear
  td-05-concurrency: sequential
  td-06-application-boundary: single-application
  td-07-data-shape: unstructured-visual-media
  td-08-transfer-operations: none
  td-09-human-involvement: unattended
  td-10-credential-boundary: authorization       # asset/attach permissions
  td-11-failure-adaptation: adaptation-required  # second run, different story+asset
  td-12-reversibility: reversible
  td-13-workflow-provenance: human-authored
```

- **Expected user goal:** "Pick the new hero photo for the draft story using
  only the mouse — scroll down the asset gallery, click the right picture,
  attach it as the hero. Then do it again for a different story with a
  different photo." (No action script is supplied — the executor is given
  this goal text only.)
- **Observable success criteria:**
  1. `[C1]` `GET /api/stories/<id>` shows `heroAssetId` equal to the asset
     the goal pinned (by name) for run 1; a second story on run 2 likewise.
  2. `[C2]` Screenshot evidence of the gallery scrolled and the chosen asset
     highlighted (oracle: asset filename visible in the story page hero
     slot).
  3. `[C2]` Input transcript shows pointer events and zero key events.
- **Required environments/capabilities/resources:** Xvfb + GUI-mode chromium;
  xdotool mouse (move/click/scroll); PressRoom (4104) persona with
  `asset:upload`/attach rights (e.g., editor per seed role table); product
  capabilities: computer-use (pointer), visual observation.
- **Difficulty justification:** two stressors (adaptation-required [hard]
  + authorization) → L3.

#### NST-D003 — Two-window clipboard transfer

```yaml
nst-id: NST-D003
lane: desktop
executing-wo: VWO-012
difficulty: L2
provenance: scripted
teaching-mode: HYBRID
taxonomy:
  td-01-environment-modality: mixed {browser, filesystem}
  td-02-interaction-modality: gui
  td-03-statefulness: stateful-session
  td-04-control-structure: linear
  td-05-concurrency: sequential
  td-06-application-boundary: cross-application
  td-07-data-shape: structured
  td-08-transfer-operations: none                # clipboard, not file transfer
  td-09-human-involvement: unattended
  td-10-credential-boundary: authentication
  td-11-failure-adaptation: happy-path-only
  td-12-reversibility: reversible
  td-13-workflow-provenance: scripted
```

- **Expected user goal:** "Open the ops dashboard and a text editor side by
  side. Copy this week's per-region trip numbers out of the dashboard table
  and paste them into a new note document, then save the note."
- **Observable success criteria:**
  1. `[C1]` The saved note file exists at the required path and its contents
     contain exactly the dashboard's per-region numbers (diff against a
     post-hoc `GET /api/trips/regions` pull).
  2. `[C2]` Screenshots show both windows mapped, focus switching between
     them, and the paste landing (oracle: pasted digits visible in the
     editor buffer before save).
  3. `[C1]` Clipboard transcript (xclip/xsel dump) captured at paste time
     matches the numbers.
- **Required environments/capabilities/resources:** Xvfb; two GUI windows
  (GUI-mode chromium on RidePilot ops 4103; a user-space GUI text editor per
  VWO-001 §3.3, or purpose-built GUI fixture per §5 note 4); clipboard
  selection mechanism; product capabilities: computer-use (window focus,
  clipboard), filesystem write.
- **Difficulty justification:** one stressor (cross-application) → L2.

#### NST-D004 — Menus, dialogs and transient warnings

```yaml
nst-id: NST-D004
lane: desktop
executing-wo: VWO-012
difficulty: L2
provenance: scripted
teaching-mode: DEMONSTRATE
taxonomy:
  td-01-environment-modality: filesystem
  td-02-interaction-modality: gui
  td-03-statefulness: stateful-session
  td-04-control-structure: conditional-branching
  td-05-concurrency: sequential
  td-06-application-boundary: single-application
  td-07-data-shape: unstructured-text
  td-08-transfer-operations: none
  td-09-human-involvement: unattended
  td-10-credential-boundary: none
  td-11-failure-adaptation: happy-path-only
  td-12-reversibility: reversible
  td-13-workflow-provenance: scripted
```

- **Expected user goal:** "In the editor's File menu, save the open note as
  a new file with a specific name; then try to close the window and, when
  the unsaved-changes warning appears, dismiss it without saving."
- **Observable success criteria:**
  1. `[C1]` The named file exists on disk with the buffer's exact contents
     (byte compare against the intended text).
  2. `[C2]` Screenshots or window-tree dumps show: File menu opened; Save-As
     dialog completed; transient close-confirmation dialog appeared and was
     dismissed (oracle: dialog window titles).
  3. `[C1]` The editor process exits cleanly after dismissal (exit code 0).
- **Required environments/capabilities/resources:** user-space GUI editor
  (VWO-001 §3.3 acquisition path) or purpose-built GUI fixture; Xvfb;
  xdotool menu/dialog traversal; product capabilities: computer-use (menus,
  dialogs), filesystem write.
- **Difficulty justification:** one stressor (conditional-branching — the
  close path branches on the warning) → L2.

#### NST-D005 — File-picker upload and save-as download

```yaml
nst-id: NST-D005
lane: desktop
executing-wo: VWO-012
difficulty: L3
provenance: scripted
teaching-mode: HYBRID
taxonomy:
  td-01-environment-modality: mixed {browser, filesystem}
  td-02-interaction-modality: mixed {gui, browser-ui}
  td-03-statefulness: stateful-session
  td-04-control-structure: linear
  td-05-concurrency: sequential
  td-06-application-boundary: cross-application
  td-07-data-shape: mixed {unstructured-visual-media, structured}
  td-08-transfer-operations: upload+download
  td-09-human-involvement: unattended
  td-10-credential-boundary: authentication+authorization
  td-11-failure-adaptation: happy-path-only
  td-12-reversibility: reversible
  td-13-workflow-provenance: scripted
```

- **Expected user goal:** "Upload today's site photo to the project page by
  picking it through the file chooser; then export the project's report as a
  file and save it into the weekly folder through the save dialog."
- **Observable success criteria:**
  1. `[C1]` `GET /api/assets/<new-id>` (post-hoc) shows the uploaded photo
     record (name + checksum) created by the picker-driven upload.
  2. `[C1]` The exported report file exists in the target folder with
     non-trivial size and stable checksum across a re-pull.
  3. `[C2]` Screenshots show the native file-open and file-save dialogs
     driving both transfers (oracle: dialog titles + chosen filename).
- **Required environments/capabilities/resources:** Xvfb + GUI-mode chromium;
  native file dialogs (chromium GUI-mode chooser; purpose-built fixture
  fallback recorded if the headless-profile chooser is unavailable); SiteBuild
  (4101) `marco.silva` (progress:create/asset upload) and a download surface
  (purpose-built fixture endpoint serving a report file, or fixture static
  route); product capabilities: computer-use (dialogs), browser-use, file
  transfer.
- **Difficulty justification:** three stressors (transfer operations +
  credential boundary + cross-application) → L3.

#### NST-D006 — GUI file management (create/rename/move/copy/delete)

```yaml
nst-id: NST-D006
lane: desktop
executing-wo: VWO-012
difficulty: L3
provenance: scripted
teaching-mode: INSTRUCT
taxonomy:
  td-01-environment-modality: filesystem
  td-02-interaction-modality: gui
  td-03-statefulness: stateless
  td-04-control-structure: conditional-branching
  td-05-concurrency: sequential
  td-06-application-boundary: single-application
  td-07-data-shape: unstructured-text
  td-08-transfer-operations: none
  td-09-human-involvement: unattended
  td-10-credential-boundary: none
  td-11-failure-adaptation: happy-path-only
  td-12-reversibility: destructive-irreversible
  td-13-workflow-provenance: scripted
```

- **Expected user goal:** "Tidy the field-notes folder: create a new
  'archive' folder, move last week's note files into it, make one copy as a
  backup, rename the copy, and delete the original scratch file."
- **Observable success criteria:**
  1. `[C1]` Final tree state matches the expected layout (files in archive/,
     renamed backup present, scratch file gone) — `find` output equality.
  2. `[C2]` A confirmation/undo gate is observed before the delete (oracle:
     screenshot or dialog dump of the confirm step; the guard requirement
     for TD-12 destructive tasks).
  3. `[C1]` The deleted file's contents are preserved in the renamed backup
     (byte equality).
- **Required environments/capabilities/resources:** GUI file-management
  surface (user-space file manager or purpose-built GUI fixture); Xvfb;
  xdotool; a seeded scratch tree; product capabilities: computer-use,
  filesystem CRUD.
- **Difficulty justification:** two stressors (destructive-irreversible
  [hard] + conditional-branching) → L3.

#### NST-D007 — Document editing end-to-end

```yaml
nst-id: NST-D007
lane: desktop
executing-wo: VWO-012
difficulty: L1
provenance: human-authored
teaching-mode: HYBRID
taxonomy:
  td-01-environment-modality: filesystem
  td-02-interaction-modality: gui
  td-03-statefulness: stateful-session
  td-04-control-structure: linear
  td-05-concurrency: sequential
  td-06-application-boundary: single-application
  td-07-data-shape: unstructured-text
  td-08-transfer-operations: none
  td-09-human-involvement: unattended
  td-10-credential-boundary: none
  td-11-failure-adaptation: happy-path-only
  td-12-reversibility: reversible
  td-13-workflow-provenance: human-authored
```

- **Expected user goal:** "Write up the safety briefing as a one-page
  document in the editor — title, three bullet points, today's date — and
  save it where the team can find it." (Goal text only; no script.)
- **Observable success criteria:**
  1. `[C1]` Saved file exists at the expected path, contains the title, ≥3
     bullet lines and the fixture date (2026-09-12).
  2. `[C2]` Screenshots of the editing session show typing in progress
     (oracle: buffer content at save time).
- **Required environments/capabilities/resources:** user-space GUI editor or
  purpose-built GUI fixture; Xvfb; product capabilities: computer-use
  (keyboard), text composition, filesystem write.
- **Difficulty justification:** zero stressors (single surface, linear,
  uncredentialled, happy path) → L1.

#### NST-D008 — Spreadsheet/table manipulation

```yaml
nst-id: NST-D008
lane: desktop
executing-wo: VWO-012
difficulty: L1
provenance: scripted
teaching-mode: INSTRUCT
taxonomy:
  td-01-environment-modality: filesystem
  td-02-interaction-modality: gui
  td-03-statefulness: stateful-session
  td-04-control-structure: linear
  td-05-concurrency: sequential
  td-06-application-boundary: single-application
  td-07-data-shape: structured
  td-08-transfer-operations: none
  td-09-human-involvement: unattended
  td-10-credential-boundary: none
  td-11-failure-adaptation: happy-path-only
  td-12-reversibility: reversible
  td-13-workflow-provenance: scripted
```

- **Expected user goal:** "Take the empty expense sheet, fill in five rows
  from the receipt list, add a total formula in the summary cell, and export
  the sheet as CSV."
- **Observable success criteria:**
  1. `[C1]` Exported CSV contains the five rows with correct values and a
     final row/field whose value equals the sum (parsed and checked).
  2. `[C2]` Screenshots show cell selection/editing across rows (oracle:
     formula bar content).
- **Required environments/capabilities/resources:** spreadsheet-capable GUI
  application where installable (user-space), else purpose-built GUI table
  fixture (recorded as reduced fidelity); Xvfb; product capabilities:
  computer-use, structured cell editing, export.
- **Difficulty justification:** zero stressors per the §4 set (structured
  data and session state are not stressors) → L1.

#### NST-D009 — GUI media manipulation with visual confirmation

```yaml
nst-id: NST-D009
lane: desktop
executing-wo: VWO-012
difficulty: L1
provenance: human-authored
teaching-mode: HYBRID
taxonomy:
  td-01-environment-modality: filesystem
  td-02-interaction-modality: mixed {gui, file-op}
  td-03-statefulness: stateless
  td-04-control-structure: linear
  td-05-concurrency: sequential
  td-06-application-boundary: single-application
  td-07-data-shape: unstructured-visual-media
  td-08-transfer-operations: none
  td-09-human-involvement: unattended
  td-10-credential-boundary: none
  td-11-failure-adaptation: happy-path-only
  td-12-reversibility: reversible
  td-13-workflow-provenance: human-authored
```

- **Expected user goal:** "Resize the four site photos to fit the report
  template and rotate the sideways one; confirm each result on screen before
  saving over the originals." (Goal text only.)
- **Observable success criteria:**
  1. `[C1]` Output image dimensions/rotation match the template spec
     (machine parse of image headers).
  2. `[C2]` On-screen visual confirmation step evidenced per image (oracle:
     before/after screenshots; visual confirmation is part of the task, per
     VWO-012's screenshots/visual-confirmation family).
- **Required environments/capabilities/resources:** GUI image tool where
  installable (user-space), else purpose-built GUI fixture wrapping a
  deterministic image backend; Xvfb; seeded images; product capabilities:
  computer-use, visual observation, file overwrite.
- **Difficulty justification:** zero stressors per the §4 set (visual-media
  data and GUI-only interaction are not stressors) → L1.

#### NST-D010 — Crash mid-edit and recovery

```yaml
nst-id: NST-D010
lane: desktop
executing-wo: VWO-012
difficulty: L3
provenance: scripted
teaching-mode: DEMONSTRATE
taxonomy:
  td-01-environment-modality: mixed {filesystem, desktop-gui}
  td-02-interaction-modality: gui
  td-03-statefulness: stateful-session
  td-04-control-structure: linear
  td-05-concurrency: sequential
  td-06-application-boundary: single-application
  td-07-data-shape: unstructured-text
  td-08-transfer-operations: none
  td-09-human-involvement: unattended
  td-10-credential-boundary: none
  td-11-failure-adaptation: recovery-required    # SIGKILL mid-edit, restart, finish
  td-12-reversibility: reversible
  td-13-workflow-provenance: scripted
```

- **Expected user goal:** "Halfway through typing the incident note the
  editor is killed without warning. Reopen the editor, get back the draft
  (autosave or recovery), finish the note, and save it."
- **Observable success criteria:**
  1. `[C1]` `kill -9` of the editor process occurs mid-buffer (transcript
     timestamped before save).
  2. `[C1]` Final saved file contains the complete note including the
     pre-crash portion (byte compare against the intended full text).
  3. `[C2]` Post-restart screenshot shows the recovered draft state (oracle:
     recovered buffer content visible).
  4. `[C2]` No duplicate final artifacts exist (exactly one saved file).
- **Required environments/capabilities/resources:** GUI editor with
  autosave/crash-recovery or purpose-built fixture with the same property;
  process control (kill/restart); Xvfb; product capabilities: computer-use,
  process recovery, durable draft state.
- **Difficulty justification:** recovery-required is a hard stressor → L3
  floor.

### 6.2 NST-B — browser lane (executing WO: VWO-013; 10 tasks)

#### NST-B001 — Multi-tab cross-application journey

```yaml
nst-id: NST-B001
lane: browser
executing-wo: VWO-013
difficulty: L3
provenance: scripted
teaching-mode: HYBRID
taxonomy:
  td-01-environment-modality: browser
  td-02-interaction-modality: browser-ui
  td-03-statefulness: stateful-session
  td-04-control-structure: linear
  td-05-concurrency: sequential
  td-06-application-boundary: cross-application   # two origins (ports) = two apps
  td-07-data-shape: structured
  td-08-transfer-operations: none
  td-09-human-involvement: unattended
  td-10-credential-boundary: authentication+authorization
  td-11-failure-adaptation: happy-path-only
  td-12-reversibility: reversible
  td-13-workflow-provenance: scripted
```

- **Expected user goal:** "Keep two tabs open — the ride-share ops dashboard
  and the marketplace catalog — and cross-check this week's busiest region
  name against the workflow listing that mentions it, then request the
  matching install on the marketplace tab."
- **Observable success criteria:**
  1. `[C1]` Both tabs retain logged-in sessions through the whole journey
     (session cookies live; API pulls confirm identity per tab).
  2. `[C1]` The FlowMart install record for the chosen listing exists with
     the requesting persona as actor (post-hoc API pull).
  3. `[C2]` Tab inventory snapshot shows two concurrent tabs with the two
     apps (oracle: tab list + URLs).
- **Required environments/capabilities/resources:** agent-browser
  multi-tab; RidePilot (4103) ops persona; FlowMart (4105) buyer persona;
  product capabilities: browser-use (tabs, sessions), cross-app data carry.
- **Difficulty justification:** two stressors (cross-application +
  authentication+authorization) → L3.

#### NST-B002 — Dynamic form with validation and ambiguous states

```yaml
nst-id: NST-B002
lane: browser
executing-wo: VWO-013
difficulty: L3
provenance: human-authored
teaching-mode: INSTRUCT
taxonomy:
  td-01-environment-modality: browser
  td-02-interaction-modality: browser-ui
  td-03-statefulness: stateful-session
  td-04-control-structure: conditional-branching
  td-05-concurrency: sequential
  td-06-application-boundary: single-application
  td-07-data-shape: structured
  td-08-transfer-operations: none
  td-09-human-involvement: unattended
  td-10-credential-boundary: none
  td-11-failure-adaptation: happy-path-only
  td-12-reversibility: reversible
  td-13-workflow-provenance: human-authored
```

- **Expected user goal:** "Submit the vendor registration form correctly:
  it hides the license field until you pick 'contractor', it rejects an
  invalid tax id inline, and the submit button stays disabled until the
  fields are valid. Get the green confirmation." (Goal text only; the form
  is a purpose-built dynamic fixture.)
- **Observable success criteria:**
  1. `[C1]` The fixture's submissions API records exactly one accepted
     submission with the correct conditional field values, and ≥1 rejected
     attempt (validation actually exercised).
  2. `[C2]` Screenshots/a11y snapshots show the conditional reveal, the
     inline error state, and the disabled→enabled transition (oracle: the
     fixture's documented state classes).
  3. `[C1]` A delayed-loading section (skeleton → content) was awaited, not
     skipped (fixture timing log).
- **Required environments/capabilities/resources:** purpose-built dynamic
  web fixture (conditional fields, client validation, skeleton loaders —
  per §5 note 4); agent-browser; product capabilities: browser-use (dynamic
  UI interpretation, wait-for-state).
- **Difficulty justification:** two stressors (conditional-branching +
  dynamic-UI interpretation hard stressor) → L3.

#### NST-B003 — Table search/filter/pagination sweep

```yaml
nst-id: NST-B003
lane: browser
executing-wo: VWO-013
difficulty: L2
provenance: scripted
teaching-mode: DEMONSTRATE
taxonomy:
  td-01-environment-modality: browser
  td-02-interaction-modality: browser-ui
  td-03-statefulness: stateless
  td-04-control-structure: linear
  td-05-concurrency: sequential
  td-06-application-boundary: single-application
  td-07-data-shape: structured
  td-08-transfer-operations: none
  td-09-human-involvement: unattended
  td-10-credential-boundary: authentication
  td-11-failure-adaptation: happy-path-only
  td-12-reversibility: reversible
  td-13-workflow-provenance: scripted
```

- **Expected user goal:** "In the forge tracker, filter the open PRs to the
  payments repo, page through all results, and post a comment on every PR
  whose CI is failing, saying which check failed."
- **Observable success criteria:**
  1. `[C1]` Every PR matching the filter at run time carries exactly one new
     comment naming its failing check (post-hoc `/api` pull; comment text
     checked against the CI status enum, not free-form).
  2. `[C1]` No PR outside the filter was commented.
  3. `[C2]` Page-traversal evidence shows each results page visited (oracle:
     page snapshots).
- **Required environments/capabilities/resources:** ForgeOps (4102)
  `noor.haddad` or `raj.patel`; agent-browser; product capabilities:
  browser-use (tables, search, pagination loops).
- **Difficulty justification:** one stressor (authentication) → L2.

#### NST-B004 — Upload pipeline with changed-state re-run

```yaml
nst-id: NST-B004
lane: browser
executing-wo: VWO-013
difficulty: L3
provenance: scripted
teaching-mode: DEMONSTRATE
taxonomy:
  td-01-environment-modality: browser
  td-02-interaction-modality: browser-ui
  td-03-statefulness: stateful-session
  td-04-control-structure: linear
  td-05-concurrency: sequential
  td-06-application-boundary: single-application
  td-07-data-shape: mixed {unstructured-visual-media, structured}
  td-08-transfer-operations: upload
  td-09-human-involvement: unattended
  td-10-credential-boundary: authorization
  td-11-failure-adaptation: adaptation-required   # second run: different asset+project
  td-12-reversibility: reversible
  td-13-workflow-provenance: scripted
```

- **Expected user goal:** "Upload the new trench photo to the Harborview
  project through the site page's upload form, and check it shows in the
  project gallery. Do the same tomorrow with a different photo on a
  different project."
- **Observable success criteria:**
  1. `[C1]` Asset library shows the new asset row (name + checksum) for
     run 1's project; the gallery page renders it.
  2. `[C1]` Run 2 on a different project with a different asset succeeds
     identically (changed-but-equivalent state).
  3. `[C2]` Upload confirmation visible in the page (oracle: success flash).
- **Required environments/capabilities/resources:** SiteBuild (4101)
  `marco.silva` (progress:create = upload permission); the browser file-input
  upload path (`/ui/assets/upload`); seeded images; product capabilities:
  browser-use (file inputs), upload.
- **Difficulty justification:** three stressors (upload + adaptation
  [hard] + mixed data shapes) → L3.

#### NST-B005 — Download pipeline with integrity check

```yaml
nst-id: NST-B005
lane: browser
executing-wo: VWO-013
difficulty: L3
provenance: scripted
teaching-mode: HYBRID
taxonomy:
  td-01-environment-modality: mixed {browser, filesystem}
  td-02-interaction-modality: browser-ui
  td-03-statefulness: stateless
  td-04-control-structure: linear
  td-05-concurrency: sequential
  td-06-application-boundary: cross-application
  td-07-data-shape: structured
  td-08-transfer-operations: download
  td-09-human-involvement: unattended
  td-10-credential-boundary: authentication
  td-11-failure-adaptation: happy-path-only
  td-12-reversibility: reversible
  td-13-workflow-provenance: scripted
```

- **Expected user goal:** "Download the current marketplace package digest
  file from the listing page into the audit folder and make sure it arrived
  whole."
- **Observable success criteria:**
  1. `[C1]` The file exists in the target folder with the exact byte size
     and checksum the listing/server advertises.
  2. `[C1]` The download was produced by a browser download action (browser
     download event/ledger, not a shell `curl`).
  3. `[C2]` Download UI state observed (oracle: download bar/entry).
- **Required environments/capabilities/resources:** a browser-reachable
  download surface (purpose-built fixture endpoint or fixture static file —
  §5 note 4; FlowMart listing digests as content); agent-browser with
  download handling; product capabilities: browser-use (downloads), file
  landing verification.
- **Difficulty justification:** three stressors (download + credential
  boundary + cross-application) → L3.

#### NST-B006 — Authentication and session renewal mid-journey

```yaml
nst-id: NST-B006
lane: browser
executing-wo: VWO-013
difficulty: L3
provenance: scripted
teaching-mode: INSTRUCT
taxonomy:
  td-01-environment-modality: browser
  td-02-interaction-modality: browser-ui
  td-03-statefulness: stateful-session
  td-04-control-structure: conditional-branching
  td-05-concurrency: sequential
  td-06-application-boundary: single-application
  td-07-data-shape: structured
  td-08-transfer-operations: none
  td-09-human-involvement: unattended
  td-10-credential-boundary: authentication
  td-11-failure-adaptation: recovery-required    # session expiry = controlled failure
  td-12-reversibility: reversible
  td-13-workflow-provenance: scripted
```

- **Expected user goal:** "Start filing the safety incident as the safety
  officer; when the portal logs you out mid-form, sign back in and finish —
  the incident must land exactly once."
- **Observable success criteria:**
  1. `[C1]` Exactly one incident record exists after the run with the
     intended fields (post-hoc `/api/safety` pull).
  2. `[C1]` The re-authentication is visible in the fixture's session/auth
     event log (login ×2, no failed-submit corruption).
  3. `[C2]` The logout interstitial and re-login form were observed (oracle:
     page snapshots).
- **Required environments/capabilities/resources:** SiteBuild (4101)
  `sam.oconnell`; a session-expiry capability (fixture session TTL override
  or armed switch; purpose-built profile if absent — §5 note 4); product
  capabilities: browser-use, session renewal via approved credential
  binding (credential must come from the product binding path, never
  hard-coded in the workflow).
- **Difficulty justification:** three stressors (recovery-required [hard]
  + conditional-branching + credential boundary) → L3.

#### NST-B007 — Redirects and transient failures

```yaml
nst-id: NST-B007
lane: browser
executing-wo: VWO-013
difficulty: L3
provenance: scripted
teaching-mode: DEMONSTRATE
taxonomy:
  td-01-environment-modality: browser
  td-02-interaction-modality: browser-ui
  td-03-statefulness: stateful-session
  td-04-control-structure: conditional-branching
  td-05-concurrency: sequential
  td-06-application-boundary: single-application
  td-07-data-shape: structured
  td-08-transfer-operations: none
  td-09-human-involvement: unattended
  td-10-credential-boundary: authentication
  td-11-failure-adaptation: recovery-required
  td-12-reversibility: reversible
  td-13-workflow-provenance: scripted
```

- **Expected user goal:** "Complete the ride-share support-ticket filing
  even though the portal throws a redirect loop once and a 503 once — retry
  sensibly and get the ticket reference number."
- **Observable success criteria:**
  1. `[C1]` One ticket exists with the intended content (post-hoc pull).
  2. `[C1]` Fixture access log (or failure-switch ledger) shows the
     armed `service_unavailable` firing exactly once and being retried.
  3. `[C2]` Redirect chain observed (oracle: navigation transcript).
- **Required environments/capabilities/resources:** RidePilot (4103)
  support persona; failure switch `service_unavailable`; a redirect-happy
  path (fixture route or purpose-built wrapper); product capabilities:
  browser-use (transient failure handling, retry).
- **Difficulty justification:** three stressors (recovery-required [hard]
  + conditional-branching + credential boundary) → L3.

#### NST-B008 — Rich editor with attachment

```yaml
nst-id: NST-B008
lane: browser
executing-wo: VWO-013
difficulty: L3
provenance: human-authored
teaching-mode: HYBRID
taxonomy:
  td-01-environment-modality: browser
  td-02-interaction-modality: browser-ui
  td-03-statefulness: stateful-session
  td-04-control-structure: linear
  td-05-concurrency: sequential
  td-06-application-boundary: single-application
  td-07-data-shape: mixed {unstructured-text, unstructured-visual-media}
  td-08-transfer-operations: upload
  td-09-human-involvement: unattended
  td-10-credential-boundary: authentication
  td-11-failure-adaptation: happy-path-only
  td-12-reversibility: reversible
  td-13-workflow-provenance: human-authored
```

- **Expected user goal:** "Write the post-mortem note in the team's web
  editor: bold the headline, add the timeline as a list, attach the incident
  screenshot, save, then reopen it and make sure everything is still there."
- **Observable success criteria:**
  1. `[C1]` Reopened document round-trips: headline formatting (bold markup)
     preserved, list items preserved, attachment present (fixture API state).
  2. `[C2]` Editor interactions evidenced (oracle: before/after snapshots of
     the contenteditable state).
  3. `[C1]` The attachment file is stored and re-served byte-identically.
- **Required environments/capabilities/resources:** purpose-built rich-editor
  web fixture (contenteditable + attachment storage — §5 note 4); seeded
  screenshot file; product capabilities: browser-use (rich editing), upload.
- **Difficulty justification:** three stressors (upload + mixed data
  shapes + credential boundary) → L3.

#### NST-B009 — Browser-to-API handoff

```yaml
nst-id: NST-B009
lane: browser
executing-wo: VWO-013
difficulty: L3
provenance: scripted
teaching-mode: INSTRUCT
taxonomy:
  td-01-environment-modality: mixed {browser, api-tool-mcp}
  td-02-interaction-modality: mixed {browser-ui, api-tool-call}
  td-03-statefulness: stateful-session
  td-04-control-structure: conditional-branching
  td-05-concurrency: sequential
  td-06-application-boundary: cross-application   # UI surface + API surface of record
  td-07-data-shape: structured
  td-08-transfer-operations: none
  td-09-human-involvement: unattended
  td-10-credential-boundary: authentication+authorization
  td-11-failure-adaptation: happy-path-only
  td-12-reversibility: reversible
  td-13-workflow-provenance: scripted
```

- **Expected user goal:** "Discover the refund-guard workflow in the
  marketplace UI, then check its published manifest through the documented
  JSON API, and only if the manifest lists the payments capability, submit
  the install request through the API with the values you read off the
  listing page."
- **Observable success criteria:**
  1. `[C1]` The install record exists via API with values matching the
     listing page (post-hoc pull; field-by-field compare).
  2. `[C1]` If (control run) the manifest omits the capability, NO install
     is submitted (conditional branch both ways).
  3. `[C2]` Browser-side discovery evidenced (listing page snapshot).
- **Required environments/capabilities/resources:** FlowMart (4105) buyer
  persona; documented JSON API twin used here as a FIRST-CLASS goal-critical
  step (unlike wave-1/2 where API twins were post-hoc diagnostics only);
  product capabilities: browser-use + api-tool-call in one workflow, shared
  session/credential binding.
- **Difficulty justification:** three stressors (cross-application +
  credential boundary + conditional-branching) → L3.

#### NST-B010 — Browser-to-terminal/file handoff

```yaml
nst-id: NST-B010
lane: browser
executing-wo: VWO-013
difficulty: L3
provenance: human-authored
teaching-mode: HYBRID
taxonomy:
  td-01-environment-modality: mixed {browser, terminal-cli, filesystem}
  td-02-interaction-modality: mixed {browser-ui, terminal}
  td-03-statefulness: stateful-session
  td-04-control-structure: linear
  td-05-concurrency: sequential
  td-06-application-boundary: cross-application
  td-07-data-shape: structured
  td-08-transfer-operations: download
  td-09-human-involvement: unattended
  td-10-credential-boundary: authentication
  td-11-failure-adaptation: happy-path-only
  td-12-reversibility: reversible
  td-13-workflow-provenance: human-authored
```

- **Expected user goal:** "Pull the deployment log export out of the forge
  console page, count the production deploys this week in the terminal, and
  post the number back as a comment on the release thread in the browser."
- **Observable success criteria:**
  1. `[C1]` The posted comment's number equals a recomputed count from the
     export (independent post-hoc recompute).
  2. `[C1]` The export file landed via the browser download path.
  3. `[C2]` Terminal step evidenced (oracle: command transcript).
- **Required environments/capabilities/resources:** ForgeOps (4102);
  download surface (as NST-B005); terminal; product capabilities:
  browser-use + terminal + filesystem in one workflow (browser-primary).
- **Difficulty justification:** three stressors (download + credential
  boundary + cross-application) → L3.

### 6.3 NST-T — terminal/filesystem/developer lane (executing WO: VWO-014; 10 tasks)

#### NST-T001 — Event-log analysis pipeline

```yaml
nst-id: NST-T001
lane: terminal
executing-wo: VWO-014
difficulty: L1
provenance: scripted
teaching-mode: DEMONSTRATE
taxonomy:
  td-01-environment-modality: mixed {terminal-cli, filesystem}
  td-02-interaction-modality: terminal
  td-03-statefulness: stateless
  td-04-control-structure: linear
  td-05-concurrency: sequential
  td-06-application-boundary: single-application
  td-07-data-shape: structured
  td-08-transfer-operations: none
  td-09-human-involvement: unattended
  td-10-credential-boundary: none                 # public event API (auth:false)
  td-11-failure-adaptation: happy-path-only
  td-12-reversibility: reversible
  td-13-workflow-provenance: scripted
```

- **Expected user goal:** "Pull today's event feed from the press app and
  tell me how many events of each type there were, as a tidy summary file."
- **Observable success criteria:**
  1. `[C1]` Summary file exists with one `type=count` line per distinct
     event type, counts equal to an independent recomputation (post-hoc jq
     recount of the same pull).
  2. `[C1]` The pipeline is a real shell pipeline (transcript shows fetch +
     transform commands).
- **Required environments/capabilities/resources:** bash; curl/jq (or
  python); PressRoom (4104) public `/api/events`; product capabilities:
  terminal execution, HTTP fetch, text processing.
- **Difficulty justification:** zero stressors (public API, single dominant
  surface, happy path) → L1.

#### NST-T002 — Process inspection and service restart

```yaml
nst-id: NST-T002
lane: terminal
executing-wo: VWO-014
difficulty: L3
provenance: scripted
teaching-mode: INSTRUCT
taxonomy:
  td-01-environment-modality: mixed {terminal-cli, browser}
  td-02-interaction-modality: mixed {terminal, browser-ui}
  td-03-statefulness: stateful-session
  td-04-control-structure: conditional-branching
  td-05-concurrency: sequential
  td-06-application-boundary: cross-application
  td-07-data-shape: structured
  td-08-transfer-operations: none
  td-09-human-involvement: unattended
  td-10-credential-boundary: none
  td-11-failure-adaptation: recovery-required
  td-12-reversibility: reversible
  td-13-workflow-provenance: scripted
```

- **Expected user goal:** "Find which fixture app process is wedged, check
  its port and health endpoint, restart just that one cleanly, and confirm
  in the browser that its world came back intact."
- **Observable success criteria:**
  1. `[C1]` Process listing evidences identification (ps/lsof transcript);
     only the target app's PID changes across restart.
  2. `[C1]` Post-restart: health endpoint 200 and a browser page render of
     the same app shows the same seeded record counts as before (state
     intact — durable-state discipline).
  3. `[C2]` Browser confirmation snapshot (oracle: dashboard page).
- **Required environments/capabilities/resources:** bash; ps/lsof/curl;
  fixtures run-all; agent-browser for the confirmation; product
  capabilities: terminal (process inspection, service control), browser
  observation.
- **Difficulty justification:** three stressors (cross-application +
  recovery-required [hard] + conditional-branching) → L3.

#### NST-T003 — File discovery and text transformation with re-run

```yaml
nst-id: NST-T003
lane: terminal
executing-wo: VWO-014
difficulty: L3
provenance: scripted
teaching-mode: DEMONSTRATE
taxonomy:
  td-01-environment-modality: filesystem
  td-02-interaction-modality: mixed {file-op, terminal}
  td-03-statefulness: stateless
  td-04-control-structure: linear
  td-05-concurrency: sequential
  td-06-application-boundary: single-application
  td-07-data-shape: mixed {unstructured-text, structured}
  td-08-transfer-operations: none
  td-09-human-involvement: unattended
  td-10-credential-boundary: none
  td-11-failure-adaptation: adaptation-required   # second, differently-shaped input set
  td-12-reversibility: reversible
  td-13-workflow-provenance: scripted
```

- **Expected user goal:** "Find every weekly-notes markdown file under the
  field-notes tree, convert each to a CSV row (date, site, note count), and
  build one combined CSV. Then do it again when I drop a differently-named,
  differently-ordered batch into a second folder."
- **Observable success criteria:**
  1. `[C1]` Combined CSV rows equal an independent parse of the same inputs
     (field-by-field).
  2. `[C1]` Run 2 on the changed batch produces correct output with no
     carried-over rows from run 1.
  3. `[C1]` Discovery matched exactly the intended file set (find transcript
     vs expected list).
- **Required environments/capabilities/resources:** bash; find/grep/awk or
  python; a seeded notes tree (worker-created fixture data); product
  capabilities: filesystem discovery, text transformation.
- **Difficulty justification:** adaptation-required is a hard stressor →
  L3 floor.

#### NST-T004 — Structured-data reshape

```yaml
nst-id: NST-T004
lane: terminal
executing-wo: VWO-014
difficulty: L2
provenance: human-authored
teaching-mode: HYBRID
taxonomy:
  td-01-environment-modality: mixed {api-tool-mcp, filesystem}
  td-02-interaction-modality: mixed {api-tool-call, terminal}
  td-03-statefulness: stateless
  td-04-control-structure: linear
  td-05-concurrency: sequential
  td-06-application-boundary: cross-application
  td-07-data-shape: structured
  td-08-transfer-operations: none
  td-09-human-involvement: unattended
  td-10-credential-boundary: none
  td-11-failure-adaptation: happy-path-only
  td-12-reversibility: reversible
  td-13-workflow-provenance: human-authored
```

- **Expected user goal:** "Give me one CSV of all open purchase requests
  across the construction app — just id, vendor, amount, status — sorted by
  amount, biggest first." (Goal text only.)
- **Observable success criteria:**
  1. `[C1]` CSV header + rows equal an independent jq/python recomputation
     from `/api/procurement` state.
  2. `[C1]` Sort order verified descending by amount.
- **Required environments/capabilities/resources:** curl/jq or python;
  SiteBuild public procurement API; product capabilities: api-tool-call,
  data reshape, file output.
- **Difficulty justification:** one stressor (cross-application — data from
  app + file output) → L2.

#### NST-T005 — Archive and integrity workflow

```yaml
nst-id: NST-T005
lane: terminal
executing-wo: VWO-014
difficulty: L1
provenance: scripted
teaching-mode: DEMONSTRATE
taxonomy:
  td-01-environment-modality: filesystem
  td-02-interaction-modality: terminal
  td-03-statefulness: stateless
  td-04-control-structure: linear
  td-05-concurrency: sequential
  td-06-application-boundary: single-application
  td-07-data-shape: unstructured-text
  td-08-transfer-operations: none
  td-09-human-involvement: unattended
  td-10-credential-boundary: none
  td-11-failure-adaptation: happy-path-only
  td-12-reversibility: reversible
  td-13-workflow-provenance: scripted
```

- **Expected user goal:** "Bundle this month's note files into one
  compressed archive, extract it somewhere else, and prove nothing changed."
- **Observable success criteria:**
  1. `[C1]` `tar`/archive created; extraction at the second location yields
     a tree whose checksums match the original file-for-file (sha256 list
     equality).
  2. `[C1]` Archive listing contains exactly the intended file set.
- **Required environments/capabilities/resources:** bash; tar/gzip;
  sha256sum; product capabilities: terminal, archive operations, integrity
  checks.
- **Difficulty justification:** zero stressors → L1.

#### NST-T006 — Git operations at a pinned revision

```yaml
nst-id: NST-T006
lane: terminal
executing-wo: VWO-014
difficulty: L1
provenance: scripted
teaching-mode: HYBRID
taxonomy:
  td-01-environment-modality: mixed {terminal-cli, filesystem}
  td-02-interaction-modality: terminal
  td-03-statefulness: stateful-session
  td-04-control-structure: linear
  td-05-concurrency: sequential
  td-06-application-boundary: single-application
  td-07-data-shape: unstructured-text
  td-08-transfer-operations: none
  td-09-human-involvement: unattended
  td-10-credential-boundary: none                 # anonymous clone; no push
  td-11-failure-adaptation: happy-path-only
  td-12-reversibility: reversible
  td-13-workflow-provenance: scripted
```

- **Expected user goal:** "Clone the validation repo at exactly the recorded
  base SHA, create a branch, commit a small change to a scratch file, and
  show me the log proving the branch sits on the right base."
- **Observable success criteria:**
  1. `[C1]` `git merge-base --is-ancestor <base> HEAD` exits 0; HEAD commit
     contains the scratch change.
  2. `[C1]` Working tree clean after commit; branch name matches the
     instruction.
  3. `[C1]` No push occurred (no remote refs advanced — check via
     `git ls-remote` equality before/after).
- **Required environments/capabilities/resources:** git 2.47.x; network to
  the public repo (anonymous); product capabilities: terminal, git
  operations. Credentials stay out of the workflow (no-push by design).
- **Difficulty justification:** zero stressors (anonymous, linear,
  happy-path) → L1.

#### NST-T007 — Build/test/lint gate

```yaml
nst-id: NST-T007
lane: terminal
executing-wo: VWO-014
difficulty: L2
provenance: human-authored
teaching-mode: INSTRUCT
taxonomy:
  td-01-environment-modality: terminal-cli
  td-02-interaction-modality: terminal
  td-03-statefulness: stateful-session
  td-04-control-structure: conditional-branching
  td-05-concurrency: sequential
  td-06-application-boundary: single-application
  td-07-data-shape: structured
  td-08-transfer-operations: none
  td-09-human-involvement: unattended
  td-10-credential-boundary: none
  td-11-failure-adaptation: happy-path-only
  td-12-reversibility: reversible
  td-13-workflow-provenance: human-authored
```

- **Expected user goal:** "Make sure the fixture world is healthy before I
  demo: run the fixture sweep, and if anything fails, run the reset and the
  sweep once more — tell me the final verdict." (Goal text only.)
- **Observable success criteria:**
  1. `[C1]` Final sweep transcript records `59 passed, 0 failed` (the
     canonical VWO-002 gate).
  2. `[C1]` If the first run failed, a reset+re-run is evidenced
     (conditional branch both ways; determinism preserved).
  3. `[C1]` Verdict message matches the final sweep result (string check
     against the transcript).
- **Required environments/capabilities/resources:** bash; fixtures
  run-all/verify-sweep/reset scripts; product capabilities: terminal, build/
  test tooling, conditional reporting.
- **Difficulty justification:** one stressor (branching) → L2.

#### NST-T008 — Code edit plus repository change

```yaml
nst-id: NST-T008
lane: terminal
executing-wo: VWO-014
difficulty: L3
provenance: scripted
teaching-mode: HYBRID
taxonomy:
  td-01-environment-modality: mixed {terminal-cli, filesystem}
  td-02-interaction-modality: mixed {terminal, file-op}
  td-03-statefulness: stateful-session
  td-04-control-structure: linear
  td-05-concurrency: sequential
  td-06-application-boundary: single-application
  td-07-data-shape: unstructured-text
  td-08-transfer-operations: none
  td-09-human-involvement: unattended
  td-10-credential-boundary: none
  td-11-failure-adaptation: recovery-required    # sweep fails on bad edit → fix → rerun
  td-12-reversibility: reversible
  td-13-workflow-provenance: scripted
```

- **Expected user goal:** "Fix the typo in the fixture help text, make sure
  the sweep still passes, and commit the change on a branch with a message
  that says what you fixed."
- **Observable success criteria:**
  1. `[C1]` A seeded typo is repaired in the committed diff (string check).
  2. `[C1]` An intermediate failing check occurs and is recovered: the run
     first applies a WRONG edit (per the task script), observes the sweep
     failure, reverts, applies the correct edit, and the final sweep is
     59/59.
  3. `[C1]` Commit message contains the fix description (pattern check).
- **Required environments/capabilities/resources:** git; fixture tree;
  verify-sweep; product capabilities: terminal, code editing, recovery from
  failing checks.
- **Difficulty justification:** recovery-required is a hard stressor → L3
  floor.

#### NST-T009 — API-first task with CLI credential binding

```yaml
nst-id: NST-T009
lane: terminal
executing-wo: VWO-014
difficulty: L2
provenance: scripted
teaching-mode: INSTRUCT
taxonomy:
  td-01-environment-modality: api-tool-mcp       # world state = fixture service records
  td-02-interaction-modality: mixed {terminal, api-tool-call}
  td-03-statefulness: stateful-session
  td-04-control-structure: linear
  td-05-concurrency: sequential
  td-06-application-boundary: single-application
  td-07-data-shape: structured
  td-08-transfer-operations: none
  td-09-human-involvement: unattended
  td-10-credential-boundary: authentication+authorization
  td-11-failure-adaptation: happy-path-only
  td-12-reversibility: reversible
  td-13-workflow-provenance: scripted
```

- **Expected user goal:** "Approve this month's pending invoice from your
  terminal, without opening a browser — sign in through the CLI the way the
  product stores credentials, then approve it."
- **Observable success criteria:**
  1. `[C1]` The invoice record is approved (post-hoc state pull shows
     approved + actor = the finance persona).
  2. `[C1]` The approval was performed by an authenticated API call whose
     credential came from the product's approved resource-binding path
     (binding identity recorded in evidence; NOT hard-coded in the workflow
     source — credential text must never appear in workflow source or
     evidence).
  3. `[C2]` Terminal transcript shows login + approve commands (oracle:
     command names, with secrets redacted).
- **Required environments/capabilities/resources:** bash; curl (or product
  CLI); SiteBuild (4101) `priya.nair` (`invoice:approve`); product
  capabilities: credential/resource binding for CLI/API use, api-tool-call
  (VWO-014 family: "CLI authentication through approved resource bindings").
- **Difficulty justification:** one stressor (credential boundary) → L2.

#### NST-T010 — Failing pipeline with checkpoint resume

```yaml
nst-id: NST-T010
lane: terminal
executing-wo: VWO-014
difficulty: L4
provenance: scripted
teaching-mode: HYBRID
taxonomy:
  td-01-environment-modality: mixed {terminal-cli, api-tool-mcp, filesystem}
  td-02-interaction-modality: mixed {terminal, api-tool-call}
  td-03-statefulness: stateful-session
  td-04-control-structure: conditional-branching
  td-05-concurrency: sequential
  td-06-application-boundary: cross-application
  td-07-data-shape: structured
  td-08-transfer-operations: none
  td-09-human-involvement: unattended
  td-10-credential-boundary: none
  td-11-failure-adaptation: recovery+adaptation   # 503 mid-pipeline + stale checkpoint
  td-12-reversibility: reversible
  td-13-workflow-provenance: scripted
```

- **Expected user goal:** "Build the weekly ops summary by pulling each
  region's trip counts one region at a time, writing a checkpoint after
  each. The service will hiccup once (503) and leave a stale checkpoint
  behind — deal with both, and the summary must count each region exactly
  once."
- **Observable success criteria:**
  1. `[C1]` Final summary counts equal an independent recomputation; no
     region double-counted (checkpoint exactly-once semantics).
  2. `[C1]` The armed 503 occurred mid-run (switch ledger) and the pipeline
     recovered (retry evidenced).
  3. `[C1]` The stale checkpoint was detected and superseded, not blindly
     trusted (transcript shows stale detection).
- **Required environments/capabilities/resources:** bash; checkpoint file;
  RidePilot public APIs; failure switch `service_unavailable`; product
  capabilities: terminal, resilient pipelines, idempotent checkpointing.
- **Difficulty justification:** L4 — frontier trigger (TD-01 environment
  set ≥3: terminal, api, filesystem) + three stressors (recovery+adaptation
  [hard], conditional-branching, cross-application).

### 6.4 NST-X — cross-application/multi-environment lane (executing WO: VWO-015; 8 tasks)

#### NST-X001 — Export → transform → review → approve → publish

```yaml
nst-id: NST-X001
lane: cross-application
executing-wo: VWO-015
difficulty: L4
provenance: scripted
teaching-mode: HYBRID
taxonomy:
  td-01-environment-modality: mixed {browser, terminal-cli, filesystem, desktop-gui}
  td-02-interaction-modality: mixed {browser-ui, terminal, gui, human}
  td-03-statefulness: stateful-session
  td-04-control-structure: conditional-branching
  td-05-concurrency: sequential
  td-06-application-boundary: cross-application
  td-07-data-shape: mixed {structured, unstructured-text}
  td-08-transfer-operations: download
  td-09-human-involvement: human-in-the-loop    # manager approves before publish
  td-10-credential-boundary: authentication+authorization
  td-11-failure-adaptation: recovery-required   # transient 503 on export
  td-12-reversibility: destructive-irreversible # publication is terminal (in-world)
  td-13-workflow-provenance: scripted
```

- **Expected user goal:** "Get the story stats out of the press app, have
  the terminal turn them into the Tuesday bulletin, let the editor eyeball
  it on screen, get the desk chief's sign-off, and only then publish the
  bulletin to the newsroom channel."
- **Observable success criteria:**
  1. `[C1]` Bulletin published exactly once (channel record + event) with
     content derived from the export (field compare).
  2. `[C1]` Approval gate recorded BEFORE the publish event (event
     ordering check) — the human approval is semantically required.
  3. `[C1]` The armed 503 on export was recovered (ledger + retry
     evidence).
  4. `[C2]` GUI review step evidenced (screenshot of the bulletin in the
     review window; oracle: bulletin title visible).
- **Required environments/capabilities/resources:** PressRoom (4104)
  editor + desk-chief personas; download surface; terminal; Xvfb GUI review
  window; operator chat channel as the approval gate (fidelity note §5-1);
  failure switch `service_unavailable`; product capabilities: browser-use,
  terminal, computer-use, human approval gate, publication side-effects.
  Matches VWO-015 pattern 1 (`browser → downloaded file → terminal
  processing → GUI review → approval → browser publication`).
- **Difficulty justification:** L4 — frontier trigger (TD-01 set ≥4) plus
  seven stressors including destructive publication.

#### NST-X002 — Terminal → API → desktop → approval → notification

```yaml
nst-id: NST-X002
lane: cross-application
executing-wo: VWO-015
difficulty: L4
provenance: scripted
teaching-mode: INSTRUCT
taxonomy:
  td-01-environment-modality: mixed {terminal-cli, api-tool-mcp, filesystem, desktop-gui}
  td-02-interaction-modality: mixed {terminal, api-tool-call, gui, human}
  td-03-statefulness: stateful-session
  td-04-control-structure: linear
  td-05-concurrency: sequential
  td-06-application-boundary: cross-application
  td-07-data-shape: mixed {structured, unstructured-text}
  td-08-transfer-operations: none
  td-09-human-involvement: human-in-the-loop
  td-10-credential-boundary: authentication+authorization
  td-11-failure-adaptation: happy-path-only
  td-12-reversibility: reversible
  td-13-workflow-provenance: scripted
```

- **Expected user goal:** "From the terminal, pull tonight's deployment list
  through the forge API, write the digest to a file, open it in the editor
  for the release manager to review, get her approval, and notify the
  on-call SRE."
- **Observable success criteria:**
  1. `[C1]` Digest file content equals an independent API recomputation.
  2. `[C1]` Approval recorded (operator-channel transcript timestamped
     before the SRE notification) and the SRE notification row exists in
     the fixture.
  3. `[C2]` Editor-window review evidenced (screenshot; oracle: digest
     title).
- **Required environments/capabilities/resources:** ForgeOps (4102)
  API + personas; GUI editor; operator channel; product capabilities:
  terminal, api-tool-call, computer-use, human gate, notifications.
  Matches VWO-015 pattern 2.
- **Difficulty justification:** L4 — frontier trigger (TD-01 set ≥4:
  terminal, api, filesystem, desktop-gui) + human-in-the-loop.

#### NST-X003 — Browser → API/tool → filesystem → terminal → browser

```yaml
nst-id: NST-X003
lane: cross-application
executing-wo: VWO-015
difficulty: L4
provenance: scripted
teaching-mode: HYBRID
taxonomy:
  td-01-environment-modality: mixed {browser, api-tool-mcp, filesystem, terminal-cli}
  td-02-interaction-modality: mixed {browser-ui, api-tool-call, terminal}
  td-03-statefulness: stateful-session
  td-04-control-structure: conditional-branching
  td-05-concurrency: sequential
  td-06-application-boundary: cross-application
  td-07-data-shape: structured
  td-08-transfer-operations: download
  td-09-human-involvement: unattended
  td-10-credential-boundary: authentication+authorization
  td-11-failure-adaptation: happy-path-only
  td-12-reversibility: reversible
  td-13-workflow-provenance: scripted
```

- **Expected user goal:** "Vet the marketplace package before we install
  it: read the listing in the browser, fetch the manifest through the API,
  stage the digest file, verify it with the terminal checksum tool, and
  only then hit Install on the listing page."
- **Observable success criteria:**
  1. `[C1]` Install record exists, and ONLY if the digest verification
     passed (control run with a tampered digest must NOT install).
  2. `[C1]` Verification transcript shows the checksum comparison.
  3. `[C1]` The manifest values read in the browser match the API-fetched
     manifest (field compare recorded).
- **Required environments/capabilities/resources:** FlowMart (4105);
  download/digest surface; terminal; product capabilities: browser-use,
  api-tool-call, filesystem, terminal, conditional gating. Matches VWO-015
  pattern 3.
- **Difficulty justification:** L4 — ≥3 stressors, ≥3 environments,
  conditional gate.

#### NST-X004 — Long-running weekly report with restart

```yaml
nst-id: NST-X004
lane: cross-application
executing-wo: VWO-015
difficulty: L4
provenance: scripted
teaching-mode: DEMONSTRATE
taxonomy:
  td-01-environment-modality: mixed {browser, terminal-cli, api-tool-mcp}
  td-02-interaction-modality: mixed {browser-ui, terminal, api-tool-call}
  td-03-statefulness: long-running-resumable
  td-04-control-structure: linear
  td-05-concurrency: sequential
  td-06-application-boundary: cross-application
  td-07-data-shape: structured
  td-08-transfer-operations: none
  td-09-human-involvement: unattended
  td-10-credential-boundary: authentication
  td-11-failure-adaptation: recovery-required
  td-12-reversibility: reversible
  td-13-workflow-provenance: scripted
```

- **Expected user goal:** "The Friday site report: read the week's progress
  entries in the portal, compute the totals with the terminal script, and
  post the weekly summary back to the portal. It runs for a while — if it
  gets killed halfway it must pick up where it left off without posting
  twice."
- **Observable success criteria:**
  1. `[C1]` Exactly ONE weekly summary record exists after the run
     (duplicate-post check = exactly-once).
  2. `[C1]` `kill -9` occurred mid-run (transcript timestamps) and the
     resumed run did not redo completed side-effects (progress/audit trail
     shows no duplicate reads-turned-writes).
  3. `[C1]` Summary totals equal an independent recomputation.
- **Required environments/capabilities/resources:** SiteBuild (4101);
  terminal; process control; product capabilities: durable long-running
  workflow, restart resumption, idempotency (the VWO-007 guarantee class,
  now through the product surface).
- **Difficulty justification:** L4 — long-running-resumable + ≥3 stressors.

#### NST-X005 — Cross-app reconciliation report

```yaml
nst-id: NST-X005
lane: cross-application
executing-wo: VWO-015
difficulty: L4
provenance: scripted
teaching-mode: HYBRID
taxonomy:
  td-01-environment-modality: mixed {browser, api-tool-mcp, filesystem}
  td-02-interaction-modality: mixed {browser-ui, api-tool-call, file-op}
  td-03-statefulness: stateful-session
  td-04-control-structure: linear
  td-05-concurrency: sequential
  td-06-application-boundary: cross-application
  td-07-data-shape: structured
  td-08-transfer-operations: none
  td-09-human-involvement: unattended
  td-10-credential-boundary: authentication
  td-11-failure-adaptation: happy-path-only
  td-12-reversibility: reversible
  td-13-workflow-provenance: scripted
```

- **Expected user goal:** "Pull the construction project list from the
  portal and the deployment list from the forge API, match them by week,
  and drop one combined status file the PM can read."
- **Observable success criteria:**
  1. `[C1]` Combined file rows equal an independent join recomputation.
  2. `[C1]` Both sources were read through their authenticated surfaces
     (session/binding evidence).
  3. `[C2]` PM-readable formatting (oracle: header row + one line per
     project).
- **Required environments/capabilities/resources:** SiteBuild + ForgeOps;
  product capabilities: cross-app data carry, api-tool-call, file output.
- **Difficulty justification:** L4 — frontier trigger (TD-01 set ≥3:
  browser, api, filesystem) + two stressors (cross-application +
  credential boundary).

#### NST-X006 — Concurrent dual-region surge decision

```yaml
nst-id: NST-X006
lane: cross-application
executing-wo: VWO-015
difficulty: L4
provenance: scripted
teaching-mode: INSTRUCT
taxonomy:
  td-01-environment-modality: mixed {browser, api-tool-mcp}
  td-02-interaction-modality: mixed {browser-ui, api-tool-call, human}
  td-03-statefulness: stateful-session
  td-04-control-structure: conditional-branching
  td-05-concurrency: parallel-fanout            # two regions decide simultaneously
  td-06-application-boundary: cross-application
  td-07-data-shape: structured
  td-08-transfer-operations: none
  td-09-human-involvement: human-in-the-loop    # ops approval on the combined call
  td-10-credential-boundary: authorization
  td-11-failure-adaptation: happy-path-only
  td-12-reversibility: reversible
  td-13-workflow-provenance: scripted
```

- **Expected user goal:** "Two regions hit the surge threshold at the same
  time. Watch both dashboards, get ops approval for the combined surge
  plan, and broadcast both regions' changes together — each exactly once."
- **Observable success criteria:**
  1. `[C1]` Both region broadcasts land exactly once (dedup/audit check —
     the VWO-007/008 duplicate-class oracle, now concurrent by design).
  2. `[C1]` The two decision steps overlapped in time (timestamps prove
     concurrency, not serialization).
  3. `[C1]` Approval precedes both broadcasts (ordering check).
- **Required environments/capabilities/resources:** RidePilot (4103) ops
  personas + surge seeds; concurrent sessions; operator channel; product
  capabilities: parallel workflow branches, human gate, exactly-once
  broadcasts.
- **Difficulty justification:** L4 — parallel-fanout + human-in-loop +
  branching.

#### NST-X007 — Media repurpose batch round-trip

```yaml
nst-id: NST-X007
lane: cross-application
executing-wo: VWO-015
difficulty: L4
provenance: human-authored
teaching-mode: HYBRID
taxonomy:
  td-01-environment-modality: mixed {browser, terminal-cli, api-tool-mcp, filesystem}
  td-02-interaction-modality: mixed {browser-ui, terminal, api-tool-call}
  td-03-statefulness: stateful-session
  td-04-control-structure: linear
  td-05-concurrency: sequential
  td-06-application-boundary: cross-application
  td-07-data-shape: unstructured-visual-media
  td-08-transfer-operations: upload+download
  td-09-human-involvement: unattended
  td-10-credential-boundary: authorization
  td-11-failure-adaptation: happy-path-only
  td-12-reversibility: reversible
  td-13-workflow-provenance: human-authored
```

- **Expected user goal:** "Grab this week's published story images, resize
  them for the social crop, and put the resized set back as renditions so
  the social desk can publish." (Goal text only.)
- **Observable success criteria:**
  1. `[C1]` New rendition records exist for each source asset with
     machine-checkable dimensions (image-header parse).
  2. `[C1]` The set matches the week's story assets exactly (no extras, no
     misses).
  3. `[C1]` Both transfer directions occurred (download of sources; upload
     of renditions).
- **Required environments/capabilities/resources:** PressRoom (4104);
  terminal image tooling (deterministic resize backend); upload path;
  product capabilities: browser-use, terminal, filesystem, upload/download
  round-trip.
- **Difficulty justification:** L4 — ≥3 stressors + upload+download + ≥3
  environments.

#### NST-X008 — Install-verify-rollback across surfaces

```yaml
nst-id: NST-X008
lane: cross-application
executing-wo: VWO-015
difficulty: L4
provenance: scripted
teaching-mode: INSTRUCT
taxonomy:
  td-01-environment-modality: mixed {browser, terminal-cli, api-tool-mcp}
  td-02-interaction-modality: mixed {browser-ui, terminal, api-tool-call}
  td-03-statefulness: stateful-session
  td-04-control-structure: conditional-branching
  td-05-concurrency: sequential
  td-06-application-boundary: cross-application
  td-07-data-shape: structured
  td-08-transfer-operations: none
  td-09-human-involvement: unattended
  td-10-credential-boundary: authentication+authorization
  td-11-failure-adaptation: recovery-required   # entitlement flag mid-upgrade
  td-12-reversibility: reversible               # explicit rollback path exists
  td-13-workflow-provenance: scripted
```

- **Expected user goal:** "Install the package through the marketplace,
  verify its digest from the terminal, survive a mid-flight entitlement
  revocation, and land back on the last good version through the rollback
  path — with the audit trail showing why."
- **Observable success criteria:**
  1. `[C1]` Final install pin equals the last good version; upgrade and
     rollback records both present with reasons.
  2. `[C1]` The entitlement revocation (armed `stale_entitlement`) fired
     mid-upgrade and the fail-closed behavior is evidenced (VWO-009 class).
  3. `[C1]` Terminal digest verification transcript exists and gates the
     install step.
- **Required environments/capabilities/resources:** FlowMart (4105) buyer +
  entitlement seeds; failure switch `stale_entitlement`; terminal; product
  capabilities: browser-use, terminal, entitlement gates, rollback.
- **Difficulty justification:** L4 — frontier trigger (TD-01 set ≥3:
  browser, terminal, api) + four stressors (recovery-required [hard],
  conditional-branching, credential boundary, cross-application).

### 6.5 NST-H — held-out lane (executing WO: VWO-016 — SELECTION PROCEDURE ONLY)

**No held-out task is enumerated in this catalog.** Enumerating held-out
tasks (goal text, criteria, or expected paths) in any repository file that
executors can read would defeat holding them out (VWO-016 anti-cheating;
VWO-011 packet). This section defines only the procedure. The task IDs
`NST-H001…` are reserved and are assigned in the sealed ledger at
authoring time (§7.5), never in this file.

---

## 7. NST-H — held-out selection procedure (normative)

### 7.1 Who authors held-out tasks

- **Independent human authors** who did NOT author any catalog task in this
  benchmark, did NOT implement workflow contracts/engine code, and will NOT
  execute the tasks. Minimum pool of three distinct authors.
- At least one author must be **unfamiliar with internal workflow
  contracts** (VWO-016 held-out diversity requirement) — authors are given
  only the north-star README's user-model description ("a normal user
  teaches a computer goal and gets a reusable workflow"), not engine docs.
- The **Tech Lead** recruits authors, seals their tasks, and dispatches
  them at execution time. The executing worker (VWO-016) receives tasks
  only at execution time.

### 7.2 When

- Held-out tasks are authored **after VWO-015 is accepted** (VWO-016's
  dependency) in a sealed authoring window, and executed in the VWO-016
  window immediately after. Nothing about an authored held-out task is
  committed to the repository before its execution completes.

### 7.3 Task-space sampling rules (stratification)

The Tech Lead draws the authoring brief from THIS grid so the pool covers
the space rather than clustering (numbers are minimums for the executed
draw; the authored pool is ≥12 tasks):

- **Environment modality (TD-01):** ≥1 `desktop-gui`-containing, ≥2
  `browser`-containing, ≥2 `terminal-cli`/`filesystem`-containing, ≥2
  `api-tool-mcp`-containing, ≥3 `mixed` with environment sets NOT already
  present in the v1.0.0 catalog (novel combination rule).
- **Provenance (TD-13):** all held-out (definitionally).
- **Human involvement (TD-09):** ≥2 `human-in-the-loop`.
- **Data shape (TD-07):** ≥2 `unstructured-visual-media`-containing, ≥2
  `unstructured-text`-containing, ≥2 `structured`-only.
- **Credential boundary (TD-10):** ≥2 tasks with `authorization`-gated
  goals.
- **Failure/adaptation (TD-11):** ≥3 with `recovery-required` or stronger;
  VWO-016's method steps 5–6 (changed-state re-run + controlled
  perturbation) apply to every executed task regardless of TD-11.
- **Difficulty:** ≥2 at L4-equivalent novelty (held-out tasks are ≥L4 by
  definition; ≥2 must additionally carry long-running, parallel, or
  destructive stressors).
- **Teaching mode:** the executed draw rotates DEMONSTRATE / INSTRUCT /
  HYBRID so each mode gets ≥2 held-out tasks (VWO-016 acceptance: success
  rate by teaching mode).
- **Novelty constraint:** no held-out task may duplicate a v1.0.0 catalog
  task's goal; each must be checked against the catalog at authoring time
  by the Tech Lead (goal-level similarity, not surface-level).

### 7.4 How they stay unseen until execution

- Held-out tasks live **outside the repository** in the Tech Lead's sealed
  store until their execution window. This repository is public — no
  secret content may be committed to it, so secrecy is procedural, not
  cryptographic, for content; integrity IS cryptographic (below).
- In-repo, the catalog records only a **commitment ledger**: for each
  sealed task, its `NST-H###` ID, the SHA-256 of the canonical task text
  (goal + criteria), the stratification strata it satisfies, and the
  author-window timestamp — NO content. The ledger is appended by the Tech
  Lead in a MINOR benchmark-version bump (additive metadata) at sealing
  time, which is after VWO-015 acceptance and before VWO-016 execution.
- At execution time the Tech Lead reveals each task text to the VWO-016
  worker; the worker verifies the SHA-256 matches the ledger commitment
  (integrity + no silent task swapping); after execution the full task
  text and results are committed under `docs/validation/evidence/VWO-016/`
  and the ledger row is marked executed.
- **Anti-cheating (binds VWO-016 workers; restated from VWO-016):** no
  peeking at expected action sequences (none exist for held-out tasks —
  authors write goals, not scripts); no hard-coded scenario IDs in workflow
  semantics; no direct internal workflow-store construction; no
  benchmark-specific planner/runtime; model text is never execution
  evidence. Workers must not inspect benchmark-specific workflow plans
  before executing (VALIDATION-PROGRAM.md §8).

### 7.5 Ledger format (empty at v1.0.0 — filled at sealing time)

```yaml
# docs/validation/north-star/BENCHMARK-CATALOG.md §7.5 (appended by Tech Lead)
heldout-ledger:
  # - nst-id: NST-H001
  #   sha256: <64-hex of canonical task text>
  #   strata: [browser-containing, recovery-required, INSTRUCT, ...]
  #   sealed: <UTC timestamp>
  #   executed: <UTC timestamp or null>
```

v1.0.0 ships the ledger **empty** (zero rows) — by design.

---

## 8. Sizing and lane-coverage claims

| Lane | Tasks | Executing WO | Minimum for the WO's coverage claim |
|---|---|---|---|
| NST-D | 10 | VWO-012 | ≥8 required → 10 enumerated ✓ |
| NST-B | 10 | VWO-013 | ≥8 required → 10 enumerated ✓ |
| NST-T | 10 | VWO-014 | ≥8 required → 10 enumerated ✓ |
| NST-X | 8 | VWO-015 | ≥6 required → 8 enumerated ✓ |
| NST-H | procedure only | VWO-016 | pool ≥12 authored, ≥10 executed, ≥3 authors (§7) |

**VWO-required-family coverage per lane** (every bullet family of the
executing WO's "Required task families" maps to ≥1 task; gaps are explicitly
conditional or noted):

- **VWO-012 families → NST-D:** launch/focus/switch → D003; windows/dialogs/
  menus/forms → D004; keyboard-only → D001; mouse click/drag/scroll → D002;
  clipboard → D003; file picker/open/save dialogs → D005; filesystem
  navigation → D006; create/rename/move/copy/delete → D006; document editing
  → D007; spreadsheet → D008; presentation/media manipulation *where
  tooling exists* → D009 (conditional family by VWO-012's own wording);
  screenshots/visual confirmation → evidence requirement on every D task +
  explicit on-screen confirmation in D009; transient dialogs/notifications/
  focus changes → D004 (+D010); recovery after unexpected GUI state → D010.
  **GUI↔filesystem/terminal crossing ≥3** → D003, D005, D006, D007, D010 ✓
  (5 tasks).
- **VWO-013 families → NST-B:** navigation/multi-page journeys → B003;
  dynamic forms/validation → B002; tables/search/filters/pagination →
  B003; uploads → B004; downloads → B005; authentication/session renewal →
  B006; multi-tab → B001; redirects/transient failures → B007; rich
  editors/attachments → B008; browser-to-API handoffs → B009;
  browser-to-terminal/file handoffs → B010; visually ambiguous/changing UI
  → B002 (skeleton/delayed states); cross-site/multi-application → B001
  (distinct origins), B005/B009/B010. **Human-authored ≥3** → B002, B008,
  B010 ✓.
- **VWO-014 families → NST-T:** shell pipelines → T001; environment/process
  inspection → T002; file discovery/transformation → T003; text and
  structured-data manipulation → T003/T004; archives → T005; local service
  interaction → T002; Git operations → T006; build/test/lint → T007; log
  analysis → T001; code editing/repository changes → T008; CLI auth via
  approved bindings → T009; terminal↔browser/GUI transitions → T002
  (browser confirmation), T004 (API source), T009 (API-first);
  recovery from command failure/stale state/restart → T002, T008, T010.
  **Terminal as one stage of a larger mixed workflow ≥3** → T002, T004,
  T009, T010 ✓ (4 tasks).
- **VWO-015 requirements → NST-X:** ≥6 tasks crossing ≥2 modalities → all
  8 ✓; ≥3 crossing ≥3 modalities → X001, X002, X003, X004, X007 ✓ (5);
  ≥1 long-running resumable after restart → X004 ✓; the three
  representative patterns → X001 (pattern 1), X002 (pattern 2), X003
  (pattern 3) ✓.

---

## 9. Taxonomy value coverage (self-check #1 input)

Every TD value with exemplar tasks; values with no task get an explicit
note here (there are none at dimension level; value-level gaps are stated):

| Dimension | Values with ≥1 task | Notes / explicit gaps |
|---|---|---|
| TD-01 | browser: D001, B001…; desktop-gui: D010 (mixed set); terminal-cli: T005 (pure), T006, T007; filesystem: D006, D007, T003, T005; api-tool-mcp: T009 (pure), T004 (mixed); mixed: D003, D005, B005, B009, B010, T001, T002, X001–X008 | **MCP-specific gap (value-level, explicit):** no MCP server exists in the fallback proving ground (VWO-001 §1: ABSENT). `api-tool-mcp` is exercised via HTTP APIs and platform tools only. **out-of-scope-v1** for MCP-server-specific behavior; revisit when an MCP surface lands (also recorded in COVERAGE-MATRIX.md). |
| TD-02 | gui: D001–D010; browser-ui: B001–B010; terminal: T001–T010; file-op: D006, D007, T003; api-tool-call: B009, T004, T009, X002/X003; human: X001, X002, X006; mixed: D005, B009, B010, T002, X001… | none |
| TD-03 | stateless: B003, T001, T003, T005; stateful-session: most D/B/T tasks; long-running-resumable: X004 | none |
| TD-04 | linear: many; conditional-branching: D004, B002, B006, B007, B009, T002, T007, T008, T010, X001, X003, X006, X008 | none |
| TD-05 | sequential: all except X006; parallel-fanout: X006 | none |
| TD-06 | single-application: D001, D002, B002…; cross-application: D003, D005, B001, B009, B010, T002, T004, X001–X008 | none |
| TD-07 | structured: B003, T001, T004…; unstructured-text: D004, D007, T005, T006…; unstructured-visual-media: D002, D009, B008, X007; mixed: D005, B004, B008, X001, X002, X005 | none |
| TD-08 | none: most; upload: B004, B008, D005, X007; download: B005, B010, D005, X001, X003; upload+download: D005, X007 | D005 records upload+download in one task |
| TD-09 | unattended: all except X001/X002/X006; human-in-the-loop: X001, X002, X006 | none |
| TD-10 | none: D004, D006–D009, T001, T003, T005–T008, T010; authentication: D001, B003, B005, B006, B007, T006, X004, X005; authorization: D002, B004, X006, X007; authentication+authorization: D005, B001, B009, T009, X001, X002, X003, X008 | none |
| TD-11 | happy-path-only: many; recovery-required: B006, B007, T002, T008, X001, X004, X008; adaptation-required: D002, B004, T003; recovery+adaptation: T010 | none |
| TD-12 | reversible: most; destructive-irreversible: D006 (delete), X001 (publication) | none |
| TD-13 | scripted: 31 tasks; human-authored: D002, D007, D009, B002, B008, B010, T004, T007, X007 (9 tasks); held-out: NST-H procedure (§7) | held-out tasks are not enumerable by design |

No placement errata are open at v1.0.0. Difficulty values and justifications
were reconciled against the §4 scale before the catalog's establishing commit;
future placement errata follow SCORING.md §8 (PATCH bump, erratum register
entry, no silent edits).

---

## 10. Teaching-mode distribution (canonical bindings)

| Lane | DEMONSTRATE | INSTRUCT | HYBRID |
|---|---|---|---|
| NST-D (10) | D001, D004, D010 | D002, D006, D008 | D003, D005, D007, D009 |
| NST-B (10) | B003, B004, B007 | B002, B006, B009 | B001, B005, B008, B010 |
| NST-T (10) | T001, T003, T005 | T002, T007, T009 | T004, T006, T008, T010 |
| NST-X (8) | X004 | X002, X006, X008 | X001, X003, X005, X007 |
| NST-H | rotated by §7.3 (each mode ≥2 in the executed draw) | | |

Every lane covers all three modes (VWO-012 acceptance: modes across the
worker pool; program §6). Additional runs in non-canonical modes are
recorded as such (scenario-catalog convention).

---

## 11. What this catalog does NOT claim

- It is not a finite definition of the computer-task space (north-star
  README claim boundary; VALIDATION-PROGRAM.md §12: "A finite benchmark must
  not be described as mathematical proof of all possible computer tasks").
- Executing these tasks proves coverage OF THESE TASKS AND THEIR TAXONOMY
  CELLS, not universal capability — the VWO-017 verdict discipline governs
  any broader claim.
- Tasks requiring surfaces the product does not yet mount (Family A) are
  kept at full ambition; BLOCKED outcomes are honest results that feed the
  remediation loop — never reasons to narrow tasks or claim passes.
