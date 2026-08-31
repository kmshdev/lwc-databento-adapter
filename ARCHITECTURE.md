# Architecture

This repository is one local gateway, one framework-neutral browser adapter, and one plain TypeScript demo. It is not a microservice fleet. Public contracts live in `docs/technical-design.md` and `contracts/protocol-v1.md`. This file is the layout map and layer boundary.

```text
Lightweight Charts application
        |
        | TypeScript API: getBars / openBars / subscribeBars
        v
packages/databento-lightweight-charts
        |
        | HTTPS JSON + WSS databento-lwc.v1
        v
services/databento-gateway
  HTTP  -> history + symbology -> Databento Historical
  WS    -> dataset sessions    -> Databento Live
  shared normalization + aggregation + deduplication
```

## Layers and ownership

| Layer | Owns | Must not |
| --- | --- | --- |
| `services/databento-gateway` | `DATABENTO_API_KEY`, official Databento clients, DBN decode, symbology, normalization, aggregation, replay, live continuity, HTTP/WS transport | Import browser packages or send credentials to the client |
| `contracts/` | Versioned JSON wire contract and shared valid/invalid fixtures | Language-specific fixtures or implementation code |
| `packages/databento-lightweight-charts` | Browser HTTP/WebSocket, Zod decode, cancellation, subscription handles, mapping frames to Lightweight Charts types | Gateway internals, DBN types, UI frameworks, private Lightweight Charts modules, `ISeriesApi` |
| `examples/lightweight-charts-demo` | Chart creation, panes, toolbars, `setData`/`update`, demo e2e | Reusable adapter APIs or Databento credentials |

Dependency direction is application → adapter → protocol → gateway → Databento. Rust is authoritative for market-data rules. TypeScript validates the browser contract and converts protocol objects into exported chart types. No market-data rule is implemented independently in both languages.

`scripts/check-structure.sh` rejects `lightweight-chart-react`, browser-to-gateway imports, source outside the three workspace roots, and tracked `node_modules/`, `dist/`, `coverage/`, `target/`, `.cache/`.

## Structure

```text
.
├── AGENTS.md
├── ARCHITECTURE.md
├── CODING_GUIDELINES.md
├── Project_Folders_Structure_Blueprint.md  # pointer to this file
├── Cargo.toml
├── deny.toml
├── package.json
├── pnpm-workspace.yaml
├── mise.toml
├── contracts/
│   ├── AGENTS.md
│   ├── protocol-v1.md
│   └── fixtures/{http,websocket}/{valid,invalid}/
├── packages/databento-lightweight-charts/
│   ├── AGENTS.md
│   └── src/{client,errors,provider,subscriptions,types}/
├── services/databento-gateway/
│   ├── AGENTS.md
│   ├── src/{live,aggregation,historical,normalization,protocol,transport}.rs
│   └── tests/real_databento.rs
├── examples/lightweight-charts-demo/
│   ├── AGENTS.md
│   ├── src/{demo-app.ts,main.ts,style.css}
│   └── test/e2e/
├── docs/
├── scripts/
└── .github/workflows/ci.yml
```

pnpm workspaces: `packages/*`, `examples/*`. Cargo workspace member: `services/databento-gateway`.

## Placement

- New public browser API: adapter `src/index.ts`, then wire shape plus valid/invalid fixtures, then matching gateway and adapter tests.
- Demo-only controls stay in `examples/`.
- Gateway integration tests stay in the gateway crate.
- Structural decisions get an ADR under `docs/adr/`.
- Extension shapes: `packages/databento-lightweight-charts/src/<feature>/`, `services/databento-gateway/src/<domain>/`, `examples/lightweight-charts-demo/src/<area>/`.

## Navigation

Start from `docs/EXECPLAN.md`, then `contracts/protocol-v1.md`, `services/databento-gateway/src/main.rs`, `packages/databento-lightweight-charts/src/index.ts`, `examples/lightweight-charts-demo/src/main.ts`.

Update this file when a top-level workspace, public entry point, or enforced placement rule changes. Count non-generated files with:

`rg --files -g '!node_modules/**' -g '!target/**' -g '!dist/**' -g '!coverage/**' -g '!.cache/**' | wc -l`
