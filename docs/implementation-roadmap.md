# Codex Universal Implementation Roadmap

This roadmap is the implementation sequence. It is subordinate to the frozen architecture.

## M0 — Repository and architecture bootstrap

Establish agent operating contract, frozen architecture, state files, Work Orders, and verification conventions.

**Gate:** agents can bootstrap from repository truth without conversation context.

## M1 — Universal model contract

Audit all provider-specific model assumptions. Establish provider-neutral model contracts and isolate OpenAI-specific behavior behind adapters.

**Gate:** OpenAI remains behaviorally compatible; provider selection does not alter durable thread semantics.

## M2 — Provider portability

Implement/verify OpenAI-compatible, native-provider, and local-runtime adapter paths using capability-aware contracts.

**Gate:** the same agent task can run against multiple providers with equivalent runtime semantics.

## M3 — Workflow semantic contracts

Introduce workflow domain types: capability, resource requirement, role, workflow IR, WorkflowDefinition, immutable WorkflowVersion, WorkflowInstance, execution identity, triggers, guards, approvals, and evidence references.

**Gate:** workflow semantics exist independently of execution providers and UI.

## M4 — Workflow control plane

Implement durable workflow lifecycle, transition authority, versioning, pause/resume, cancellation, recovery, approvals, and idempotent external-event handling.

**Gate:** workflow state survives session loss and cannot be silently mutated by an agent/provider.

## M5 — Execution abstraction

Map Codex native execution primitives to workflow execution adapters: terminal, tool, API, human, and browser. Preserve a single execution abstraction.

**Gate:** one workflow step can select among compatible modalities without changing semantic meaning.

## M6 — Teaching and workflow compiler

Implement demonstration/instruction/hybrid teaching, trajectory capture, candidate generation, semantic validation, binding proposals, replay/simulation, and publication into immutable workflow versions.

**Gate:** a user can teach a workflow and publish a validated immutable version.

## M7 — Browser runtime integration

Implement controlled browser sessions, profiles, tabs, observation/action/result events, human takeover, recovery, origin scoping, and evidence capture.

**Gate:** browser workflows run deterministically enough to support replayable validation and safe recovery.

## M8 — Scheduling, triggers, sharing, installation

Add scheduled runs, normalized events/webhooks, workflow package export/import, discovery, sharing, compatibility checks, resource rebinding, and version pinning.

**Gate:** a workflow can be installed, configured, scheduled, and executed from the application.

## M9 — Application integration

Expose workflow creation, library, editor, run monitor, approvals, history, versioning, sharing, scheduling, and execution evidence through the app while keeping the control plane authoritative.

**Gate:** workflows are first-class product objects, not CLI-only features.

## M10 — Evaluation and differential compatibility

Create reproducible coding/workflow benchmark harnesses. Compare the fork against upstream Codex where behavior is intended to remain compatible.

**Gate:** changes are measured by task success, efficiency, safety, recovery, and compatibility rather than agent self-report.

## M11 — Learning/evolution

Add governed workflow/skill/binding improvement candidates from execution evidence, with replay, approval, and immutable new versions.

**Gate:** learning improves workflows without silent semantic mutation.

## M12 — Future execution environments

Add desktop/mobile/other environments only through new execution adapters that satisfy the existing contracts.

**Gate:** no second workflow engine or semantic protocol is introduced.
