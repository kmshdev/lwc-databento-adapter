# Demo Agent Guidance

Private Vite app `@lwc-databento/demo`. Inherit root `AGENTS.md`. Closest file wins.

## Commands
| Task | Command |
| --- | --- |
| Dev (no Databento key) | `mise run dev:demo` |
| Offline e2e | `pnpm --filter @lwc-databento/demo test:e2e` |
| Local beta e2e | `pnpm --filter @lwc-databento/demo test:e2e:local` |
| Live provider e2e | `mise run test:live-integration` |

## Boundaries
- Import `@lwc-databento/adapter` and the public `lightweight-charts` module only. No gateway internals.
- Demo owns chart/pane/DOM. Do not move demo-only controls into the adapter.
- `dev:demo` runs as `env -u DATABENTO_API_KEY`. Do not reintroduce the key into Vite or browser assets.
- Dispose: unsubscribe, `provider.dispose()`, `chart.remove()` on unmount (`src/main.ts`).

## Conventions
- Entry: `src/main.ts`. App: `src/demo-app.ts`.
- E2e: `test/e2e/*.e2e.ts` via `tsx` and Puppeteer.

## Links
- `../../ARCHITECTURE.md`
- `../../packages/databento-lightweight-charts/README.md`
- `../../docs/lightweight-charts-core-knowledge.md`
