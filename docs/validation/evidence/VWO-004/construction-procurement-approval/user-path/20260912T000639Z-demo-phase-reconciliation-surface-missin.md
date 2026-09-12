# HYBRID Phase 3 (reconcile) — NOT PERFORMABLE: teaching surface missing

The binding reconciliation check (TEACHING-MODE-MATRIX.md §4 step 5) cannot
be executed: with the instruct phase (see instruct-phase-instruction-text.md)
and the demonstrated exception (prq-5001 expired-contract rejection; see
step05 artifacts) both completed on the application side, there is NO Codex
Universal surface that accepts instructions, observes demonstrations, or
produces a merged/reconciled workflow. Root cause identical to
construction-daily-progress step06: the workflow/teaching crates are
libraries with no user-facing product surface at SHA
0d314fd09cec70d000f33c81286edcd8cbf72501.

The instruction-vs-demonstration invariant ("the expired contract must
remain a hard gate, not be 'fixed' by the demonstration") could therefore
not be verified at the product level. What COULD be verified at the
application level: the expired-contract gate held through every path
attempted (approve blocked [stale_entitlement]; the switch-based conflict
left the record untouched; the second-session decide failed closed
[data_conflict]; exactly one decision landed — no double PO).
