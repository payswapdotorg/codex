# Codex Universal Architecture Lock

**Version:** 0.1.0-bootstrap
**Status:** FROZEN
**Effective base:** `4b0d9669cc46ba97bf85fa6312431b630d80498d`
**Supersession:** Architecture Change Request only.

## Frozen decisions

1. `payswapdotorg/codex` is the primary implementation repository.
2. Upstream OpenAI Codex is the runtime substrate, not a reference-only rewrite target.
3. The first architectural extension is a Universal Model Plane.
4. Provider-specific model APIs are adapters behind universal model contracts.
5. The Codex runtime must not become coupled to one model vendor.
6. The second architectural extension is the Workflow Plane.
7. Workflows are implemented inside this repository as a product/runtime layer over Codex primitives.
8. The workflow layer is semantically informed by `payswapdotorg/workflows` V1.1, but it is not a separate engine or copied repository.
9. Workflow definitions and published versions are durable semantic state owned by the workflow control plane.
10. Workflow execution may use Codex agents, native tools, Composio, browser automation, APIs, terminal execution, human execution, and future desktop/mobile adapters.
11. Execution modality and reasoning mode are separate dimensions.
12. Providers, models, browsers, tools, connectors, and execution environments are replaceable adapters.
13. LLM outputs and external execution outputs are untrusted input by default.
14. Credentials never become workflow semantics or ordinary evidence/memory/prompt content.
15. Published workflow versions are immutable.
16. Workflow triggers must pass through deterministic validation, authorization, idempotency, and state-transition logic.
17. Workflow learning produces candidates and new versions; it cannot silently mutate an active version.
18. Application clients are not workflow state authorities.
19. The Codex session/event protocol remains the canonical agent interaction surface.
20. Every behavior-changing feature requires tests and exact commit-tied verification evidence.

## Intentionally unfrozen implementation choices

The following remain implementation choices until the relevant Work Order freezes them:

- exact Rust crate split for the new model plane;
- exact storage implementation for workflow state;
- browser automation backend;
- cloud/remote execution topology;
- workflow package distribution service;
- model-routing algorithm;
- persistent memory implementation;
- desktop/mobile adapters.

## Architecture change rule

A change to a frozen decision requires an Architecture Change Request that states:

- current decision;
- proposed decision;
- motivation/evidence;
- affected invariants and interfaces;
- migration strategy;
- compatibility impact;
- test/verification impact.

Agents may propose such changes but may not silently implement them as ordinary feature work.
