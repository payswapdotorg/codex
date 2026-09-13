# Thread removal

`thread/archive` and `thread/delete` reject attempts to remove a live internal
worker with JSON-RPC error `-32600`. The worker's owner controls its shutdown.
For example, a Guardian reviewer remains available to its parent conversation
after a client tries to archive or delete it.

After the owner releases the worker, its saved conversation can be archived or
deleted normally. Ordinary client-controlled threads keep their existing behavior.

## Workflow control plane (experimental)

Thirteen experimental `workflow/*` methods (RWO-001) mount the workflow
family's existing engine ports behind the app-server: teach a workflow
(DEMONSTRATE, INSTRUCT, or HYBRID), compile it, review it, approve it,
publish an immutable version, and operate durable instances. They require
the `experimentalApi` opt-in.

| Method | Params | Result |
| --- | --- | --- |
| `workflow/teach/start` | `{mode, name?}` | `{sessionId, name, mode, status, recordCount}` |
| `workflow/teach/instruct` | `{sessionId, text, evidence?}` | `{sessionId, mode, status, sequence, recordCount}` |
| `workflow/teach/demonstrate` | `{sessionId, kind, text, evidence?}` | `{sessionId, mode, status, sequence, recordCount}` |
| `workflow/teach/reconcile` | `{sessionId}` | `{sessionId, mode, status, demonstrationRecords, instructionRecords}` |
| `workflow/compile` | `{sessionId}` | `{candidateId, status, origin, epoch, stepCount, validation, simulation}` |
| `workflow/review` | `{candidateId}` | `{candidateId, status, origin, epoch, description, steps, capabilityInferences, bindingProposals, triggerIntents, validation, simulation}` |
| `workflow/approve` | `{candidateId, approver, reference, decision}` | `{candidateId, status, epoch, decision, approver, reference}` |
| `workflow/publish` | `{candidateId, repository?, commitSha, semanticVersion?}` | `{workflow, versionId, semanticVersion, definitionDigest, dependencyLockDigest, repository, commitSha, bindingResolution}` |
| `workflow/instance/run` | `{versionId}` | `{instanceId, workflow, versionId, status, terminal, path}` |
| `workflow/instance/list` | `{}` | `{instances}` |
| `workflow/instance/get` | `{instanceId}` | `{instance, evidence}` |
| `workflow/instance/resume` | `{instanceId}` | `{instance}` |
| `workflow/instance/cancel` | `{instanceId, reason}` | `{instance}` |

Publishing seals the approved content through the frozen contracts: the
response carries the Workflow Identity fields (semantic version, canonical
repository, immutable commit SHA, definition digest, dependency-lock
digest, version digest), so any later mutation is detectable. Instances
live in the file-backed durable stores under `<codex-home>/workflow` and
survive kill/restart; a post-crash `Running` orphan is reconciled to
`Paused` with recovery evidence and resumes only through an explicit
`workflow/instance/resume`. Teaching sessions and compiled candidates are
process-local (the in-memory-double bound of RWO-001): teaching is one
process's work, and the durable artifacts are the immutable versions
publication seals. Ordinary Codex behavior is unchanged when no workflow
method is called.
