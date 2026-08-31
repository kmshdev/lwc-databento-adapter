# Test strategy

## Purpose

Tests prove data correctness, protocol compatibility, lifecycle safety, and the user-visible demo without requiring paid Databento access for the mandatory suite. Network tests with authorized credentials are optional and never substitute for deterministic offline coverage.

## Test layers

### 1. Rust unit and property tests

Run with `cargo test --workspace --all-features`.

- Timestamp conversion: epoch, second/minute/hour/day boundaries, maximum accepted JavaScript second, `UNDEF_TIMESTAMP`, sub-second truncation, and overflow.
- Price normalization: zero, negative prices where schema permits, maximum/minimum defined `i64`, undefined price sentinel, finite result, and OHLC ordering.
- Resolution parsing and source-schema selection for every supported and representative unsupported string.
- UTC bucket start at exact boundaries and one nanosecond/second on either side.
- Aggregation: first open, maximum high, minimum low, last close, checked volume sum, duplicate component replacement, and deterministic result under duplicate arrival orders.
- Gap policies: no point, whitespace, carry-forward after a real close, no carry before a real close, and no synthetic volume.
- Symbol routing: date-ranged historical mappings, live mapping before data, unresolved instrument drop, instrument change, and same text symbol in different datasets.
- Requested keys never route records; resolved symbol plus instrument ID and source schema are required for fan-out.
- State machines: every permitted transition and rejection of invalid transitions.

Property tests generate valid OHLCV component sequences and assert:

- `low <= open/close <= high` remains true after aggregation;
- output target times are bucket-aligned and nondecreasing;
- adding a duplicate component with identical data is idempotent;
- replacing one component changes volume by `new - old`, never double-counts it;
- partitioning the same ordered base sequence into arbitrary chunks produces the same final bars.

### 2. TypeScript unit tests

Run with `pnpm --filter databento-lightweight-charts test`.

- Zod accepts every valid v1 fixture and rejects unknown versions, missing IDs, unsafe numbers, malformed bars, oversized inputs supplied to client-side guards, and unknown event types.
- HTTP errors map to stable `DatabentoProviderError` values and preserve retryability without exposing raw response bodies.
- Symbol resolution serializes `from`/`to`; symbol search decodes `SymbolSearchResult` without interval fields and verifies deterministic sort/cap behavior; dataset metadata decodes the exact sorted publisher/schema shape.
- `getBars` splits adjacent windows, respects `[from,to)`, cancels via `AbortSignal`, merges in order, and deduplicates boundary bars.
- A bar with equal time invokes a current-bar update; greater time appends; lesser time is rejected as protocol corruption.
- Subscription IDs route only to their own callbacks across multiple charts and symbols.
- `unsubscribe`/`dispose` is idempotent and resolves on acknowledgement or terminal socket closure.
- `dispose` closes all subscriptions and prevents later callbacks.
- Aborting a pending `openBars` sends `cancel`, rejects with the typed cancelled error, releases buffered/upstream references, and permits no later callback for that subscription.
- Unexpected WebSocket close retains the stable subscription ID, applies the configured deterministic retry sequence, sends `resume_bars` from the last emitted target time, and deduplicates equal-time replay.
- A close before the initial `openBars` snapshot retries `open_bars`; explicit dispose, protocol errors, and exhausted retry budgets do not loop.
- Browser-local timezone changes do not change emitted UTC timestamps.

### 3. Shared protocol contract tests

`contracts/fixtures` is consumed by both Rust and TypeScript tests. Each fixture declares `valid: true|false`, a message direction, and the expected decoded variant or error code.

Required fixtures cover every command/event/error type, including `cancel`/`cancelled`, minimum and maximum safe values, unknown protocol version, unknown type, missing correlation ID, duplicate subscription ID, metadata/bar time mismatch, whitespace with volume, synthetic bar with volume, and a credential-shaped field that must be rejected. The Rust serializer's output must parse in TypeScript; TypeScript fixture commands must deserialize in Rust.

The protocol task is complete only when changing a required field breaks at least one Rust and one TypeScript contract test.

### 4. Gateway component tests with fakes

The gateway defines `HistoricalSource` and `LiveSource` traits around the official client. Tests use scripted fakes at those network boundaries, not mocks of normalization or aggregation logic.

Scenarios:

- Historical empty, invalid symbol, access denied, partial upstream failure, cancellation, multiple mapping intervals, duplicate DBN record, and unsupported schema.
- Dataset session sharing, one session per dataset, downstream reference counts, immediate stop-forwarding, and graceful stop when the final dataset reference disappears.
- No upstream unsubscribe call exists in the fake interface.
- Reconnect after disconnect, replay overlap, replay-completed system message, duplicate overlap, backoff budget exhaustion, non-retryable access error, and replay outside the 24-hour window.
- Bounded queues, same-bucket coalescing, protected interval transition, `slow_consumer` close, and upstream overflow entering reconnect.
- Origin, allowlist, range, frame-size, subscription, client, and dataset-session limits with explicit small values.
- Log and error redaction using a sentinel fake API key.

### 5. Handoff scenario suite

These deterministic scenarios run through the real gateway handoff coordinator and a fake historical/live pair:

| Scenario | Historical input | Live/replay input | Required output |
| --- | --- | --- | --- |
| Boundary duplicate | Includes base bar at `T-source` | Replays same bar | One component and one target contribution |
| Current target continuation | History has first components | Replay/live has later components in same target bucket | Snapshot then equal-time replacement |
| Exact next interval | History ends before `T` | Live starts at next target bucket | Strict append, no gap |
| Replay arrives before history | Live buffer fills first | History returns later | Same output as history-first order |
| Historical availability lags live edge | Symbology/history end at account `available_to` | Replay begins one source interval before that boundary | Historical calls remain valid and live continuity reaches the current edge |
| Replay exceeds downstream queue while history loads | Historical response is delayed | Replay contains more records than queue capacity | Replay is drained concurrently and `ReplayCompleted` is retained |
| Out-of-order duplicate | Replay duplicate arrives late | Steady state already accepted it | No backward browser event |
| Mapping change | Old instrument then mapping then new instrument | Both IDs appear | No candle mixes instruments |
| Buffer overflow | History is delayed | More live base bars than configured capacity | Typed failure and no snapshot |
| Replay unavailable | Boundary exceeds eligibility | Live refuses requested replay | `replay_unavailable`, never false `live` |
| Non-aligned live-edge `to` | `to` lies inside current source bucket | Replay starts at floored live edge | Snapshot excludes source start `T`; later equal target replaces partial custom bar |
| Stale `to` | `to` floors before current source bucket | No live work starts | `invalid_range` before resolution or subscription; caller must use `getBars` |

For every scenario, assert the exact ordered sequence `subscribed -> snapshot -> bar/status`, bar contents, volume, metadata, and final state.

### 6. HTTP/WebSocket integration tests

Start the real Axum router on an ephemeral port with fake sources. Use real HTTP and WebSocket clients.

- Verify status, headers, request IDs, JSON bodies, subprotocol negotiation, frame validation, acknowledgement ordering, and close codes.
- Open two browser connections with overlapping and distinct requests; assert isolation and upstream sharing.
- Disconnect a client without unsubscribe; assert downstream cleanup and correct remaining references.
- Restart the fake upstream while downstream connections remain; assert `reconnecting -> replaying -> live` and continuity.
- Restart the gateway/WebSocket after at least one live target bar; assert the TypeScript client issues `resume_bars` with the stable subscription ID and reconstructs the uninterrupted result.
- Assert a Databento credential sentinel is absent from every HTTP body, WebSocket frame, captured log, and built browser asset.

### 7. Offline browser end-to-end test

Run the demo against the fake-source gateway in a real browser. The test must:

1. select a dataset, symbol, type, range, and `5m` resolution;
2. load and display the known initial candles and volume;
3. observe `replaying` then `live`;
4. receive a same-time candle mutation;
5. receive a new target interval;
6. force an upstream disconnect and observe successful reconnect without duplicate points;
7. switch symbols and prove the old symbol no longer updates the chart;
8. edit the form without submitting, then pan left and prove older history retains the request that produced the displayed chart;
9. assert the history request is single-flight and the visible logical range is preserved after prepend;
10. resize, remove, and restore the separate volume pane using public APIs;
11. exercise keyboard focus, named controls, state announcements, legend fallback, and tooltip boundary hiding/repositioning;
12. run native `ResizeObserver` autosize and manual fallback scenarios without an observer-loop error;
13. disconnect during active and delayed work, prove callbacks and late results stop, and prove `chart.remove()` cleans the canvas/subscriptions;
14. inspect browser requests, assets, and dependency graph for absence of `DATABENTO_API_KEY`, the sentinel credential, `lightweight-chart-react`, and private chart-model imports.

The test asserts chart series data and logical ranges through application-observable state exposed only in test builds, not screenshots or timing guesses. A separate visual suite may use golden images for primitives at device-pixel ratios 1, 1.25, 1.5, 2, and 3; those images supplement rather than replace behavioral assertions.

### 8. Optional real Databento qualification

`mise run test:live-databento` is an explicit zero-argument provider qualification. It reads `DATABENTO_API_KEY` from the ignored environment, inventories account-aware dataset ranges, resolves the live-edge `ES.c.0` continuous symbol, and opens one `GLBX.MDP3` `ES.FUT` `ohlcv-1m` session with a fifteen-minute replay bound. It must observe `SubscriptionAck`, `ReplayCompleted`, normalize every returned OHLCV record, and close gracefully. The key, authorization, raw error bodies, and unrestricted payloads are never printed.

The evidence labels are deliberately narrow:

- `list_datasets` means valid dataset codes on Databento, not account subscriptions.
- `list_publishers` maps publisher IDs to datasets and venues, but is not entitlement-specific.
- a successful `get_dataset_range` proves the historical range made available under the user's entitlements; it does not prove a live-data license.
- there is no bulk live-entitlement endpoint. The successful dataset-specific session proves live access for `GLBX.MDP3` only; it does not infer live licenses for the other catalog datasets.

Primary contracts: [list datasets](https://databento.com/docs/api-reference-historical/metadata/metadata-list-datasets), [get dataset range](https://databento.com/docs/api-reference-historical/metadata/metadata-get-dataset-range), [list publishers](https://databento.com/docs/api-reference-historical/metadata/metadata-list-publishers), [Live sessions](https://databento.com/docs/api-reference-live), and [portal live-data licensing](https://databento.com/docs/portal).

`mise run test:live-integration` adds the real gateway, provider WebSocket protocol, continuous mapping, history/replay merge, Lightweight Charts consumer, and disconnect. `mise run test:package-consumer` verifies the archive contains its manifest, package README, JavaScript, and declarations; checks public publication and peer metadata; installs the tarball in an isolated project; and compiles history, handoff, streaming, symbology, metadata, teardown, candlestick, and volume usage. A closed-market run may legitimately return zero bars, but acknowledgement, replay completion, mapping, live state, and close remain mandatory; zero bars are never presented as evidence of bar-update behavior.

## Golden and generated fixtures

- Prefer small hand-authored `BaseBar` records for exhaustive edge cases.
- Store sanitized DBN-derived bytes only after confirming the applicable Databento license permits repository storage.
- Every golden fixture has provenance metadata, schema, dataset, time range, sanitization note, and expected normalized JSON; it contains no credential or personal data.
- Golden transformations are reviewed intentionally. Tests never rewrite expected fixtures automatically during a normal run.

## Performance and backpressure benchmark

The benchmark runner accepts an explicit scenario file containing dataset count, canonical subscriptions, downstream fan-out, source record rate, queue capacities, and duration. The repository does not invent production thresholds.

For a user-approved scenario, record:

- machine and toolchain versions;
- input records and target bars;
- p50/p95/p99 processing and end-to-end latency;
- maximum queue utilization;
- coalesced same-bucket updates;
- dropped records by reason;
- memory high-water mark and CPU utilization.

Mandatory acceptance is: the scenario is reproducible, there is no unexplained record loss or interval transition loss, configured queues and memory remain bounded, and measured values are reported without an unmeasured throughput claim. When the user supplies latency or resource thresholds, meeting them is an additional release criterion. Without supplied thresholds, the mandatory invariants still satisfy `REQ-Q-004`; latency/throughput results remain characterization rather than a claimed service level.

The current browser threshold is enforced by `mise run test:performance`. The route inventory is every HTML entry and configured router path; version 1 has only `/`. The task builds the demo, starts an isolated Vite production preview on loopback, launches the pinned headless Chromium, disables the browser cache, performs three warmups, and records 20 `PerformanceNavigationTiming.duration` samples at 1280 by 900 pixels. Each route must have p95 below 50 ms. Browser startup and build time are excluded. The runner prints the route, conditions, p95, and maximum so repeated runs remain comparable.

### Dedicated-connectivity acceptance

`REQ-Q-006` is an external physical qualification, not an offline test. Accept only a circuit-specific receipt containing the site, port speed, Live Raw TCP service, exact Databento boundary-switch-to-cross-connect measurement boundary, 90th percentile, sub-50-microsecond result, date, and measurement provenance. The published 42.4-microsecond estimate proves product capability, not this deployment. Public-network live tests, ICMP, browser timing, and gateway timers are explicitly non-qualifying.

## Mutation and negative verification

Before closing the normalization, aggregation, handoff, protocol, and security tasks, deliberately introduce one representative fault per area and confirm a focused test fails: wrong bucket boundary, duplicate volume addition, snapshot after bar, cross-subscription routing, and credential inclusion. Restore the code and rerun the focused suite. Use mutation tooling where it supports the code cleanly; otherwise retain the explicit fault-injection record in the implementation plan.

## Mandatory release gates

The exact scripts are established in `TASK-01`; their responsibilities are fixed:

```text
pnpm format:check
pnpm lint
pnpm typecheck
pnpm test
pnpm build
pnpm test:performance
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo deny check
pnpm test:e2e:offline
```

All mandatory gates pass with zero warnings. The real Databento suite is reported separately.

## Exit evidence

Release evidence consists of command, UTC time, exit code, concise test totals, toolchain and dependency-lock revisions, offline end-to-end scenario name, credential-leak scan result, benchmark scenario/result when thresholds were supplied, and real-Databento status (`passed`, `not run`, or `inconclusive` with reason).
