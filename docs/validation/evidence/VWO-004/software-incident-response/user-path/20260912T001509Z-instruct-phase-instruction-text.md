# software-incident-response — HYBRID Phase 1 (INSTRUCT)

Instruction text the human prepared for Codex Universal (canonical content
from SCENARIO-CATALOG.md, entry software-incident-response step 6):

  "When a production incident opens, the SRE on-call acknowledges and
   mitigates. The fix travels to production only after it is merged to
   staging AND the release manager promotes it from the terminal deploy
   console. Promotion is a human approval gate. Resolving the incident
   requires the SRE. Provider failures (CI runner pool down, artifact
   missing) must fail safe with no partial production state and allow a
   clean retry."

Intended semantics: incident-opened trigger, mitigate→promote→resolve
ordering, release-manager promotion gate bound to the TERMINAL surface
(mixed modality: browser + terminal), fail-safe promote semantics, retry
after provider recovery.

SUBMISSION NOT POSSIBLE — no Codex Universal teaching surface exists at
this SHA (root cause as construction-daily-progress step06). Recorded per
TEACHING-MODE-MATRIX.md §3.
