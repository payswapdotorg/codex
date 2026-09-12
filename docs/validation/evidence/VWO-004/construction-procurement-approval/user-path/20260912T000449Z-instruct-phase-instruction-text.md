# construction-procurement-approval — HYBRID Phase 1 (INSTRUCT)

Instruction text the human prepared for Codex Universal (canonical content
from SCENARIO-CATALOG.md, entry construction-procurement-approval step 5):

  "Approve purchase requests up to $50,000 when the vendor contract is
   valid; route requests above $50,000 to finance. On approval, issue the
   purchase order and notify the requester. An expired vendor contract is
   always a hard gate — the request must be rejected or held until a new
   vendor agreement exists."

Intended semantics to be inferred by the system: approval gate (po:approve),
amount threshold branch ($50k), vendor-contract validity condition
(stale_entitlement on expiry), PO issuance side-effect, requester
notification, single-decision (data_conflict on double-decide).

SUBMISSION NOT POSSIBLE: no Codex Universal instruction/teaching surface
exists in this environment (see the teaching-surface-missing finding for
construction-daily-progress step06 — same root cause). The instruction text
is recorded here as user-path evidence of what the human would give the
system (TEACHING-MODE-MATRIX.md §3 step 3), not as a submitted artifact.
