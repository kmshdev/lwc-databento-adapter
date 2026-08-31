# Adapter Agent Guidance

Framework-neutral browser package `@lwc-databento/adapter`. Inherit root `AGENTS.md`. Closest file wins.

## Commands
| Task | Command |
| --- | --- |
| Unit + contract | `pnpm --filter @lwc-databento/adapter test` |
| Contract only | `pnpm --filter @lwc-databento/adapter test:contract` |
| Typecheck | `pnpm --filter @lwc-databento/adapter typecheck` |
| Lint | `pnpm --filter @lwc-databento/adapter lint` |
| Build | `pnpm --filter @lwc-databento/adapter build` |
| Pack proof | `mise run test:package-consumer` |

## Boundaries
- Export only through `src/index.ts`.
- Own HTTP/WebSocket, Zod schemas, subscriptions, and Lightweight Charts types.
- Do not import gateway crate internals, DBN types, or a UI framework. Use the public `lightweight-charts` module only.
- Do not create or retain `ISeriesApi`. Callers call `setData` / `update`.
- Never read `DATABENTO_API_KEY`.

## Conventions
- Files: kebab-case. Relative imports: `.js` suffix.
- Tests: `test/unit/`, `test/contract/`. Contract tests consume `contracts/fixtures`.
- Peer: `lightweight-charts` `^5.2.1`.

## Links
- `../../ARCHITECTURE.md`
- `../../CODING_GUIDELINES.md`
- `../../contracts/protocol-v1.md`
- `README.md`
