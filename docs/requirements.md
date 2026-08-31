# Requirements

## Product outcome

A browser application can load Databento historical candles, render them with Lightweight Charts, transition to live updates without gaps or duplicates, switch among multiple symbols, and load older history. The Databento API key remains in a server-side Rust gateway. The browser uses a reusable TypeScript package and works with Lightweight Charts data objects rather than DBN records.

## Actors and trust boundaries

- **Application developer:** installs the TypeScript package and configures the gateway URL.
- **Browser user:** selects dataset, symbol, symbol type, resolution, and time range in the demo.
- **Gateway operator:** supplies Databento credentials, origin and dataset allowlists, and capacity limits.
- **Databento:** untrusted external network dependency and source of historical and live records.
- **Browser input:** untrusted. Every request and WebSocket message is validated at the gateway.

The initial release is a loopback local, single-tenant adapter. It must bind to loopback in development and must not be deployed as a public market-data proxy without an application authentication, entitlement, and licensing layer.

## In scope for version 1

- A framework-neutral TypeScript browser package compatible with Lightweight Charts 5.2.
- A Rust gateway using Databento's official Rust client.
- Historical and live candlestick bars plus matching volume points.
- Multiple datasets, symbols, charts, panes, and browser clients within configured limits.
- Raw symbols, instrument IDs, parent-symbol resolution, and continuous symbols where Databento supports them. Live continuous sessions are instrument-pinned; parent bar methods require explicit child selection.
- Native `1s`, `1m`, `1h`, and `1d` bars and UTC-aligned custom `5s`, `15s`, `30s`, `5m`, `15m`, `30m`, `2h`, and `4h` bars.
- Deterministic history-to-live handoff, replay, reconnect, deduplication, backpressure, observability, and a verification demo.

## Out of scope for version 1

- Order entry, brokerage, TradingView Charting Library's `IDatafeedChartApi`, and TradingView-hosted chart datafeeds.
- Trade tape, bid/ask, spread, market depth, options analytics, and custom exchange-session candles.
- A public identity provider, customer entitlements, billing, or market-data redistribution authorization.
- Persistent market-data storage or a general-purpose caching service.
- UI-framework bindings; the package exposes plain TypeScript APIs.
- Sub-second chart resolutions.

## Functional requirements

### REQ-F-001 Historical bars

`getBars` accepts a dataset, symbol, Databento input symbol type, resolution, and a start-inclusive/end-exclusive UTC range. `from` and `to` must be integer UTC seconds. The half-open range matches the Databento historical client contract. If `from` is not aligned to the selected native source interval, the gateway advances it to the next complete source-bar start; it never emits a partial leading source bar. It returns chronologically ordered Lightweight Charts candlestick or whitespace objects and aligned volume points. Empty ranges return empty arrays, not errors. Invalid ranges, unavailable data, invalid symbols, access failures, cancellation, and upstream errors produce typed errors.

### REQ-F-002 Incremental history

The provider can split a large requested range into configured time windows, cancel outstanding windows through `AbortSignal`, merge results chronologically, and deduplicate them. The demo subscribes to the visible logical range and uses `series.barsInLogicalRange(range)`; when `barsBefore` is below the configured threshold it requests exactly one preceding page, suppresses concurrent duplicate requests, prepends ordered data, and restores the prior logical viewport. No UI framework is embedded in the provider.

### REQ-F-003 Live bars

`subscribeBars` returns a disposable subscription. A successful subscription is acknowledged before bar callbacks run. A live update with the same timestamp as the current bar replaces that bar; a greater timestamp appends one bar. A timestamp less than the current bar is never sent to the chart as a normal update.

### REQ-F-004 Coordinated history-to-live handoff

`openBars` performs one gateway-coordinated operation that yields an initial history snapshot and a live subscription. Around the boundary, every upstream base bar is applied at most once, no target interval is skipped, emitted target timestamps never decrease, and a replayed current bar replaces rather than duplicates the current chart bar.

### REQ-F-005 Resolution and aggregation

The gateway accepts exactly the version 1 resolution set. It uses native Databento OHLCV for exact native resolutions and aggregates the largest native interval that evenly divides a custom target interval. Buckets are aligned to the Unix epoch in UTC. Open is the first component open, high the maximum high, low the minimum low, close the last component close, and volume the sum of component volume. Duplicate base intervals replace their earlier component before recomputation and do not double volume.

### REQ-F-006 Time and price normalization

The gateway validates Databento nanosecond timestamps and undefined sentinels, converts bar starts to integer Unix seconds, and sends no raw nanosecond integer to browser JavaScript. It converts Databento `int64` fixed-precision prices to finite JavaScript-compatible numbers on the server and rejects undefined or non-finite values. UTC is preserved; browser-local timezone is never used for bucketing.

### REQ-F-007 Empty intervals

The request accepts `preserve-gaps`, `whitespace`, or `carry-forward`; the default is `preserve-gaps`. `preserve-gaps` emits nothing for an empty target interval. `whitespace` emits only `{ time }`. `carry-forward` emits a synthetic OHLC point equal to the previous close and does not emit a volume point for that interval. No mode fabricates a trade or positive volume.

### REQ-F-008 Symbology and isolation

Every routing key contains dataset, input symbol type, requested symbol, resolution, and resolved instrument identity. Historical mappings are resolved for the requested date range. Live `SymbolMappingMsg` records update instrument routing before corresponding data is forwarded. Records that cannot be mapped unambiguously are dropped with a metric and typed status; they are never routed by ticker text alone. Parent resolution returns all children, but every parent bar method fails with `unsupported_parent_series` before an upstream bar request until the caller supplies one raw child or instrument ID.

### REQ-F-009 Multiple consumers

The same gateway supports multiple browser clients and multiple subscriptions per client. Downstream consumers with the same canonical request share one upstream dataset session and normalized stream. Unsubscribing stops forwarding immediately and decrements a reference count. It does not claim to unsubscribe from Databento, which exposes no live unsubscribe method.

### REQ-F-010 Lifecycle and reconnect

The provider exposes `idle`, `connecting`, `replaying`, `live`, `reconnecting`, `failed`, and `closed` states. On an unexpected upstream disconnect, the gateway reconnects with bounded exponential backoff and jitter, restores active subscriptions, replays from one source interval before the last accepted base interval where the 24-hour replay window permits, and deduplicates overlap. For a live continuous subscription, reconnect re-resolves the requested symbol. If the resolved instrument is unchanged, replay and live delivery resume; if it changed, the gateway terminates that subscription with `resolved_instrument_changed` and the caller must start a new explicit subscription. On an unexpected browser-to-gateway WebSocket loss, the TypeScript provider reconnects within its configured budget and resumes each active subscription from its last emitted target time through the gateway's replay path. Exhausted or ineligible replay becomes a typed failure rather than silent continuity.

### REQ-F-011 Browser protocol

Historical and symbology operations use JSON over HTTP. Live subscriptions use a versioned JSON WebSocket subprotocol. Commands and events are validated, correlated by IDs, bounded in size, and represented by the exact messages in the technical design. Databento DBN and system-message structures do not cross the browser boundary.

### REQ-F-012 Demo

The demo exposes dataset, symbol, symbol type, resolution, historical range, and connect/disconnect controls. It displays candlesticks in pane 0 and histogram volume in a separate, user-resizable pane 1 using public pane APIs. It displays connection state, requested symbol, resolved instrument, last market-data timestamp, replay/live state, and typed errors. It can switch symbols, reconnect, load an earlier range through the `barsInLogicalRange` trigger, fit content, return to realtime, and add/remove the volume pane without private model or DOM access. Application controls, accessible status, legends, and tooltips are DOM owned by the demo rather than canvas-library features. Going live announces that the view will reset, retains the arbitrary historical view while a bounded live-edge `openBars` snapshot loads, atomically replaces the view only on success, and otherwise retains history with a typed inline error.

## Quality requirements

### REQ-Q-001 Security

`DATABENTO_API_KEY` exists only in gateway process memory and environment configuration. It is never returned, logged, committed, or included in an error. The gateway validates origin, dataset, symbol type, symbol length, resolution, range, message size, concurrent client count, and subscription count against operator-supplied configuration. Development unauthenticated mode is explicit, loopback-only, and cannot start on a non-loopback bind address.

### REQ-Q-002 Backpressure

All queues are bounded by operator-supplied configuration. A slow downstream client may have same-bucket updates coalesced to the newest value. The gateway must not silently discard an interval transition, error, or state event; if it cannot enqueue those after coalescing, it closes that client with typed `slow_consumer`. Upstream queue exhaustion fails and reconnects the affected dataset session with replay rather than continuing from an unknown gap.

### REQ-Q-003 Observability

Structured logs and metrics cover upstream connection state, downstream clients, active downstream and upstream subscriptions, received records, emitted bars, deduplicated and dropped records by reason, reconnect attempts, request and live latency, queue utilization, and normalization errors. Dataset, schema, request ID, and subscription ID may be logged; credentials and complete raw market-data records may not. Per-record tracing is opt-in.

### REQ-Q-004 Performance

The hot path is asynchronous and does not perform blocking network or file operations. Fan-out normalizes and aggregates each canonical upstream record once before distributing it. The mandatory benchmark passes when it is reproducible, loses no unexplained records or interval transitions, respects configured queue/memory bounds, and labels its measured results without an unmeasured throughput claim. User-supplied latency or resource thresholds, when provided, become additional release criteria; their absence does not make `REQ-Q-004` untestable. The current user-supplied browser criterion requires every production-preview page to have `PerformanceNavigationTiming.duration` p95 below 50 ms across 20 cache-disabled local Chromium navigations after three warmups.

### REQ-Q-005 Compatibility and release quality

The package targets the current verified Lightweight Charts 5.2 contract and declares `lightweight-charts` as a peer dependency. It must not depend on `lightweight-chart-react` or another chart wrapper. Production modules import the package API; the no-bundler demo fixture may use the equivalent `window.LightweightCharts` surface emitted from `src/standalone.ts`. Rust and TypeScript public APIs have documentation. Formatting, linting, type checking, unit tests, contract tests, integration tests, a production build, dependency audit, and the offline end-to-end test all pass without warnings.

## Assumptions

- **ASM-001:** The repository is new, so the brief's preferred TypeScript client and Rust gateway architecture applies.
- **ASM-002:** The project is in online beta testing and rapid feature iteration. A non-loopback deployment must supply the host application's authentication and entitlement boundary; the adapter and gateway never invent a permissive shared browser credential.
- **ASM-003:** UTC-aligned daily bars are sufficient for version 1. Exchange-session daily bars require an explicit future requirement and calendar/session metadata.
- **ASM-004:** Databento OHLCV is the only upstream data schema used to construct version 1 bars. Trades, BBO, and depth are deferred.
- **ASM-005:** Capacity values are operator inputs. The design intentionally does not invent connection, queue, message, or subscription limits.
- **ASM-006:** Offline tests use synthetic records and only sanitized DBN-derived fixtures whose storage is permitted.

## Accepted product decisions

### DEC-014 Session-pinned live continuous symbols

A live continuous request resolves to one instrument for that subscription session. Reconnect re-resolves: an unchanged resolution resumes, while a changed resolution terminates with `resolved_instrument_changed`. The caller must explicitly start a new subscription, ensuring one chart series never mixes instruments.

### DEC-015 Parent resolution followed by child selection

`resolveSymbol` returns all parent children. `getBars`, `openBars`, and `subscribeBars` reject `stypeIn: 'parent'` with `unsupported_parent_series` before an upstream bar request. The caller selects one child and submits it as a raw symbol or `instrument_id`; children are never implicitly combined.

### DEC-016 Explicit atomic Go-live reset

When Go live is selected from an arbitrary historical view, the demo announces that the viewport will move, computes a bounded live-edge lookback, and calls `openBars`. Existing history stays visible until the snapshot succeeds, then candles and volume are replaced atomically. Failure retains the historical view and shows a typed inline error; no stale-to-live continuity is claimed.

## Release definition of done

Version 1 is done only when all requirements except explicitly out-of-scope items have passing traceability entries; every task acceptance criterion is met; all mandatory offline gates pass without warnings; optional Databento tests either pass with authorized credentials or are explicitly reported as not run; the demo proves history, handoff, live mutation, new-bar transition, reconnect, symbol switch, older-history load with viewport preservation, separate-pane volume lifecycle, resize fallback, and accessible controls; dependency checks prove no chart-wrapper package is present; security checks confirm the Databento key is absent from browser assets and messages; and documentation matches the shipped interfaces.
