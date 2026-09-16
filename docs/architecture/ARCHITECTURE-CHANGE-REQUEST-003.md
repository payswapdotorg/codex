# Architecture Change Request 003 — Universal Client / Adapter Portability

**Status:** APPROVED
**Applies to:** Codex Universal Architecture 0.3.0
**Resulting architecture version:** 0.4.0

## Decision

Codex Universal shall expose its Agent, Workflow, Pack, Evidence, Model, Execution, and policy-controlled capabilities through versioned client/application contracts so additional clients and adapters can be added as bounded integration projects.

Initial target surfaces include native desktop (Flauz.app), web, mobile, browser extensions, VS Code extensions, and future IDE/device/remote clients.

## Invariants

1. Client surfaces are presentation/integration boundaries, not alternate runtime authorities.
2. Client-specific UX may differ; underlying Universal semantics and authority boundaries remain shared.
3. A client does not create a second agent runtime, workflow engine, Pack engine, skills/plugins runtime, evidence authority, credential authority, or authorization authority.
4. Client and execution adapters are distinct concepts even when implemented together.
5. New environments and host capabilities enter through explicit adapters and capability contracts.
6. Durable Workflow and Pack state remains control-plane authoritative.
7. New client support should not require semantic changes unless a real cross-client contract is missing; such changes use the normal Architecture Change Request process.
8. Authentication, capability negotiation, streaming, cancellation, reconnect/resume, bounded queues, redaction, and compatibility are first-class cross-client concerns.

## Intended consequence

Adding a web client, mobile app, browser extension, VS Code extension, or future IDE/device client should primarily require presentation, host integration, protocol bindings, authentication/session integration, and platform packaging rather than another implementation of Codex semantics.

## Compatibility

Existing desktop and workflow behavior remain valid. This change formalizes and generalizes the existing app-server/client boundary and does not require replacing the Flauz.app foundation.
