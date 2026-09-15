# Codex Universal — Tech Lead Handoff: Make the Product Downloadable and Launchable

## Mission

Take the current Codex Universal repository from **architecturally implemented and validated** to a **real, downloadable, launchable end-user application**.

The repository already provides a documented source-build path for the underlying Codex CLI/TUI. That is not the completion target. The target is a user who can download a release artifact, install/unpack it, launch it without cloning the repository, and reach the real Codex Universal workflow experience.

## Current verified boundary

The existing `docs/install.md` documents building and launching the underlying Codex CLI/TUI from source. Do not represent that source-build path as proof that the Codex Universal product is distributable.

The frozen architecture remains authoritative. Reuse existing Codex runtime/client/app-server/application-protocol capabilities. Do not create a second agent runtime, workflow engine, skill/plugin runtime, or provider-specific workflow semantics.

## Execution

Read first:

- `ARCHITECT_START_HERE.md`
- `TECH_LEAD_START_HERE.md`
- `AGENTS.md`
- `docs/architecture/CODEX-UNIVERSAL-ARCHITECTURE.md`
- `docs/architecture/CODEX-UNIVERSAL-LOCK.md`
- `docs/product-delivery/APP-LAUNCH-PROGRAM.md`
- `docs/product-delivery/work-orders/APP-001.md`
- `docs/product-delivery/work-orders/APP-002.md`
- `docs/product-delivery/work-orders/APP-003.md`
- `docs/product-delivery/work-orders/APP-004.md`

Execute strictly:

```text
APP-001
   ↓
APP-002
   ↓
APP-003
   ↓
APP-004
```

Maximum three workers may be active, but these Work Orders are intentionally sequential because each consumes the previous deliverable.

## APP-001 — establish the launch surface

Audit the current Codex TUI, app-server, protocol and client surfaces. Determine the smallest correct existing Codex surface that can expose the implemented workflow/teaching experience.

Do not design a hypothetical product before inspecting the existing source.

The result must be a real user path, not an internal Rust invocation or fixture shortcut.

## APP-002 — package it

Use existing Codex build/release infrastructure. Produce a reproducible downloadable artifact with version, commit SHA, platform/architecture identity, checksum, exact build command and exact launch command.

Prefer existing DotSlash/release machinery where it satisfies the user gate. Do not introduce a packaging framework merely for convenience.

## APP-003 — prove it outside the checkout

Use a fresh environment. The test machine/workspace must not contain the source checkout.

The validator must:

1. download the artifact from GitHub;
2. verify the checksum;
3. install/unpack it;
4. launch it;
5. start a session;
6. reach workflow/teaching functionality;
7. complete a representative real workflow action;
8. close the app;
9. relaunch it;
10. verify supported persistence/restart behavior;
11. verify actionable failure behavior when required credentials/dependencies are absent.

The test must use normal product pathways. Internal state inspection is diagnostic evidence only.

## APP-004 — final user handoff

Publish the download location and exact user instructions.

Documentation must tell a user:

- what to download;
- how to verify it;
- how to install/unpack;
- how to launch;
- how to configure model access;
- how to start a workflow;
- what computer environments are supported by this release;
- how to restart/update/uninstall;
- how to troubleshoot common first-run failures.

## Acceptance gate

The program is green only when all are true:

- a release artifact exists;
- it is downloadable from GitHub;
- a source checkout is not required;
- the artifact launches on the documented platform;
- the user can reach the actual Codex Universal workflow surface;
- one representative workflow is completed through the normal application path;
- the app can be closed and relaunched successfully;
- artifact identity and checksum are documented;
- fresh-environment evidence is committed;
- exact user documentation is committed.

A passing `cargo build`, unit suite, fixture sweep, or developer-only TUI launch is not sufficient.

## Engineering guardrails

Reuse existing Codex primitives whenever possible:

- runtime;
- TUI/client;
- app-server/application protocol;
- sessions;
- approvals;
- sandboxing;
- MCP;
- browser/computer capabilities;
- workflow control plane;
- existing release/build infrastructure.

Reject proposals that introduce:

- a second agent runtime;
- a second workflow engine;
- duplicated workflow semantics in the client;
- credential leakage;
- provider-specific semantics in universal contracts;
- a benchmark-only UI that cannot execute real workflows.

## Completion report

After APP-004, commit a final report under `docs/product-delivery/reports/APP-RELEASE-REPORT.md` containing:

- exact release version;
- exact source SHA;
- release artifact names;
- platform/architecture matrix;
- checksums;
- build commands;
- installation commands;
- launch commands;
- fresh-environment identity;
- smoke-test evidence;
- workflow performed;
- persistence/relaunch evidence;
- known limitations;
- explicit verdict: `DOWNLOADABLE_AND_LAUNCHABLE` or `NOT_READY`.

Then reconcile all stale development/product state records against the verified merged tree.
