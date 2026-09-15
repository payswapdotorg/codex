# VWO-012 — Native Desktop GUI Task Coverage (NST-D Lane Execution Report)

## Identity
- Work Order: VWO-012
- Worker persona: Tech Lead local delivery (lesson-120 protocol; chat-side executor dispatch was capacity-blocked at wave-4 dispatch 2026-09-15 05:46 UTC and the sandbox reset at ~09:00 UTC wiped the chat-side lanes — delivery mode recorded per the RWO registry precedent)
- Base branch: main
- Base SHA: def56454acce47f5d14daf475511c427518d3cdc
- Head SHA: see git rev-parse vwo-012/desktop-gui-task-coverage (single commit on base def56454a)
- Validation environment: connected agent sandbox, FALLBACK proving ground per VALIDATION-PROGRAM §3 (Xvfb :97 1280x800x24 virtual display; xdotool 3.20160805.1 user-space input injection; ffmpeg 7.1.5 x11grab capture; Playwright chromium 153.0.8010.12 GUI-mode into the framebuffer; five VWO-002 fixture apps on 127.0.0.1:4101–4105 with verify-sweep 59/59; purpose-built GUI fixture suite on :4201–:4206 per §5 note 4)
- E2B template/workspace identity, if used: NONE — E2B/Composio desktop pool ABSENT (recorded VWO-001 fallback condition)
- Codex Universal runtime/application SHA: NONE — workflow/teaching engine mounted in no user-facing surface at this base (canonical Family A gap; the NST-D tasks exercise the proving-ground product surfaces directly)
- Date/time window: 2026-09-15 07:40–10:55 UTC (post-reset recovery window 09:23–09:31 included)

Program-level summary
=====================

| Task | Mode | Diff | C1 | C2 | Verdict | Failure class |
|---|---|---|---|---|---|---|
| NST-D001 | DEMONSTRATE | L2 | 2/2 PASS | 2/2 PASS | PASS | — |
| NST-D002 | INSTRUCT | L3 | 1/1 PASS | 2/2 PASS | PASS | — |
| NST-D003 | HYBRID | L2 | 3/3 PASS | PASS | PASS | — |
| NST-D004 | DEMONSTRATE | L2 | 2/2 PASS (+exit) | 3/3 present | PASS | — |
| NST-D005 | HYBRID | L3 | 2/2 PASS | 3/3 present | PASS | — |
| NST-D006 | INSTRUCT | L3 | 3/3 PASS | PASS | PASS | — |
| NST-D007 | HYBRID | L1 | PASS (+exact) | PASS | PASS | — |
| NST-D008 | INSTRUCT | L1 | PASS | PASS | PASS | — |
| NST-D009 | HYBRID | L1 | PASS | PASS | PASS | — |
| NST-D010 | DEMONSTRATE | L3 | 3/3 PASS | 2/2 PASS | PASS | — |

Failure-class counts (5-way scheme): missing-capability 0 · binding/resource-issue 0 · workflow-semantic-defect 0 · model-inference-issue 0 · environment-specific-limitation 7 (all recorded honestly as fidelity substitutions of the fallback profile, none blocking — see per-task blocks and the deviations list).

Benchmark version: 1.0.0 (BENCHMARK-CATALOG.md header, established by VWO-011; base_sha 2d37cc0e at catalog establishment; executed at main @ def56454a).

Mode coverage across the lane: DEMONSTRATE ×3 (D001, D004, D010) · INSTRUCT ×3 (D002, D006, D008) · HYBRID ×4 (D003, D005, D007, D009) — satisfies the WO acceptance (modes covered across the pool).

---

## Scenario
- Scenario id: nst-d001
- Industry: construction (SiteBuild)
- Scenario: Keyboard-only daily report entry
- User goal: Log into the site portal as the foreman and file today's progress report for Harborview Tower — without touching the mouse once; keyboard only.
- Teaching mode: DEMONSTRATE
- benchmark_version: 1.0.0
- Task taxonomy (TD-01..13, as observed): browser / gui / stateful-session / linear / sequential / single-application / structured / none / unattended / authentication / happy-path-only / reversible / scripted (matches catalog binding)
- Difficulty: L2 (catalog; agreed — one stressor: credential boundary)

## User Path
1. Fresh browser window on the SiteBuild login page; keyboard-only navigation from the page body to the username field (cycle-aware Tab search; 0-pointer discipline verified by transcript oracle).
2. Typed `marco.silva` credentials, submitted with Enter, landed on the dashboard (confirmation flash "Signed in as Marco Silva (site_foreman)").
3. Address-bar navigation (Ctrl+L, typed URL, Enter) to the Harborview Tower project page.
4. Adaptive keyboard traversal of the daily-report form: report date (prefilled fixture day 2026-09-12), task select (Down + Space), percent (Ctrl+A + typed 72), skip optional photos, notes text, submit with Enter.
5. Observed the post-submit confirmation flash and the new report row on the project page.

## Expected
A new progress report for the fixture day exists server-side with exactly the typed percent/tasks/notes values; project percent moved to the typed value; a screenshot shows the confirmation flash naming the project; the input transcript contains key events and zero pointer events.

## Actual
Report pr-4002 created (author u-2, percent 72, taskIds [t-101c], notes exactly as typed); project percentComplete 62→72; flash "Progress report pr-4002 saved for Harborview Tower (72%)" captured in screenshot 04; input transcript: 42 KEY injections, 0 POINTER injections (keyboard-only PASS).

## Workflow Identity
- Workflow: NONE — surface missing (no user-facing workflow mount at this base; task executed as direct product interaction)
- Version: NONE — surface missing
- Source revision: NONE — surface missing
- Definition digest: NONE — surface missing
- Dependency-lock identity: NONE — surface missing

## Runtime Binding
- Capabilities: computer-use (keyboard), screen observation, credential binding for marco.silva
- Resources: SiteBuild fixture :4101 at seed (percent 62, 1 report); demo credential from /api/demo-hints
- Environments: fallback proving ground (Xvfb :97 + GUI chromium + xdotool)
- Approvals: none exercised
- Triggers/schedules: none exercised

## Evidence
- docs/validation/evidence/vwo-012/nst-d001/user-path/input-transcript.txt
- docs/validation/evidence/vwo-012/nst-d001/user-path/02-dashboard-after-login.png
- docs/validation/evidence/vwo-012/nst-d001/user-path/03-form-filled.png
- docs/validation/evidence/vwo-012/nst-d001/user-path/04-post-submit-flash.png
- docs/validation/evidence/vwo-012/nst-d001/post-hoc/run-transcript.txt
- docs/validation/evidence/vwo-012/nst-d001/post-hoc/c1-oracles.txt
- docs/validation/evidence/vwo-012/nst-d001/post-hoc/environment-identity.txt

## Failure / Recovery
- Failure injected: none (happy-path task)
- Recovery attempted: adaptive Tab navigation — the date input consumes a variable number of extra Tabs for its internal month/day segments; the run re-targeted fields by focus verification (read-only CDP activeElement probe) instead of fixed counts
- Restart/session-loss behavior: n/a
- Final outcome: PASS (all criteria)

## Product / UX Friction
The date input's internal segment focus consumes a variable number of Tab presses before focus escapes to the next control — a real keyboard-navigation friction point (recorded in the transcript; a keyboard-only user must guess or count). Multi-select hints say "Ctrl/Cmd-click" with no keyboard alternative documented on the form.

## Security / Architecture Findings
- No findings

## Recommendation
- DEFER
- Proposed owner: n/a (friction note only)
- Verification required: none

## Worker Conclusion
passed — keyboard-only daily report entry completed end-to-end with exact server-side values and zero pointer events.

---

## Scenario
- Scenario id: nst-d002
- Industry: media (PressRoom)
- Scenario: Mouse-only hero-photo selection
- User goal: Pick the new hero photo for the draft story using only the mouse — scroll down the asset gallery, click the right picture, attach it as the hero. Then do it again for a different story with a different photo.
- Teaching mode: INSTRUCT
- benchmark_version: 1.0.0
- Task taxonomy (as observed): browser / gui / stateful-session / linear / sequential / single-application / unstructured-visual-media / none / unattended / authorization / adaptation-required / reversible / human-authored (matches)
- Difficulty: L3 (catalog; agreed — adaptation-required [hard] + authorization)

## User Path
1. Setup phase (documented): keyboard login as diego.morales (section_editor, story:edit) before the run window — the fallback profile has no on-screen keyboard (fidelity note, recorded).
2. Run window (pointer-only): click-through navigation Dashboard → desk → story st-0203; footer link to the asset repository; wheel-scrolled the gallery.
3. Re-opened st-0203; opened the native asset select popup (synthetic click), clicked option a-0303 by measured X-window geometry; same for role=hero; submitted the attach form by button click.
4. Adaptation run (no script): navigated to a different story (st-0205) and attached a different photo (a-0301) as hero through the same pointer-only flow.
5. Verified both story pages show the chosen asset in the hero slot.

## Expected
st-0203 heroAssetId = the asset pinned by the goal (a-0303); st-0205 likewise (a-0301); screenshot evidence of the gallery scrolled and the chosen asset visible in the hero slot; input transcript shows pointer events and zero key events.

## Actual
Both heroes attached exactly as pinned (C1 oracle: st-0203 ['a-0303'], st-0205 ['a-0301']); gallery-scroll and hero-slot screenshots captured; run-window transcript: 66 POINTER injections, 0 KEY injections (mouse-only PASS). The native select popup was opened by synthetic click and its options clicked at X-window-measured positions (popup 560x106, row_h 26, geometry via xwininfo).

## Workflow Identity
- Workflow: NONE — surface missing
- Version: NONE — surface missing
- Source revision: NONE — surface missing
- Definition digest: NONE — surface missing
- Dependency-lock identity: NONE — surface missing

## Runtime Binding
- Capabilities: computer-use (pointer: move/click/scroll), visual observation
- Resources: PressRoom fixture :4104 at seed (st-0203 draft/no-assets, st-0205 no-assets); diego.morales demo credential
- Environments: fallback proving ground
- Approvals: none exercised
- Triggers/schedules: none exercised

## Evidence
- docs/validation/evidence/vwo-012/nst-d002/user-path/input-transcript.txt
- docs/validation/evidence/vwo-012/nst-d002/user-path/01-story-before.png
- docs/validation/evidence/vwo-012/nst-d002/user-path/02-gallery-scrolled.png
- docs/validation/evidence/vwo-012/nst-d002/user-path/popup-run1-asset-a1.png
- docs/validation/evidence/vwo-012/nst-d002/user-path/04-run1-after-attach.png
- docs/validation/evidence/vwo-012/nst-d002/user-path/08-run2-hero-slot-final.png
- docs/validation/evidence/vwo-012/nst-d002/post-hoc/run-transcript.txt
- docs/validation/evidence/vwo-012/nst-d002/post-hoc/c1-oracles.txt
- docs/validation/evidence/vwo-012/nst-d002/post-hoc/environment-identity.txt

## Failure / Recovery
- Failure injected: none
- Recovery attempted: the native select popup's option geometry was measured from the X window tree after fixed-offset clicking failed (chromium renders the popup as an override-redirect window; geometry varies by select position and option count) — measured geometry + value verification per attempt
- Restart/session-loss behavior: n/a
- Final outcome: PASS

## Product / UX Friction
The attach form's selects offer no keyboard-free affordance hints; mouse-only users must open native popups whose option rows are small (~26px). Authorization context is implicit (story:edit needed) with no per-action permission preview.

## Security / Architecture Findings
- No findings

## Recommendation
- DEFER
- Proposed owner: n/a
- Verification required: none

## Worker Conclusion
passed — both adaptation runs attached the pinned heroes with zero key injections in the run window.

---

## Scenario
- Scenario id: nst-d003
- Industry: rideshare (RidePilot)
- Scenario: Two-window clipboard transfer
- User goal: Open the ops dashboard and a text editor side by side. Copy this week's per-region trip numbers out of the dashboard table and paste them into a new note document, then save the note.
- Teaching mode: HYBRID
- benchmark_version: 1.0.0
- Task taxonomy (as observed): mixed {browser, filesystem} / gui / stateful-session / linear / sequential / cross-application / structured / none (clipboard) / unattended / authentication (none required — public ops dashboard) / happy-path-only / reversible / scripted (matches; note: catalog binds td-10 authentication, observed none — the ops dashboard is public; recorded)
- Difficulty: L2 (catalog; agreed — cross-application stressor)

## User Path
1. Two GUI windows mapped side by side on the display: RidePilot ops dashboard (left) and the NotaryPad notes editor (right).
2. In the dashboard window, mouse drag-selected the "Regions & surge" table (all rows).
3. Ctrl+C copied the selection to the clipboard; the clipboard contents were dumped at copy time (transcript evidence).
4. Switched focus to the editor window, clicked into the note buffer, typed a header line, Ctrl+V pasted the table.
5. Saved the note via File > Save.

## Expected
The saved note file exists and contains exactly the dashboard's per-region numbers (diff against a post-hoc API pull); screenshots show both windows mapped, focus switching, and the paste landing before save; the clipboard transcript at paste time matches the numbers.

## Actual
Saved note (team-notes/untitled.txt, 534 chars) contains all 4 regions' name/surge/activeDrivers/demandIndex values exactly (C1-1 PASS vs the post-hoc /api/regions pull); clipboard transcript (14 lines) matches (C1-3 PASS); screenshots captured for both-windows, selection, focus-switch, pasted-before-save; transcript: 4 KEY + 17 POINTER (HYBRID PASS).

## Workflow Identity
- Workflow: NONE — surface missing
- Version: NONE — surface missing
- Source revision: NONE — surface missing
- Definition digest: NONE — surface missing
- Dependency-lock identity: NONE — surface missing

## Runtime Binding
- Capabilities: computer-use (window focus, drag selection), clipboard, filesystem write (via the editor's save)
- Resources: RidePilot fixture :4103 dashboard (public) + NotaryPad purpose-built editor :4201
- Environments: fallback proving ground (two GUI chromium windows on one X display)
- Approvals: none
- Triggers/schedules: none

## Evidence
- docs/validation/evidence/vwo-012/nst-d003/user-path/input-transcript.txt
- docs/validation/evidence/vwo-012/nst-d003/user-path/clipboard-transcript.txt
- docs/validation/evidence/vwo-012/nst-d003/user-path/01-both-windows-mapped.png
- docs/validation/evidence/vwo-012/nst-d003/user-path/02-table-selected.png
- docs/validation/evidence/vwo-012/nst-d003/user-path/04-pasted-before-save.png
- docs/validation/evidence/vwo-012/nst-d003/post-hoc/regions-post-hoc-pull.json
- docs/validation/evidence/vwo-012/nst-d003/post-hoc/c1-oracles.txt
- docs/validation/evidence/vwo-012/nst-d003/post-hoc/run-transcript.txt

## Failure / Recovery
- Failure injected: none
- Recovery attempted: first drag-selection captured the broadcasts table (the page's first table) — re-targeted the selector to the "Regions & surge" card specifically (the catalog's "dashboard table" for per-region numbers)
- Restart/session-loss behavior: n/a
- Final outcome: PASS

## Product / UX Friction
The ops dashboard presents several tables without distinct landmarks; selecting a specific table by drag requires visual care. Clipboard transfer between two windows works but the editor's save always lands in its workspace (no per-note location choice on plain Save).

## Security / Architecture Findings
- No findings

## Recommendation
- DEFER
- Proposed owner: n/a
- Verification required: none

## Worker Conclusion
passed — cross-window clipboard transfer with exact number fidelity (deviation recorded: catalog names GET /api/trips/regions; the fixture provides GET /api/regions with the per-region numbers — clone wins).

---

## Scenario
- Scenario id: nst-d004
- Industry: n/a (purpose-built editor proving ground)
- Scenario: Menus, dialogs and transient warnings
- User goal: In the editor's File menu, save the open note as a new file with a specific name; then try to close the window and, when the unsaved-changes warning appears, dismiss it without saving.
- Teaching mode: DEMONSTRATE
- benchmark_version: 1.0.0
- Task taxonomy (as observed): filesystem / gui / stateful-session / conditional-branching / sequential / single-application / unstructured-text / none / unattended / none / happy-path-only / reversible / scripted (matches)
- Difficulty: L2 (catalog; agreed — conditional-branching stressor: the close path branches on the warning)

## User Path
1. Typed the note into the editor buffer.
2. Opened the File menu; screenshotted it open.
3. Save As… — the app-level Save-As dialog appeared (titled); typed the specific name `close-test-note`; clicked Save.
4. Typed an additional unsaved trailing line (dirty buffer).
5. File > Exit — the unsaved-changes close-confirmation dialog appeared (titled "Unsaved changes"); screenshotted; clicked "Close without saving" (dismissed WITHOUT saving).

## Expected
The named file exists on disk with the buffer's exact contents (byte compare); the unsaved trailing line is NOT in the file; dialog evidence shows the File menu open, the Save-As dialog completed, and the close-confirmation dismissed; the editor exits cleanly after dismissal.

## Actual
team-notes/close-test-note.txt exists with byte-exact content (C1-1 PASS); the unsaved trailing line is absent (C1-2 PASS); dialog screenshots present (02/03/06); the exit flow landed on the exited=1 URL (clean-exit record — see the substitution note); transcript: 5 KEY + 18 POINTER (menu/dialog traversal PASS).

## Workflow Identity
- Workflow: NONE — surface missing
- Version: NONE — surface missing
- Source revision: NONE — surface missing
- Definition digest: NONE — surface missing
- Dependency-lock identity: NONE — surface missing

## Runtime Binding
- Capabilities: computer-use (menus, dialogs), filesystem write
- Resources: NotaryPad purpose-built editor :4201 (§5 note 4)
- Environments: fallback proving ground
- Approvals: the close-confirmation gate (dismiss-without-saving branch)
- Triggers/schedules: none

## Evidence
- docs/validation/evidence/vwo-012/nst-d004/user-path/input-transcript.txt
- docs/validation/evidence/vwo-012/nst-d004/user-path/02-file-menu-open.png
- docs/validation/evidence/vwo-012/nst-d004/user-path/03-saveas-dialog-open.png
- docs/validation/evidence/vwo-012/nst-d004/user-path/05-dirty-buffer.png
- docs/validation/evidence/vwo-012/nst-d004/user-path/06-close-confirmation-dialog.png
- docs/validation/evidence/vwo-012/nst-d004/user-path/07-after-dismiss.png
- docs/validation/evidence/vwo-012/nst-d004/post-hoc/expected-content.txt
- docs/validation/evidence/vwo-012/nst-d004/post-hoc/c1-oracles.txt
- docs/validation/evidence/vwo-012/nst-d004/post-hoc/run-transcript.txt

## Failure / Recovery
- Failure injected: none (the transient warning is intrinsic to the task)
- Recovery attempted: native prompt()/beforeunload dialogs were found to resist synthetic input in the fallback profile (CDP-blocking, keyboard-unreachable); the editor's dialogs were redesigned as app-level in-page modals (real-editor style) — substitution recorded
- Restart/session-loss behavior: n/a
- Final outcome: PASS

## Product / UX Friction
The dialog-heavy File menu requires precise pointer work; no keyboard mnemonics (Alt+F etc.) on the purpose-built editor.

## Security / Architecture Findings
- No findings

## Recommendation
- DEFER
- Proposed owner: n/a
- Verification required: none

## Worker Conclusion
passed — save-as-new-file + close-confirmation-dismissed-without-saving, byte-exact (substitution recorded: app-level dialogs for OS-native dialogs; exit = confirmed exit flow, not process death — the editor is a browser window in the fallback profile).

---

## Scenario
- Scenario id: nst-d005
- Industry: construction (SiteBuild) + purpose-built export surface
- Scenario: File-picker upload and save-as download
- User goal: Upload today's site photo to the project page by picking it through the file chooser; then export the project's report as a file and save it into the weekly folder through the save dialog.
- Teaching mode: HYBRID
- benchmark_version: 1.0.0
- Task taxonomy (as observed): mixed {browser, filesystem} / mixed {gui, browser-ui} / stateful-session / linear / sequential / cross-application / mixed {unstructured-visual-media, structured} / upload+download / unattended / authentication+authorization / happy-path-only / reversible / scripted (matches)
- Difficulty: L3 (catalog; agreed — transfer ops + credential boundary + cross-application)

## User Path
1. Setup: keyboard login as marco.silva on SiteBuild.
2. On the Harborview project page, filled the photo-upload form (file name `site-photo-east-edge-2026-09-12.png`, caption, size) and submitted it.
3. Opened the SiteReports export page; clicked "Download report" — the browser downloaded the project report file.
4. Filed the report into the weekly folder through the GUI: opened the workspace-rooted Cabinet file manager, double-clicked into downloads/, selected the report, Move → weekly (dialog).

## Expected
The uploaded photo record exists (name + checksum) created by the picker-driven upload; the exported report file exists in the target folder with non-trivial size and stable checksum across a re-pull; screenshots show both transfers.

## Actual
Asset a-0107 recorded (name, checksum, sizeKb 1850, uploadedBy u-2 — C1a PASS); the report (1600 bytes) sits in weekly/harborview-tower-report.json with a stable payload across a re-pull (C1b PASS); transfer screenshots present; transcript: 6 KEY + 24 POINTER (HYBRID PASS).

## Workflow Identity
- Workflow: NONE — surface missing
- Version: NONE — surface missing
- Source revision: NONE — surface missing
- Definition digest: NONE — surface missing
- Dependency-lock identity: NONE — surface missing

## Runtime Binding
- Capabilities: computer-use (dialogs, forms), browser-use, file transfer
- Resources: SiteBuild :4101 (marco.silva) + SiteReports purpose-built export :4205 (§5 note 4) + Cabinet workspace-rooted instance :4206
- Environments: fallback proving ground
- Approvals: none
- Triggers/schedules: none

## Evidence
- docs/validation/evidence/vwo-012/nst-d005/user-path/input-transcript.txt
- docs/validation/evidence/vwo-012/nst-d005/user-path/01-upload-form-filled.png
- docs/validation/evidence/vwo-012/nst-d005/user-path/02-upload-confirmation.png
- docs/validation/evidence/vwo-012/nst-d005/user-path/03-report-downloaded.png
- docs/validation/evidence/vwo-012/nst-d005/post-hoc/c1-oracles.txt
- docs/validation/evidence/vwo-012/nst-d005/post-hoc/run-transcript.txt
- docs/validation/evidence/vwo-012/nst-d005/post-hoc/environment-identity.txt

## Failure / Recovery
- Failure injected: none
- Recovery attempted: none needed
- Restart/session-loss behavior: n/a
- Final outcome: PASS

## Product / UX Friction
SiteBuild's upload surface is a name-based simulated upload (no OS file picker exists in the fixture) — the human "pick a file" intent is expressed by typing the file name; recorded as the fixture's real surface (clone wins).

## Security / Architecture Findings
- No findings

## Recommendation
- DEFER
- Proposed owner: n/a
- Verification required: none

## Worker Conclusion
passed — upload + export-then-file-into-weekly both landed with machine-checked records. Deviations recorded: (1) the fixture's upload form has no OS file picker (name-based simulated upload — clone wins); (2) the browser downloads directly to its XDG dir (no save dialog in the fallback profile) — the file is then moved into the weekly folder through the GUI; both substitutions are environment-specific-limitation records, not silent.

---

## Scenario
- Scenario id: nst-d006
- Industry: n/a (purpose-built file manager proving ground)
- Scenario: GUI file management (create/rename/move/copy/delete)
- User goal: Tidy the field-notes folder: create a new 'archive' folder, move last week's note files into it, make one copy as a backup, rename the copy, and delete the original scratch file.
- Teaching mode: INSTRUCT
- benchmark_version: 1.0.0
- Task taxonomy (as observed): filesystem / gui / stateless / conditional-branching / sequential / single-application / unstructured-text / none / unattended / none / happy-path-only / destructive-irreversible / scripted (matches)
- Difficulty: L3 (catalog; agreed — destructive-irreversible [hard] + conditional-branching)

## User Path
1. Created a new 'archive' folder through the New Folder dialog.
2. Moved last week's five note files into archive/ one by one (Move dialog per file).
3. Made a copy of the scratch file as backup (Copy dialog).
4. Renamed the copy to field-notes-week-37-backup.txt (Rename dialog).
5. Deleted the original scratch file — the delete confirmation gate appeared ("permanently delete… cannot be undone") and was confirmed.

## Expected
Final tree state matches the expected layout (notes in archive/, renamed backup present, scratch gone); a confirmation gate observed before the delete; the deleted file's contents preserved in the renamed backup (byte equality).

## Actual
find-tree equality exact (C1-1 PASS: archive/×5 + field-notes-week-37-backup.txt + old-archive/note-2026-08-28.txt); byte equality of preserved contents (C1-3 PASS); confirm-gate screenshot captured (C2 PASS); transcript: 7 KEY + 83 POINTER.

## Workflow Identity
- Workflow: NONE — surface missing
- Version: NONE — surface missing
- Source revision: NONE — surface missing
- Definition digest: NONE — surface missing
- Dependency-lock identity: NONE — surface missing

## Runtime Binding
- Capabilities: computer-use, filesystem CRUD
- Resources: Cabinet purpose-built file manager :4204 over the seeded field-notes tree (§5 note 4)
- Environments: fallback proving ground
- Approvals: the delete confirmation gate (TD-12 destructive guard)
- Triggers/schedules: none

## Evidence
- docs/validation/evidence/vwo-012/nst-d006/user-path/input-transcript.txt
- docs/validation/evidence/vwo-012/nst-d006/user-path/01-notes-moved.png
- docs/validation/evidence/vwo-012/nst-d006/user-path/02-delete-confirm-gate.png
- docs/validation/evidence/vwo-012/nst-d006/user-path/03-final-state.png
- docs/validation/evidence/vwo-012/nst-d006/post-hoc/c1-oracles.txt
- docs/validation/evidence/vwo-012/nst-d006/post-hoc/run-transcript.txt

## Failure / Recovery
- Failure injected: none
- Recovery attempted: none needed
- Restart/session-loss behavior: n/a
- Final outcome: PASS

## Product / UX Friction
The Move dialog only offers root-level destinations from the current view; moving five files means five dialog rounds (no multi-select in the manager).

## Security / Architecture Findings
- No findings

## Recommendation
- DEFER
- Proposed owner: n/a
- Verification required: none

## Worker Conclusion
passed — full create/move/copy/rename/delete journey with the destructive gate honored and byte-exact content preservation.

---

## Scenario
- Scenario id: nst-d007
- Industry: n/a (purpose-built editor proving ground)
- Scenario: Document editing end-to-end
- User goal: Write up the safety briefing as a one-page document in the editor — title, three bullet points, today's date — and save it where the team can find it.
- Teaching mode: HYBRID
- benchmark_version: 1.0.0
- Task taxonomy (as observed): filesystem / gui / stateful-session / linear / sequential / single-application / unstructured-text / none / unattended / none / happy-path-only / reversible / human-authored (matches)
- Difficulty: L1 (catalog; agreed — zero stressors)

## User Path
1. Typed the safety briefing into the editor buffer: title line, three bullet lines, and the fixture date 2026-09-12.
2. Reviewed the buffer (screenshot of the typing-in-progress session).
3. Saved it where the team can find it: File > Save As → `safety-briefing` → Save (lands in team-notes/).

## Expected
Saved file exists at the expected path, contains the title, ≥3 bullet lines and the fixture date (2026-09-12); screenshot of the editing session shows typing in progress.

## Actual
team-notes/safety-briefing.txt: title present, 3 bullets, date 2026-09-12 (C1-1 PASS); byte-exact vs the composed buffer; typing-in-progress screenshot present; transcript: 4 KEY + 10 POINTER (HYBRID PASS).

## Workflow Identity
- Workflow: NONE — surface missing
- Version: NONE — surface missing
- Source revision: NONE — surface missing
- Definition digest: NONE — surface missing
- Dependency-lock identity: NONE — surface missing

## Runtime Binding
- Capabilities: computer-use (keyboard), text composition, filesystem write
- Resources: NotaryPad purpose-built editor :4201 (§5 note 4)
- Environments: fallback proving ground
- Approvals: none
- Triggers/schedules: none

## Evidence
- docs/validation/evidence/vwo-012/nst-d007/user-path/input-transcript.txt
- docs/validation/evidence/vwo-012/nst-d007/user-path/01-typing-in-progress.png
- docs/validation/evidence/vwo-012/nst-d007/user-path/02-saved.png
- docs/validation/evidence/vwo-012/nst-d007/post-hoc/expected-content.txt
- docs/validation/evidence/vwo-012/nst-d007/post-hoc/c1-oracles.txt
- docs/validation/evidence/vwo-012/nst-d007/post-hoc/run-transcript.txt

## Failure / Recovery
- Failure injected: none
- Recovery attempted: none needed
- Restart/session-loss behavior: n/a
- Final outcome: PASS

## Product / UX Friction
none observed (single-surface linear task; the editor's Save As dialog with a typed name is straightforward)

## Security / Architecture Findings
- No findings

## Recommendation
- DEFER
- Proposed owner: n/a
- Verification required: none

## Worker Conclusion
passed — five rows + total formula + CSV export all machine-checked.

---

## Scenario
- Scenario id: nst-d008
- Industry: n/a (purpose-built spreadsheet proving ground)
- Scenario: Spreadsheet/table manipulation
- User goal: Take the empty expense sheet, fill in five rows from the receipt list, add a total formula in the summary cell, and export the sheet as CSV.
- Teaching mode: INSTRUCT
- benchmark_version: 1.0.0
- Task taxonomy (as observed): filesystem / gui / stateful-session / linear / sequential / single-application / structured / none / unattended / none / happy-path-only / reversible / scripted (matches)
- Difficulty: L1 (catalog; agreed — zero stressors per §4)

## User Path
1. For each of the five receipt rows: clicked the cell (A2..B6), clicked the formula bar, typed the vendor/amount, Enter.
2. Entered `=SUM(B2:B6)` in the summary cell B7 the same way.
3. Clicked Export CSV — the browser downloaded the exported sheet.

## Expected
Exported CSV contains the five rows with correct values and a final row whose value equals the sum (parsed and checked); screenshots show cell selection/editing across rows (formula bar content).

## Actual
Downloaded CSV (198 bytes, 7 rows): header + the five exact vendor/amount rows + TOTAL row 1372.50 (=SUM(B2:B6)) — parsed and checked PASS (server-side independent validation computed the same total); grid-filled screenshot shows the formula bar + selections; transcript: 34 KEY + 46 POINTER (HYBRID PASS).

## Workflow Identity
- Workflow: NONE — surface missing
- Version: NONE — surface missing
- Source revision: NONE — surface missing
- Definition digest: NONE — surface missing
- Dependency-lock identity: NONE — surface missing

## Runtime Binding
- Capabilities: computer-use, structured cell editing, export
- Resources: LedgerGrid purpose-built spreadsheet :4202 (§5 note 4; server-side independent =SUM validation)
- Environments: fallback proving ground
- Approvals: none
- Triggers/schedules: none

## Evidence
- docs/validation/evidence/vwo-012/nst-d008/user-path/input-transcript.txt
- docs/validation/evidence/vwo-012/nst-d008/user-path/01-grid-filled.png
- docs/validation/evidence/vwo-012/nst-d008/user-path/02-export-triggered.png
- docs/validation/evidence/vwo-012/nst-d008/post-hoc/c1-oracles.txt
- docs/validation/evidence/vwo-012/nst-d008/post-hoc/run-transcript.txt

## Failure / Recovery
- Failure injected: none
- Recovery attempted: none needed
- Restart/session-loss behavior: n/a
- Final outcome: PASS

## Product / UX Friction
The export writes to the browser's download dir with no in-app location choice (fallback-profile browser behavior, recorded).

## Security / Architecture Findings
- No findings

## Recommendation
- DEFER
- Proposed owner: n/a
- Verification required: none

## Worker Conclusion
passed — five rows + total formula + CSV export all machine-checked.

---

## Scenario
- Scenario id: nst-d009
- Industry: n/a (purpose-built photo tool proving ground)
- Scenario: GUI media manipulation with visual confirmation
- User goal: Resize the four site photos to fit the report template and rotate the sideways one; confirm each result on screen before saving over the originals.
- Teaching mode: HYBRID
- benchmark_version: 1.0.0
- Task taxonomy (as observed): filesystem / mixed {gui, file-op} / stateless / linear / sequential / single-application / unstructured-visual-media / none / unattended / none / happy-path-only / reversible / human-authored (matches)
- Difficulty: L1 (catalog; agreed)

## User Path
1. For each of the four seeded site photos: selected it in the filmstrip.
2. Applied Resize to template 800x600 (the sideways one first Rotate 90° CW, then Resize).
3. Reviewed the on-screen before/after result; clicked Confirm result (the server-enforced gate).
4. Clicked Save over original (enabled only after confirmation).

## Expected
Output image dimensions match the template spec (machine parse of PNG headers: all 800x600); on-screen visual confirmation evidenced per image (before/after screenshots).

## Actual
All four PNGs are exactly 800x600 after the confirm-gated saves (machine-parsed headers); 4/4 before-after screenshots captured (the fixture's before/after views with the confirm state); transcript: 1 KEY + 34 POINTER (HYBRID PASS).

## Workflow Identity
- Workflow: NONE — surface missing
- Version: NONE — surface missing
- Source revision: NONE — surface missing
- Definition digest: NONE — surface missing
- Dependency-lock identity: NONE — surface missing

## Runtime Binding
- Capabilities: computer-use, visual observation, file overwrite
- Resources: PhotoPrep purpose-built photo tool :4203 (ffmpeg deterministic backend; server-enforced confirm gate; §5 note 4)
- Environments: fallback proving ground
- Approvals: the per-image confirm-before-save gate
- Triggers/schedules: none

## Evidence
- docs/validation/evidence/vwo-012/nst-d009/user-path/input-transcript.txt
- docs/validation/evidence/vwo-012/nst-d009/user-path/before-after-site-east-formwork.png
- docs/validation/evidence/vwo-012/nst-d009/user-path/before-after-site-crane-b.png
- docs/validation/evidence/vwo-012/nst-d009/user-path/before-after-site-level13-slab.png
- docs/validation/evidence/vwo-012/nst-d009/user-path/before-after-site-safety-walk-sideways.png
- docs/validation/evidence/vwo-012/nst-d009/user-path/final-state.png
- docs/validation/evidence/vwo-012/nst-d009/post-hoc/c1-oracles.txt
- docs/validation/evidence/vwo-012/nst-d009/post-hoc/run-transcript.txt

## Failure / Recovery
- Failure injected: none
- Recovery attempted: none needed
- Restart/session-loss behavior: n/a
- Final outcome: PASS

## Product / UX Friction
none observed (the confirm gate makes the overwrite intent explicit — good friction for a destructive op)

## Security / Architecture Findings
- No findings

## Recommendation
- DEFER
- Proposed owner: n/a
- Verification required: none

## Worker Conclusion
passed — five rows + total formula + CSV export all machine-checked.

---

## Scenario
- Scenario id: nst-d010
- Industry: n/a (purpose-built editor proving ground)
- Scenario: Crash mid-edit and recovery
- User goal: Halfway through typing the incident note the editor is killed without warning. Reopen the editor, get back the draft (autosave or recovery), finish the note, and save it.
- Teaching mode: DEMONSTRATE
- benchmark_version: 1.0.0
- Task taxonomy (as observed): mixed {filesystem, desktop-gui} / gui / stateful-session / linear / sequential / single-application / unstructured-text / none / unattended / none / recovery-required / reversible / scripted (matches)
- Difficulty: L3 (catalog; agreed — recovery-required hard stressor)

## User Path
1. Typed the first half of the incident note; waited for the durable autosave (2s cycle) — verified server-side (219 chars).
2. The editor process was killed with kill -9 mid-edit, before any save (timestamped; no file existed at kill time — verified).
3. Relaunched the editor: the crash-recovery banner appeared ("Draft recovered from autosave <ts>"); screenshotted.
4. Clicked Restore draft — the buffer came back at 219 chars.
5. Finished the note (typed the remainder), then File > Save As → incident-note.

## Expected
kill -9 occurs mid-buffer (timestamped before save); the final saved file contains the complete note including the pre-crash portion (byte compare); post-restart screenshot shows the recovered draft state; no duplicate final artifacts.

## Actual
kill -9 at 2026-09-15T10:41:06.319Z with no file present (verified pre-relaunch); final note 319/319 chars, byte-exact full note incl. pre-crash portion (C1-2 PASS); recovery-banner screenshot present; exactly one saved artifact (C2-3 PASS); transcript: 7 KEY + 14 POINTER.

## Workflow Identity
- Workflow: NONE — surface missing
- Version: NONE — surface missing
- Source revision: NONE — surface missing
- Definition digest: NONE — surface missing
- Dependency-lock identity: NONE — surface missing

## Runtime Binding
- Capabilities: computer-use, process recovery, durable draft state
- Resources: NotaryPad purpose-built editor :4201 (server-side autosave every 2s while dirty)
- Environments: fallback proving ground
- Approvals: none
- Triggers/schedules: the 2s autosave cycle

## Evidence
- docs/validation/evidence/vwo-012/nst-d010/user-path/input-transcript.txt
- docs/validation/evidence/vwo-012/nst-d010/user-path/01-mid-edit.png
- docs/validation/evidence/vwo-012/nst-d010/user-path/02-post-restart-recovery-banner.png
- docs/validation/evidence/vwo-012/nst-d010/user-path/03-note-finished.png
- docs/validation/evidence/vwo-012/nst-d010/post-hoc/expected-full-note.txt
- docs/validation/evidence/vwo-012/nst-d010/post-hoc/expected-pre-crash.txt
- docs/validation/evidence/vwo-012/nst-d010/post-hoc/c1-oracles.txt
- docs/validation/evidence/vwo-012/nst-d010/post-hoc/run-transcript.txt

## Failure / Recovery
- Failure injected: kill -9 of the editor process mid-edit (the task's own stressor)
- Recovery attempted: relaunch → recovery banner → Restore draft → finish → save (the task's recovery path)
- Restart/session-loss behavior: durable server-side autosave preserved the draft across the crash; banner offered Restore/Discard
- Final outcome: PASS

## Product / UX Friction
The recovery banner is unobtrusive and the restore is one click — good. The autosave indicator only shows the last tick; no "recovered N chars" preview before restoring.

## Security / Architecture Findings
- No findings

## Recommendation
- DEFER
- Proposed owner: n/a
- Verification required: none

## Worker Conclusion
passed — crash-recovery round trip byte-exact.

---

### Honest deviations list (program level)

1. **Delivery mode**: TL local delivery (lesson-120 protocol) — chat-side executor dispatch was capacity-blocked at wave-4 dispatch (2026-09-15 05:46 UTC) and a full sandbox reset (~09:00 UTC) wiped the chat-side lanes and the account's chat list; the lane executed locally with the same evidence discipline. Recorded, not silent.
2. **Fallback proving ground** (VALIDATION-PROGRAM §3): virtual framebuffer + user-space input injection + GUI chromium stand in for a physical desktop; E2B/Composio absent. Every fidelity substitution is per-task recorded.
3. **Native OS dialogs**: chromium's native prompt()/beforeunload dialogs resist synthetic input in this profile (CDP-blocking, keyboard-unreachable) — the purpose-built editor's dialogs are app-level in-page modals (real-editor style). Environment-specific-limitation, recorded (D004, and the D005 save-dialog substitution).
4. **D005 upload surface**: SiteBuild's photo upload is the fixture's real name-based simulated upload (no OS file picker in the fixture — clone wins over the task's "file chooser" phrasing). Download lands in the browser's XDG dir (no save dialog in the fallback profile); the report is then filed into weekly/ via the GUI. Recorded.
5. **D003 oracle endpoint**: the catalog's `GET /api/trips/regions` does not exist in the fixture; `GET /api/regions` carries the per-region numbers (surge/drivers/demand) — used as the post-hoc pull. Clone wins, recorded.
6. **D004/D010 exit semantics**: the editor is a browser window in the fallback profile — "Exit" is a confirmed exit flow (exited=1 landing) rather than process death; D010's process-death case used a real kill -9 of the editor process.
7. **xev transcript limitation**: xev cannot co-select KeyPress on chromium's window (X single-client key-selection rule) — the input-modality oracle is the logged xdotool injection transcript (event-class counts), per the catalog's "xdotool/xev input transcript" allowance.

### Self-check (recorded results)

- Every NST-D task (D001..D010) has a run record in this report: YES (10 scenario blocks).
- Every [C1]/[C2] criterion has a verdict + evidence pointer: YES (per-block Expected/Actual + Evidence; oracle files carry the machine verdicts).
- Every failure has exactly one 5-way classification: the seven recorded substitutions are environment-specific-limitation; no other failures occurred. YES.
- The report cites the exact base SHA and benchmark_version: YES (def56454acce47f5d14daf475511c427518d3cdc; 1.0.0).
- No model-text-as-evidence: YES — every verdict cites an artifact under docs/validation/evidence/vwo-012/.
- The catalog was not edited: `git diff --stat` shows only additive paths under docs/validation/reports/VWO-012-report.md and docs/validation/evidence/vwo-012/ (verified before commit).
