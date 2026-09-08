# Codex Universal Work Orders

Work Orders are the only normal authorization mechanism for implementation agents. The roadmap expresses program outcomes; Work Orders express bounded, reviewable implementation ownership.

## Required fields

Every Work Order must define:

- objective;
- dependencies and downstream unlocks;
- exact authorized change surface;
- source areas that must be inspected;
- forbidden surfaces;
- architecture references;
- behavioral acceptance criteria;
- verification commands/evidence;
- completion report requirements.

## Lifecycle

```text
BLOCKED -> READY -> DISPATCHED -> IMPLEMENTED -> VERIFYING -> ACCEPTED -> MERGED
```

A Work Order may instead enter `NEEDS-ARCHITECTURE` when implementation reveals a missing or contradictory frozen decision.

## Dispatch rules

A Work Order may enter `READY` only when all dependencies are accepted/merged and the architecture version is current.

Every dispatch uses an exact base SHA. If the base changes materially while work is in progress, the Tech Lead decides whether to rebase/re-dispatch rather than allowing workers to silently mix baselines.

One bounded Work Order per implementation branch/PR unless the Work Order explicitly defines a composed change.

Do not dispatch two workers to edit the same semantic contract concurrently.

## Surface ownership

The dependency graph is the high-level ownership map. A worker may touch another surface only when the primary contract cannot be implemented otherwise, the change is minimal, and the completion report identifies the cross-surface reason.

## Architecture boundary

A Work Order implements frozen architecture. It does not authorize architecture changes. Missing decisions require an Architecture Change Request before the affected implementation resumes.

## Acceptance

A worker saying `done` is not acceptance. Acceptance requires current-tree inspection and verification by the Tech Lead or delegated review worker, with evidence tied to an exact SHA.

At minimum, acceptance checks:

1. scope and ownership;
2. architecture compliance;
3. upstream compatibility impact;
4. security invariants;
5. tests and lint/format requirements;
6. downstream readiness.

## Current execution order

```text
M0
  WO-001

M1 + M3 foundations
  WO-002 <--- WO-001
  WO-003 <--- WO-001

M2
  WO-004 <--- WO-002

M5
  WO-005 <--- WO-003

M6 + M7 + M8 + M9
  WO-006 <--- WO-005
  WO-007 <--- WO-005
  WO-008 <--- WO-003, WO-005
  WO-009 <--- WO-003, WO-005

M10
  WO-010 <--- WO-006, WO-007, WO-008, WO-009

M10/M11
  WO-011 <--- WO-010
  WO-012 <--- WO-010, WO-011

M12
  WO-013 <--- WO-002, WO-010

M13
  WO-014 <--- WO-010, WO-013

M14
  WO-015 <--- WO-005, WO-010
```

Workers must not jump the dependency graph merely because downstream implementation appears easy.

## Completion report

Every implementation report must include:

```text
work_order
base_branch
base_sha
head_sha
changed_paths
implementation_summary
test_commands
test_results
lint_format_results
acceptance_evidence
compatibility_impact
security_review
known_limitations
deferred_items
architecture_divergence
```
