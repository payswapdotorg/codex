# Codex Universal Pack Work Orders

**Status:** ARCHITECTURE-READY / STAGED
**Authority:** `docs/architecture/CODEX-PACK-ARCHITECTURE.md`
**Architecture:** 0.3.0

The Pack program is deliberately staged so its contracts land before GUI coupling, while heavy generation/evolution work waits until the core Workflow + desktop experience is proven.

## Graph

```text
PACK-001 Contract Foundation
        │
        ├───────────────┐
        ▼               ▼
PACK-002 System State   PACK-003 Assurance / Determinism
        │               │
        └───────┬───────┘
                ▼
        PACK-004 Pack / Workflow Integration
                │
                ▼
        PACK-005 Composition
                │
                ▼
        PACK-006 Generation + Evaluation
                │
                ▼
        PACK-007 Experiment + Promotion/Rollback
                │
                ▼
        PACK-008 Governed Evolution
                │
                ▼
        PACK-009 Distribution / Marketplace
```

## Near-term work

### PACK-001 — Contract Foundation

Define typed Pack identity, revision, mission, value model, context model, Pack Constitution, Pack Policy, dependencies, and provenance.

Acceptance:
- Pack identity is immutable and content-addressable where applicable;
- mission and policy authority are explicit;
- no workflow or runtime authority is duplicated.

### PACK-002 — System State

Define `PackSystemState` and candidate/promoted revision relationships. Reference existing immutable WorkflowVersions rather than embedding alternate workflow semantics.

Acceptance:
- candidate state and promoted state are distinct;
- parent/provenance is preserved;
- WorkflowVersion pinning is explicit and immutable.

### PACK-003 — Assurance / Determinism

Define the policy contract for determinism, replayability, approval, rollback, evidence, model pinning, dependency pinning, and environment pinning.

Acceptance:
- assurance is policy-scoped;
- no global deterministic-mode requirement is introduced;
- execution can report which policy dimensions were enforced.

### PACK-004 — Pack / Workflow Integration

Make Packs first-class consumers/owners of WorkflowVersion references, capability dependencies, evaluations, and evidence without moving Workflow authority into Packs.

Acceptance:
- existing workflows remain valid;
- Pack dependency updates create candidate Pack state;
- durable workflow transitions remain owned by the Workflow Control Plane.

## Deferred work

### PACK-005 — Composition

Implement specialization, orthogonal composition, contextual activation, conflict detection, provenance, and policy-compatible composition.

### PACK-006 — Generation + Evaluation

Implement governed multi-agent Pack construction, mission traceability, candidate architecture/system-state generation, evaluation, and red-team review.

### PACK-007 — Experiment + Promotion/Rollback

Implement simulation, replay, shadow/canary evaluation, experiment identity, promotion gates, rollback checkpoints, and evidence-backed state transitions.

### PACK-008 — Governed Evolution

Implement the observe → diagnose → hypothesize → candidate → assure → experiment → promote/rollback loop, with self-evolution unable to bypass its own governance.

### PACK-009 — Distribution / Marketplace

Implement Pack publishing, installation, forking, licensing, sharing, optional monetization, and immutable release management.

## Scheduling rule

PACK-001 through PACK-004 may be implemented alongside the current Universal workflow/GUI foundation when they reduce future coupling.

PACK-005 onward must not block the current Flauz.app desktop parity and first end-to-end Workflow vertical slice.
