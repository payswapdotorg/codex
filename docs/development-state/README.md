# Development State

This directory is the machine-readable state of the Codex Universal implementation program.

## Files

- `program-state.json` — current milestone, frozen architecture version, and gates.
- `dependency-graph.json` — Work Order dependencies and allowed parallel surfaces.
- `execution-state.json` — current implementation-agent execution state.
- `dispatch-policy.json` — machine-readable Tech Lead/worker dispatch rules and mandatory evidence.

## Operating model

`TECH_LEAD_START_HERE.md` is the repository-level autonomous implementation handoff. `docs/agent-operating-model.md` defines worker roles, ownership, lifecycle, branch rules, evidence, and escalation.

## Rules

These files describe implementation state; they do not override frozen architecture or source code.

Every state transition must be tied to a Git commit or merged PR where applicable. Stale state must be treated as stale, not authoritative.

Before dispatching work, the architect/tech lead must inspect live Git state and reconcile these files with the repository.

The Tech Lead should keep state updates small and evidence-backed. Worker reports must reference exact SHAs; a status field alone never proves implementation or acceptance.
