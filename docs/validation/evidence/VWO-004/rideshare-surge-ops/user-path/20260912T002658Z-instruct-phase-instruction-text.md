# rideshare-surge-ops — HYBRID Phase 1 (INSTRUCT)

Instruction text the human prepared for Codex Universal (canonical content
from SCENARIO-CATALOG.md, entry rideshare-surge-ops step 6):

  "Watch region demand. Raise the surge multiplier when demand exceeds
   supply by 2×. Every surge change is broadcast to that region's drivers.
   Only the ops director or a regional ops manager may change surge. If
   two ops managers change the same region around the same time, the
   conflict must be surfaced explicitly — never silently."

Intended semantics: demand-threshold event trigger, surge decision gate
(surge:adjust), broadcast side-effect, conflict surfacing (no silent
last-write-wins), finance-analyst exclusion.

SUBMISSION NOT POSSIBLE — no Codex Universal teaching surface at this SHA
(root cause as construction-daily-progress step06). Recorded per
TEACHING-MODE-MATRIX.md §3.
