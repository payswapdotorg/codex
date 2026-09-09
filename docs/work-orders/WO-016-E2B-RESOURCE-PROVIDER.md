# WO-016 — Execution Resource Provider Plane

**Status:** PARKED / NON-BLOCKING
**Architecture:** 0.2.0
**Scheduling rule:** Must not be dispatched during the active roadmap. Re-evaluate only when the provider/resource extension boundary is reached and the Tech Lead confirms the universal resource contract is stable.

## Objective

Validate a **provider-neutral Execution Resource Provider contract** beneath the existing Codex Universal execution abstraction. E2B, Daytona, Modal, local/container/VM infrastructure, Kubernetes, and future providers are candidate implementations of that contract; this Work Order is **not** an instruction to build an E2B-specific semantic layer.

Codex/Workflow owns long-running agent/workflow orchestration and durable workflow state. Resource providers supply execution infrastructure and resource lifecycle; they are not responsible for long-running task semantics.

## Dependencies / activation

Preferred activation point: after the current core roadmap reaches the execution/resource-provider extension boundary (currently expected after `WO-015`).

This packet is intentionally outside the active dependency graph. The Tech Lead must not pull it forward, block current Work Orders on it, or reorder current roadmap execution because of it.

When activated, the Tech Lead must first audit the then-current architecture and may revise, split, or retire this Work Order if the stable resource abstraction has changed. Provider neutrality is the invariant; the provider list is not fixed.

## Authorized change surface

Only the smallest surfaces required to implement a generic resource-provider contract, for example:

- execution/resource provider interfaces and adapters;
- resource capability declarations;
- lifecycle abstractions;
- provider configuration/reference plumbing;
- resource selection/binding logic;
- scoped integration tests;
- provider-specific test fixtures/mocks;
- documentation describing the provider boundary.

Exact paths must be determined from the live source tree when the Work Order becomes READY.

## Required source audit

Before coding, inspect:

1. `docs/architecture/CODEX-UNIVERSAL-ARCHITECTURE.md`;
2. `docs/architecture/CODEX-UNIVERSAL-LOCK.md`;
3. `docs/architecture/CODEX-CAPABILITY-IMPLEMENTATION-MAP.md`;
4. the current execution abstraction and `ResourceBinding` implementation;
5. current `BrowserUseAdapter` and `ComputerUseAdapter` implementations;
6. current Codex long-running/background task, session, rollout, resume, cancellation, and persistence mechanisms;
7. existing provider-adapter patterns;
8. configuration/secret/reference patterns;
9. recovery, tracing, evidence, and authorization paths;
10. current client/application protocol surfaces before proposing any client work.

The live source tree at dispatch time is authoritative.

## Architectural requirements

The implementation MUST preserve these boundaries:

- Codex runtime remains the primary agent substrate.
- Workflow control remains authoritative over durable workflow meaning.
- Long-running task semantics remain in Codex/workflow orchestration, not resource providers.
- Resource providers remain replaceable infrastructure implementations beneath universal execution/resource contracts.
- Provider/environment-specific types MUST NOT leak into universal workflow semantics.
- E2B, Daytona, Modal, Kubernetes, local infrastructure, or any other provider MUST NOT become mandatory.
- Computer Use remains the semantic contract; a desktop provider is only a resource binding beneath it.
- Browser Use remains the semantic contract; provider infrastructure is only a runtime binding where applicable.
- Resource capability does not imply authorization.
- Resource lifecycle state does not become workflow semantic state.
- Provider snapshots/checkpoints/forks are execution artifacts and do not replace immutable workflow/version/source/dependency identity.
- Credentials remain runtime configuration/reference data and never become workflow data.
- No second agent runtime, workflow engine, or parallel semantic protocol may be introduced.

## Provider-neutral resource model

The future abstraction may describe, where applicable:

- provider identity and provenance;
- resource/environment class;
- resource identity;
- lifecycle capabilities;
- isolation properties;
- CPU, memory, storage, and GPU capacity;
- operating-system/runtime characteristics;
- persistence and snapshot/checkpoint/fork capabilities;
- network capabilities and policy inputs;
- location/region;
- cost metadata;
- health/availability;
- authorization requirements;
- evidence/observability metadata.

Do not encode provider-specific fields into universal Workflow IR merely to consume one provider.

## Candidate-provider validation

At activation, evaluate at least two materially different providers/implementations rather than designing the contract around a single vendor. Candidate evidence should include E2B and at least one of Daytona, Modal, local/container/VM infrastructure, Kubernetes, or another implementation that can satisfy the target resource class.

The objective is to prove the abstraction is genuinely provider-neutral rather than an E2B-shaped interface with alternative names.

## Functional target

Validate, where supported by selected providers:

- isolated compute resource creation and release;
- optional remote graphical desktop resource suitable for Computer Use;
- lifecycle operations such as create, pause/resume, snapshot/checkpoint, fork, or resize;
- network policy/configuration inputs;
- resource capacity/health metadata;
- resource provenance and evidence capture through existing Codex mechanisms;
- resource failure/rebinding through the existing recovery model;
- optional GPU/specialized compute without changing workflow semantics;
- optional self-hosted/customer-managed infrastructure without provider-specific workflow semantics.

Unsupported operations must remain explicitly unsupported.

## Long-running task rule

Codex already owns long-running/background agent execution. This Work Order must therefore **not** introduce provider-level task orchestration merely because a resource can persist for long periods.

A resource provider may offer persistence, pause/resume, snapshots, or other lifecycle primitives that make long-lived execution more robust, but the workflow/task remains owned by Codex/Workflow and can theoretically move between providers without changing its semantic identity.

## Client boundary rule

The public `openai/codex` repository is not synonymous with every Codex client product. It contains substantial runtime and application-protocol surfaces, while some rich client products may exist outside the repository.

Do not add "build the Codex web app" or "build the Codex mobile app" as resource-provider work merely because those clients are not present in this repository. Future client work must be justified through the existing application/client protocol and a separate bounded Work Order.

## Acceptance criteria

1. A workflow can request a generic resource need and bind an implementation without embedding provider-specific identifiers in workflow semantics.
2. At least two independent providers/implementations satisfy the same universal resource contract in tests or compatible conformance fixtures.
3. A compute resource executes through the existing execution abstraction.
4. A remote desktop resource, where supported, is reached through the existing Computer Use semantic contract.
5. Resource lifecycle actions remain resource lifecycle semantics rather than workflow semantics.
6. Long-running workflow/task semantics remain owned by Codex/workflow orchestration.
7. Provider/resource failure is surfaced through the existing recovery/error model and can support rebinding without mutating workflow meaning.
8. Credentials/API keys are injected only through approved runtime configuration/reference mechanisms.
9. Workflow version/source/dependency pinning remains independent of provider resource IDs, snapshots, regions, or infrastructure-specific identifiers.
10. Evidence distinguishes provider/resource facts from workflow semantic facts.
11. Existing Browser Use, Computer Use, workflow, model, and execution contract regressions remain green.
12. No provider-specific types or semantics leak into universal workflow contracts.

## Verification

At dispatch time, the Tech Lead chooses exact repository-required commands from current `AGENTS.md` and source configuration. The verification report must include:

- scoped unit/integration tests;
- provider-neutral conformance tests;
- regression tests for existing execution/browser/computer/workflow contracts;
- formatting;
- lint/clippy where applicable;
- compile/build verification;
- security/secrets scan appropriate to provider configuration changes;
- exact provider capability evidence from current APIs/SDKs.

All evidence must be tied to the exact verified commit SHA.

## Forbidden changes

Do not alter:

- frozen architectural invariants;
- current Work Order dependencies while roadmap Work Orders are in flight;
- workflow immutability/pinning;
- long-running/background task semantics owned by Codex/workflow;
- authorization semantics;
- existing Codex-native Browser Use or Computer Use contracts;
- model-provider abstraction;
- Git forge semantics;
- marketplace/distribution semantics;
- client/application semantics through an unrelated resource-provider change;
- credential handling rules.

Do not make E2B, Daytona, Modal, or another provider a mandatory runtime dependency.

## Non-disruption rule

This Work Order is deliberately parked. The active Tech Lead must continue the existing roadmap without waiting for or integrating WO-016.

When the provider/resource extension boundary is reached, the Tech Lead may activate this packet, re-scope it to the then-current generic resource architecture, or retire it if the provider abstraction has already been satisfied elsewhere.

## Completion report

Use the standard completion report required by `docs/work-orders/README.md` and `TECH_LEAD_START_HERE.md`, with explicit evidence for:

- provider neutrality;
- at least two independent implementations;
- long-running task/orchestration separation;
- authorization separation;
- credential isolation;
- resource lifecycle mapping;
- recovery/rebinding;
- workflow immutability and pinning;
- client/application boundary preservation.
