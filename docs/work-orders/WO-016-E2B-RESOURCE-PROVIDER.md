# WO-016 — E2B Resource Provider Adapter

**Status:** PARKED / NON-BLOCKING
**Architecture:** 0.2.0
**Scheduling rule:** Must not be dispatched until the current roadmap dependency graph reaches this Work Order.

## Objective

Add E2B as a concrete infrastructure/resource provider behind the existing Codex Universal execution and resource contracts, validating that the architecture can consume isolated compute and remote desktop infrastructure without introducing E2B-specific workflow semantics.

## Dependencies

Preferred dependency: `WO-015`.

This dependency is intentionally conservative: the current core roadmap must finish before E2B becomes implementation work. The Work Order is not part of the active dependency graph until the Tech Lead explicitly incorporates it after the current roadmap is complete.

## Authorized change surface

Only the smallest surfaces required to implement a provider-neutral resource adapter, for example:

- execution/resource provider adapter code;
- resource capability declarations;
- provider configuration/reference plumbing;
- scoped integration tests;
- provider-specific test fixtures/mocks;
- documentation describing the adapter boundary.

The exact paths must be determined from the current source tree when the Work Order becomes READY.

## Required source audit

Before coding, inspect:

1. `docs/architecture/CODEX-UNIVERSAL-ARCHITECTURE.md`;
2. `docs/architecture/CODEX-UNIVERSAL-LOCK.md`;
3. `docs/architecture/CODEX-CAPABILITY-IMPLEMENTATION-MAP.md`;
4. the current execution abstraction and `ResourceBinding` implementation;
5. the current `ComputerUseAdapter` implementation;
6. existing provider-adapter patterns;
7. existing configuration/secret/reference patterns;
8. current recovery, tracing, evidence, and authorization paths.

The live source tree at dispatch time is authoritative. Do not rely on this packet to predict future file locations.

## Architectural requirements

E2B must be modeled as an implementation/provider behind universal contracts.

The implementation MUST NOT:

- introduce a second agent runtime;
- introduce a second workflow engine;
- introduce E2B-specific workflow IR or semantic primitives;
- replace `ComputerUseAdapter` with an E2B-specific workflow contract;
- put E2B provider types into universal workflow semantics;
- treat E2B credentials as workflow data;
- infer authorization from resource availability;
- make E2B a mandatory dependency of Codex Universal.

A generic resource/provider model should be preferred over a one-off E2B-only abstraction when the existing architecture supports it.

## Functional target

Validate support for the following resource classes where E2B actually provides them:

- isolated Linux compute;
- remote graphical desktop suitable for Computer Use;
- lifecycle operations such as create, pause/resume, snapshot/checkpoint, or fork where the selected SDK/API supports them;
- configurable network behavior;
- resource health/capacity metadata;
- artifact/evidence capture through existing Codex mechanisms.

Do not implement unsupported E2B features merely to satisfy the Work Order.

## Acceptance criteria

1. A workflow can request a generic resource need and bind an E2B-backed resource without embedding E2B identifiers in workflow semantics.
2. E2B compute executes through the existing execution abstraction.
3. E2B Desktop, where selected, is reached through the existing Computer Use semantic contract.
4. Resource lifecycle actions are represented as resource lifecycle semantics rather than workflow semantics.
5. Provider/resource failure is surfaced through the existing recovery/error model and does not mutate workflow meaning.
6. Credentials/API keys are injected only through approved runtime configuration/reference mechanisms.
7. Workflow version/source/dependency pinning remains independent of E2B sandbox IDs, snapshots, or regions.
8. Evidence distinguishes provider/resource facts from workflow semantic facts.
9. At least one alternative/mock resource provider can satisfy the same universal contract in tests.
10. Existing Browser Use, Computer Use, workflow, and execution contract regressions remain green.

## Verification

At dispatch time, the Tech Lead must choose the exact repository-required commands from current `AGENTS.md` and source configuration. The verification report must include:

- scoped unit/integration tests;
- workspace regression tests for affected contracts;
- formatting;
- lint/clippy where applicable;
- compile/build verification;
- security/secrets scan appropriate to provider configuration changes.

All evidence must be tied to the exact verified commit SHA.

## Forbidden changes

Do not alter:

- frozen architectural invariants;
- current Work Order dependencies while other roadmap Work Orders are in flight;
- workflow immutability/pinning;
- authorization semantics;
- existing Codex-native Browser Use or Computer Use contracts;
- model-provider abstraction;
- Git forge semantics;
- marketplace/distribution semantics;
- credential handling rules.

## Non-disruption rule

This Work Order is deliberately parked. The active Tech Lead must continue implementing the existing roadmap without waiting for or integrating WO-016.

When the current roadmap reaches the provider/resource extension boundary, the Tech Lead may re-evaluate and activate this packet against the then-current architecture and source tree. It is valid to revise or retire this Work Order if the execution/resource architecture has changed while preserving the same invariant: infrastructure providers remain replaceable implementation details beneath universal semantics.

## Completion report

Use the standard completion report required by `docs/work-orders/README.md` and `TECH_LEAD_START_HERE.md`, with explicit evidence for provider neutrality, authorization separation, credential isolation, resource lifecycle mapping, and alternative-provider compatibility.
