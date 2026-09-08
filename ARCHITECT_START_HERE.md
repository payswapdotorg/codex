# Codex Universal — Architect Start Here

**Status:** FROZEN architecture bootstrap
**Current architecture:** `0.2.0`

This repository is the `payswapdotorg/codex` fork of OpenAI Codex. The Codex runtime is the primary agent/execution substrate. The fork evolves it into a model-independent agent runtime plus a Git-native workflow platform that can automate work across many execution environments.

## Mission

1. Keep the Codex coding-agent runtime as the primary execution/agent substrate.
2. Make the LLM/model provider replaceable behind universal model contracts.
3. Let users create reusable workflows from instruction, demonstration, or both.
4. Let workflows execute across browser, computer/desktop, terminal, API/tool, MCP, human, and future mobile/device/remote environments.
5. Let one workflow mix multiple environments and multiple Codex skills/plugins.
6. Treat workflows as software repositories: forkable, branchable, reviewable, composable, version-pinned, publishable, installable, and collaboratively maintainable.
7. Let users create, maintain, run, share, install, schedule, and eventually monetize workflows from the Codex application.

## Operating handoffs

- **Architect:** owns frozen architecture and Architecture Change Requests.
- **Tech Lead:** owns implementation sequencing, worker dispatch, verification, acceptance, integration, and state reconciliation. See `TECH_LEAD_START_HERE.md`.
- **Workers:** implement bounded Work Orders only. See `docs/agent-operating-model.md` and `docs/work-orders/README.md`.

The repository is intentionally prepared so an autonomous Tech Lead can take over without conversation history.

## Mandatory bootstrap

Before changing code:

1. Read this file completely.
2. Read `TECH_LEAD_START_HERE.md` if acting as Tech Lead.
3. Read `AGENTS.md`.
4. Read `docs/architecture/CODEX-UNIVERSAL-ARCHITECTURE.md`.
5. Read `docs/architecture/CODEX-UNIVERSAL-LOCK.md`.
6. Read `docs/architecture/CODEX-CAPABILITY-IMPLEMENTATION-MAP.md`.
7. Read `docs/development-state/README.md` and current state files.
8. Read `docs/work-orders/README.md` and the active Work Order(s).
9. Inspect the exact live target branch and relevant Git history.
10. Identify the active Work Order and dependency graph.
11. Verify that the requested change is inside the authorized change surface.
12. Re-read the live target branch before dispatch and before final review.

## Authority hierarchy

Frozen architecture and approved architecture changes > Work Orders/dependency graph > exact source/tests > exact verification evidence > upstream behavior where not intentionally diverged > agent reports/conversation summaries.

## Non-negotiable rules

- Never create a second Codex agent runtime.
- Never create a second workflow engine.
- Never make an LLM, browser, desktop app, tool, connector, Git branch, GitHub UI, cache, or chat transcript the owner of durable workflow semantics.
- Provider and environment specifics stay behind adapters/contracts.
- Workflow semantics are independent of execution environment.
- A workflow may mix execution environments in a single graph/execution.
- Browser steps prefer Codex Browser Use; computer/desktop steps prefer Codex Computer Use; use compatible Codex skills/plugins before inventing new mechanisms.
- Skills/plugins provide capabilities and instructions; workflows provide orchestration semantics. Do not conflate them.
- Workflow execution pins immutable source and dependency identities.
- Forks/branches/PRs are development mechanisms; published workflow revisions are immutable execution artifacts.
- External model/browser/desktop/tool/API/MCP/webhook/event output is untrusted input by default.
- Credentials never enter workflow source or ordinary logs/prompts/memory/evidence.
- Durable workflow state is policy/control-plane authority, not agent authority.
- Use existing Codex approvals, sandboxing, skills, plugins, MCP, subagents, planning, sessions, worktrees, and tracing wherever applicable.

## Workflow relationship to `payswapdotorg/workflows`

`payswapdotorg/workflows` is a semantic reference only. This repository owns the runtime implementation. Its V1.1 concepts inform teaching, compilation, control-plane authority, resources, capabilities, evidence, and learning, while this project deliberately broadens execution to as many compatible environments as practical.

## Current phase

**Phase 0 — Repository/architecture bootstrap.**

The next implementation work must first establish the provider-neutral model boundary and workflow repository/control contracts. Runtime implementation must proceed through bounded Work Orders and exact verification.

## Completion report

Every implementation agent must report Work Order, base SHA, head SHA, changed surfaces, tests/results, acceptance evidence, known limitations, artifacts, and architecture divergence (if any). `Done` is never sufficient.
