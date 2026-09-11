# Codex Universal — North-Star Computer Automation Validation

**Status:** ACTIVE VALIDATION GATE
**North star:** automate anything that can be done on a computer, subject to capabilities, authorization, resources, interfaces, and environment access actually available to the system.

This is an empirical generality gate, not a claim that a finite benchmark can mathematically prove an infinite task space.

## Why this exists

The original human-workflow validation program proves that Codex Universal can perform realistic workflows and survive adversarial conditions. It does not, by itself, establish that the architecture is genuinely general-purpose rather than optimized for the scenarios we designed.

This program therefore tests the breadth and generalization of computer work after the original validation wave.

## North-star test model

```text
human goal
   ↓
DEMONSTRATE / INSTRUCT / HYBRID
   ↓
workflow construction/compilation
   ↓
capability + resource discovery
   ↓
explicit authorization/binding
   ↓
execution across one or more computer environments
   ↓
observation/evidence
   ↓
recovery/adaptation when necessary
   ↓
correct result on the original goal
   ↓
correct result on changed but semantically equivalent state
```

The decisive test is not whether a pre-written workflow can run. It is whether a normal user can teach an arbitrary, previously unseen computer goal through the product and obtain a reusable workflow that executes correctly without developer intervention.

## Coverage dimensions

Measure separately:

- **semantic universality:** one workflow model spans tested environments;
- **interaction universality:** GUI, browser, terminal, file, API/tool/MCP and human interaction are representable/executable;
- **environment universality:** a single workflow can cross modalities;
- **capability/resource universality:** required capabilities/resources can be resolved and bound without leaking provider semantics;
- **teaching universality:** DEMONSTRATE, INSTRUCT and HYBRID can teach materially different tasks;
- **execution reliability:** persistence, restart, recovery, idempotency and cancellation;
- **generalization:** performance on held-out human goals;
- **product usability:** ordinary users can accomplish tasks without understanding Rust/runtime internals.

## Evidence discipline

Every result must identify:

- exact repository SHA;
- scenario/task ID;
- human goal;
- teaching mode;
- environment/workspace;
- application(s) used;
- capabilities/resources inferred;
- workflow/version identity;
- expected vs actual outcome;
- evidence of the actual computer-side result;
- recovery/adaptation;
- human intervention, if any;
- failure classification;
- product friction.

Never treat model text claiming success as execution evidence.

## Universal claim boundary

A finite test suite can demonstrate breadth and uncover architecture limits, but it cannot establish that every conceivable computer task is solvable. The final report must therefore distinguish:

- what has been directly demonstrated;
- what has been strongly generalized from held-out tasks;
- what remains untested;
- what is impossible because required computer capabilities/interfaces are unavailable;
- what is blocked by product limitations rather than architecture.

The final north-star verdict must be evidence-led.