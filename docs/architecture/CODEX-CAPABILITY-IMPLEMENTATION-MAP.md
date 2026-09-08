# Codex Universal — Codex Capability Implementation Map

**Status:** FROZEN implementation guidance for architecture v0.2.0
**Purpose:** Map existing upstream Codex capabilities to the Workflow Platform so implementation agents reuse the proven substrate instead of creating parallel systems.

## 1. Rule

Before introducing a workflow capability, implementation agents MUST search the current Codex tree for an existing mechanism that already provides the required behavior. Prefer wrapping or exposing the existing mechanism behind a workflow contract over reimplementing it.

Workflow semantics remain independent of the implementation. A workflow says **what capability is required**; the runtime decides **which compatible Codex capability/adapter executes it**.

## 2. Reuse matrix

| Codex capability | Existing surface | Workflow role | Action |
|---|---|---|---|
| Browser Use | `codex-rs/config/src/browser_use.rs`, `codex-rs/config/src/browser_computer_use_requirements.rs`, BrowserUse feature/config/protocol surfaces | Browser execution | Reuse native Browser Use as preferred binding; expose normalized workflow observations/actions/results and preserve origin/access policy. |
| Computer Use | `codex-rs/config/src/computer_use.rs`, `browser_computer_use_requirements.rs`, ComputerUse feature/config/protocol surfaces | Desktop/application execution | Reuse native Computer Use as preferred binding; normalize screen/app/window actions and evidence. |
| Skills | `codex-rs/skills` | Reusable procedural capability/instruction | Reuse skill discovery, loading, metadata, dependencies, interfaces, mentions, and installation. A workflow may declare skill dependencies. |
| Plugins | `codex-rs/plugin`, `codex-rs/core-plugins` | Packaged capability bundles | Reuse local/remote plugin materialization, capability discovery, policy, installation, and lifecycle. Workflow remains semantic orchestration. |
| MCP | `codex-rs/codex-mcp`, core MCP tool lifecycle | External tool/connector environment | Bind workflow capabilities to MCP without exposing MCP server names as workflow semantics. |
| Subagents | `codex-rs/core` multi-agent context + agent orchestration | Workflow roles, parallelism, review, recovery | Reuse Codex subagent lifecycle and notifications. Pass typed workflow task/artifact context where needed. |
| Plan mode | collaboration/planning primitives | Complex workflow planning | Reuse for proposal/planning, never as durable workflow-state authority. |
| Worktrees | `codex-rs/worktree`, `codex-rs/git-utils` | Workflow source collaboration | Reuse managed worktrees for isolated workflow edits/parallel development. |
| Hooks | `codex-rs/hooks` | Lifecycle automation/guards/observability | Reuse hook lifecycle where workflow policy can safely bind to it; do not make hooks the workflow state machine. |
| Approvals | core approval/permission paths | Human authorization | Reuse native approval semantics and audit trail. |
| Sandbox/policy | execution policy/sandbox facilities | Runtime security | Reuse; workflow adapters must never bypass sandbox/policy. |
| Sessions / rollouts | thread-store/session infrastructure | Durable execution context | Reuse as operational execution/session substrate; workflow identity is layered above. |
| Rollout trace | `codex-rs/rollout-trace` | Evidence, debugging, replay | Reuse structured trace references and correlate them with workflow/version/step IDs. |
| Remote Control | `codex-rs/state/src/runtime/remote_control.rs`, app-server/transport surfaces | Remote interaction/control | Treat as an execution/transport capability and candidate future remote workflow environment. |
| Plugin sharing/catalog | `codex-rs/core-plugins/src/remote/*` | Distribution reference | Reuse distribution/permission/cache patterns; implement WorkflowRepository semantics separately. |
| Skill installer/discovery | embedded skill installer and skills extension | Workflow dependency installation | Reuse where compatible for skill dependencies and tooling. |
| Git integration | `codex-rs/git-utils`, `codex-rs/worktree` | Workflow repository lifecycle | Reuse local Git mechanics; put forge-independent workflow semantics behind `WorkflowRepository` / `WorkflowForge`. |

## 3. Browser Use boundary

The upstream browser configuration already distinguishes history access, origin access, downloads, uploads, and full CDP access. Workflow bindings MUST preserve these as runtime policy rather than collapsing them into a boolean `browser_enabled` flag.

Required binding chain:

```text
Workflow capability: navigate_web / interact_web
    ↓
Codex Browser Use binding
    ↓
Browser session/profile/tab
    ↓
origin policy / approval / capability readiness
    ↓
action
    ↓
observation + evidence
```

A browser capability that is declared but not runtime-ready is not executable. The workflow execution planner must surface the readiness failure explicitly.

## 4. Computer Use boundary

Computer Use configuration already distinguishes default application access and platform-specific application identity constraints (for example macOS bundle IDs and Windows AUMIDs/executables). These become resource/policy inputs, not workflow semantics.

Required binding chain:

```text
Workflow capability: control_desktop_app / read_screen
    ↓
Codex Computer Use binding
    ↓
computer session + target application/window
    ↓
platform access policy / approval / readiness
    ↓
action
    ↓
observation + evidence
```

## 5. Skills and plugins

Codex skill discovery already supports explicit `skill://` references, `$tool-name` mentions, skill metadata/dependencies/interfaces, root loading, and installation. Plugin infrastructure already models skills, MCP, apps, hooks, remote catalog search, install/sync, share targets, and discoverability.

Workflow dependencies should therefore use a common dependency declaration with resolved immutable implementation identities:

```text
WorkflowVersion
  ├── required skill: <id>@<compatible-range>
  ├── required plugin: <id>@<compatible-range>
  ├── required MCP capability: <id>
  └── resolved implementation digest(s)
```

The workflow does not inherit arbitrary permissions from a dependency. Installed policy remains the upper bound.

## 6. Git-native workflow repository

Codex worktree support already binds thread ownership using versioned Git metadata and avoids replacing another owner. Workflow development should leverage this for collaborative editing while adding workflow-specific repository metadata:

```text
WorkflowRepository
  ├── forge identity
  ├── repository identity
  ├── origin/fork lineage
  ├── default branch
  ├── branch/ref state
  ├── commit/revision identity
  ├── workflow manifest(s)
  ├── dependency lock
  └── release/version metadata
```

Moving branches remain development inputs. Published WorkflowVersions pin immutable revisions.

## 7. Plugin distribution as reference, not semantics

Upstream remote plugin infrastructure demonstrates useful patterns we should reuse:

- catalog search and pagination;
- cached catalog reads;
- installation and materialization reconciliation;
- share URLs;
- listed/unlisted/private discoverability;
- user/group/workspace share targets;
- reader/editor roles;
- installed-by-me/workspace/shared-with-me views;
- explicit identity reconciliation;
- bundle validation and cache mutation guards.

These patterns should inform Workflow distribution and marketplace implementation but MUST NOT cause workflows and plugins to share one semantic identity model.

## 8. Hooks as workflow extension points

Codex hooks expose lifecycle events such as pre/post tool use, permission request, pre/post compaction, session start/end, user prompt submission, subagent start/stop, stop, and interrupt. Hooks are useful for:

- observability;
- policy checks;
- workflow execution telemetry;
- controlled pre/post actions;
- integration with external workflow lifecycle tooling.

Hooks do not replace durable workflow transitions, versioning, or authorization logic.

## 9. Remote Control and future environments

Codex already has remote-control state, enrollment, transport, and app-server surfaces. Do not invent a second remote-control mechanism for workflows. Instead, determine which existing remote-control path can satisfy a workflow environment requirement and wrap it behind the execution contract.

The same principle applies to future mobile, remote desktop, device-control, and specialized application environments.

## 10. Capability readiness state machine

Every environment/capability binding must expose at least:

```text
DECLARED
AVAILABLE
READY
AUTHORIZED
BOUND
EXECUTING
FAILED
UNAVAILABLE
```

Transitions must be observable and diagnosable.

## 11. Fallback rule

For a semantic workflow capability:

```text
semantic capability
    ↓
preferred Codex-native binding
    ↓
compatible alternate binding(s)
    ↓
policy/resource/evidence checks
    ↓
selected binding
```

Fallback is allowed only if the alternate binding satisfies the same semantic contract and installed policy. The selected binding and fallback reason must be recorded as evidence.

## 12. Model independence

None of the above execution capabilities may require a specific LLM provider. A model selects or proposes actions; Codex runtime capabilities execute them. Model capability metadata is separate from execution capability metadata.

## 13. Implementation consequence

Implementation order should follow dependencies:

```text
WorkflowRepository + semantic contracts
        ↓
Execution contract + capability/resource registry
        ↓
Codex-native capability bridges
        ├── Browser Use
        ├── Computer Use
        ├── Skills / Plugins
        ├── MCP
        ├── Subagents / Plan
        ├── Approvals / Sandbox
        └── Session / Trace / Worktree
        ↓
Teaching/compiler
        ↓
Git collaboration/composition
        ↓
App / scheduling / distribution
```

Do not implement a workflow-specific copy of a capability already present in Codex unless an architecture decision explicitly establishes that the upstream mechanism cannot satisfy the workflow contract.
