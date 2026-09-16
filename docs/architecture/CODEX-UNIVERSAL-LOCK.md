# Codex Universal Architecture Lock

**Version:** 0.3.0
**Status:** FROZEN
**Effective base:** `4b0d9669cc46ba97bf85fa6312431b630d80498d`
**Change record:** `ARCHITECTURE-CHANGE-REQUEST-001.md`, `ARCHITECTURE-CHANGE-REQUEST-002.md`
**Supersedes:** `0.2.0`

## Frozen decisions

1. `payswapdotorg/codex` is the primary implementation repository.
2. Upstream OpenAI Codex is the runtime substrate, not a reference-only rewrite target.
3. The runtime must be model-provider independent behind universal model contracts.
4. Workflows are a first-class product layer inside this repository and must not create a second agent or workflow runtime.
5. Workflow execution is multi-environment from the start: browser, computer/desktop, terminal, API, tool, MCP, human, and future environments such as mobile are peers behind adapters.
6. Browser workflow steps preferentially use Codex Browser Use when available and authorized.
7. Computer/desktop workflow steps preferentially use Codex Computer Use when available and authorized.
8. A single workflow may mix execution environments and Codex skills/plugins.
9. Codex skills, plugins, MCP, subagents, planning, approvals, sandboxing, worktrees, session/rollout/trace systems, and sharing/distribution mechanisms are reused where applicable.
10. Workflow semantics are provider- and environment-independent; environment selection is a runtime binding.
11. Workflows are Git-native software artifacts with repository identity, immutable revisions, forks, branches, review/merge, releases, dependency declarations, and provenance.
12. Published WorkflowVersions pin immutable source/dependency identities. Moving branches are not execution authority.
13. Workflows may compose other workflows through explicit, version-pinned subworkflow dependencies.
14. GitHub is the first supported forge, not the semantic authority. Additional forges may be added behind an adapter.
15. Skills/plugins and workflows are distinct: skills/plugins provide capability; workflows define reusable semantic orchestration.
16. Workflow packages contain no raw credentials. Resource bindings are resolved during installation/execution.
17. Workflow sharing, installation, scheduling, publication, and future monetization are control-plane concerns and must not mutate workflow semantics.
18. Published workflow versions are immutable. Changes create new versions.
19. LLM outputs and all external environment outputs are untrusted input by default.
20. Durable workflow transitions remain deterministic/policy-controlled; agents propose, they do not own workflow authority.
21. Every behavior-changing feature requires tests and exact commit-tied verification evidence.
22. **Packs are a first-class governed system-state abstraction above Workflows.** A Pack is a versioned candidate/promoted system state organized around a user/organization mission, value model, context, policies, capabilities, domain semantics, integrations, evaluation, and immutable WorkflowVersion references.
23. **Pack revisions are immutable.** A new candidate or promoted state creates a new revision and preserves its parent/provenance.
24. **Pack authority remains bounded.** Packs may define domain behavior and composition but cannot create a second agent runtime, workflow engine, authorization authority, credential authority, or evidence authority.
25. **Mission authority remains above generated architecture.** LLMs and Pack workers may propose system states but may not silently redefine the user/organization mission or weaken platform invariants.
26. **Assurance and determinism are policy-scoped.** Pack execution can require different levels of determinism, replay, approval, evidence, model pinning, dependency pinning, and environment pinning without making the entire platform globally deterministic.
27. **Pack composition is explicit and provenance-bearing.** Material conflicts in identity, types, dependencies, capabilities, policies, or authority are errors unless resolved by an explicit governed decision.
28. **Pack evolution is a future governed control-plane loop.** Observe, diagnose, hypothesize, candidate-state construction, assurance, experiment, promote/rollback, and learning may be implemented later without changing Pack authority boundaries.

## Intentionally unfrozen implementation choices

- exact Rust crate split for workflow/model contracts;
- exact Rust crate split for Pack contracts;
- storage backend and deployment topology;
- exact browser/computer-use integration mechanism where multiple Codex-native mechanisms exist;
- Git forge abstraction implementation beyond GitHub;
- workflow package registry/service topology;
- Pack registry/marketplace topology;
- marketplace/monetization settlement implementation;
- mobile runtime implementation;
- model routing algorithm;
- persistent memory implementation;
- Pack architecture-search algorithms;
- advanced formal verification/model-checking implementation.

## Architecture change rule

A frozen decision changes only through an Architecture Change Request containing current/proposed decision, motivation/evidence, affected invariants/interfaces, migration, compatibility impact, and verification impact. The approved request must be recorded before implementation work begins.
