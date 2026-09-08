# Codex-Native Capability Reuse Matrix

**Status:** FROZEN guidance for implementation
**Source:** live upstream Codex capability structure audited during architecture v0.2

The workflow platform reuses Codex-native mechanisms wherever they already provide the required capability. New workflow code must first determine whether an existing Codex capability can be bridged into the workflow execution contract.

| Capability | Workflow use | Default policy |
|---|---|---|
| Browser Use | browser navigation, observation, interaction, evidence | Prefer native Codex Browser Use when available/authorized |
| Computer Use | desktop application/window/screen automation | Prefer native Codex Computer Use when available/authorized |
| Skills | reusable capability/instructions, progressive disclosure | Reuse existing skill runtime; declare dependencies |
| Plugins | packaged multi-skill/tool capabilities | Reuse plugin runtime; declare dependency and compatibility |
| MCP | external tools/connectors and specialized environments | Treat as an execution/tool adapter |
| Node REPL / browser-computer bridge | underlying browser/computer capability plumbing where required | Integrate through capability readiness checks; never assume eager readiness |
| Subagents | workflow roles, parallel work, review, recovery | Reuse Codex agent orchestration |
| Plan mode | explicit planning for complex workflows | Reuse where useful; plan is not workflow state authority |
| Worktrees | collaborative workflow source development and isolated parallel edits | Reuse for development collaboration |
| Approvals / sandbox / policy | execution safety and authorization | Reuse; workflow must not bypass |
| Session / rollout / trace | execution history, evidence, replay/debugging | Reuse existing infrastructure where semantics permit |
| Plugin sharing / remote plugin catalog | discovery/distribution patterns | Reuse UX/protocol patterns; workflows remain a distinct semantic package |
| Skill installer / skill discovery | installing and discovering workflow dependencies | Reuse capability where compatible |
| Git integration | workflow source, collaboration and release | Extend behind WorkflowRepository/Forge contract |

## Important runtime lesson

Browser Use and Computer Use are capability-gated and may depend on runtime provisioning/bridge readiness. The workflow scheduler must distinguish:

`DECLARED` → `AVAILABLE` → `READY` → `AUTHORIZED` → `BOUND` → `EXECUTING`

A declared capability that is not ready is not executable. Failure must be explicit and diagnostic. Workflow startup must avoid blocking on unnecessary eager initialization; initialize capabilities lazily when the workflow requires them.

## Capability preference and fallback

For each workflow step:

```text
semantic capability
→ preferred Codex-native binding
→ compatible alternate binding(s)
→ policy/resource/evidence check
→ selected binding
```

Fallback is permitted only when the alternate binding satisfies the same semantic contract and policy. The system must record the selected binding and the reason for any fallback.

## Workflow dependency rule

A workflow may depend on a skill/plugin/capability package by identity and compatible version range. The installed execution environment resolves that dependency to an immutable implementation identity. A workflow cannot silently pick a different implementation whose behavior or authority is not compatible with the declared dependency.

## Features not treated as workflow semantics

Image generation, document-specific skills, and other specialized Codex capabilities can be exposed as optional execution capabilities. They should not become part of the core workflow semantic model unless a concrete workflow use case requires a new capability contract.
