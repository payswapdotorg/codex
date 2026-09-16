# Codex Universal Client / Adapter Architecture

**Status:** FROZEN as part of Architecture 0.3.0

## Purpose

Codex Universal semantics must be consumable from multiple client surfaces without duplicating the runtime, workflow engine, Pack authority, model plane, execution plane, or evidence authority.

Supported and planned client/adaptor classes include:

- native desktop (Flauz.app)
- web application
- mobile application
- browser extension
- VS Code extension
- IDE/editor integrations
- future remote desktop/device clients
- automation/SDK clients

These are **client/adaptor surfaces**, not alternate Codex runtimes.

## Boundary

```text
                  Codex Universal Authority
                           │
              typed client/application protocol
                           │
          ┌────────────────┼────────────────┐
          │                │                │
       Desktop            Web            Mobile
      Flauz.app             │                │
          │              Browser Ext      Device UI
          │                │
        VS Code         IDE clients
          │                │
          └────────────────┼────────────────┘
                           │
                     Same semantics
                           │
       ┌───────────────────┼───────────────────┐
       │                   │                   │
    Workflows            Packs             Evidence
       │                   │                   │
       └───────────────────┼───────────────────┘
                           │
                  Universal Runtime
```

## Core rule

A new client surface should primarily require:

```text
presentation + interaction adapter
+ authentication/session integration
+ capability negotiation
+ protocol bindings
+ platform-specific packaging
```

It must not require reimplementing:

- agent runtime
- Workflow Control Plane
- Pack Control Plane
- Model Plane
- Execution semantics
- evidence authority
- credential authority
- policy/authorization authority

## Two adapter categories

### Client adapter

Presents Codex semantics to a user or host application.

Examples:

- Flauz.app
- web client
- mobile client
- browser extension UI
- VS Code extension

### Execution adapter

Provides an environment in which Workflow steps execute.

Examples:

- browser
- desktop/computer
- terminal
- API/tool/MCP
- mobile device
- remote desktop
- browser-extension page context
- IDE/editor action surface

A single product may implement both kinds, but they remain conceptually distinct.

## Portability requirements

Every new client/adaptor should consume versioned contracts with:

- protocol version
- capability negotiation
- authentication/authorization semantics
- request/response types
- streamed events
- correlation IDs
- bounded framing/queue rules
- cancellation
- reconnect/resume
- error taxonomy
- redaction/secret handling
- feature compatibility

Durable semantics must remain server/control-plane authoritative.

## UI parity rule

Client surfaces may expose different interaction patterns appropriate to their host:

- desktop: full workspace
- web: browser-native workspace
- mobile: compact/notification-driven interaction
- browser extension: page-context teaching/execution controls
- VS Code: editor/task/review integration

They do not need identical UI. They must preserve the same underlying semantic contracts and authority model.

## Extension-specific guidance

### Browser extension

A browser extension may provide page-context capture, teaching, annotations, evidence capture, approval/takeover controls, and execution affordances. It must communicate through bounded Universal contracts and must not become a standalone workflow authority.

### VS Code extension

The VS Code extension may expose project-aware agent sessions, workflow teaching/execution, terminal/editor context, review, evidence, and Pack views. It should reuse the existing VS Code host primitives while delegating durable semantics to Codex Universal.

### Web

The web client may provide the broadest cross-platform access to Agent, Workflow, Pack, evidence, and administration surfaces but remains a client of the same contracts.

### Mobile

Mobile clients should use the same semantic model while adapting for notifications, constrained screen size, intermittent connectivity, approvals, human gates, and device capabilities. Mobile-specific device access belongs behind execution/client adapters.

## Architectural consequence

Adding a new client surface is intentionally a bounded adapter project, not a platform fork.

A new surface may be implemented without changing Workflow or Pack semantics unless it demonstrates a genuine missing cross-client contract. Any such contract change must follow the normal Architecture Change Request process.
