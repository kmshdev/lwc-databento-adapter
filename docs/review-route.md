# Implementation-readiness review route

## Purpose

This file is the traceable route for the multipass review. It records what was inspected, what evidence was produced, and which gates prevent a review result from being mistaken for an implementation or runtime qualification.

## Pinned inputs

| Input | Pin or observed state | Use |
| --- | --- | --- |
| Lightweight Charts checkout | local sibling checkout at `65e78a0d61e086aeceee15eda32be1614d16c246` (`5.2.1`) | Primary source for documentation, tutorials, public API, renderers, views, models, and test-runner design |
| Core documentation | 26 indexed files under `website/docs` | Pass A: core chart contracts and primitive rendering |
| Tutorials | 93 indexed files under `website/tutorials` | Pass B: examples, user workflows, and production-like composition patterns |
| Source | `src/standalone.ts` plus bounded `api`, `model`, `renderers`, `views`, `helpers`, and `plugins` inventory | Confirm documentation against implementation and prefer built-in APIs |
| Databento Rust client | Context7 `/databento/databento-rs`; Firecrawl official repository and documentation results | Confirm live, replay, historical, and symbology boundaries |
| DBN | Context7 `/databento/dbn` | Confirm record, timestamp, schema, and metadata contracts |

Mistaken paths discovered during the review were normalized to the checkout above. No upstream files are changed by this review.

## Review graph

```text
A1 core docs + source ─┐
A2 tutorials + how-to ├─> B adapter alignment ─> C document repairs ─> D gates ─> E two reviews
A3 Databento + DBN ───┘            │                    │
                                    └── product forks ───┴──> user decision
```

Dependencies are strict: adapter-level acceptance criteria may be written after A1–A3, but implementation cannot begin until every product fork in `decision-log.md` is resolved.

## Pass definitions

### A1 — Lightweight Charts core

- Inspect series types and their data shapes.
- Inspect chart, pane, time-scale, and price-scale lifecycle APIs.
- Inspect series and pane primitive lifecycles, views, renderers, hit testing, z-order, autoscale participation, and canvas targets.
- Confirm browser time is seconds-based UTC and that time-zone transformation is an adapter or application concern.
- Compare proposed adapter abstractions with public APIs exported through `src/standalone.ts`.

### A2 — Tutorials and production-like composition

- Review non-`how_to` tutorials first, excluding Vue and Web Components integration guides.
- Review `how_to` only after the first tutorial inventory exists.
- Extract visible controls, chart states, data-loading workflows, responsiveness behavior, accessibility implications, and rendering edge cases.
- Include pixel-perfect widths, device-pixel-ratio handling, and `CanvasRenderingTarget2D` usage.

### A3 — Databento and DBN

- Confirm historical range streaming, live subscription/start/close behavior, intraday replay, symbol mappings, and control/error records.
- Separate documented contracts from adapter policy.
- Never place the API key in browser code, logs, fixtures, command output, or documentation.

### B — Adapter alignment

- Map every requirement to a component, data model, dependency, task, test, and definition-of-done statement.
- Identify custom code that can be replaced with Lightweight Charts public APIs.
- Keep chart/toolbars/sidebars ownership explicit: charts use the standalone library; platform chrome remains application code.

### C — Repair

Each round fixes the single largest ambiguity or contradiction that could cause two competent engineers to build different systems. Repairs must update all affected artifacts in the same round.

### D — Gates

- Run `scripts/check-doc-contracts.sh`.
- Validate local mise declarations without printing secret values.
- Run offline type, unit, integration, and browser gates after the implementation exists.
- Run optional live Databento qualification only when explicitly selected and bounded.

### E — Independent review

Two fresh-context reviewers independently describe the components, data model, dependencies, and definition of done. The loop stops only when their descriptions materially agree and all acceptance criteria are executable, or when a user decision is required.

## Runtime evidence status

The upstream `.puppeteerrc.cjs` exists, but this checkout currently lacks `node_modules/puppeteer` and both `dist/lightweight-charts.standalone.*.js` bundles. Therefore this round can verify the runner contract and define the browser test, but cannot honestly claim a Puppeteer runtime pass without first bootstrapping the upstream checkout. That bootstrap is a separate, state-changing gate.

## Assumptions and stop conditions

- `mise.local.toml` is local secret-bearing state owned by the user. The loop may verify that `DATABENTO_API_KEY` is declared, but it must not print or copy its value.
- The user explicitly authorizes live testing with the ignored key. The zero-argument provider task may inventory metadata and run the fixed, bounded `GLBX.MDP3` / `ES.FUT` / `ohlcv-1m` fifteen-minute replay session. It may not infer other live licenses or expand into unbounded/paid record requests.
- Documentation review is not production qualification.
- The former product forks are closed by `DEC-014`, `DEC-015`, and `DEC-016`; implementation stops only on a new material product fork, compatibility contradiction, credential risk, or failed milestone gate.
