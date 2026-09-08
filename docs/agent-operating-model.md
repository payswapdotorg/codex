# Codex Universal — Agent Operating Model

This document defines how autonomous agents work on the repository. It is subordinate to the frozen architecture and `TECH_LEAD_START_HERE.md`.

## Roles

| Role | Authority | May modify code? | Primary output |
|---|---|---:|---|
| Tech Lead | Program execution | Yes, when needed | dispatch decisions, integration, acceptance, state |
| Architect | Architecture | By approved architecture change only | frozen decisions, ACRs |
| Implementation Worker | Assigned Work Order | Yes | focused implementation + tests |
| Verification Worker | Assigned verification | Only in verification scope | test/evidence report |
| Review Worker | Adversarial review | Normally no | findings against architecture/WO |
| Research Worker | Repository/source research | No runtime semantics | source-backed audit |

## Ownership rules

1. Every code change belongs to exactly one active Work Order.
2. Every Work Order has one accountable Tech Lead owner at the program level and one implementation worker owner while dispatched.
3. A worker may ask another worker for research, but may not expand its own semantic ownership without a Tech Lead decision.
4. Shared foundational contracts must have one write owner at a time.
5. Generated/build/vendor artifacts are never treated as semantic ownership boundaries.

## Dispatch states

```text
BLOCKED -> READY -> DISPATCHED -> IMPLEMENTED -> VERIFYING -> ACCEPTED -> MERGED
                         \-> BLOCKED / NEEDS-ARCHITECTURE
```

## Worker branch rule

Workers create a branch from the exact SHA supplied by the Tech Lead. Worker branches are disposable implementation contexts. They must not force-push a shared branch and must not merge other Work Orders into themselves merely to make tests pass.

## Change budget

Prefer one coherent Work Order per PR. Keep mechanical formatting separate from semantic changes unless formatting is required to validate the implementation. Follow the repository's 800-line change guidance and split larger work into coherent stages.

## Verification evidence

Evidence must identify the exact SHA it verifies. Generic statements such as "tests pass" are insufficient.

Minimum evidence fields:

```text
work_order
base_sha
verified_sha
changed_paths
test_commands
test_results
lint/format_results
acceptance_criteria
compatibility_impact
security_review
architecture_divergence
known_limitations
```

## Escalation triggers

Escalate instead of guessing when a worker finds:

- a contradiction with the frozen architecture;
- a requirement owned by another Work Order;
- a provider/environment-specific type crossing a universal boundary;
- a need for a second runtime/control plane;
- an authorization/security bypass;
- an immutable-version violation;
- a material upstream compatibility break;
- a dependency or repository primitive that does not work as the Work Order assumes.

## Review checklist

A Tech Lead or review worker should answer:

1. Did the change implement only the assigned Work Order?
2. Did it reuse the existing Codex mechanism where one exists?
3. Are semantic contracts independent of model provider, environment, UI, GitHub, and credential internals?
4. Are durable state transitions owned by the correct control plane?
5. Are external outputs treated as untrusted?
6. Are credentials absent from source/logs/semantic evidence?
7. Are version/source/dependency identities immutable where required?
8. Are tests tied to the actual changed behavior?
9. Is upstream behavior unchanged unless the architecture explicitly permits divergence?
10. Could the next Work Order start from this result without reverse-engineering undocumented assumptions?

## Handoff principle

A worker's code is not the handoff. The combination of implementation, tests, exact SHA, acceptance evidence, known limitations, and updated state is the handoff.
