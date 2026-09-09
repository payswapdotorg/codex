# Codex Universal — Execution Resource Provider Advisory

**Status:** DEFERRED / NON-BLOCKING
**Architecture version:** 0.2.0
**Purpose:** record the resource-provider opportunity without changing the frozen architecture or interrupting active roadmap execution.

## 1. Decision

E2B, Daytona, Modal, local/container/VM infrastructure, Kubernetes, and future infrastructure providers are candidates for a **provider-neutral Execution Resource Provider Plane** beneath the existing Codex Universal execution abstraction.

None of these providers is a semantic authority, workflow engine, or agent runtime. The universal execution/resource contract is authoritative; providers are replaceable implementations.

The intended relationship is:

```text
                    Codex Universal
                          │
                   Workflow Control
                          │
                  Execution Contracts
                          │
              Execution Resource Plane
                          │
      ┌───────────┬───────┼────────┬───────────┐
      │           │       │        │           │
  Codex-native    E2B   Daytona   Modal   other providers
```

## 2. Long-running task boundary

Codex already owns agent/workflow orchestration, including long-running/background task semantics. A resource provider is therefore **not required to make a task long-running**.

Provider features such as persistence, pause/resume, snapshots, checkpoints, forks, or durable filesystems are useful execution-resource capabilities. They do not become workflow semantics and must not duplicate Codex's task orchestration.

## 3. Provider-neutral resource model

The future abstraction may describe, where applicable:

- provider identity and provenance;
- resource/environment class;
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

Provider-specific fields must not leak into universal Workflow IR or semantic contracts.

## 4. Computer Use and other environment contracts

Remote graphical infrastructure is a **resource binding beneath `ComputerUseAdapter`**, not a replacement for it.

```text
Workflow Step
  → ComputerUseAdapter
  → ResourceBinding
  → selected desktop resource
       ├─ Codex-native
       ├─ E2B Desktop
       ├─ Daytona desktop
       └─ future provider
```

The same principle applies to Browser Use, terminal, API/tool, MCP, and future environments: workflow meaning stays provider-neutral while runtime binding selects infrastructure.

## 5. Why multiple providers matter

The architecture should not be shaped around E2B simply because E2B is attractive. Daytona and Modal overlap with E2B while emphasizing somewhat different strengths; local infrastructure and Kubernetes provide additional implementation diversity.

A provider-neutral contract is only credible if multiple materially different implementations can satisfy it. At activation, validate at least two providers/implementations rather than creating an E2B-shaped abstraction and renaming it later.

## 6. Reproducibility and recovery

Provider snapshots/checkpoints/forks may support:

- preserved execution environments;
- retry/recovery with environment state;
- parallel worker environments;
- evaluation sandboxes;
- reproducible test environments;
- remote desktop sessions.

Workflow reproducibility remains based on immutable workflow/version/source/dependency identities. Provider resource identifiers and snapshots are execution artifacts, not workflow identity.

Provider failure must be recoverable/rebindable without mutating workflow meaning.

## 7. Client boundary

The public `openai/codex` repository contains substantial runtime and application-protocol surfaces, but it is not synonymous with every Codex client product. Rich clients can be outside the repository, including Codex experiences in ChatGPT.

Do not infer that Codex Universal must build a web or mobile client merely because those product implementations are not present in this repository. Client surfaces should be addressed through the existing application/client protocol and their own bounded Work Orders.

## 8. Security boundary

All provider observations, command results, desktop observations, application outputs, network responses, and other external outputs remain untrusted input.

Provider credentials/API keys are runtime configuration references only. They must never be placed into workflow source, prompts, ordinary logs, or semantic workflow evidence.

Resource availability does not imply authorization.

## 9. Non-disruption rule

This advisory is intentionally non-blocking.

It does not:

- change architecture version 0.2.0;
- change the active Work Order dependency graph;
- block, reorder, or rescope WO-010 through WO-015;
- replace WO-007 Computer Use;
- introduce a second runtime or workflow engine;
- make any provider mandatory;
- require the active Tech Lead to interrupt current implementation.

The active Tech Lead must continue the current dependency graph exactly as recorded.

## 10. Timing

The resource-provider work should be reconsidered only after the current roadmap reaches the provider/resource extension boundary and the existing execution/resource contracts are stable enough for concrete provider validation.

The Tech Lead should not pull provider integration forward merely because a provider is available or technically interesting.

## 11. Future acceptance shape

A future implementation should prove at minimum:

1. a generic resource need can bind multiple providers without provider-specific workflow semantics;
2. compute and, where supported, remote desktop execute through existing execution/environment contracts;
3. provider lifecycle capabilities remain resource lifecycle semantics;
4. long-running task semantics remain owned by Codex/workflow orchestration;
5. provider failure can trigger recovery/rebinding without changing workflow meaning;
6. authorization and credentials remain separate from resource availability;
7. workflow source/version/dependency pinning remains independent of provider resource IDs;
8. evidence separates provider/resource facts from workflow semantic facts;
9. at least two independent providers can satisfy the same universal contract in conformance tests.

## 12. Architectural conclusion

**Adopt a generic Execution Resource Provider Plane. Treat E2B, Daytona, Modal, local infrastructure, Kubernetes, and future providers as interchangeable implementations beneath it.**

Do not turn any provider into a new Codex runtime, workflow engine, or provider-specific workflow language.
