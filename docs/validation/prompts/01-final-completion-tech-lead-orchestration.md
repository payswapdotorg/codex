# Canonical Prompt 01 — Final Completion Tech Lead Orchestration

This is the repository-native form of the final-completion prompt previously supplied in chat.

The Tech Lead must:

1. Re-read the frozen architecture and current source.
2. Treat WO-001..WO-015 as architect-reviewed, with WO-010/WO-011 not fully complete until the durable M4 control plane is implemented.
3. Create bounded M4 remediation Work Orders for durable stores, legal transitions, persisted execution position, restart reconciliation, cancellation, and cross-plane durability.
4. Use up to three workers concurrently on disjoint surfaces.
5. Verify every Work Order from current main rather than trusting historical status.
6. Run tests, lint, formatting, security checks and restart/recovery verification.
7. Reconcile stale development state.
8. Execute the human-workflow validation program in `docs/validation/VALIDATION-PROGRAM.md` after the product is stable enough to exercise.
9. Create remediation Work Orders for P0/P1 product or architecture findings and re-run the failing scenarios after every fix.
10. Push every accepted implementation, report, evidence summary and final conclusion to GitHub.
11. Do not declare final completion merely because the original 15 Work Orders are merged.
12. Declare completion only when architecture gates, durable control-plane gates, human-workflow validation, security tests and final product readiness all pass.

Detailed execution rules live in:

- `TECH_LEAD_START_HERE.md`
- `docs/validation/VALIDATION-PROGRAM.md`
- `docs/validation/validation-dependency-graph.json`
