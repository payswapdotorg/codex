# Codex Universal

Codex Universal is the `payswapdotorg/codex` fork of OpenAI Codex.

The repository preserves the Codex coding-agent runtime while evolving it toward a **model-independent agent runtime** and a first-class **workflow layer** for reusable computer/browser workflows, with future desktop/mobile execution support.

## Agent contributors: start here

**Read [`ARCHITECT_START_HERE.md`](./ARCHITECT_START_HERE.md) before changing anything.**

Then read:

- [`docs/architecture/CODEX-UNIVERSAL-ARCHITECTURE.md`](./docs/architecture/CODEX-UNIVERSAL-ARCHITECTURE.md)
- [`docs/architecture/CODEX-UNIVERSAL-LOCK.md`](./docs/architecture/CODEX-UNIVERSAL-LOCK.md)
- [`docs/development-state/README.md`](./docs/development-state/README.md)
- [`docs/implementation-roadmap.md`](./docs/implementation-roadmap.md)
- [`docs/work-orders/`](./docs/work-orders/)

The repository's existing `AGENTS.md` remains authoritative for Codex/Rust-specific engineering rules.

## Project direction

```text
                        Codex Universal
                              |
              +---------------+----------------+
              |                                |
       Universal Model Plane             Workflow Plane
              |                                |
     any supported LLM              teach / author / version
              |                     run / share / install
              |                     schedule / learn
              |                                |
              +-------------+------------------+
                            |
                    Codex Execution Runtime
                            |
               terminal / tools / browser / API
                     human / future mobile
```

The workflow layer is semantically informed by the architecture in [`payswapdotorg/workflows`](https://github.com/payswapdotorg/workflows), but the implementation lives in this repository and reuses the Codex runtime rather than creating a second agent/workflow engine.

## Upstream compatibility

Until a deliberate architecture change is approved, upstream Codex behavior remains the compatibility target. Provider portability and workflows are implemented behind explicit architectural boundaries.

## Upstream documentation

- [Codex Documentation](https://developers.openai.com/codex)
- [Contributing](./docs/contributing.md)
- [Installing & building](./docs/install.md)
- [Open source fund](./docs/open-source-fund.md)

This repository is licensed under the [Apache-2.0 License](LICENSE).
