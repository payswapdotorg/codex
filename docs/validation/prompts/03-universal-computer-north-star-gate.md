# Codex Universal — Universal Computer Automation North-Star Gate

This prompt is a repository artifact for the Tech Lead and validation workers. It is subordinate to the frozen architecture and active validation dependency graph.

## Mission

Do not stop product validation after the original human-workflow suite. The north star is:

> Codex Universal should be capable of automating arbitrary computer-based work that a human can perform, subject to the computer capabilities, authorization, resources, interfaces and environment actually available.

The purpose of this gate is to gather evidence that the architecture is genuinely general-purpose rather than tuned to the scenarios already designed.

## Mandatory sequence

After VWO-010 is accepted, execute:

```text
VWO-011 → VWO-012 + VWO-013 + VWO-014 → VWO-015 → VWO-016 → VWO-017
```

Maximum three workers concurrently. VWO-012, VWO-013 and VWO-014 are intentionally parallel.

## Rules

1. Test native desktop GUI work, not only browser automation.
2. Test browser/web application work broadly.
3. Test terminal, filesystem, data and developer-tool work.
4. Test workflows crossing multiple applications and execution environments.
5. Hold out some human goals until execution time.
6. For held-out goals, do not provide workflow IR or an expected action sequence.
7. Teach held-out goals through DEMONSTRATE, INSTRUCT and HYBRID across the worker pool.
8. Re-run successful workflows with changed but semantically equivalent state.
9. Record whether the system adapts, recovers, requests legitimate human intervention, or fails.
10. Never treat model text as proof that a computer-side operation succeeded; require environment evidence.
11. Never create a benchmark-specific workflow engine, planner or semantic runtime.
12. Do not redefine the north star as “passes the benchmark.” The benchmark measures evidence of breadth and generalization.
13. VWO-017 must classify the result as `NORTH_STAR_SUPPORTED`, `NORTH_STAR_PARTIALLY_SUPPORTED`, or `NORTH_STAR_NOT_SUPPORTED`.

## Required final distinction

The final report must separate:

- directly demonstrated capabilities;
- capabilities generalized to held-out tasks;
- capabilities still untested;
- capabilities blocked because the required environment capability does not exist;
- product defects;
- architecture defects.

A finite validation suite cannot mathematically prove an infinite task space. The objective is a rigorous empirical determination of whether Codex Universal is behaving as a general-purpose computer-work automation platform.