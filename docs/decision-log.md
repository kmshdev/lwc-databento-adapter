# Decision log

## Status vocabulary

- **Accepted:** implementation may rely on it.
- **Assumption:** grounded working boundary that must be revisited if product context changes.
- **Open:** implementation must not choose among options without user direction.

## Decisions

### DEC-001 — TypeScript browser package and Rust gateway

- Status: Accepted
- Decision: Use a framework-neutral TypeScript package in the browser and a Rust gateway using Databento's official Rust client.
- Rationale: The repository has no existing backend to adapt. Browsers cannot use Databento's raw TCP Live API and must not receive `DATABENTO_API_KEY`.
- Consequence: Normalization, aggregation, symbology, replay, and upstream lifecycle exist only in Rust; TypeScript owns browser transport and Lightweight Charts types.
- Traces: `REQ-F-006`, `REQ-F-011`, `REQ-Q-001`, `TASK-00`, `TASK-03`, `TASK-09`.

### DEC-002 — HTTP for queries, WebSocket for live streams

- Status: Accepted
- Decision: Use JSON HTTP for history/symbology/metadata and a versioned JSON WebSocket subprotocol for live control and events.
- Rationale: Compared with SSE plus separate HTTP controls, one bidirectional channel gives explicit ordering for subscribe acknowledgement, snapshot, updates, unsubscribe, state, and errors while multiplexing many symbols.
- Consequence: Protocol v1 and its shared fixtures are a first-class compatibility contract.
- Traces: `REQ-F-011`, `TASK-02`, `TASK-10`.

### DEC-003 — Gateway-coordinated handoff

- Status: Accepted
- Decision: Only `openBars` claims gap-free history-to-live handoff. It registers and buffers replay/live at the gateway before merging history, then emits one snapshot followed by updates.
- Rationale: Separate `getBars` and `subscribeBars` calls leave an unobservable race. Databento supports recent subscription replay and explicit replay-completed system records.
- Consequence: `getBars + subscribeBars` remains available but is documented as caller-coordinated.
- Traces: `REQ-F-004`, `TASK-06`, handoff scenario suite.

### DEC-004 — Native OHLCV composition only in version 1

- Status: Accepted
- Decision: Use native `ohlcv-1s`, `ohlcv-1m`, `ohlcv-1h`, and `ohlcv-1d` directly and aggregate custom intervals from the largest native interval that evenly divides them.
- Rationale: It provides required candlesticks and live mutations for custom bars while avoiding a separate version 1 contract for trade conditions, corrections, and trade-level throughput.
- Consequence: Trade tape, BBO, depth, and trade-derived bars are out of scope. Native bars may update only when Databento publishes the interval.
- Traces: `REQ-F-005`, `ASM-004`, `TASK-03`.

### DEC-005 — UTC epoch-aligned buckets and UTC daily semantics

- Status: Accepted
- Decision: Align custom buckets to Unix-epoch multiples in UTC. Treat `1d` as Databento's native UTC-calendar aggregate, not an exchange-session candle.
- Rationale: Databento timestamps are nanoseconds since Unix epoch and default to UTC. Exchange-session candles need calendar/session metadata not included in version 1.
- Consequence: `2h` and `4h` begin at UTC multiples; exchange-session bars require a new explicit resolution type.
- Traces: `REQ-F-005`, `REQ-F-006`, `ASM-003`, `TASK-03`.

### DEC-006 — Truthful gaps by default

- Status: Accepted
- Decision: Default to `preserve-gaps`; allow opt-in `whitespace` and `carry-forward`, with no synthetic volume.
- Rationale: Databento emits no OHLCV record when no trade occurs. The default must not imply an observed price or trade.
- Consequence: Without exchange calendars, opt-in whitespace/carry-forward operates on UTC intervals and may include closed-market time.
- Traces: `REQ-F-007`, `TASK-03`.

### DEC-007 — Downstream unsubscribe is reference removal

- Status: Accepted
- Decision: Browser unsubscribe stops forwarding and removes a downstream reference. The gateway retains an upstream dataset subscription while any consumer still needs that dataset, and gracefully stops the dataset session when its final reference disappears.
- Rationale: Databento's live client exposes no unsubscribe method; a dataset session can contain multiple subscriptions.
- Consequence: Unused upstream symbols may continue to consume records until the dataset session becomes empty; they are counted and discarded.
- Traces: `REQ-F-009`, `TASK-05`.

### DEC-008 — Backpressure protects interval transitions

- Status: Accepted
- Decision: Same-bucket bar mutations may be coalesced; interval transitions and control/state/error messages may not be silently dropped. A client that cannot accept them is closed as `slow_consumer`. Upstream overflow triggers replay-based recovery.
- Rationale: Dropping a same-time mutation is recoverable by a later mutation, while dropping a new interval can permanently corrupt chart continuity.
- Consequence: Queue sizes and retry limits are operator configuration and need measured deployment values.
- Traces: `REQ-Q-002`, `TASK-07`.

### DEC-009 — No permissive public proxy in version 1

- Status: Accepted, superseded in delivery scope by `DEC-017`
- Decision: Support an explicit loopback-only development mode. Require a separate application authentication/entitlement integration before non-loopback deployment.
- Rationale: The brief requires credentials to remain server-side and says public-facing authentication and entitlement boundaries must be identified. A reusable shared browser secret would not satisfy that boundary.
- Consequence: Version 1 is private/local and single tenant. Public identity, entitlements, and market-data redistribution rights are deployment work, not hidden defaults.
- Traces: `REQ-Q-001`, `ASM-002`, `TASK-10`.

### DEC-017 — Online beta delivery is the active phase

- Status: Accepted
- Decision: Prioritize full workflow integration, basic functions, algorithm and execution-pipeline refinement, ToB/C public interfaces, reusable package consumption, and deployment to an identified online-beta target. Keep deep security, soak, penetration, and complex test programs deferred unless they become necessary to preserve essential boundary safety.
- Rationale: The user advanced the project from local qualification to online beta testing and rapid feature iteration on 2026-08-31.
- Consequence: Package publication and online deployment are active delivery concerns, but neither may guess registry ownership, licensing, deployment target, identity integration, or credentials.
- Traces: `REQ-Q-001`, `REQ-Q-005`, `ASM-002`, `TASK-10`, `TASK-11`.

### DEC-010 — Version pinning follows compatibility spikes

- Status: Accepted
- Decision: Treat researched versions as inputs, not immutable promises. Compile the required API surface, then commit lockfiles and record exact features/toolchains before implementation.
- Rationale: Registry versions and API details can drift between preparation and implementation.
- Consequence: `TASK-00` may adjust a version without changing product behavior, but any semantic contradiction returns to design review.
- Traces: `REQ-Q-005`, `TASK-00`.

### DEC-011 — Replay-based downstream resume

- Status: Accepted
- Decision: The TypeScript provider generates stable subscription IDs, retains last emitted target times, and uses `resume_bars` after an unexpected browser-to-gateway WebSocket loss.
- Rationale: Upstream reconnect alone does not cover gateway restarts or browser network interruption. Resuming from a target timestamp through the gateway's source-overlap replay closes that gap without asking the UI to rebuild all history.
- Consequence: Browser and gateway reconnect budgets are separate configuration; explicit close/protocol/auth errors do not reconnect; replay ineligibility is terminal and visible.
- Traces: `REQ-F-010`, `TASK-07`, `TASK-09`, offline E2E step 6.

### DEC-012 — Resolved identity is the routing boundary

- Status: Accepted
- Decision: `RequestedStreamKey` is only for request correlation, resolution, and upstream reference ownership. All record routing and normalized fan-out use `ResolvedStreamKey`, including resolved symbol, instrument ID, and source schema.
- Rationale: Requested ticker text is insufficient across datasets, historical mapping intervals, parent groups, and continuous mappings.
- Consequence: A mapping must exist before a record can reach an aggregator or downstream consumer; unresolved/ambiguous records are dropped visibly.
- Traces: `REQ-F-008`, `TASK-04`, `TASK-05`.

### DEC-013 — `openBars` operates only at the live edge

- Status: Accepted
- Decision: Floor current time to the source interval as boundary `T`; require the caller's `to` to floor to that same live edge; query completed source starts below `T`; deliver the source bucket at `T` only through replay/live.
- Rationale: This removes ambiguity for non-aligned `to` and prevents a historical request from being mislabeled as a continuous handoff.
- Consequence: Older snapshots use `getBars`; custom target snapshots may contain a partial target bucket that equal-time live updates replace.
- Traces: `REQ-F-004`, `TASK-06`.

## Accepted product decisions

### DEC-014 — Session-pinned live continuous symbols

- Status: Accepted.
- Decision: Resolve a continuous symbol at session start. On reconnect, resume only when re-resolution is unchanged; otherwise emit terminal `resolved_instrument_changed` and require an explicit new subscription.
- Rationale: Databento does not remap an existing live continuous subscription, and a chart series must never silently mix instruments.
- Traces: `REQ-F-008`, `TASK-08`, live reconnect fixtures, demo status.

### DEC-015 — Resolve parent then select one child

- Status: Accepted.
- Decision: `resolveSymbol` returns all children; bar methods reject `stypeIn: 'parent'` with `unsupported_parent_series` before upstream access until the caller supplies one child.
- Rationale: One candlestick series represents one resolved instrument; fan-out would expand the public API and combining children lacks an accepted financial rule.
- Traces: `REQ-F-008`, `TASK-04`, `TASK-05`, `TASK-08`, `TASK-09`.

### DEC-016 — Explicit atomic Go-live reset

- Status: Accepted.
- Decision: Preserve the arbitrary historical view while a bounded live-edge `openBars` snapshot loads; announce the reset and replace the chart atomically only after success. Failure retains history.
- Rationale: This separates historical exploration from coordinated live handoff without inventing unbounded gap-bridging semantics.
- Traces: `REQ-F-004`, `REQ-F-012`, `TASK-08`, `TASK-11`.

### Provider qualification discovery

- Status: Accepted.
- Decision: `test:live-databento` accepts no user-supplied arguments. It inventories account-aware metadata and runs one fixed fifteen-minute `GLBX.MDP3` / `ES.FUT` / `ohlcv-1m` live replay proof.
- Rationale: the fixed probe is bounded, reproducible, and proves a real dataset-specific live entitlement without pretending the global catalog describes live licenses.
- Consequence: the successful session qualifies `GLBX.MDP3` authentication, acknowledgement, replay completion, and close; open-market bar events and other live datasets remain separate evidence.
- Traces: `TASK-11`, `docs/test-strategy.md`, `mise run test:live-databento`.

### DEC-018 — Sub-50-microsecond dedicated connectivity boundary

- Status: Accepted by user target on 2026-08-31.
- Decision: Treat sub-50 microseconds as the 90th-percentile Databento Live Raw TCP physical handoff from Databento's boundary switch to the customer cross-connect, using a 10G or 25G port at DC3 or Equinix NY4/5.
- Rationale: This is the boundary and minimum topology documented by Databento for the published 42.4-microsecond estimate. Browser and gateway timers measure different systems.
- Consequence: The target remains externally blocked until the user selects and authorizes a site/provider/port, the circuit is installed, and circuit-specific evidence is collected.
- Traces: `REQ-Q-006`, `TASK-12`, `docs/dedicated-connectivity-plan.md`.

## Assumptions to validate during implementation

- `ASM-001`: the new repository may use the prepared mixed-language layout.
- `ASM-002`: version 1 is not an internet-facing multi-tenant redistribution service.
- `ASM-003`: UTC-calendar daily bars meet version 1 needs.
- `ASM-004`: native OHLCV sources are sufficient; trade-derived bars are unnecessary.
- `ASM-005`: operators will supply capacity and retry values; project defaults will be measured, not guessed.
- `ASM-006`: only license-permitted sanitized DBN fixtures may be committed.

If validation contradicts an assumption and changes user-visible behavior or scope, stop before choosing a branch and request a product decision.

## Research snapshot

Verified on 2026-08-30 and refreshed on 2026-08-31:

- Lightweight Charts documentation reported version 5.2; npm reported `lightweight-charts` 5.2.1. Official guidance says `setData` replaces all data and recommends `update` for realtime current/new points.
- Databento documents one dataset per Live client/session, multiple subscriptions per session, subscription-specific replay within the last 24 hours, no unsubscribe method, live `SymbolMappingMsg`, parent symbology as a group of related instruments, OHLCV `ts_event` at inclusive interval start, no OHLCV record for no-trade intervals, 1e-9 fixed-price units, nanoseconds since Unix epoch, UTC defaults, and start-inclusive/end-exclusive time parameters.
- Databento documents `list_datasets` as all valid dataset codes, `list_publishers` as publisher/dataset/venue mappings, and `get_dataset_range` as the available range given the user's entitlements. The live portal documentation, rather than a bulk API endpoint, manages subscription plans and venue licensing.
- Package registries reported TypeScript 7.0.2, Vite 8.2.2, Vitest 4.1.11, Zod 4.5.4, `ws` 8.21.3, `databento` 0.60.0, Axum 0.8.9, Tokio 1.53.1, Serde 1.0.229, Tracing 0.1.44, and Tower HTTP 0.7.0.
- The local toolchain reported Node 24.20.0, pnpm 11.19.0, and Rust/Cargo 1.97.1.

Authoritative URLs are listed at the end of `docs/technical-design.md`. Registry/toolchain facts are snapshots and must be refreshed by `TASK-00`.
