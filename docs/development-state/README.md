# Development State

This directory is the machine-readable state of the Codex Universal implementation program.

## Files

- `program-state.json` — current milestone, frozen architecture version, and gates.
- `dependency-graph.json` — Work Order dependencies and allowed parallel surfaces.
- `execution-state.json` — current implementation-agent execution state.

## Rules

These files describe implementation state; they do not override frozen architecture or source code.

Every state transition must be tied to a Git commit or merged PR where applicable. Stale state must be treated as stale, not authoritative.

Before dispatching work, the architect/tech lead must inspect live Git state and reconcile these files with the repository.
