# Codex Universal Architecture

**Status:** FROZEN — Version 0.2.0
**Parent:** upstream OpenAI Codex runtime.
**Supersedes:** 0.1.0-bootstrap via `ARCHITECTURE-CHANGE-REQUEST-001.md`.
**Product objective:** Codex-compatible agent runtime + model portability + a Git-native, multi-environment workflow platform.

## 1. System mission

Codex Universal is a fork of OpenAI Codex. The Codex runtime remains the primary agent substrate. The fork adds a provider-neutral Model Plane and a first-class Workflow Plane while preserving upstream behavior unless an explicit architecture version authorizes divergence.

The workflow platform is not browser-first. A workflow may use any supported execution environment and may mix environments within one graph, stage, or execution. Browser, desktop/computer, terminal, API/tool, human, and future mobile environments are all peers behind execution contracts.

## 2. Layered architecture

```text
Clients
CLI | IDE | Desktop | Web | SDK | Automation
                     |
                Agent / App Protocol
                     |
             +-------v--------+
             | Codex Runtime   |
             | threads/turns   |
             | context/memory  |
             | tools/skills    |
             | approvals       |
             | agents/plans    |
             +---+---------+---+
                 |         |
        +--------v--+   +--v----------------+
        | Model     |   | Workflow Platform |
        | Plane     |   | teaching/compiler |
        | adapters  |   | repo/versioning   |
        +--------+--+   | control/schedule  |
                 |      +---------+----------+
                 |                |
                 |        +-------v--------+
                 |        | Execution      |
                 +------->| Plane          |
                          | browser        |
                          | computer/desk  |
                          | terminal       |
                          | API/tool/MCP   |
                          | human          |
                          | future mobile |
                          +-------+--------+
                                  |
                       +----------v-----------+
                       | Evidence / Memory    |
                       +-----------------------+
```

## 3. Authority boundaries

### Codex Runtime

Owns agent turn lifecycle, model interaction orchestration, context assembly, tool invocation, approvals, interruption, cancellation, session transport, plan/subagent primitives, and existing Codex execution semantics.

### Universal Model Plane

Owns provider discovery, model descriptors/capabilities, request/response normalization, streaming/cancellation, retry/transport, authentication adapters, provider-specific configuration, and provider routing. It is the only layer allowed to know provider-specific model protocol details.

### Workflow Platform

Owns workflow semantics: definitions, immutable semantic versions, workflow repository identity, graph structure, instances, stages, transitions, guards, loops, subworkflow references, scheduling, triggers, approvals, resource/capability requirements, dependency constraints, publication, and lifecycle state.

The workflow control plane is the sole authority for legal durable workflow transitions. LLMs and execution adapters may propose or perform actions but cannot mutate workflow meaning directly.

### Execution Plane

Owns actual interaction with supported environments and normalizes observations/results. It must never become workflow semantic authority.

### Evidence Plane

Owns execution observations, screenshots, accessibility/DOM observations, terminal output, tool results, test results, approvals, recovery history, artifacts, lineage, timing, and resource/cost telemetry.

### Memory / Knowledge Plane

Owns episodic, semantic, procedural, and organizational knowledge with provenance/trust metadata.

## 4. Universal Model Plane

The core contract is:

```text
ModelProvider
ModelDescriptor
ModelCapabilities
ModelRequest
ModelResponse
ModelEvent
ModelError
ModelSession
```

Provider classes include native providers, OpenAI-compatible gateways, local inference runtimes, and future adapters. Switching models must not rewrite thread, workflow, or repository state.

Model capabilities are distinct from runtime capabilities. Browser control, desktop control, shell execution, repository search, screenshots, MCP, skills, and workflow scheduling remain available to any model when the runtime can supply the capability.

## 5. Codex-native capability reuse

The fork SHALL reuse existing Codex mechanisms where applicable rather than creating parallel mechanisms:

- **Browser Use** for browser workflow execution when the Browser Use runtime/skill is available and authorized.
- **Computer Use** for desktop/application workflow execution when the Computer Use runtime/skill is available and authorized.
- **Skills** as reusable capability/instruction packages, including progressive disclosure and skill dependency discovery.
- **Plugins** and plugin manifests as the extension substrate for packaged capabilities.
- **MCP** as a tool/connector modality.
- **Subagents / agent orchestration** for workflow roles, parallel research, execution, review, and recovery.
- **Plan mode / planning primitives** for explicit workflow planning when useful.
- **Worktrees / isolated execution** for collaborative workflow development and parallel changes where applicable.
- **Existing approvals, sandboxing, policy, session, rollout, and tracing infrastructure** instead of workflow-specific security or lifecycle bypasses.
- **Remote-plugin and plugin-sharing concepts** as implementation references for discovery/distribution of capability packages; workflow distribution remains a distinct semantic layer.

Codex feature/skill availability is runtime capability state. A workflow must receive a diagnostic failure when a declared required capability is unavailable rather than silently substituting a weaker mechanism.

## 6. Execution environments

Execution environments are open-ended adapters. Initial environments are:

`BROWSER`, `COMPUTER`, `TERMINAL`, `API`, `TOOL`, `MCP`, `HUMAN`.

Future environments include `MOBILE`, `REMOTE_DESKTOP`, `DEVICE`, and other adapters that satisfy the execution contract.

A workflow may freely mix environments:

```text
Browser -> Computer -> Browser -> API -> Human -> Terminal
```

Environment choice is a runtime binding decision, not workflow semantics.

### Browser

Browser steps preferentially use the Codex Browser Use skill/runtime. Required runtime concepts include session, profile, tab identity/ownership, observation, action, action result, recovery, takeover, and browser artifacts.

### Computer / desktop

Computer steps preferentially use the Codex Computer Use skill/runtime. The adapter must expose normalized application/window/screen observations, actions, results, recovery, takeover, and evidence without leaking provider-specific Computer Use semantics into workflow definitions.

### Mobile

Mobile is reserved for an execution adapter. If a currently compatible Codex skill, plugin, connector, or external execution bridge can satisfy the mobile execution contract, it may be used without changing workflow semantics. A future native mobile adapter must plug into the same interface.

## 7. Workflow authoring and compilation

Teaching modes remain:

`DEMONSTRATE`, `INSTRUCT`, `HYBRID`.

Compilation is:

```text
TeachingSession
→ Trajectory + Evidence
→ WorkflowCandidate
→ WorkflowIR
→ semantic validation
→ capability/resource/skill dependency analysis
→ execution binding proposal
→ replay/simulation/verification
→ approval
→ immutable WorkflowVersion
```

The compiler may optimize an observed interaction into a more deterministic capability binding (for example Browser Use action -> API/tool call) when equivalence, policy, evidence, and resource constraints are satisfied.

## 8. Workflow as a Git-native software artifact

A workflow is treated like software, not like a saved prompt.

### WorkflowRepository

A WorkflowRepository has:

```text
repository identity
forge/provider
origin
fork lineage
default branch
branches
commits
workflow manifests
workflow definitions
skill/plugin dependencies
subworkflow dependencies
release/tag identities
access policy
maintainers
```

GitHub is the first supported forge. The workflow semantic model must not depend on GitHub-specific APIs so additional forges can be added later.

### Version identity

A published WorkflowVersion is immutable and identified by at least:

```text
workflow semantic version
repository identity
immutable source revision
workflow definition digest
dependency lock / resolved dependency identities
```

Branches and moving refs are development inputs only; execution pins an immutable revision/version.

### Collaboration

Workflows support:

```text
fork
branch
edit
review
pull request
merge
release/tag
rebase/upgrade
cherry-pick where supported
```

Multiple users may collaborate on the same workflow repository. The Codex application should provide this lifecycle without requiring users to leave the workflow surface for routine operations.

## 9. Workflow composition

A workflow may depend on other workflows:

```text
Workflow A
  -> Subworkflow B@1.4.2
  -> Subworkflow C@commit:<immutable-id>
```

Dependencies are explicit, version-pinned, integrity-checked, and provenance-bearing. A dependency upgrade creates a new candidate and cannot silently change an active published workflow.

Composition primitives include:

`SUBWORKFLOW`, `SEQUENCE`, `PARALLEL_FORK`, `PARALLEL_JOIN`, `CONDITIONAL_BRANCH`, `LOOP`, `WAIT`, `HUMAN_GATE`, `COMPENSATION`.

## 10. Skills, plugins, and workflow dependencies

A workflow may declare:

```text
required capabilities
required skills
required plugins
required MCP connectors
required tools
required resources
required model capabilities (optional policy constraint)
```

A skill/plugin is reusable implementation capability. A workflow is semantic orchestration. They must remain distinct.

Skill/plugin references are resolved at install/execute time according to policy and compatibility rules. Workflows should consume stable capability contracts rather than hard-coding transient tool identifiers where possible.

## 11. Capability and resource registry

Workflow semantics express capabilities:

```text
navigate_web
click_element
fill_form
control_desktop_app
read_screen
send_email
create_github_issue
run_terminal_command
approve_payment
```

A capability can have multiple bindings:

```text
navigate_web
  -> Codex Browser Use
  -> compatible external browser adapter
  -> API connector where semantically equivalent
  -> Human
```

Resources are typed dependencies:

```text
browser profile
desktop session
GitHub account
Slack workspace
Composio connected account
terminal workspace
human approver
mobile device
API credential capability
```

Raw credentials never enter workflow source, prompts, ordinary memory, or semantic workflow state.

## 12. Scheduling and triggers

Triggers:

`USER`, `SCHEDULE`, `WEBHOOK`, `CONNECTOR_EVENT`, `BROWSER_EVENT`, `COMPUTER_EVENT`, `WORKFLOW_EVENT`, `HUMAN_EVENT`.

Scheduling evaluates eligibility, capabilities, policy, resource/account availability, skill/plugin availability, evidence requirements, reliability, latency/cost, user preferences, and execution modality before binding.

No external event directly mutates workflow state.

## 13. Sharing, publishing, installation, and monetization

Workflow distribution is a first-class application capability.

A published workflow package contains semantic source plus dependency metadata and compatibility/integrity metadata. Installation creates a local/tenant-owned binding to an immutable version and asks the user to bind required resources.

The product must support, as separate concepts:

```text
private workflow
shared workflow
public workflow
forked workflow
installed workflow
published release
paid workflow
subscription/licensed workflow
```

Monetization is deliberately separated from workflow semantics. A future marketplace can price access to a version/release without changing its executable definition.

The architecture must support ownership/attribution, licensing terms, optional commercial entitlements, install provenance, and version upgrade policy without putting payment credentials into workflow source.

## 14. Execution, evidence, and recovery

Every workflow step produces normalized evidence. Evidence references the exact workflow/version/step/execution/resource/session identity.

Failure handling is:

```text
failure
→ classify
→ recover/retry/takeover/rebind/replan
→ verify
→ continue or escalate
```

The execution engine may rebind an execution modality when the semantic contract remains satisfied and policy allows it. Such a rebind is recorded as evidence.

## 15. Security invariants

- LLM output is untrusted input.
- Browser, webpage, desktop application, tool, API, MCP, connector, webhook, and external event output is untrusted input.
- External instructions never become executable merely because an agent/model observed them.
- Credentials are capability-scoped and excluded from workflow source and ordinary evidence/memory.
- Human takeover cannot bypass authorization.
- Execution capability does not imply permission to execute it.
- A workflow dependency cannot silently escalate permissions beyond the installed policy.
- Provider/skill/plugin failure cannot mutate workflow semantics.
- Published workflow revisions are immutable.

## 16. Observability

Every run should be traceable to:

```text
workflow repository
workflow/version
source revision
workflow instance
step
role
model/provider
skill/plugin dependencies
capability binding
resource binding
execution modality
approval
actions
observations
recovery
artifacts
latency/cost
outcome
```

## 17. Dependency direction

```text
clients
  ↓
agent/app protocol
  ↓
workflow control / agent orchestration
  ↓
universal model + tool + execution contracts
  ↓
skills/plugins/connectors/execution adapters
  ↓
provider/environment implementations
```

Workflow semantics may depend on capability, resource, evidence, dependency, and repository abstractions, but never directly on Browser Use, Computer Use, Playwright, a model SDK, Composio internals, or UI state.

## 18. Compatibility and change discipline

Upstream Codex behavior remains the default compatibility target. Any intentional divergence must identify the upstream behavior, new behavior, affected surface, compatibility risk, migration, and tests.

Architecture changes require an Architecture Change Request and a new immutable architecture version. Implementation changes require bounded Work Orders.

## 19. Explicit non-goals

- Building a second agent runtime.
- Building a second skill/plugin runtime.
- Restricting workflows to browser-only execution.
- Hard-coding workflow semantics to GitHub.
- Hard-coding workflow semantics to a specific model provider.
- Treating workflows as prompts without versioned semantics.
- Silent mutation of published workflows.
