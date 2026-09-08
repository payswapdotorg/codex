# Codex Universal Architecture Lock

**Version:** 0.2.0
**Status:** FROZEN
**Effective base:** `4b0d9669cc46ba97bf85fa6312431b630d80498d`
**Change record:** `ARCHITECTURE-CHANGE-REQUEST-001.md`
**Supersedes:** `0.1.0-bootstrap`

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

## Intentionally unfrozen implementation choices

- exact Rust crate split for workflow/model contracts;
- storage backend and deployment topology;
- exact browser/computer-use integration mechanism where multiple Codex-native mechanisms exist;
- Git forge abstraction implementation beyond GitHub;
- workflow package registry/service topology;
- marketplace/monetization settlement implementation;
- mobile runtime implementation;
- model routing algorithm;
- persistent memory implementation.

## Architecture change rule

A frozen decision changes only through an Architecture Change Request containing current/proposed decision, motivation/evidence, affected invariants/interfaces, migration, compatibility impact, and verification impact. The approved request must be recorded before implementation work begins.
