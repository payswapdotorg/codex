# Architecture Change Request 002 — Governed Pack / System-State Architecture

**Status:** APPROVED
**Supersedes:** None
**Applies to:** Codex Universal Architecture 0.2.0
**Resulting architecture version:** 0.3.0
**Authority:** Explicit product-architecture approval in the Codex Universal design review

## 1. Current decision

Codex Universal currently models reusable durable semantics primarily as Git-native Workflows. This remains correct for executable orchestration, but it is insufficient for the future product requirement that Codex construct, evaluate, evolve, compose, and distribute larger user-governed software systems organized around an explicit mission.

## 2. Proposed decision

Introduce **Pack** as a first-class architectural object above Workflow.

A Pack is a governed, versioned system state organized around a user-owned mission and value model. A Pack may reference and compose immutable WorkflowVersions, agents, capabilities, policies, domain models, UX configuration, integrations, evidence/evaluation records, and evolution history.

The Pack becomes a product/control-plane abstraction; it does not become a second runtime.

## 3. New authority relationship

```text
User / Organization Mission
            ↓
      Pack / System State
            ↓
   Workflows / Agents / Policies
            ↓
       Runtime Execution
```

Codex Constitution and platform security/authority invariants remain above Packs. Pack-generated agents and LLMs remain proposal/reasoning actors, not authority owners.

## 4. Invariants

1. A Pack is not a second agent runtime.
2. A Pack is not a second workflow engine.
3. Workflow semantics remain owned by the Universal Workflow Control Plane.
4. Published Workflows remain immutable and execution remains pinned to immutable identities.
5. Pack revisions are immutable; mutation creates a new candidate/revision.
6. A Pack cannot silently weaken Codex security, authorization, evidence, or credential invariants.
7. A Pack cannot replace the user/organization mission with an LLM-derived objective.
8. Domain capabilities may be Pack-owned when they are domain semantics; general platform authority remains in Codex.
9. Determinism/replay/approval requirements are policy-scoped to Pack operations and execution paths rather than globally imposed on every interaction.
10. Pack composition must preserve provenance, version identity, dependency boundaries, and policy constraints.

## 5. Initial implementation boundary

Architecture and contracts are introduced now, before the desktop GUI implementation becomes deeply coupled to Workflow-only assumptions.

Initial implementation is intentionally limited to:

- Pack identity and immutable revision concepts;
- Mission, ValueModel, ContextModel, PackConstitution and PackPolicy contracts;
- SystemState references to immutable WorkflowVersions, capabilities, policies, evaluations, and dependencies;
- Pack-to-Workflow relationship and lifecycle boundaries;
- assurance/determinism policy hooks;
- client-facing protocol placeholders where required.

The following remain future implementation phases:

- Pack generation;
- architecture/system-state search;
- multi-agent Pack construction;
- semantic Pack composition compiler;
- experimentation and promotion controller;
- automated evolution;
- Pack marketplace/distribution at full scale;
- advanced model checking/formal proof workflows.

## 6. GUI impact

Flauz.app remains a client/presentation layer. Pack state may be displayed and manipulated through explicit Universal contracts, but it must not own durable Pack authority or create a second semantic engine.

The existing Agent Experience and Workflow Experience remain the two primary desktop surfaces. Pack is a system abstraction within the Workflow Experience, not a third competing top-level product surface.

## 7. Migration

Existing Workflows remain valid. No existing WorkflowVersion changes meaning.

A future Pack may reference existing WorkflowVersions without rewriting them. Existing workflows may continue to exist outside a Pack context until explicitly incorporated into one.

## 8. Verification impact

Architecture verification must cover:

- immutable Pack revision identity;
- mission/policy traceability;
- WorkflowVersion pinning;
- provenance and dependency integrity;
- authority separation;
- scoped determinism/assurance policy;
- restart/reconnect behavior across Pack-aware client boundaries.

No existing Universal workflow control-plane authority is weakened by this change.
