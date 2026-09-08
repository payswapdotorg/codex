# Architecture Change Request 001 — Multi-Environment Workflow + Git-Native Workflow Repositories

**Status:** APPROVED by product architect
**Supersedes:** `CODEX-UNIVERSAL-ARCHITECTURE.md` 0.1.0-bootstrap decisions only where stated below.

## Motivation

The workflow layer must not be browser-first in capability. Browser is one execution environment among many. Codex already contains first-class Browser Use, Computer Use, skills, plugins, MCP, remote-plugin, plugin-sharing, subagent, plan, execution, and session primitives that should be reused rather than recreated.

Workflows must also behave like software repositories: forkable, branchable, reviewable, composable, version-pinned, installable, publishable, and collaboratively maintainable. GitHub is the first forge integration, not the semantic authority.

## Approved changes

1. Workflow execution environments are open-ended. Initial targets include browser, desktop/computer use, terminal, API/tool, human, and any additional environment that can satisfy the execution contract. Mobile is a planned adapter and may be implemented as soon as a compatible runtime exists.
2. Browser workflow steps preferentially execute through Codex's Browser Use skill/runtime when available and authorized.
3. Desktop workflow steps preferentially execute through Codex's Computer Use skill/runtime when available and authorized.
4. A workflow may mix execution environments within one graph and within one stage.
5. A workflow may use multiple Codex skills/plugins and multiple execution modalities in the same execution.
6. Skill/plugin dependencies are first-class workflow dependencies with compatibility and capability declarations.
7. Existing Codex skill/plugin infrastructure is reused. We do not duplicate the skill system merely for workflows.
8. Workflow packages are Git-native artifacts. A WorkflowRepository has origin, revision, branch/ref, fork lineage, dependency declarations, and immutable version identity.
9. A published WorkflowVersion is pinned to an immutable Git revision plus workflow semantic version. A mutable branch is never an execution authority.
10. Workflows support fork, branch, pull-request/review, merge, release/tag, dependency composition, and upgrade flows through a Git forge adapter.
11. Workflow composition must support subworkflow dependencies pinned to immutable versions/revisions and must preserve each dependency's contract and provenance.
12. Workflow sharing/installation uses repository/package metadata and explicit local resource rebinding; it never transfers credentials.
13. Codex plugin sharing/marketplace mechanisms may be reused for discovery/distribution of skills/plugins, but workflow distribution remains a distinct semantic package layer.
14. The scheduler selects execution environments per step using capability, resource, policy, evidence, reliability, cost, latency, and user preference constraints.
15. Execution adapters are pluggable. Adding a new environment never changes workflow semantics or creates another workflow engine.

## Codex capabilities explicitly adopted

The implementation must evaluate and reuse, where technically applicable:

- Browser Use and external-browser/browser-pane integration.
- Computer Use / desktop UI control.
- Skills with progressive disclosure and skill dependencies.
- Plugins and plugin manifests.
- MCP as a connector/tool modality.
- Remote plugin catalog and plugin sharing concepts.
- Subagents and agent orchestration primitives.
- Plan mode and explicit execution planning.
- Worktrees/isolated execution where applicable to workflow development and parallel workflow editing.
- Existing rollout/session/trace infrastructure for execution evidence and replayability.
- Existing approvals/policy/sandbox mechanisms rather than workflow-specific permission bypasses.
- Existing configuration/capability gating infrastructure, but workflow availability must be explicit and diagnostically visible when a runtime capability is missing.

## Explicit architectural invariant

A workflow is semantic code plus dependency metadata. A workflow execution is a runtime instance of an immutable workflow revision. A GitHub repository is one supported storage/collaboration substrate for workflow source, while the workflow control plane remains the authority for executable semantics.
