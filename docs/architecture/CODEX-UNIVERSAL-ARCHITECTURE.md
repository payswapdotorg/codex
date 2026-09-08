# Codex Universal Architecture

**Status:** Proposed frozen architecture for implementation bootstrap.
**Parent:** upstream OpenAI Codex runtime.
**Product objective:** Codex-compatible agent runtime + model portability + first-class reusable workflow system.

## 1. System mission

Codex Universal is a fork of Codex whose core agent, execution, session, tool, skill, MCP, approval, and client semantics remain Codex-compatible unless explicitly changed by an approved architecture version.

The fork adds two foundational capabilities:

- a **Universal Model Plane** so the Codex runtime can use different model providers without provider semantics leaking into core;
- a **Workflow Plane** so users can teach, author, version, execute, share, install, and schedule reusable computer/browser workflows from the application.

The workflow layer is a product layer over the Codex runtime. It does not fork or duplicate the Codex agent loop, tool runtime, approval engine, or execution substrate.

## 2. Layered architecture

```text
Clients
CLI | IDE | Desktop | Web | SDK | Automation
                     |
                Agent Protocol
                     |
             +-------v--------+
             | Codex Runtime   |
             | threads/turns   |
             | context         |
             | tools/skills    |
             | approvals       |
             | agents          |
             +---+---------+---+
                 |         |
        +--------v--+   +--v---------------+
        | Model     |   | Workflow Plane   |
        | Plane     |   | teaching/compiler|
        | providers |   | control/schedule |
        +--------+--+   | versions/instances|
                 |      +---------+---------+
                 |                |
                 |        +-------v--------+
                 |        | Execution      |
                 +------->| Abstraction    |
                          | browser/tool/API|
                          | terminal/human  |
                          +-------+--------+
                                  |
                       +----------v-----------+
                       | Evidence + Memory    |
                       +-----------------------+
```

## 3. Canonical authority boundaries

### Codex Runtime

Owns agent turn lifecycle, model interaction orchestration, tool invocation lifecycle, context assembly, approvals, interruption, cancellation, session transport, and agent-to-agent runtime primitives.

### Universal Model Plane

Owns provider discovery, model configuration, capability declaration, request/response normalization, streaming, cancellation, retry/transport behavior, authentication adapters, and provider-specific options.

The core runtime consumes only universal model contracts. Provider-specific SDKs/types never become semantic authority in core.

### Workflow Plane

Owns workflow definitions, immutable versions, workflow instances, stages, transitions, guards, loops, scheduling, approvals, resource requirements, capability requirements, workflow learning candidates, and workflow lifecycle state.

The workflow plane is the sole authority for workflow semantics and legal durable workflow transitions.

### Execution Plane

Owns concrete execution against a browser, terminal, API, tool, human participant, desktop runtime, or future mobile runtime. Execution adapters return normalized observations/results and never redefine workflow meaning.

### Evidence Plane

Owns observed actions/results, screenshots, DOM/accessibility observations, tool results, terminal output, test results, approvals, decisions, lineage, timing, recovery history, and cost/resource telemetry.

### Memory/Knowledge Plane

Owns episodic, semantic, procedural, and organizational knowledge with provenance/trust metadata.

## 4. Universal Model Plane

### Contract

The minimum internal model contract is:

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

The runtime must be able to select or switch a model without rewriting thread/workflow state.

### Provider tiers

1. Native providers: adapters for providers with materially distinct APIs/semantics.
2. OpenAI-compatible providers: configurable base URL/auth/header/query/model mapping.
3. Local runtimes: Ollama, LM Studio, vLLM, llama.cpp-compatible gateways, or equivalent.
4. Future custom adapters.

### Capability model

Model capabilities describe what the model itself can provide, e.g. context window, tool calling, vision, reasoning, structured output, streaming, web search, caching, and native compaction.

Agent capabilities are distinct. For example, web access, code execution, repository search, or browser control are runtime capabilities and do not depend on native model features.

### Provider isolation invariant

No core crate may depend directly on provider-specific request/response types, authentication flows, or SDK semantics when a universal contract can represent the behavior.

## 5. Workflow Plane

The workflow architecture is adapted from `payswapdotorg/workflows` V1.1. Its concepts are imported at the semantic level; its implementation remains inside this repository.

### Teaching modes

- `DEMONSTRATE`: observe a person or agent performing work.
- `INSTRUCT`: user describes the procedure.
- `HYBRID`: demonstration + instruction + correction/clarification/approval.

Teaching produces trajectories and evidence. It does not directly mutate a published workflow.

### Compilation pipeline

```text
TeachingSession
→ Trajectory
→ WorkflowCandidate
→ WorkflowIR
→ semantic validation
→ capability/resource binding proposal
→ execution-plan validation
→ approval
→ immutable WorkflowVersion
```

A compiler may replace an observed browser action with a semantically equivalent connector/API/tool action where policy and evidence permit. This is an optimization, not a semantic change.

### Workflow graph

Minimum graph primitives:

`SEQUENCE`, `PARALLEL_FORK`, `PARALLEL_JOIN`, `CONDITIONAL_BRANCH`, `LOOP`, `SUBWORKFLOW`, `WAIT`, `HUMAN_GATE`, `COMPENSATION`.

Join conditions, loop bounds, idempotency, failure transitions, and compensation semantics are versioned properties.

### Workflow instance

A WorkflowInstance references exactly one immutable WorkflowVersion plus resolved resources, execution identity, policies, and runtime state.

A running instance can pause, resume, recover, retry, or request human takeover without mutating the published WorkflowVersion.

## 6. Execution modality and reasoning mode are independent

Execution modalities:

`BROWSER`, `TOOL`, `API`, `TERMINAL`, `HUMAN`, future `DESKTOP`, `MOBILE`.

Reasoning modes:

`OPEN_ENDED`, `SEMANTIC`, `DETERMINISTIC`.

A semantic action can execute through a browser, API, tool, or terminal. Modality never implies reasoning policy.

## 7. Capability registry

Workflow semantics express capabilities, not provider tool names.

Example:

```text
create_issue
  -> GitHub API adapter
  -> Composio GitHub tool
  -> browser execution
  -> human execution
```

Bindings are selected by deterministic policy using capability, resource, authorization, reliability, evidence, cost, latency, and modality constraints.

## 8. Resource model

Resources are typed runtime dependencies, not credentials embedded in workflows.

Examples:

```text
GitHub account
Chrome profile
browser tab
Composio connected account
Slack workspace
human approver
API capability
terminal workspace
```

A resource requirement references identity/scope/capability/policy metadata. Raw credentials never become workflow semantic content.

## 9. Browser execution

Browser execution is an adapter. The initial implementation may use Chromium + Playwright/CDP/BiDi or an equivalent controlled browser bridge.

Required concepts:

```text
BrowserSession
BrowserProfile
TabIdentity
TabOwnership
Observation
Action
ActionResult
Recovery
HumanTakeover
BrowserArtifact
```

Browser scope is capability-based by profile/session/tab/origin.

## 10. Scheduling

Scheduling is separate from development-team concurrency.

A workflow scheduler evaluates:

```text
trigger
→ eligibility
→ capability requirements
→ authorization/policy
→ resource availability
→ account constraints
→ evidence requirements
→ reliability
→ latency/cost
→ execution modality
→ binding
→ execution
```

Triggers include:

`USER`, `SCHEDULE`, `WEBHOOK`, `CONNECTOR_EVENT`, `BROWSER_EVENT`, `WORKFLOW_EVENT`, `HUMAN_EVENT`.

External events never directly mutate workflow state.

## 11. Workflow sharing/installing

A workflow package is a portable, versioned artifact containing at minimum:

```text
Workflow manifest
WorkflowVersion(s)
Capability requirements
Resource requirements
Role definitions
Skill dependencies
Policy declarations
Input/output schema
Compatibility metadata
Integrity metadata
```

Installation creates a local/tenant-owned workflow definition. External providers, credentials, accounts, and local resources are rebound explicitly; secrets are never imported as workflow content.

Published versions are immutable. An update installs a new version and never silently mutates an existing active instance.

## 12. Workflow scheduling from the application

The application exposes workflow creation, browsing, version management, run/pause/resume/cancel, sharing/installing, and scheduling. These are clients of the workflow control plane, not owners of workflow state.

## 13. Learning loop

```text
Execution
→ trajectory + evidence
→ outcome/feedback
→ improvement candidate
→ replay/simulation/validation
→ explicit approval
→ new WorkflowVersion / SkillVersion / binding policy
```

No learned change silently replaces an active version.

## 14. Security invariants

- LLM output is untrusted input, never policy authority.
- Browser, webpage, connector, API, MCP, webhook, and tool output is untrusted by default.
- Credentials are capability-scoped and excluded from ordinary prompts, workflow contents, memory, logs, and evidence payloads.
- Human takeover does not bypass authorization or evidence requirements.
- Approval is explicit and auditable.
- External instructions become executable only through authorized capability/policy pathways.
- Provider adapters cannot silently alter workflow semantics.

## 15. Observability

Every agent turn and workflow execution should be traceable to:

```text
thread
turn
workflow/version/instance
step
role
model/provider
capability
resource binding
execution modality
approval
actions
observations
recovery
artifacts
cost/latency
outcome
```

Evidence must be tied to an exact execution/version identity.

## 16. Dependency direction

```text
clients
  ↓
protocol
  ↓
workflow / agent orchestration
  ↓
universal model + tool contracts
  ↓
adapters
  ↓
provider/execution implementations
```

Provider implementations may depend on universal contracts. Universal contracts may not depend on providers.

Workflow semantics may depend on capability/resource/evidence abstractions. Workflow semantics may not depend directly on browser automation libraries, Composio SDK details, model SDKs, or UI state.

## 17. Backward compatibility

Upstream Codex behavior is preserved by default. Any behavior-changing fork modification must identify:

- upstream behavior;
- intended new behavior;
- affected protocol/configuration/CLI surfaces;
- compatibility risks;
- migration and test strategy.

## 18. What is explicitly out of scope for bootstrap

- Training a foundation model.
- Replacing the Codex agent loop with an unrelated framework.
- Building a second standalone workflow repository/runtime.
- Mobile execution implementation before the execution abstraction supports it.
- Silent autonomous workflow learning without versioning/approval.
