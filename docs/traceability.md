# Traceability matrix

This matrix is a release gate, not an index for convenience. When a requirement, interface, task, or test changes, update the whole row in the same change. A row is complete only when its named automated proof exists and passes; prose review alone is not proof.

| Requirement | Design contract | Implementation owner | Automated proof | Release evidence |
| --- | --- | --- | --- | --- |
| `REQ-F-001` Historical bars | Technical design: HTTP API; Historical processing | `TASK-03`, `TASK-04`, `TASK-09` | Rust normalization tests; gateway history HTTP tests; TypeScript response/error tests | Offline unit/integration totals; optional real historical result |
| `REQ-F-002` Incremental history | Technical design: `POST /v1/history/bars`; public `getBars`; Demo single-flight logical-range trigger | `TASK-04`, `TASK-09`, `TASK-11` | TS window split/merge/cancel tests; `barsInLogicalRange` threshold, single-flight, and viewport-preservation browser tests | Offline E2E steps 8-9 |
| `REQ-F-003` Live bars | Technical design: public TS contract; server events; ordering | `TASK-05`, `TASK-09`, `TASK-10` | Equal/newer/older unit tests; WS ordering tests | Offline E2E steps 4-5 |
| `REQ-F-004` Handoff | Technical design: Coordinated handoff algorithm | `TASK-06`, `TASK-09` | Live-edge and account-availability boundary tests; concurrent bounded replay drain; WS `subscribed -> snapshot -> bar` contract | Offline E2E steps 2-5 plus bounded real-provider browser flow |
| `REQ-F-005` Resolution/aggregation | Technical design: Resolution plan; invariants | `TASK-03` | Rust table tests and properties for every resolution/boundary/duplicate | Unit/property totals and mutation check |
| `REQ-F-006` Time/price | Technical design: Canonical model invariants | `TASK-03`, `TASK-04` | Sentinel/overflow/boundary/fixed-price tests; non-UTC TS run | Unit/property totals |
| `REQ-F-007` Empty intervals | Technical design: Gap policy | `TASK-03`, `TASK-09` | Three policy tests; no synthetic volume fixtures | Unit and protocol contract totals |
| `REQ-F-008` Symbology/isolation | Technical design: Historical processing; ordering; symbol mapping event | `TASK-04`, `TASK-05`, `TASK-08` | Historical mapping intervals; typed DBN live mapping-before-data; instrument-ID-pinned mapping-change termination; raw-symbol enrichment; cross-dataset tests | Offline integration, latest-available continuous resolution, and symbol-switch evidence |
| `REQ-F-009` Multiple consumers | Technical design: Upstream session and unsubscribe policy | `TASK-05`, `TASK-10` | Reference-count/session-sharing/two-client/disconnect tests | Offline integration totals |
| `REQ-F-010` Lifecycle/reconnect | Technical design: Reconnect and state transitions | `TASK-07`, `TASK-09` | State-machine, deterministic backoff, replay overlap, exhausted/nonretryable tests | Offline E2E step 6 |
| `REQ-F-011` Browser protocol | Technical design: HTTP API; WebSocket protocol | `TASK-02`, `TASK-10` | Shared fixtures in Rust/TS; real HTTP/WS transport tests | Contract/integration totals and protocol version |
| `REQ-F-012` Demo | Technical design: Demo; tutorial knowledge surface | `TASK-11` | Fourteen-step real-browser scenario including separate pane, accessibility, resize, and cleanup | E2E result and built demo artifact |
| `REQ-Q-001` Security | Technical design: Security model | `TASK-01`, `TASK-04`, `TASK-10`, `TASK-11` | Startup guard, validation/limit tests, origin tests, credential-sentinel scans | Leak scan and mandatory gate output |
| `REQ-Q-002` Backpressure | Technical design: Backpressure | `TASK-07`, `TASK-10` | Queue/coalescing/interval transition/slow client/upstream overflow tests | Integration totals; benchmark queue evidence |
| `REQ-Q-003` Observability | Technical design: Observability | `TASK-07`, `TASK-10` | In-memory log/metric transition, reason, label, and redaction assertions | Observability test totals |
| `REQ-Q-004` Performance | Technical design: component ownership/fan-out; Test strategy benchmark | `TASK-05`, `TASK-07`, `TASK-11` | Reproducible scenario; no-loss, interval-transition, configured-bound assertions; production-preview route p95 below 50 ms | Mandatory invariant pass plus `mise run test:performance` receipt under recorded conditions |
| `REQ-Q-005` Compatibility/release | Technical design: Dependencies and Demo; core knowledge wrapper-removal contract; Test strategy gates | `TASK-00`, `TASK-01`, `TASK-11` | Compatibility spikes, generated typings comparison, official reference-data table model test, dependency/private-import scan, and all mandatory gates | Toolchain, lock revisions, zero-warning gate receipt |

## Accepted-decision trace

`DEC-014` traces session-pinned continuous symbols through `REQ-F-008`, `TASK-08`, terminal `resolved_instrument_changed` reconnect fixtures, demo status, and public documentation. `DEC-015` traces parent resolution and explicit child selection through `REQ-F-008`, `TASK-04`, `TASK-05`, `TASK-08`, `TASK-09`, `unsupported_parent_series` fixtures, and the public API. `DEC-016` traces the explicit atomic Go-live reset through `REQ-F-004`, `REQ-F-012`, `TASK-08`, `TASK-11`, demo state, and browser fixtures. An affected row is complete only when its selected behavior and tests pass together.

`DEC-017` traces the online-beta phase through `ASM-002`, `REQ-Q-001`, `REQ-Q-005`, `TASK-10`, `TASK-11`, npm-compatible package metadata, package-level consumer documentation, the isolated full-API consumer compile, and the deferred-hardening memo. Deployment evidence remains target-specific and cannot be claimed until a target and access boundary are identified.

## Cross-artifact consistency checks

The documentation gate fails if any normative or knowledge document is missing, requirement IDs differ, task IDs differ, an accepted decision is missing, a superseded open-decision marker remains, or release commands diverge between the plan and test strategy. Runtime protocol and source validation are separate build and test gates.

During `TASK-01` and `TASK-02`, extend that script so the implementation-time gate also fails if:

- a `REQ-F-*` or `REQ-Q-*` ID in requirements has no matrix row;
- a matrix row names a missing `TASK-*` heading;
- a stable HTTP/WS error code appears in one normative document but not the error-code registry in the technical design;
- the supported resolution set or source-schema table differs between requirements, protocol fixtures, and code;
- a WebSocket message type appears in fixtures but not the technical design, or the reverse;
- an accepted decision is missing from requirements, the technical design, the implementation plan, the decision log, the traceability note, or the ExecPlan;
- a source or dependency snapshot lacks a verification date.

## Definition-of-done audit

Before release, a reviewer checks each row against actual test names and evidence. “Implemented,” a screenshot, a manual happy path, an HTTP 200 alone, or an optional provider test alone does not satisfy a row. A row may be marked `not applicable` only by changing the requirement and recording the product decision first.
