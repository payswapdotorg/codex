# Canonical Prompt 02 — Codex Universal Human-Workflow Validation & Adversarial Product Testing

The Tech Lead must execute the full human-workflow validation program without waiting for the operator to repost the prompt.

Use:

`docs/validation/VALIDATION-PROGRAM.md`

as the authoritative executable specification.

The validation must:

- use up to three workers concurrently;
- prefer isolated E2B desktop-capable proving grounds through the connected Composio/E2B integration;
- fall back explicitly when desktop E2B is unavailable;
- build/use realistic synthetic enterprise applications;
- exercise DEMONSTRATE, INSTRUCT and HYBRID teaching modes;
- test construction, software/technology, ride-share, media and marketplace workflows;
- exercise creation, compilation, approval, publication, installation, configuration, scheduling, events, execution, recovery, upgrades, rollback and governed evolution;
- deliberately test restart/session-loss, duplicate triggers, provider/environment failure, prompt injection, credential exfiltration, authorization races, version confusion, cancellation races and marketplace entitlement races;
- test human usability and product friction, not just code behavior;
- never bypass the real application path when one exists;
- record missing product surfaces rather than fabricating them;
- classify findings P0/P1/P2/P3;
- create bounded remediation Work Orders for P0/P1 findings;
- re-run failing scenarios after fixes;
- push all plans, reports, evidence summaries, remediation commits and the final readiness report to GitHub.

The final acceptance question is whether a normal person can use Codex Universal end-to-end without understanding its internal architecture.

Detailed scenario definitions, Work Orders, concurrency, evidence requirements and completion gates are in `docs/validation/VALIDATION-PROGRAM.md` and `docs/validation/work-orders/`.
