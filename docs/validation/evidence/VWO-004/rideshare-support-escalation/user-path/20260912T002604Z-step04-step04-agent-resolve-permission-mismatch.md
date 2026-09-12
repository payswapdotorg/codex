# Step 4 deviation — catalog/permission mismatch: support agent cannot resolve

Catalog user-path step 4: "Resolve the ticket after the refund is issued;
observe closure" (persona: ana.silva, support agent).

The seeded permission model (rideshare/seed.js @ 0d314fd09cec70d000f33c81286edcd8cbf72501)
gives ana.silva only: driver:read, ticket:read, ticket:respond,
ticket:escalate, incident:report. `ticket:resolve` belongs to ops roles
(jules.moreau, farah.khan). The /support UI renders NO Resolve form for
the agent. Therefore the catalog's step-4-as-agent is NOT PERFORMABLE as
written. Observed instead (semantically coherent with the taught policy —
"escalation hands the ticket to ops"): regional ops manager jules.moreau
resolves the escalated ticket through the same browser surface. The
agent-side resolution gap is recorded as a catalog/fixture consistency
finding (harness-level), not simulated.
