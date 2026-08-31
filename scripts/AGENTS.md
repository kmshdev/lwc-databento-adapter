# Scripts Agent Guidance

Deterministic repository checks. Inherit root `AGENTS.md`. Closest file wins.

## Commands
| Task | Command |
| --- | --- |
| Docs | `sh scripts/check-doc-contracts.sh` |
| Layout | `sh scripts/check-structure.sh` |
| Secrets | `sh scripts/scan-secrets.sh` |
| Packed consumer | `sh scripts/test-package-consumer.sh` |
| Live probe | `sh scripts/test-live-databento.sh` |

## Rules
- These scripts are the mechanical source of truth. Update them when adding a required doc, entry point, or placement rule.
- `check-structure.sh` forbids tracked generated output, source outside the three workspace roots, `lightweight-chart-react`, and browser imports of gateway internals.
- `scan-secrets.sh` scans tracked files; ignored `mise.local.toml` and `.env*` are excluded.
- `test-live-databento.sh` accepts no arguments and prints `not run` without `DATABENTO_API_KEY`.
- Do not add a Databento-key fallback in committed scripts.
