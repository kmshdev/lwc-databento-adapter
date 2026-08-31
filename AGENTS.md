# Agent Instructions

Mixed Rust/TypeScript monorepo: Rust gateway owns Databento credentials and market-data rules; the TypeScript package owns browser transport and Lightweight Charts types; the demo owns UI. Closest `AGENTS.md` wins.

## Current Phase
- Online beta testing and rapid feature iteration.
- Prioritize history-to-live workflow, basic product functions, algorithm and execution-pipeline refinement, ToB/C APIs, reusable adapter integration, and online-beta deployment.
- Do not turn feature work into a production-hardening program.

## Scope Rules
- Deliver the smallest coherent online-beta slice that exercises the real workflow and stable public interfaces.
- Credentials stay server-side; bound and validate external inputs; errors must not claim false continuity or success.
- Do not start network-security overhauls, full production hardening, broad security scans, penetration testing, soak testing, or complex test-infrastructure projects.
- After a conversation that changes or reviews the project, record deferred work in `docs/deferred-hardening-memo.md`.
- Online-beta deployment is in scope only when the task names the target and access boundary; never invent credentials or expose Databento keys to a browser.
- Preserve user changes. Stop before destructive or secret-bearing actions. Product decisions: `docs/decision-log.md`.

## Commands
| Task | Command |
| --- | --- |
| Setup (no Databento key) | `mise run setup` |
| Documentation contract | `scripts/check-doc-contracts.sh` |
| Layout and import boundaries | `scripts/check-structure.sh` |
| Full offline gate | `mise run check` |
| External package consumer | `mise run test:package-consumer` |
| Real provider integration | `mise run test:live-integration` |

## Workspace
| Tree | Instructions |
| --- | --- |
| Layout and layer direction | `ARCHITECTURE.md` |
| Observed style and tests | `CODING_GUIDELINES.md` |
| Wire contract | `contracts/AGENTS.md` |
| Browser adapter | `packages/databento-lightweight-charts/AGENTS.md` |
| Gateway | `services/databento-gateway/AGENTS.md` |
| Demo | `examples/lightweight-charts-demo/AGENTS.md` |
| Docs | `docs/AGENTS.md` |
| Checks | `scripts/AGENTS.md` |

## Project References
| Need | File |
| --- | --- |
| Requirements | `docs/requirements.md` |
| Architecture and public contracts | `docs/technical-design.md` |
| Tasks and acceptance | `docs/implementation-plan.md` |
| Test scope | `docs/test-strategy.md` |
| Requirement-to-proof | `docs/traceability.md` |
| Decisions | `docs/decision-log.md` |
| Lightweight Charts constraints | `docs/lightweight-charts-core-knowledge.md` |
| Deferred hardening | `docs/deferred-hardening-memo.md` |
| Mise ownership | `docs/mise-plan.md` |
| Consumer API | `packages/databento-lightweight-charts/README.md` |

## Phase Transition
- Reassess when online-beta traffic is representative, public APIs stabilize, onboarding expands, or release-candidate work begins.
- When deferred hardening becomes the active priority, tell the user exactly: “The project should enter the next phase; agents should be updated.”
