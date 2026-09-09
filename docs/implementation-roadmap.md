# Codex Universal Implementation Roadmap

This roadmap is subordinate to the frozen architecture. Work proceeds through bounded Work Orders.

## M0 — Repository and architecture bootstrap

Establish agent operating contract, frozen architecture, state files, Work Orders, verification conventions, and upstream provenance.

**Gate:** agents can bootstrap from repository truth without conversation context.

## M1 — Universal model contract

Audit every provider-specific assumption. Establish provider-neutral model contracts, capability declarations, model-session semantics, streaming/cancellation normalization, and provider isolation.

**Gate:** OpenAI behavior remains compatible and the same durable thread/workflow can change models without state mutation.

## M2 — Provider portability

Implement/verify OpenAI-compatible gateways, native-provider adapters, and local runtime adapters. Add capability-aware provider selection and diagnostics.

**Gate:** equivalent agent tasks run through multiple providers with the same runtime semantics.

## M3 — Workflow repository + semantic contracts

Introduce WorkflowRepository, WorkflowManifest, WorkflowIR, WorkflowDefinition, immutable WorkflowVersion, repository/source revision identity, fork/branch metadata, capabilities, resources, roles, skill/plugin dependencies, subworkflow dependencies, WorkflowInstance, triggers, guards, approvals, and evidence references.

**Gate:** a workflow is a versioned Git-native software artifact independent of any execution environment.

## M4 — Workflow control plane

Implement durable workflow lifecycle, legal transitions, version pinning, pause/resume/cancel/recovery, approvals, idempotent triggers, dependency resolution, and execution-instance authority.

**Gate:** workflow state survives session loss and cannot be silently mutated by a model/provider/environment.

## M5 — Execution abstraction + Codex capability bridge

Map Codex-native tools/skills/plugins/MCP/subagents/planning/approvals/sandbox/session-trace primitives into the workflow execution contract. Define normalized EnvironmentAdapter, CapabilityBinding, ResourceBinding, Observation, Action, ActionResult, and Recovery contracts.

The execution/resource model must remain provider-neutral. It may later support a generic Execution Resource Provider Plane beneath `ResourceBinding`, allowing interchangeable infrastructure providers such as Codex-native/local resources, E2B, Daytona, Modal, Kubernetes, and future providers. Do not shape universal semantics around a single vendor.

Codex/workflow remains authoritative for long-running agent/task orchestration. Resource persistence, snapshots, pause/resume, checkpoints, forks, and similar features are resource lifecycle capabilities, not substitutes for workflow/task semantics.

**Gate:** one workflow graph can invoke multiple environment classes while preserving semantic meaning.

## M6 — Codex-native Browser Use integration

Integrate the Codex Browser Use skill/runtime as the preferred browser workflow path. Support external/browser-pane surfaces as capabilities, explicit session/profile/tab binding, evidence capture, takeover, and diagnostic failures when the required runtime bridge is unavailable.

**Gate:** browser workflow steps use the Codex-native browser capability when available and are replay/evidence traceable.

## M7 — Codex-native Computer Use integration

Integrate Codex Computer Use as the preferred desktop/application path. Normalize screen/application/window observations and actions and support mixed Browser + Computer Use workflows.

**Gate:** a workflow can move between browser and desktop execution in one immutable graph.

## M8 — Teaching + workflow compiler

Implement instruction, demonstration, and hybrid teaching; trajectory capture; candidate generation; semantic compilation; skill/plugin dependency inference; execution binding proposals; replay/simulation; and publication into immutable versions.

**Gate:** a user can teach a mixed-environment workflow and publish a validated version.

## M9 — Workflow composition + collaboration

Implement GitHub forge integration first: clone/fetch, fork metadata, branches, commits, pull requests/review, merge/release/tag, workflow dependency graphs, pinned subworkflow versions, upgrade proposals, and conflict/rebase support.

**Gate:** multiple users can collaboratively evolve a workflow like software without corrupting published execution versions.

## M10 — Scheduling, triggers, sharing, installation

Implement schedules, webhooks, connector/browser/computer/workflow events, package discovery, install/configuration, resource rebinding, compatibility checks, and application-level run control.

Client implementations are separate concerns from the runtime substrate. The public `openai/codex` repository contains runtime and application-protocol surfaces but is not assumed to contain every Codex product client. Web/mobile client work must use the application/client protocol and its own bounded Work Order rather than being inferred from repository absence.

**Gate:** a workflow can be shared, installed, configured, scheduled, and run from the application.

## M11 — Marketplace / monetization / distribution

Add public/private workflow publishing, attribution, licensing, commercial entitlement metadata, paid releases/subscriptions, install provenance, upgrade policy, and marketplace discovery. Reuse Codex plugin-sharing/distribution patterns where useful without conflating plugin and workflow semantics.

**Gate:** workflow authors can distribute and monetize immutable releases without exposing credentials or altering execution semantics.

## M12 — Evaluation + differential compatibility

Build reproducible coding and workflow benchmarks across models, environments, permissions, context sizes, long sessions, failures, and recovery paths. Compare intended-compatible behavior against upstream Codex.

**Gate:** progress is measured by task success, safety, efficiency, recovery, compatibility, and user outcomes.

## M13 — Learning / governed evolution

Use execution evidence to generate workflow, skill, plugin-binding, recovery, and scheduling improvement candidates. Validate by replay/simulation and publish only as new immutable versions.

**Gate:** execution continuously improves without silent semantic mutation.

## M14 — Additional execution environments

Add mobile, remote desktop, device control, specialized applications, robotics/IoT where practical, always through the same execution/capability/resource contracts.

Before selecting a concrete infrastructure provider, evaluate multiple providers against the same resource contract. Candidate resource backends include E2B, Daytona, Modal, local/container/VM infrastructure, Kubernetes, and future providers. Provider choice must remain a runtime binding and must not leak into workflow semantics.

**Gate:** new environments require adapters only; no new workflow engine or semantic protocol.
