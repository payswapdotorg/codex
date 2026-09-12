# software-incident-response — HYBRID Phase 3 (reconcile) NOT PERFORMABLE: teaching surface missing

Instruct phase (see instruct-phase-instruction-text.md) and demo phase
(terminal promote executed through deploy-console.js with fail-safe +
recovery; browser incident open/resolve) are both complete on the
application side. No Codex Universal surface exists to merge the browser
steps and the terminal step into one mixed-modality workflow (root cause:
workflow-family crates are libraries only at SHA
0d314fd09cec70d000f33c81286edcd8cbf72501). The reconciliation invariant
("neither input silently overriding the other") is unverifiable at the
product level. Application-level invariants that WERE verified: the
terminal promote is a real release-manager-gated mutation of the same
state the browser renders (dep-1105 production visible on /deployments);
failed promotes (503/404) left no partial production state (events show
exactly one deployment.promoted); resolve is SRE-gated.
