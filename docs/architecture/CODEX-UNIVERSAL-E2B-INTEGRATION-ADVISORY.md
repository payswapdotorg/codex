# Codex Universal — E2B Integration Advisory

**Status:** DEFERRED / NON-BLOCKING
**Architecture version:** 0.2.0
**Purpose:** record the E2B infrastructure opportunity without changing the frozen architecture or interrupting active roadmap execution.

## 1. Decision

E2B is a **compatible infrastructure/resource provider**, not a replacement for any Codex Universal semantic layer.

The current Codex Universal architecture remains authoritative and unchanged. E2B must not become a workflow engine, agent runtime, semantic authority, or provider-specific workflow language.

The intended future relationship is:

```text
                 Codex Universal
                       │
                Workflow Control
                       │
                Execution Contracts
                       │
          ┌────────────┼────────────┐
          │            │            │
     Codex-native     E2B       Other resource
     capabilities   provider      providers
```

## 2. What E2B contributes

E2B currently provides secure isolated cloud sandboxes, JavaScript/Python SDKs, Code Interpreter support, and a Desktop SDK for computer-use interactions. Its infrastructure uses Firecracker microVMs and separates control-plane placement/state from data-plane VM orchestration. E2B also supports snapshot/resume patterns and self-hosted infrastructure. These properties make it a strong candidate for the **resource/infrastructure side** of Codex Universal rather than the semantic side.

Potential Codex Universal mappings include:

| E2B capability | Codex Universal role | Constraint |
| --- | --- | --- |
| Isolated Linux sandbox | `ISOLATED_COMPUTE` resource | Must remain behind `ResourceBinding` / execution contracts |
| Desktop sandbox | `REMOTE_DESKTOP` resource | Must not replace the `ComputerUseAdapter` semantic contract |
| Snapshot / resume | workflow execution resource lifecycle | Workflow control plane remains authoritative over durable state |
| Forkable/reproducible environments | parallel worker/evaluation resource | Workflow version/source identity remains the reproducibility authority |
| Network configuration | resource/network policy input | Availability never implies authorization |
| Code Interpreter | compute/data-analysis capability | Capability stays distinct from workflow semantics |
| Self-hosted infrastructure | alternative deployment/resource provider | No E2B-specific assumptions in universal contracts |

## 3. Architectural placement

The preferred abstraction is a generic execution-resource concept already implied by the existing `ResourceBinding` direction.

A future implementation may strengthen the resource model to describe, where applicable:

- provider identity;
- resource/environment class;
- lifecycle capabilities;
- isolation properties;
- snapshot/checkpoint support;
- persistence characteristics;
- network policy;
- capacity/quotas;
- location/region;
- cost metadata;
- availability/health;
- authorization requirements;
- resource identity and provenance.

Do **not** add E2B-specific fields to universal workflow semantics merely to consume E2B.

## 4. Computer Use rule

E2B Desktop is an implementation option for a remote graphical execution resource. It does not replace Codex Computer Use.

The semantic chain remains:

```text
Workflow Step
  → ComputerUseAdapter
  → ResourceBinding
  → selected desktop resource
       ├─ Codex-native desktop
       ├─ E2B Desktop
       └─ future provider
```

The same rule applies to browser/terminal/API/tool/MCP execution: provider infrastructure is selected by runtime binding, while workflow meaning remains provider-neutral.

## 5. Reproducibility and recovery

E2B snapshots/resume/fork semantics are useful infrastructure primitives for:

- long-running workflow execution;
- retry with preserved environment state;
- parallel agent workspaces;
- evaluation sandboxes;
- reproducible test environments;
- remote desktop sessions.

These primitives must be represented as **resource lifecycle capabilities**. They do not become workflow semantics by themselves.

Workflow reproducibility remains based on immutable workflow/version/source/dependency identities. A provider snapshot is an execution artifact, not a substitute for those identities.

## 6. Security boundary

All E2B observations, command results, desktop observations, application outputs, network responses, and other external outputs remain untrusted input.

E2B credentials/API keys are runtime configuration references only. They must never be placed into workflow source, prompts, ordinary logs, or semantic workflow evidence.

A bound E2B resource grants no authorization beyond the policy already granted by Codex Universal.

## 7. Non-disruption rule

This advisory is intentionally **non-blocking**.

It does not:

- modify the frozen architecture version;
- change existing Work Order dependencies;
- change the readiness of any active Work Order;
- require changes to WO-010 through WO-015 while they are in flight;
- replace WO-007 Computer Use;
- introduce a new runtime or workflow engine;
- require E2B to be present for Codex Universal to function.

The active Tech Lead MUST continue the existing dependency graph exactly as recorded. E2B integration is additive future work.

## 8. Timing

The preferred implementation point is **after the current core roadmap reaches the additional-environments/infrastructure-provider extension boundary**, when the execution/resource contracts are stable enough to validate a concrete provider adapter.

The Tech Lead should not pull this work forward merely because the provider is attractive. It must not preempt or reorder current Work Orders.

## 9. Acceptance shape for future integration

A future E2B integration should prove at minimum:

1. a generic execution/resource contract can select E2B without exposing E2B types in universal workflow semantics;
2. isolated compute can be provisioned and released through the existing execution abstraction;
3. desktop execution can bind E2B Desktop through the existing Computer Use contract;
4. lifecycle operations (create/pause/resume/snapshot/fork where supported) map cleanly to resource lifecycle semantics;
5. network and authorization policies remain enforced by Codex Universal rather than inferred from E2B availability;
6. workflow source/version/dependency pinning remains independent of provider resource IDs;
7. evidence records distinguish provider/resource facts from workflow semantic facts;
8. provider failure is recoverable/rebindable without mutating workflow meaning;
9. an alternative resource provider can satisfy the same contract in tests.

## 10. Architectural conclusion

**Adopt E2B as a candidate infrastructure provider, not as a new platform layer.**

The core architecture is already shaped correctly for this: universal execution semantics above provider-specific resources. The right integration is therefore a later adapter/resource-provider implementation, not a redesign of Workflow IR, the control plane, Browser Use, Computer Use, or the model plane.
