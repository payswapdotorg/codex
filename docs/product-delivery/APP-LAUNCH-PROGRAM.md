# Codex Universal — Downloadable App Launch Program

**Status:** ACTIVE PRODUCT-DELIVERY PROGRAM
**Purpose:** move Codex Universal from an architecturally implemented/validated repository to a reproducibly downloadable and launchable end-user application.
**Authority:** frozen architecture 0.2.0 + current source + bounded APP Work Orders.

## 1. Current conclusion

The repository already documents how to build and launch the underlying Codex CLI/TUI from `codex-rs`.

That is not sufficient to claim that the **Codex Universal product** is downloadable and launchable as an end-user application with its workflow surfaces.

The objective of this program is to produce a verified product artifact that a fresh user can download, install/unpack, launch, and use to reach the Codex Universal workflow experience without requiring source-level knowledge.

## 2. Product target

The first release target is the smallest real user-facing application that exposes the implemented Codex Universal workflow/runtime capabilities through an existing Codex client/runtime surface, while preserving the architecture's Client Boundary Rule.

The Tech Lead must reuse existing Codex TUI/app-server/application-protocol/client surfaces where practical. Do not create a second agent runtime or workflow engine.

The exact client packaging form may be native binary, desktop bundle, CLI/TUI package, or another platform-appropriate Codex client surface, but the accepted artifact must satisfy the launch gate below. A web/mobile client is not required unless source evidence establishes that it is the smallest correct route.

## 3. Required launch gate

A fresh machine/workspace with the documented prerequisites must be able to:

1. obtain the release artifact from GitHub;
2. install or unpack it using documented user steps;
3. launch the application with no repository checkout required;
4. see a clearly identified Codex Universal application surface;
5. create/start a session;
6. reach the workflow/teaching surface;
7. perform at least one end-to-end workflow action through the real product path;
8. persist enough local state for a restart/relaunch smoke test;
9. report actionable errors when required dependencies or credentials are missing;
10. complete a clean uninstall/remove procedure documented for the supported platform.

## 4. Distribution requirement

Provide at least one reproducible downloadable artifact for the primary supported platform used for product validation and one reproducible alternate-platform artifact when the existing Codex build/release machinery makes that practical.

Every artifact must have:

- version;
- commit/revision identity;
- platform/architecture identity;
- checksum;
- exact build command;
- exact launch command;
- dependency/prerequisite list;
- release notes;
- known limitations.

Do not publish credentials, tokens, local paths, or secrets.

## 5. Fresh-environment verification

The Tech Lead must verify the artifact outside the development checkout.

Verification must cover:

- clean machine/workspace;
- no source checkout;
- clean config directory where practical;
- first launch;
- launch after restart;
- normal session creation;
- workflow teaching surface;
- one representative browser/desktop/terminal workflow path where supported by the artifact;
- missing-credential behavior;
- shutdown/relaunch;
- artifact checksum verification.

## 6. Architecture constraints

The app is a client/presentation surface over the existing Codex runtime and workflow/application protocols.

It must not:

- create a second agent runtime;
- create a second workflow engine;
- create provider-specific workflow semantics;
- duplicate Codex approvals/sandbox/MCP/session/runtime machinery;
- bypass workflow control-plane authority;
- place credentials in workflow source or ordinary evidence.

## 7. APP Work Orders

```text
APP-001  Launch Surface + User-Facing Workflow Entry Point
APP-002  Reproducible Packaging + GitHub Download Artifact
APP-003  Fresh-Machine Install/Launch + Product Smoke Validation
APP-004  Final App Release Gate + Documentation
```

Recommended execution:

```text
APP-001 ─────────────┐
                     ├──> APP-002 ──> APP-003 ──> APP-004
existing Codex       │
client/runtime       │
inspection           ┘
```

APP-001 must first establish the actual supported user-facing launch surface. APP-002 packages it. APP-003 proves it outside the checkout. APP-004 publishes the final user instructions and release evidence.

## 8. Completion rule

The program is complete only when a person who has never seen the repository can follow the release instructions, download the artifact, launch Codex Universal, reach the real workflow experience, complete a representative user-path smoke test, close the app, relaunch it, and obtain the same supported product behavior.

A successful `cargo build` inside the repository is not sufficient evidence.

## 9. State hygiene

After every accepted APP Work Order, update the product-delivery state with the verified SHA and artifact identity. Reconcile stale development-state records rather than treating them as authority.
