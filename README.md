# Databento adapter for Lightweight Charts

A framework-neutral TypeScript package and Rust gateway that load Databento historical OHLCV bars into [Lightweight Charts](https://github.com/tradingview/lightweight-charts) 5.2, then hand off to live updates without gaps, duplicates, or exposing the Databento API key to the browser.

The project is in online beta: the core history-to-live workflow runs end to end, and the public API may still change before a tagged release.

## Components

| Path | Package | Role |
| --- | --- | --- |
| `services/databento-gateway` | `databento-gateway` (Rust) | Owns `DATABENTO_API_KEY`, decodes Databento DBN records, normalizes and aggregates bars, streams replay-to-live continuity over WebSocket |
| `packages/databento-lightweight-charts` | `@lwc-databento/adapter` (TypeScript) | Browser HTTP/WebSocket client; exposes `getBars`, `openBars`, `subscribeBars`, and Lightweight Charts-native types |
| `examples/lightweight-charts-demo` | `@lwc-databento/demo` (TypeScript, private) | Reference chart application: history load, infinite scroll-back, live go-live, reconnect |
| `contracts/` | n/a | Versioned wire contract (`protocol-v1.md`) and shared valid/invalid fixtures used by both languages |

See [`ARCHITECTURE.md`](ARCHITECTURE.md) for the data-flow diagram and layer boundaries.

## Quick start

```sh
mise run setup               # install toolchains and dependencies, no Databento key required
mise run dev:gateway:seeded  # start the gateway against deterministic seeded data
mise run dev:demo            # start the demo, unsets DATABENTO_API_KEY for the browser process
```

Open the demo URL printed by Vite. It loads seeded history and goes live against the seeded feed with no external account required.

To run against real Databento data, put `DATABENTO_API_KEY` in an untracked `mise.local.toml` (see `.env.example` for the variable name), then:

```sh
mise run dev:gateway   # DATABENTO_GATEWAY_SOURCE=historical, requires the key
mise run dev:demo
```

## Using the package

Install and usage examples for `@lwc-databento/adapter`, including the history-to-live handoff, symbol resolution, and the optional `bindSeries` convenience wrapper, are in [`packages/databento-lightweight-charts/README.md`](packages/databento-lightweight-charts/README.md).

## Testing

| Task | Command |
| --- | --- |
| Full offline gate (format, lint, typecheck, build, tests) | `mise run check` |
| Rust unit and property tests | `cargo test --workspace --all-features` |
| TypeScript adapter tests | `pnpm --filter @lwc-databento/adapter test` |
| Offline browser end-to-end | `pnpm --filter @lwc-databento/demo test:e2e` |
| Published-package consumer proof | `mise run test:package-consumer` |
| Bounded live Databento round trip (requires the key) | `mise run test:live-databento` |
| Real gateway + demo, real feed (requires the key) | `mise run test:live-integration` |

`docs/test-strategy.md` describes the full test layer breakdown and release gates.

## Documentation

Start at [`docs/README.md`](docs/README.md) for the architecture, decision log, and Lightweight Charts reference material a contributor needs. Repository conventions are in [`CODING_GUIDELINES.md`](CODING_GUIDELINES.md) and [`ARCHITECTURE.md`](ARCHITECTURE.md).

## Contributing

See [`CONTRIBUTING.md`](CONTRIBUTING.md) for the development setup, branch workflow, and pull request checklist. This project follows the [Contributor Covenant](CODE_OF_CONDUCT.md). Report vulnerabilities as described in [`SECURITY.md`](SECURITY.md).

## License

[MIT](LICENSE)
