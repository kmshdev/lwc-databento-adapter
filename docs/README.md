# Documentation

Reference material for building, testing, and extending this project. For process history (the original spec, task list, and traceability matrix), see [`project/`](project/).

## Architecture and design

| Document | Content |
| --- | --- |
| [`technical-design.md`](technical-design.md) | Component ownership, public interfaces, wire messages, data invariants, and lifecycle algorithms |
| [`decision-log.md`](decision-log.md) | Accepted, assumed, and open product decisions with rationale and sources |
| [`adr/`](adr/) | Architecture decision records for structural choices |

## Lightweight Charts reference

| Document | Content |
| --- | --- |
| [`lightweight-charts-core-knowledge.md`](lightweight-charts-core-knowledge.md) | Public API surface and constraints verified against the pinned Lightweight Charts release |
| [`lightweight-charts-tutorial-knowledge.md`](lightweight-charts-tutorial-knowledge.md) | User-surface, rendering, and accessibility requirements drawn from the upstream tutorials |

## Testing

| Document | Content |
| --- | --- |
| [`test-strategy.md`](test-strategy.md) | Test layers, fixtures, failure cases, and release gates |

## Other references

| Need | File |
| --- | --- |
| Layout and layer boundaries | [`../ARCHITECTURE.md`](../ARCHITECTURE.md) |
| Naming and test conventions | [`../CODING_GUIDELINES.md`](../CODING_GUIDELINES.md) |
| Wire contract | [`../contracts/protocol-v1.md`](../contracts/protocol-v1.md) |
| Consumer API for the browser package | [`../packages/databento-lightweight-charts/README.md`](../packages/databento-lightweight-charts/README.md) |
| Deferred work and known gaps | [`project/deferred-hardening-memo.md`](project/deferred-hardening-memo.md) |
