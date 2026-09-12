# Step 6 — Codex Universal teaching surface: NOT FOUND (DEMONSTRATE mode blocked)

Persona: Marco Silva (site_foreman), scenario construction-daily-progress, mode DEMONSTRATE.

What the human did to find a teaching surface (in order):
1. Scanned every page of SiteBuild Field Suite visited during this run
   (login, dashboard, /projects, /projects/p-101, /notifications, /events,
   /failures). The complete navigation is: Dashboard, Projects, Procurement,
   Safety, Notifications, Events, Failure switches. There is NO "teach",
   "record", "observe", "workflow", "compile", or "Codex Universal" entry
   point anywhere in the application UI (dashboard snapshot captured
   alongside this note).
2. No browser-visible surface outside the fixture apps is provided by the
   product either: the current repository (0d314fd09cec70d000f33c81286edcd8cbf72501)
   ships the workflow family as LIBRARY CRATES ONLY — codex-workflow-app,
   codex-teaching-compiler, codex-workflow-forge, codex-workflow-durable,
   codex-workflow-triggers, codex-workflow-evolution,
   codex-workflow-distribution, codex-eval-compat each declare [lib] only
   (no [[bin]]); no CLI subcommand, app-server route, or web UI consumes
   them (grep over cli/, app-server/, sdk/ finds no consumer).

Conclusion per VALIDATION-PROGRAM.md §8 and SCENARIO-CATALOG.md §2 binding
rule 2: the DEMONSTRATE teaching surface is MISSING. Steps 2-5 cannot be
replayed "while the system observes", and no compiled workflow can be
requested. Recorded as a finding (P1) — NOT simulated or fabricated.
