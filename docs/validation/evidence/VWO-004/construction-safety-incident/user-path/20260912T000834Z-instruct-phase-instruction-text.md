# construction-safety-incident — INSTRUCT instruction text (canonical)

Instruction the human prepared for Codex Universal (canonical content from
SCENARIO-CATALOG.md, entry construction-safety-incident step 5):

  "Foreman reports a safety incident from the field (severity, category,
   description). The safety officer investigates and closes the incident
   within 48 hours. Near-misses additionally trigger a toolbox talk. The
   reporter is notified when the incident is closed; only the safety
   officer may close."

Intended semantics to be inferred: role split (safety:report vs
safety:read/safety:close), incident-created event trigger, 48h closure
timer/schedule, near-miss conditional branch (toolbox talk), closure
notification side-effect, permission gate on close.

SUBMISSION NOT POSSIBLE — no Codex Universal teaching/instruction surface
exists at this SHA (same root cause as construction-daily-progress step06).
The text is recorded as user-path evidence of what the human would give
the system per TEACHING-MODE-MATRIX.md §3 step 3.
