# Codex Universal — Autonomous Human-Workflow Validation Program

**Status:** ACTIVE PROGRAM
**Purpose:** product-level validation and dogfooding after architectural implementation
**Concurrency:** maximum 3 implementation/validation workers
**Authority:** frozen architecture + current source + these bounded validation Work Orders

## 1. Mission

Exercise Codex Universal as a real product rather than only as a set of Rust crates.

Workers must create, teach, compile, approve, publish, install, execute, recover, upgrade, rollback, distribute, and evolve workflows using the real application/runtime surfaces wherever those surfaces exist.

Use both:

- realistic human workflows;
- adversarial failure/recovery scenarios.

The objective is to discover defects that unit tests and synthetic contract tests cannot reveal.

## 2. Governing prompts

The program incorporates two previously issued prompts as repository artifacts:

- `prompts/01-final-completion-tech-lead-orchestration.md` — completion/hardening orchestration prompt, including durable M4 control-plane completion.
- `prompts/02-human-workflow-validation.md` — human-workflow validation and adversarial product-testing prompt.

The Tech Lead must not wait for these prompts to be reposted. They are part of repository instructions.

## 3. Required proving-ground strategy

### Preferred

Provision up to three isolated E2B desktop-capable sandboxes using the connected E2B/Composio account, when the integration exposes the required lifecycle operations.

Each sandbox should contain:

- current Codex Universal repository;
- current verified application/runtime;
- browser-capable execution;
- desktop/computer-capable execution;
- terminal access;
- required API/tool/MCP access;
- realistic synthetic enterprise applications;
- seeded users/resources/permissions;
- deterministic datasets plus deliberate failure switches.

E2B is infrastructure only. It must not own workflow semantics, durable workflow state, scheduling authority, or authorization policy.

### Fallback

If the connected E2B integration or desktop template is unavailable, use the existing agent sandbox/application environment and explicitly record the reduced-fidelity limitation.

Do not delay the entire program because E2B is unavailable; continue contract/product validation in the available environment and retry desktop proving-ground setup when practical.

## 4. Synthetic application ecosystem

Do not make validation depend on unstable commercial SaaS accounts.

Build realistic synthetic applications that expose actual UI, state, APIs, files, permissions, events, and failure modes.

Required proving grounds:

### Construction

- project/task management;
- site progress/photos;
- contractor records;
- budgets/procurement;
- invoices;
- safety incidents;
- approvals;
- notifications;
- document repository.

### Software/Google-like enterprise

- Git forge;
- issue/project tracker;
- CI dashboard;
- deployment console;
- incident console;
- service dashboard;
- internal documentation;
- team/ownership directory.

### Ride-share

- driver onboarding;
- driver support;
- operations dashboard;
- trips/regions;
- maps;
- payments/earnings;
- incident management;
- communications.

### Media

- CMS/editorial system;
- asset repository;
- image/video workflow;
- approvals;
- publication/distribution;
- corrections;
- social distribution dashboard.

### Marketplace

- workflow catalog;
- listing/search;
- installation/configuration;
- entitlement/licensing;
- upgrades;
- rollback;
- attribution/provenance.

These applications may be purpose-built for the proving ground. They must behave like plausible products rather than static mocks.

## 5. Validation waves

### Wave 0 — Proving-ground bootstrap

Run up to three workers concurrently:

```text
VWO-001 E2B/Desktop Resource Proving Ground
VWO-002 Synthetic Enterprise Application Ecosystem
VWO-003 Validation Harness + Scenario/Evidence Infrastructure
```

No worker may start human workflow validation until its required environment surface is validated.

### Wave 1 — Real-user workflow creation

Run up to three workers concurrently:

```text
VWO-004 Enterprise Operations User — construction/software/ride-share/media
VWO-005 Workflow Creator + Marketplace Seller
VWO-006 Workflow Consumer + Automation Operator
```

These workers must exercise all three teaching modes across their assigned scenarios and must use the actual application path.

### Wave 2 — Adversarial runtime/product tests

Run up to three workers concurrently:

```text
VWO-007 Persistence/Restart/Recovery Adversary
VWO-008 Security/Authorization/Trust-Boundary Adversary
VWO-009 Versioning/Marketplace/Distribution Adversary
```

They must intentionally create crashes, races, stale approvals, duplicate triggers, malicious external content, entitlement changes, version confusion, cancellation races, and environment/provider failures.

### Wave 3 — Consolidation and remediation

Run:

```text
VWO-010 Validation Synthesis + Defect Classification
```

It is not accepted until all worker reports are reconciled against current source.

For every P0/P1 finding, the Tech Lead must create and dispatch a bounded remediation Work Order. Up to three independent remediation workers may run concurrently.

### Wave 4 — Revalidation

Repeat affected scenarios after fixes.

A defect is closed only when:

1. the fix is merged;
2. the original failure reproduces as fixed;
3. no regression is introduced;
4. the relevant architectural invariant remains intact.

## 6. Teaching-mode coverage

Every worker must complete at least one workflow in each:

- `DEMONSTRATE`
- `INSTRUCT`
- `HYBRID`

Across the program, prefer workflows where the modes expose different failure characteristics.

### DEMONSTRATE

Worker performs the process in the application and lets the system observe it.

### INSTRUCT

Worker describes the desired process and evaluates whether the system correctly infers semantics, dependencies, resources, conditions, and approvals.

### HYBRID

Worker gives initial instructions and demonstrates critical steps/exceptions.

The worker must verify that instruction and demonstration are reconciled rather than one silently overriding the other.

## 7. Required workflow scenario families

At minimum include:

### Construction

- daily site progress;
- procurement approval;
- safety incident intake.

### Software company

- pull-request triage;
- production incident response;
- engineering onboarding.

### Ride-share

- driver onboarding;
- support escalation;
- surge-operations decision workflow.

### Media

- editorial publishing;
- content repurposing;
- breaking-news workflow.

### Marketplace

- create and sell workflow;
- fork and improve workflow;
- discover/install/configure workflow;
- upgrade and rollback.

### Cross-cutting adversarial scenarios

- session loss;
- concurrent duplicate triggers;
- provider failure;
- environment failure and compatible rebinding;
- malicious external content;
- credential exfiltration attempt;
- version confusion;
- approval race;
- cancellation race;
- marketplace entitlement race.

## 8. Human-behavior fidelity

Workers must:

- use the application as a human would;
- avoid preconstructing internal workflow IR when a teaching surface exists;
- avoid directly invoking internal stores to bypass missing product behavior;
- use real GUI applications in the E2B proving ground where available;
- record product friction, not just technical failures;
- report missing product surfaces instead of fabricating them.

Inspection of internal state is permitted only after the normal user path has been attempted and must be marked as diagnostic evidence.

## 9. Evidence requirements

Every workflow test record must contain:

```text
scenario
industry
persona
teaching_mode
objective
environment/workspace identity
repository SHA
steps actually performed
expected outcome
actual outcome
workflow/version identity
capabilities inferred
resources inferred
environment bindings
approvals
trigger information
evidence generated
failures
recovery behavior
restart behavior
UX/product friction
security findings
architecture findings
severity
recommended fix
```

Do not store secrets in reports.

## 10. Issue severity

```text
P0 — core product blocked, security/authorization failure, or workflow authority violation
P1 — major functionality/reliability defect
P2 — meaningful but non-blocking product defect
P3 — polish/documentation issue
```

P0/P1 findings must never be silently deferred.

## 11. Remediation rules

For every P0/P1:

1. reproduce;
2. isolate root cause;
3. identify owning semantic layer;
4. create a bounded remediation Work Order;
5. implement against current main;
6. independently verify;
7. rerun the failing product scenario;
8. update the validation report.

Prefer fixing existing contracts/integration seams over creating parallel mechanisms.

## 12. Completion gate

The validation program is complete only when:

- the three teaching modes all work on meaningful workflows;
- the application can create useful workflows rather than merely represent them;
- workflows can move through publish/install/run/recover/upgrade flows;
- E2B desktop proving-ground validation has been performed when available;
- mixed browser/computer/terminal/API/tool/MCP/human execution has been exercised;
- restart/session-loss behavior has been tested against durable M4 state;
- marketplace and commercial entitlement boundaries have been tested;
- security/adversarial scenarios have been exercised;
- P0/P1 defects are closed;
- P2/P3 defects have explicit disposition;
- final reports and evidence are pushed to GitHub;
- the final report states whether the product is genuinely production-ready.

## 13. GitHub publication rule

All plans, Work Orders, reports, evidence summaries, remediation records, and final conclusions must be committed and pushed to the GitHub repository.

The Tech Lead must never leave the architect dependent on an ephemeral chat session, sandbox filesystem, or worker-local artifact.

## 14. Operating rule

The Tech Lead may execute all validation Work Orders autonomously. The operator does not need to repost the prompts.

After every accepted Work Order, recompute the validation dependency graph and immediately dispatch newly READY work, subject to the three-worker concurrency limit.
