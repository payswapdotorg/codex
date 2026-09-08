# Codex Universal — Architect Start Here

**Status:** BOOTSTRAP / FROZEN until superseded by an approved Architecture Change Request.

This repository is a fork of OpenAI Codex. The upstream Codex runtime is the implementation substrate. The purpose of this fork is to evolve Codex into a **model-independent agent runtime with a first-class workflow platform** while preserving upstream behavior until a deliberate change is specified, implemented, tested, and frozen.

## Mission

Build the strongest practical Codex-compatible agent runtime with these properties:

1. The agent runtime is independent of any single LLM provider.
2. Coding-agent behavior remains the primary compatibility target.
3. Users can create reusable workflows from instruction and/or demonstration.
4. Users can run, maintain, version, share, install, and schedule workflows from the Codex application.
5. Workflow execution can span browser, tool/API, terminal, human, and future desktop/mobile execution modalities.
6. Workflow semantics are durable, versioned, auditable, and independent of models, providers, browsers, tools, and connectors.
7. Execution, evidence, policy, and memory are explicit runtime concerns rather than prompt conventions.

## Mandatory bootstrap for every coding agent

Before changing code:

1. Read this file completely.
2. Read `AGENTS.md`.
3. Read `docs/architecture/CODEX-UNIVERSAL-ARCHITECTURE.md`.
4. Read `docs/architecture/CODEX-UNIVERSAL-LOCK.md`.
5. Read `docs/development-state/README.md` and all referenced current-state files.
6. Inspect the live target branch and relevant commit history through Git.
7. Identify the active Work Order in `docs/work-orders/`.
8. Confirm that the requested change is inside the Work Order change surface and dependency graph.
9. Run the smallest relevant verification before modifying behavior when practical.
10. Re-read current `main` or target branch immediately before dispatching work and before final review.

## Authority hierarchy

From strongest to weakest:

1. Frozen architecture and approved Architecture Change Requests.
2. Active Work Order and dependency graph.
3. Repository source code and tests on the exact target commit.
4. Persisted verification evidence tied to exact commit SHA.
5. Upstream Codex implementation and documentation when the fork has not intentionally diverged.
6. Agent reports, conversation history, screenshots, copied summaries, and generated analysis.

Agents must never treat a claim of completion as evidence.

## Non-negotiable architectural rules

- Do not create a second agent runtime beside Codex.
- Do not create a second workflow engine beside the workflow control plane.
- Do not make an LLM, chat transcript, browser, tool, connector, Redis/cache, or external provider the owner of durable semantic state.
- Keep model/provider-specific behavior behind an explicit model-provider boundary.
- Keep execution modalities behind explicit execution adapters.
- Keep workflow semantics independent from execution modality.
- Keep reasoning mode independent from execution modality.
- Treat external model, browser, tool, API, connector, webhook, and event output as untrusted input by default.
- Never store raw credentials in workflows, prompts, memory, logs, or ordinary evidence payloads.
- Workflow versions are immutable after publication. Changes create a new version.
- Durable workflow transitions are controlled by deterministic state/policy logic; LLMs propose, they do not authorize.
- Agents propose work; workflow control state remains authoritative elsewhere.
- A new provider or execution environment must not require a second workflow protocol.
- Preserve upstream Codex compatibility unless a divergence is explicitly covered by the architecture and tests.

## Change discipline

Use this lifecycle:

`SPEC → INVARIANTS → INTERFACES → TESTS → IMPLEMENTATION → ADVERSARIAL TESTS → AUDIT → FREEZE`

Normal implementation changes require a Work Order. Architecture changes require an Architecture Change Request and a new immutable architecture version.

Every implementation branch must have one bounded Work Order unless that Work Order explicitly authorizes a composed change. Avoid overlapping sibling change surfaces.

## Workflow layer relationship

The workflow layer is **informed by** `payswapdotorg/workflows` but is not a copy of that repository and does not preserve a separate runtime.

The adapted model is:

`Teaching → Workflow Compiler → Workflow IR/Definition/Version → Workflow Control Plane → Execution Planner → Codex Agent / Tool / Browser / Human → Evidence/Memory → Learning`

The browser is the first user-facing workflow execution environment. Terminal/software-development remains core Codex functionality and can also participate as an execution modality.

## Current phase

**Phase 0 — Repository & Architecture Bootstrap.**

No workflow product implementation is authorized by this document alone. The first implementation milestone is to establish the provider-neutral model boundary, workflow contracts, control-plane contracts, and test/verification scaffolding without disturbing upstream Codex behavior.

## Completion report

Every implementation agent must report:

- Work Order ID
- base SHA
- head SHA
- changed surfaces
- tests executed and results
- acceptance-criteria evidence
- known limitations
- exact artifacts produced
- any architecture divergence introduced

`Done` is never an acceptable completion report by itself.
