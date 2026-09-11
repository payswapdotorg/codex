# Codex Universal Human-Workflow Validation Program

This directory defines the repository-native, autonomous product-validation program that follows the architectural implementation roadmap.

The program exists to answer a different question from unit/integration tests:

> Can a normal human use Codex Universal to teach, validate, publish, install, execute, recover, improve, distribute, and evolve useful workflows across realistic applications and environments?

The program is executable by the Tech Lead without additional operator prompts.

## Entry point

Read:

1. `docs/validation/VALIDATION-PROGRAM.md`
2. `docs/validation/validation-dependency-graph.json`
3. `docs/validation/prompts/01-final-completion-tech-lead-orchestration.md`
4. `docs/validation/prompts/02-human-workflow-validation.md`
5. `docs/validation/work-orders/`

The Tech Lead must reconcile these documents against the current source tree before dispatch.

## Validation environment

The preferred proving ground is an E2B desktop-capable sandbox provisioned through the user's connected Composio/E2B integration when available.

E2B is treated strictly as a resource provider. It must not acquire workflow semantics or become a second workflow engine.

The fallback path is the existing agent sandbox/application environment when desktop E2B provisioning is unavailable. The fallback must be recorded rather than silently presented as equivalent to the full proving ground.

## Program outputs

All worker and Tech Lead evidence is committed under:

- `docs/validation/reports/`
- `docs/validation/evidence/`
- `docs/validation/state/`

Every report must identify the exact repository SHA used for validation.

## Final gate

The validation program is complete only after:

- all three teaching modes have been exercised against realistic workflows;
- multiple industries and the marketplace lifecycle have been exercised;
- realistic desktop/browser/API/tool/human combinations have been tested;
- restart, recovery, authorization, versioning, and adversarial cases have been attempted;
- P0/P1 findings have been fixed and re-tested;
- remaining P2/P3 findings have explicit disposition;
- the final report states whether the product is genuinely production-ready.
