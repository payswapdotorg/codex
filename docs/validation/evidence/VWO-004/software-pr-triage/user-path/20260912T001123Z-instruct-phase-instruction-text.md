# software-pr-triage — INSTRUCT instruction text (canonical)

Instruction the human prepared for Codex Universal (canonical content from
SCENARIO-CATALOG.md, entry software-pr-triage step 6):

  "CI must pass before review completes. A code owner other than the
   author must approve. Merging deploys the change to staging — never
   straight to production; promoting to production is a separate,
   release-manager-approved step. The author can never approve their own
   PR. A reviewer approves at most once."

Intended semantics to be inferred: CI-green gate, code-owner approval gate
(non-self), merge→staging deploy side-effect (+ build artifact), production
promotion as a separate approval gate, one-approval-per-reviewer
(duplicate rejection), PR-opened + CI-completed event triggers.

SUBMISSION NOT POSSIBLE — no Codex Universal teaching/instruction surface
exists at this SHA (same root cause as construction-daily-progress
step06). Recorded as user-path evidence per TEACHING-MODE-MATRIX.md §3.
