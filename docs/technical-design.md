# Technical design

## Design summary

The system is a mixed Rust/TypeScript monorepo. The Rust gateway exclusively owns Databento credentials, official clients, DBN decoding, symbology, normalization, aggregation, replay, upstream sessions, security limits, and fan-out. The TypeScript package owns browser HTTP/WebSocket transport, Lightweight Charts types, cancellation, subscription handles, reconnect-visible state, and mapping gateway frames into chart-native objects. The demo owns chart creation and UI behavior.

```text
Lightweight Charts application
        |
        | TypeScript API: getBars/openBars/subscribeBars
        v
packages/databento-lightweight-charts
        |
        | HTTPS JSON + WSS databento-lwc.v1
        v
services/databento-gateway
  HTTP/API  -> history + symbology -> Databento Historical
  WS hub    -> dataset sessions   -> Databento Live Raw/TCP
  shared normalization + aggregation + deduplication
```

## Repository layout to create

```text
Cargo.toml                         Rust workspace
package.json                       pnpm workspace scripts
pnpm-workspace.yaml
contracts/
  protocol-v1.md                   exact JSON contract copied from this design
  fixtures/                        valid and invalid shared JSON messages
packages/
  databento-lightweight-charts/
    src/{client,provider,types,mappings,subscriptions,errors}/
    test/
services/
  databento-gateway/
    src/{api,config,historical,live,normalization,aggregation,symbology,transport,observability}/
    tests/
examples/
  lightweight-charts-demo/
```

No market-data rule is implemented independently in TypeScript and Rust. Rust is authoritative for normalization, bucketing, aggregation, deduplication, and symbology. TypeScript only validates the browser contract and converts protocol objects into the exported chart types.

## Component responsibilities

The selected product policies are normative in this design: `DEC-014` session-pinned live continuous symbols, `DEC-015` parent resolution followed by explicit child selection, and `DEC-016` explicit atomic Go-live reset.

### TypeScript provider

- `provider/DatabentoDataProvider` exposes the public API.
- `client/HttpClient` performs history, symbol, and metadata requests and maps typed HTTP failures.
- `client/LiveSocket` owns one browser WebSocket, protocol negotiation, command correlation, and terminal close behavior.
- `subscriptions/SubscriptionRegistry` isolates callbacks by server subscription ID and disposes them idempotently.
- `mappings/toLightweightCharts` returns Lightweight Charts `CandlestickData<UTCTimestamp>`, `WhitespaceData<UTCTimestamp>`, and `HistogramData<UTCTimestamp>` without Databento fields.
- `types` contains public requests, results, states, errors, and metadata.

The provider does not import a UI framework and does not create or retain an `ISeriesApi`. Applications decide when to call `setData` and `update`.

### Rust gateway

- `config` loads and validates credentials, bind address, origins, allowlists, and capacity values. Configuration is immutable after startup.
- `api` validates HTTP input and maps domain errors to stable codes.
- `historical` calls the official Databento Historical client and streams records through the shared pipeline.
- `live::DatasetSessionManager` owns at most one active Databento `Live` client per dataset, subject to configuration.
- `live::SubscriptionRegistry` reference-counts canonical upstream requests and associates downstream IDs.
- `symbology` resolves requested symbols for historical ranges and consumes live `SymbolMappingMsg` records.
- `normalization` validates timestamps, prices, instrument IDs, and record types before constructing `BaseBar`.
- `aggregation` performs UTC bucketing, duplicate replacement, OHLCV reduction, and gap policy.
- `live::HandoffCoordinator` establishes replay, buffers base bars, loads history, merges by base-bar key, and opens steady-state delivery.
- `transport` implements the HTTP resources and `databento-lwc.v1` WebSocket protocol.
- `observability` emits structured logs and metrics without credentials or full raw records.

### Demo

The Vite demo imports the public `lightweight-charts` module directly, creates one chart, and adds candlesticks to pane 0 and histogram volume to a separate resizable pane 1. Historical-only loads call `getBars` and pass ordered arrays to `setData`. Go live announces that it will replace the current view, computes a bounded live-edge lookback from `historyChunkIntervals`, and calls `openBars` while retaining the historical view. On snapshot success it atomically passes snapshot arrays to `setData` and subsequent callbacks to `series.update`; on failure it retains history and shows a typed inline error. The demo calls `chart.remove()` and unsubscribes public events on unmount. Toolbars, status, legends, and tooltips are application DOM.

For incremental history, the demo subscribes to logical-range changes and calls `candles.barsInLogicalRange(range)`. When `barsBefore` crosses a configured threshold, a single-flight loader requests the immediately preceding half-open range, merges/deduplicates it, applies the ordered full data set, then restores the pre-load logical range. Exhausted history disables further requests until the symbol or range changes.

`src/standalone.ts` is only the no-bundler browser bootstrap that attaches the same public exports to `window.LightweightCharts`; it is not a separate adapter runtime or abstraction layer. Production package code uses module imports. The repository does not depend on `lightweight-chart-react`.

## Public TypeScript contract

The exported names, signatures, and semantics below are normative.

```ts
import type {
  CandlestickData,
  HistogramData,
  UTCTimestamp,
  WhitespaceData,
} from 'lightweight-charts';

export type SymbolType =
  | 'raw_symbol'
  | 'instrument_id'
  | 'parent'
  | 'continuous';

export type Resolution =
  | '1s' | '5s' | '15s' | '30s'
  | '1m' | '5m' | '15m' | '30m'
  | '1h' | '2h' | '4h' | '1d';

export type GapPolicy = 'preserve-gaps' | 'whitespace' | 'carry-forward';

export interface BarRequest {
  dataset: string;
  symbol: string;
  stypeIn: SymbolType;
  resolution: Resolution;
  gapPolicy?: GapPolicy;
}

export interface HistoryRequest extends BarRequest {
  from: UTCTimestamp;
  to: UTCTimestamp;
  signal?: AbortSignal;
}

export interface ProviderConfig {
  gatewayUrl: string;
  /** Positive maximum count of requested target-resolution intervals per HTTP window. */
  historyChunkIntervals: number;
  reconnect: {
    baseDelayMs: number;
    maxDelayMs: number;
    maxAttempts: number;
    jitterRatio: number;
  };
}

export interface BarMetadata {
  dataset: string;
  requestedSymbol: string;
  resolvedSymbol: string;
  instrumentId: number;
  sourceSchema: 'ohlcv-1s' | 'ohlcv-1m' | 'ohlcv-1h' | 'ohlcv-1d';
  synthetic: boolean;
}

export interface SymbolMapping {
  dataset: string;
  requestedSymbol: string;
  resolvedSymbol: string;
  instrumentId: number;
  effectiveFrom: UTCTimestamp;
  effectiveTo?: UTCTimestamp;
}

export type ResolvedSymbol = SymbolMapping;

export interface ResolveSymbolRequest {
  dataset: string;
  symbols: readonly string[];
  stypeIn: SymbolType;
  from: UTCTimestamp;
  to: UTCTimestamp;
  signal?: AbortSignal;
}

export interface SearchSymbolsRequest {
  dataset: string;
  query: string;
  stypeIn: SymbolType;
  signal?: AbortSignal;
}

export interface SymbolSearchResult {
  dataset: string;
  symbol: string;
  stypeIn: SymbolType;
  description?: string;
}

export interface PublisherMetadata {
  publisherId: number;
  name: string;
  venue: string;
}

export interface DatasetMetadata {
  dataset: string;
  schemas: readonly string[];
  publishers: readonly PublisherMetadata[];
  availableFrom?: UTCTimestamp;
  availableTo?: UTCTimestamp;
}

export type StatusReason =
  | 'initial_connect' | 'handoff_replay' | 'replay_completed'
  | 'upstream_disconnect' | 'downstream_disconnect'
  | 'retry_scheduled' | 'retry_exhausted'
  | 'client_unsubscribe' | 'server_shutdown' | 'slow_consumer';

export type ProviderErrorCode =
  | 'invalid_request' | 'invalid_range' | 'range_too_large'
  | 'origin_forbidden' | 'dataset_forbidden'
  | 'unsupported_dataset' | 'unsupported_schema'
  | 'unsupported_resolution' | 'symbol_not_found'
  | 'symbol_mapping_failed' | 'unsupported_parent_series'
  | 'resolved_instrument_changed'
  | 'unsupported_live_symbology' | 'access_denied'
  | 'quota_exceeded' | 'slow_consumer' | 'replay_unavailable'
  | 'upstream_unavailable' | 'cancelled' | 'protocol_error'
  | 'internal';

export class DatabentoProviderError extends Error {
  readonly code: ProviderErrorCode;
  readonly retryable: boolean;
  readonly requestId?: string;
  readonly subscriptionId?: string;
  readonly details: Readonly<Record<string, unknown>>;
}

export type ChartBar =
  | CandlestickData<UTCTimestamp>
  | WhitespaceData<UTCTimestamp>;

export interface BarPage {
  bars: ChartBar[];
  volumes: HistogramData<UTCTimestamp>[];
  metadata: ReadonlyMap<UTCTimestamp, BarMetadata>;
}

export type ProviderState =
  | 'idle' | 'connecting' | 'replaying' | 'live'
  | 'reconnecting' | 'failed' | 'closed';

export interface BarHandlers {
  onBar(bar: ChartBar, meta: BarMetadata): void;
  onVolume?(volume: HistogramData<UTCTimestamp>, meta: BarMetadata): void;
  onState?(state: ProviderState): void;
  onError?(error: DatabentoProviderError): void;
  onSymbolMapping?(mapping: SymbolMapping): void;
}

export interface Subscription {
  readonly id: string;
  readonly state: ProviderState;
  unsubscribe(): Promise<void>;
  dispose(): Promise<void>;
}

export interface OpenBarsResult {
  initial: BarPage;
  subscription: Subscription;
}

export interface DatabentoDataProvider {
  getBars(request: HistoryRequest): Promise<BarPage>;
  openBars(request: HistoryRequest, handlers: BarHandlers): Promise<OpenBarsResult>;
  subscribeBars(request: BarRequest, handlers: BarHandlers): Promise<Subscription>;
  resolveSymbol(request: ResolveSymbolRequest): Promise<ResolvedSymbol[]>;
  searchSymbols(request: SearchSymbolsRequest): Promise<SymbolSearchResult[]>;
  getDatasetMetadata(dataset: string, signal?: AbortSignal): Promise<DatasetMetadata>;
  dispose(): Promise<void>;
}

export function createDatabentoDataProvider(
  config: ProviderConfig,
): DatabentoDataProvider;
```

`openBars` is the correctness path for initial history plus live delivery. `getBars` followed by `subscribeBars` remains available for callers that intentionally manage their own boundary; that pair does not claim the coordinated handoff guarantee.

`Subscription.id` is generated by the TypeScript provider and remains stable across WebSocket reconnects; the protocol calls it `subscriptionId`. `unsubscribe` and `dispose` are aliases and are idempotent. They resolve after the gateway acknowledges downstream removal or after the socket is terminal. They never imply an upstream Databento unsubscribe.

All numeric configuration values must be finite and positive, except `jitterRatio`, which is in `[0,1]`, and `maxAttempts`, which is a positive integer. The provider does not invent reconnect or chunk limits. Authentication for a future non-loopback deployment is supplied by the host application's same-site session/cookie and gateway middleware; `ProviderConfig` never accepts a Databento key or shared browser secret.

## Canonical domain model

Rust uses integers until the browser boundary.

```rust
struct RequestedStreamKey {
    dataset: String,
    requested_symbol: String,
    stype_in: SymbolType,
    resolution: Resolution,
    gap_policy: GapPolicy,
}

struct ResolvedStreamKey {
    request: RequestedStreamKey,
    resolved_symbol: String,
    instrument_id: u32,
    source_schema: OhlcvSchema,
}

struct BaseBarKey {
    dataset: String,
    instrument_id: u32,
    schema: OhlcvSchema,
    start_ns: u64,
}

struct BaseBar {
    key: BaseBarKey,
    publisher_id: u16,
    open_nano: i64,
    high_nano: i64,
    low_nano: i64,
    close_nano: i64,
    volume: u64,
}

struct TargetBar {
    dataset: String,
    requested_symbol: String,
    resolved_symbol: String,
    instrument_id: u32,
    start_ns: u64,
    open_nano: i64,
    high_nano: i64,
    low_nano: i64,
    close_nano: i64,
    volume: u64,
    synthetic: bool,
}
```

The demo layers UI-only states over `ProviderState`: `validating` and `loading_history` map to provider `idle`; `history_ready`, `empty`, and `history_exhausted` are historical-view substates of `idle`; `recoverable_error` maps to an operation error while the provider remains usable; `terminal_error` maps to `failed`; and `disconnected` maps to `closed` only after an explicit disconnect. These names do not cross the provider protocol.

### Invariants

1. `start_ns` is neither `UNDEF_TIMESTAMP` nor greater than the supported JavaScript timestamp range after conversion.
2. Bar timestamps are inclusive starts. All requested ranges are `[from, to)`.
3. `open_nano`, `high_nano`, `low_nano`, and `close_nano` are defined; `low <= min(open, close) <= max(open, close) <= high`.
4. A `BaseBarKey` occurs at most once in an aggregator. A later duplicate replaces the component and forces target-bar recomputation.
5. All routing and normalized fan-out use `ResolvedStreamKey`; a target bucket contains one instrument only. Instrument changes never merge into one candle. `RequestedStreamKey` exists only to correlate/resolve a browser request and must not route records.
6. Output per downstream subscription is nondecreasing by target start. Equal starts are replacements; greater starts are appends; lesser starts are deduplicated or dropped with a reason metric.
7. Only the gateway converts price nanos to `f64`: `nano as f64 / 1_000_000_000.0`, followed by a finite-value check. The browser never receives raw `i64` prices or `u64` nanoseconds.

## Resolution plan

| Requested | Databento source | Target bucket |
| --- | --- | --- |
| `1s` | `ohlcv-1s` | native |
| `5s`, `15s`, `30s` | `ohlcv-1s` | UTC epoch multiple |
| `1m` | `ohlcv-1m` | native |
| `5m`, `15m`, `30m` | `ohlcv-1m` | UTC epoch multiple |
| `1h` | `ohlcv-1h` | native |
| `2h`, `4h` | `ohlcv-1h` | UTC epoch multiple |
| `1d` | `ohlcv-1d` | native UTC-calendar semantics |

The same source schema and aggregation function are used for historical, replay, and steady-state live data. A custom target bar updates whenever a new source bar for its bucket arrives. A native target bar generally arrives once its Databento source interval is published.

Version 1 does not label `1d` as an exchange-session candle. Exchange-session bars require instrument session metadata and a separate resolution type.

## HTTP API

All endpoints are under `/v1`, use JSON except health checks, accept a caller-provided `X-Request-Id` or generate one, and return it in `X-Request-Id` and the body.

### `POST /v1/history/bars`

Request:

```json
{
  "dataset": "GLBX.MDP3",
  "symbol": "ESM7",
  "stypeIn": "raw_symbol",
  "resolution": "5m",
  "from": 1788080400,
  "to": 1788098400,
  "gapPolicy": "preserve-gaps"
}
```

Response:

```json
{
  "v": 1,
  "requestId": "req-01",
  "bars": [
    {"time": 1788080400, "open": 6500.25, "high": 6501.5, "low": 6499.75, "close": 6501.0}
  ],
  "volumes": [{"time": 1788080400, "value": 1423}],
  "metadata": [{
    "time": 1788080400,
    "dataset": "GLBX.MDP3",
    "requestedSymbol": "ESM7",
    "resolvedSymbol": "ESM7",
    "instrumentId": 12345,
    "sourceSchema": "ohlcv-1m",
    "synthetic": false
  }]
}
```

The gateway rejects a range whose number of requested target-resolution intervals exceeds configured `history_max_intervals`. `historyChunkIntervals` uses the same unit: it is a positive integer count of the request's target resolution, never source intervals or elapsed seconds. The TypeScript provider splits larger requests into adjacent `[from,to)` windows no larger than that count, issues them sequentially, and merges by time. Sequential fetching provides deterministic cancellation and avoids an unbounded response.

HTTP `from` and `to` values must be finite integer seconds. The gateway computes `effectiveFrom = ceil(from / sourceInterval) * sourceInterval` and reads complete source bars starting in `[effectiveFrom,to)`. It does not synthesize a partial leading source bar. A non-aligned `to` remains an exclusive upper bound; any complete source bar whose start is before `to` is eligible, while live-edge handoff applies the stricter rule below.

### Symbology and metadata

- `POST /v1/symbols/resolve` accepts `{dataset, symbols, stypeIn, from, to}` using start-inclusive/end-exclusive Unix seconds and returns `{v, requestId, mappings: SymbolMapping[]}`. The gateway strips the TypeScript-only `signal` field before serialization.
- `POST /v1/symbols/search` accepts exactly `{dataset, query, stypeIn}` and returns `{v, requestId, results: SymbolSearchResult[]}` in ascending binary Unicode order by `symbol`, then `dataset`. Search describes identifiers; it is not interval-aware and never returns `effectiveFrom`/`effectiveTo`. The result count is capped by required gateway `symbol_search_max_results` configuration; no client-supplied limit can bypass it.
- `GET /v1/datasets/{dataset}` returns `{v, requestId, metadata: DatasetMetadata}`. `publishers` is sorted by numeric `publisherId`; `schemas` is sorted lexicographically; availability fields are Unix seconds and are omitted when the upstream metadata service does not provide them.
- `GET /health/live` proves the process loop is alive. `GET /health/ready` is successful only after configuration validation; Databento network availability is exposed as status and metrics, not made a permanent readiness dependency.

### HTTP errors

Non-2xx responses use:

```json
{
  "v": 1,
  "requestId": "req-01",
  "error": {
    "code": "invalid_range",
    "message": "from must be less than to",
    "retryable": false,
    "details": {}
  }
}
```

Stable codes are exactly the `ProviderErrorCode` union in the public TypeScript contract. Details contain no raw upstream body or credential.

## WebSocket protocol

The endpoint is `GET /v1/live`. The client requests the subprotocol `databento-lwc.v1`; the server rejects missing or unknown versions. Every text frame is one JSON object with `v: 1`, `type`, and a correlation identifier. Binary browser frames are rejected. Unknown fields are rejected in commands and may be ignored in events only after a minor compatible protocol decision is recorded.

### Client commands

Standalone live subscription:

```json
{
  "v": 1,
  "type": "subscribe_bars",
  "commandId": "cmd-01",
  "subscriptionId": "sub-01",
  "request": {
    "dataset": "GLBX.MDP3",
    "symbol": "ESM7",
    "stypeIn": "raw_symbol",
    "resolution": "5m",
    "gapPolicy": "preserve-gaps"
  }
}
```

Coordinated handoff adds history to the same command:

```json
{
  "v": 1,
  "type": "open_bars",
  "commandId": "cmd-02",
  "subscriptionId": "sub-02",
  "request": {
    "dataset": "GLBX.MDP3",
    "symbol": "ESM7",
    "stypeIn": "raw_symbol",
    "resolution": "5m",
    "from": 1788080400,
    "to": 1788098400,
    "gapPolicy": "preserve-gaps"
  }
}
```

Unsubscribe:

```json
{"v": 1, "type": "unsubscribe", "commandId": "cmd-03", "subscriptionId": "sub-01"}
```

Resume after an unexpected browser-to-gateway disconnect:

```json
{
  "v": 1,
  "type": "resume_bars",
  "commandId": "cmd-04",
  "subscriptionId": "sub-01",
  "resumeFrom": 1788091800,
  "request": {
    "dataset": "GLBX.MDP3",
    "symbol": "ESM7",
    "stypeIn": "raw_symbol",
    "resolution": "5m",
    "gapPolicy": "preserve-gaps"
  }
}
```

`resumeFrom` is the last target timestamp emitted by the provider. The gateway requests replay starting one source interval before that target bucket, deduplicates components, acknowledges with the same client-generated `subscriptionId`, emits equal-time replacement/newer bars in order, then returns to `live`. It sends no `snapshot` on resume. A duplicate `subscriptionId` on one live connection is `protocol_error`; reusing it on a new connection is expected.

Cancel a pending or active operation when its `AbortSignal` fires:

```json
{
  "v": 1,
  "type": "cancel",
  "commandId": "cmd-05",
  "targetCommandId": "cmd-02",
  "subscriptionId": "sub-02"
}
```

The server cancels history/handoff work, discards buffered data, removes the downstream reference, and sends `cancelled` with the cancel command ID, target command ID, and subscription ID. After `cancelled`, it sends no event for that subscription. An unknown target is `invalid_request`. If cancellation races with an already active stream, it still performs the same cleanup and `cancelled` terminal event; it does not also send `unsubscribed`.

### Server events

- `subscribed`: `{v,type,commandId,subscriptionId,state,resolvedSymbols}` where `state` is `replaying` for `open_bars`/`resume_bars`, `live` for a non-replay `subscribe_bars`, and `resolvedSymbols` is a JSON array of exact `SymbolMapping` objects.
- `snapshot`: `{v,type,subscriptionId,bars,volumes,metadata}`. Sent exactly once for `open_bars` before any `bar` event.
- `bar`: `{v,type,subscriptionId,data,volume,meta}`. `volume` is absent for whitespace or synthetic carry-forward points.
- `status`: `{v,type,subscriptionId,state,retryable,reason?}` where `state` is `ProviderState` and `reason`, when present, is `StatusReason`; free-form reasons are forbidden.
- `symbol_mapping`: `{v,type,subscriptionId,requestedSymbol,resolvedSymbol,instrumentId,effectiveFrom}`.
- `unsubscribed`: `{v,type,commandId,subscriptionId}`.
- `cancelled`: `{v,type,commandId,targetCommandId,subscriptionId}`.
- `error`: `{v,type,commandId?,subscriptionId?,error}` using the HTTP error object.
- `heartbeat`: `{v,type,serverTime}`. It proves downstream liveness only and is not represented as market data.

The server does not send a normal `bar` before `subscribed`; `open_bars` sends `subscribed`, then `snapshot`, then zero or more buffered `bar` events. `resume_bars` sends `subscribed`, replay/status events, then zero or more ordered `bar` events without a snapshot. `cancelled`, `unsubscribed`, and a terminal `error` end that downstream subscription and forbid later events. A command-level error prevents creation of a subscription.

Application close codes are fixed for contract tests: `4000 protocol_error`, `4001 quota_exceeded`, `4002 slow_consumer`, `4003 upstream_unavailable`, `4004 server_shutdown`, and `4005 heartbeat_timeout`. Origin or HTTP authentication failures reject the upgrade before a WebSocket exists. A normal client/server close uses WebSocket code `1000`.

## Historical processing

1. Validate request and allowlists before any Databento call.
2. Select the source schema from the resolution table.
3. Resolve the requested symbols over `[from,to)` using the official Historical symbology API. Parent resolution returns all children, but bar methods fail with `unsupported_parent_series` before upstream bar access until the caller supplies one raw child or instrument ID.
4. Stream Databento records for `[from,to)` through normalization. Databento ranges are start-inclusive and end-exclusive.
5. Route by effective mapping and `instrument_id`, replace duplicate `BaseBarKey` entries, and aggregate.
6. Apply gap policy after real target bars are finalized.
7. Sort ascending, verify strict unique timestamps per instrument, convert time/prices at the browser boundary, and return.

An aborted browser HTTP request cancels the gateway work where the official client permits cancellation; the gateway discards any result after disconnect and records `cancelled` rather than caching partial data.

## Coordinated handoff algorithm

`open_bars` is implemented entirely within one gateway operation:

1. Validate and resolve the request. Let `S` be the source interval in seconds and `liveEdge = floor(now / S) * S`. Require `floor(min(request.to, now) / S) * S == liveEdge`; otherwise return `invalid_range` because `openBars` is for a live-edge handoff, while older ranges belong to `getBars`. Set `T = liveEdge` and `effectiveFrom = ceil(request.from / S) * S`; require `effectiveFrom < T`.
2. Obtain the dataset session and register a live subscription with replay start `T - one source interval`. Databento supports subscription-specific starts within the documented 24-hour replay window.
3. Buffer normalized live/replay base bars for this downstream request. The buffer is bounded; overflow fails the operation with `quota_exceeded` and sends no snapshot.
4. Query historical source bars for `[effectiveFrom, T)` through the same normalizer. The snapshot includes every observed source bar whose start is less than `T`. For a custom target resolution, this may produce a partial current target bucket; replay/live components with the same target start replace it. The source bucket starting at `T` is never in the snapshot and first appears through replay/live.
5. Wait for the replay-completed system message for the relevant replay phase. Merge history and buffered replay by `BaseBarKey`, with the later received duplicate replacing the earlier component.
6. Aggregate the merged components. Emit `subscribed`, then one `snapshot` sorted by target time.
7. Drain buffered components received after the merge watermark. Recompute an equal current target bucket and emit it as a replacement; emit a greater bucket as an append. Never emit a lesser target bucket.
8. Enter `live`. Keep only the component state required to recompute the current target bucket and deduplicate the configured replay overlap.

If the requested boundary lies outside live replay eligibility, history still loads but the operation fails before claiming continuous handoff. It returns a retryable `replay_unavailable` error rather than silently opening an unverified stream.

## Upstream session and unsubscribe policy

- One `DatasetSession` owns one official `Live` client and therefore one dataset-scoped Raw API session.
- Compatible symbol/schema subscriptions are combined in that session. The configured dataset-session limit is checked before connecting.
- A `RequestedStreamKey` has a downstream reference count for upstream subscription ownership. After symbology resolution, every record is routed and shared only by `ResolvedStreamKey`; later downstream references may share that normalized stream.
- Removing a downstream reference stops its forwarding immediately. Because Databento has no unsubscribe method, the upstream subscription remains while any downstream request needs that dataset session.
- When the final downstream reference for a dataset reaches zero, the gateway gracefully stops the Databento client and removes the session. No idle timer is invented.
- If a symbol becomes unused while other symbols remain, its upstream records may continue arriving and are discarded before normalization fan-out, with a counter. The session is not disrupted merely to reclaim one subscription.

## Reconnect and state transitions

```text
idle -> connecting -> replaying -> live
                  \-> failed
live -> reconnecting -> replaying -> live
                     \-> failed
any non-closed state -> closed
```

Only an unexpected, retryable network or upstream service failure enters `reconnecting`. Invalid requests, authorization/access errors, unsupported schemas, and connection-limit errors fail without an automatic tight loop. Gateway upstream backoff base, cap, jitter range, and retry budget are required operator configuration. On upstream reconnect, the dataset session recreates the active request set and requests replay from one source interval before the minimum last accepted base-bar start, limited to Databento's 24-hour window. If the retry budget or replay eligibility is exhausted, affected subscriptions enter `failed` with the precise code.

The TypeScript provider separately owns downstream WebSocket recovery. It retains each active canonical request, stable client subscription ID, and last emitted target time. On an unexpected socket close after an initial snapshot/acknowledgement, it enters `reconnecting`, applies `ProviderConfig.reconnect`, opens a new socket, and issues `resume_bars` for each active subscription. Equal-time replay replaces the current bar; older events are rejected; greater times append. If a socket closes before `openBars` has received its initial snapshot, that attempt is retried as `open_bars` and its promise remains unsettled until success or budget exhaustion. Explicit `unsubscribe`, provider `dispose`, protocol/authentication errors, and normal close do not reconnect. Budget exhaustion moves all affected subscriptions to `failed` and rejects pending operations.

## Ordering and duplicate policy

- The normalizer rejects unmapped instruments, undefined values, invalid OHLC order, and timestamp overflow.
- Historical and replay phases retain a map keyed by `BaseBarKey`; a duplicate replaces its component and increments `deduplicated_records`.
- The aggregator orders components by `start_ns`, so duplicate arrival order cannot change open or close.
- In steady state, a component for the current target bucket replaces/recomputes it. A component for a future bucket finalizes the prior bucket and advances. A component older than retained state is dropped with `out_of_order_too_old`; it is not sent using Lightweight Charts' historical update mode.
- Each downstream stream is instrument-scoped. A symbol mapping change closes the current instrument bucket before data for the new instrument can begin.

## Gap policy

Gap generation occurs across explicit requested/history ranges and across observed live bucket transitions. It does not infer exchange calendars.

- `preserve-gaps`: no point.
- `whitespace`: `{time}` and metadata with `synthetic: true`; no volume.
- `carry-forward`: OHLC all equal the last real close, metadata `synthetic: true`; no volume. No carry-forward is emitted before the first real close.

Because closed-market intervals are not known without a calendar, whitespace and carry-forward may include such UTC intervals and are opt-in.

## Backpressure

Every configured bound is validated as positive at startup. There are separate bounded queues for raw upstream records, canonical updates per dataset, and outbound frames per browser.

For an outbound queue, a pending same-subscription/same-time `bar` may be replaced by the newer version. A new interval, snapshot, error, status, mapping, or acknowledgement is never silently discarded. If it cannot be queued after coalescing, the gateway sends or records `slow_consumer` and closes that browser connection. An upstream queue overflow makes the dataset session's continuity unknown, so the session fails and enters the replay-based reconnect path.

## Normative gateway configuration

Shared deployments may map these fields from a configuration file, but environment names and units are stable. Every required numeric bound is an integer greater than zero; `RECONNECT_MAX_DELAY_MS >= RECONNECT_BASE_DELAY_MS`; enumerated lists must contain at least one value; unknown keys are startup errors in a config file.

| Environment name | Type/unit | Startup contract |
| --- | --- | --- |
| `DATABENTO_API_KEY` | secret string | Required for real Databento clients; never logged or returned |
| `DATABENTO_LWC_BIND_ADDR` | socket address | Defaults to loopback; non-loopback requires the authentication-integration flag |
| `DATABENTO_LWC_ALLOWED_ORIGINS` | comma-separated origins | Required and non-empty; exact origin matching |
| `DATABENTO_LWC_ALLOWED_DATASETS` | comma-separated dataset IDs | Required and non-empty |
| `DATABENTO_LWC_ALLOW_UNAUTHENTICATED_LOCALHOST` | boolean | Valid only for a loopback bind |
| `DATABENTO_LWC_AUTH_INTEGRATION_ENABLED` | boolean | Must be true for any non-loopback bind; asserts an embedding application supplied auth/entitlement middleware |
| `DATABENTO_LWC_HISTORY_MAX_INTERVALS` | target intervals/request | HTTP history bound |
| `DATABENTO_LWC_SYMBOL_MAX_BYTES` | UTF-8 bytes | Input symbol bound |
| `DATABENTO_LWC_SYMBOL_SEARCH_MAX_RESULTS` | results/request | Server-side search result cap |
| `DATABENTO_LWC_HTTP_BODY_MAX_BYTES` | bytes | HTTP request body bound |
| `DATABENTO_LWC_WS_FRAME_MAX_BYTES` | bytes | WebSocket frame bound |
| `DATABENTO_LWC_MAX_CLIENTS` | connections | Concurrent downstream client bound |
| `DATABENTO_LWC_MAX_SUBSCRIPTIONS_PER_CLIENT` | subscriptions/client | Per-connection bound |
| `DATABENTO_LWC_MAX_DATASET_SESSIONS` | sessions | Concurrent upstream dataset-session bound |
| `DATABENTO_LWC_UPSTREAM_QUEUE_CAPACITY` | records | Raw upstream queue capacity |
| `DATABENTO_LWC_CANONICAL_QUEUE_CAPACITY` | updates | Per-dataset normalized queue capacity |
| `DATABENTO_LWC_OUTBOUND_QUEUE_CAPACITY` | frames/client | Browser outbound queue capacity |
| `DATABENTO_LWC_HANDOFF_BUFFER_CAPACITY` | source bars/subscription | Replay/history merge buffer capacity |
| `DATABENTO_LWC_RECONNECT_BASE_DELAY_MS` | milliseconds | Initial retry delay |
| `DATABENTO_LWC_RECONNECT_MAX_DELAY_MS` | milliseconds | Retry-delay ceiling |
| `DATABENTO_LWC_RECONNECT_MAX_ATTEMPTS` | attempts | Retry budget before typed failure |

The checked-in example configuration supplies explicit local development values after `TASK-00` benchmarking; production values remain operator-owned. Tests set deliberately small values to prove every startup and capacity boundary.

## Security model

- `DATABENTO_API_KEY` is read by the Rust process from the environment and passed only to official Databento clients.
- Default bind is loopback. `allow_unauthenticated_localhost` is valid only with a loopback bind and an explicit origin allowlist.
- A non-loopback deployment must supply application authentication middleware and entitlement checks outside this adapter. Startup fails without that integration flag; the project does not ship a shared browser bearer secret.
- Origin checks supplement authentication and are enforced on HTTP and WebSocket upgrades.
- Dataset allowlists and version 1 resolution/symbol-type enums prevent an unrestricted proxy.
- Request body size, WebSocket frame size, history intervals, clients, subscriptions per client, dataset sessions, queue capacities, and retry budget are required configuration. The repository provides documented example values only after they are measured in the benchmark environment.
- Errors are stable and sanitized. Logs redact values whose key name contains `key`, `secret`, `token`, or `authorization`.

## Observability

Use Rust `tracing` with JSON output and a metrics facade whose exporter is selected at deployment. Each request span includes request ID, route, dataset, resolution, and outcome. Each live span includes downstream connection ID, subscription ID, dataset-session ID, state transition, and reason. High-cardinality raw symbols are excluded from metric labels.

Required counters/gauges/histograms are named in the requirements. Tests use an in-memory recorder and log capture to assert transitions and redaction; observability is not verified by visual inspection alone.

## Dependencies and versions researched on 2026-08-30

| Dependency | Verified version | Role and boundary |
| --- | ---: | --- |
| `lightweight-charts` | 5.2.1 | Peer dependency and chart data types; the package does not wrap chart UI. |
| TypeScript | 7.0.2 | Strict browser package and demo compilation. |
| Vite | 8.2.2 | Demo-only build and development server. |
| Vitest | 4.1.11 | TypeScript unit and contract tests. |
| `ws` | 8.21.3 | Node-only test WebSocket client; browsers use native `WebSocket`. |
| `zod` | 4.5.4 | Runtime validation of untrusted gateway JSON in the TypeScript package. |
| Rust `databento` | 0.60.0 | Official Historical and Live client. |
| `axum` | 0.8.9 | HTTP and WebSocket gateway. |
| `tokio` | 1.53.1 | Async runtime, tasks, channels, cancellation, and timers. |
| `serde` | 1.0.229 | Browser protocol JSON serialization. |
| `tracing` | 0.1.44 | Structured spans and events. |
| `tower-http` | 0.7.0 | Request tracing, body limits, and origin/CORS middleware. |

Implementation must run compatibility spikes before locking versions because registry versions can change. Lockfiles, exact resolved dependency graphs, feature flags, and minimum supported toolchains are committed. New dependencies require an entry in the decision log.

## Alternatives considered

- **Browser-to-Databento direct:** rejected because credentials would be exposed and Databento Live uses raw TCP, unavailable to browsers.
- **SSE plus HTTP controls:** workable for server-to-client events, but two channels complicate correlated subscribe/unsubscribe, handoff, and symbol switching. One WebSocket is selected for interactive multiplexing.
- **TypeScript gateway:** rejected for this empty repository because the brief prefers the official Rust client and a long-running Rust stream service. No existing backend makes adaptation cheaper.
- **One upstream connection per browser subscription:** rejected because live sessions are dataset-scoped, multiple subscriptions can share one session, and connection limits are entitlement-dependent.
- **Calling `setData` for live events:** rejected because Lightweight Charts documents that it replaces all series data and recommends `update` for current/new points.
- **Trade-derived bars in version 1:** deferred. Official OHLCV produces the required native and composable intervals with a smaller scope. Trade conditions and corrections would introduce a separate aggregation contract.

## Authoritative constraints

- Databento live sessions, dataset scope, replay, and subscriptions: https://databento.com/docs/api-reference-live
- Databento live subscription has no unsubscribe method: https://databento.com/docs/api-reference-live/client/subscribe
- Databento Rust live sessions close gracefully with `client.close().await`; final-reference shutdown must exercise that path rather than only dropping downstream state.
- Databento live replay is available within the last 24 hours: https://databento.com/docs/api-reference-live/basics/intraday-replay
- Databento OHLCV fields and no-trade interval behavior: https://databento.com/docs/schemas-and-data-formats/ohlcv
- Databento timestamps, fixed prices, UTC, sentinels, and `[start,end)` ranges: https://databento.com/docs/standards-and-conventions/common-fields-enums-types
- Databento live symbology mapping behavior: https://databento.com/docs/api-reference-live/basics/symbology
- Lightweight Charts 5.2 `setData` and `update` guidance: https://tradingview.github.io/lightweight-charts/docs
- Lightweight Charts realtime example: https://tradingview.github.io/lightweight-charts/tutorials/demos/realtime-updates
- Lightweight Charts infinite-history example: https://tradingview.github.io/lightweight-charts/tutorials/demos/infinite-history
- Local Lightweight Charts source pin and review coverage: `docs/review-route.md` and `docs/lightweight-charts-core-knowledge.md`.
