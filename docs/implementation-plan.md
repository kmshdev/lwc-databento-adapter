# Implement the Databento adapter for Lightweight Charts

This is a living execution plan. Keep `Progress`, `Surprises and discoveries`, `Decision log`, and `Outcomes and retrospective` current during implementation. A new engineer should be able to implement the system from this plan and the checked-in requirements, design, test strategy, and traceability documents without prior conversation context.

## Purpose and visible result

Build a TypeScript package and Rust gateway that let a browser load Databento candles and volume into Lightweight Charts, continue with live updates, reconnect safely, switch symbols, and load older history without exposing the Databento key. The completed demo is the visible proof: its chart must pass the offline end-to-end scenario in the test strategy and, when authorized credentials are supplied, may also run a small real-Databento qualification.

## Progress

- [x] (2026-08-30) Prepared requirements, technical design, task acceptance criteria, test strategy, traceability, and decision log.
- [x] (2026-08-30) Recorded `DEC-014`, `DEC-015`, and `DEC-016` before implementation began.
- [ ] Complete `TASK-00` through `TASK-11` in order unless a task explicitly permits parallel work.
- [ ] Record mandatory release-gate and optional live-provider evidence.

## Current repository state

The repository contains the planning documents, folder blueprint, guard scripts, pnpm TypeScript workspace, Cargo Rust workspace, shared JSON protocol fixtures, browser package, Rust gateway, and runnable Vite demo. The implemented layout keeps shared fixtures in `contracts/`, the browser package in `packages/databento-lightweight-charts`, the gateway in `services/databento-gateway`, and the demo in `examples/lightweight-charts-demo`. The deterministic offline gate, local-beta browser workflow, bounded real-provider qualifications, isolated package-consumer compile, and production-preview page-load budget are current acceptance evidence; an online deployment target is not yet identified.

The normative behavior is in:

- `docs/requirements.md`: version 1 scope and requirement IDs.
- `docs/technical-design.md`: component ownership, APIs, message schema, algorithms, and dependency boundaries.
- `docs/test-strategy.md`: test layers and gates.
- `docs/traceability.md`: requirement-to-proof mapping.
- `docs/decision-log.md`: evidence, assumptions, and decisions.
- `docs/review-route.md`: source pins, pass dependencies, and evidence limitations.
- `docs/lightweight-charts-core-knowledge.md`: chart API and primitive constraints.
- `docs/lightweight-charts-tutorial-knowledge.md`: planned user surface and browser matrix.
- `docs/mise-plan.md`: configuration, secret, trust, and task ownership.

## Definitions

- **Base bar:** one Databento native OHLCV record at `1s`, `1m`, `1h`, or `1d`.
- **Target bar:** the native or custom-resolution candle emitted to a chart.
- **Canonical request:** dataset, requested symbol, Databento symbol type, resolution, and gap policy after validation.
- **Handoff:** coordinated merge of historical and replay/live base bars at boundary `T`.
- **Downstream:** browser-facing HTTP/WebSocket side.
- **Upstream:** Databento Historical or Live side.
- **Offline gate:** a test that uses scripted fake Databento boundaries and requires no network credential.

## Work sequence

### TASK-00 — Resolve external compatibility and lock the skeleton

Create minimal throwaway compatibility tests or examples that compile against the selected `databento` Rust crate, Axum WebSocket API, and Lightweight Charts types. Verify the official client operations needed for historical range streaming, live dataset sessions, pre-start replay subscriptions, replay-completed system records, live symbol mapping records, graceful stop, and error classification. Record exact versions, features, and source links in `docs/decision-log.md`; then create root Cargo/pnpm workspace manifests and lockfiles.

Acceptance criteria:

- A Rust spike compiles and proves each required official-client type/method exists; it does not need credentials to connect.
- A TypeScript spike compiles `CandlestickData<UTCTimestamp>`, `WhitespaceData<UTCTimestamp>`, `HistogramData<UTCTimestamp>`, and `ISeriesApi.update` against the locked Lightweight Charts version.
- The decision log records any difference from the prepared design. A material contradiction stops implementation for design repair.
- `Cargo.lock` and `pnpm-lock.yaml` are committed; no unused dependency remains from the spikes.

### TASK-01 — Establish project guardrails

Add strict TypeScript/Rust configurations, workspace scripts, formatting, linting, dependency auditing, and CI. Use pnpm, `oxfmt`, `oxlint`, `tsc --noEmit`, Vitest, Cargo fmt, Clippy with warnings denied, and `cargo deny`. Install and configure `prek` for the repository; the CI commands must also run without relying on local hooks.

Acceptance criteria:

- Every mandatory release-gate command in the test strategy exists and succeeds on the skeleton with zero warnings.
- CI runs the same formatting, lint, type, test, build, audit, and offline end-to-end entry points.
- A deliberate TypeScript type error, Rust Clippy warning, and forbidden dependency each fail the appropriate gate before being reverted.
- `.env.example` contains names and explanations only; no usable secret is committed.
- Configuration decoding implements every name, type, and unit in the technical design's normative gateway table; unknown file keys and each invalid relation fail startup in focused tests.

### TASK-02 — Freeze protocol v1 and shared fixtures

Create `contracts/protocol-v1.md` by extracting the normative HTTP and WebSocket shapes from the technical design. Create valid and invalid JSON fixtures for every message and error. Implement Rust `serde` domain/transport types and TypeScript Zod schemas/types against those fixtures.

Acceptance criteria:

- Both languages accept every valid fixture and reject every invalid fixture with the expected stable error.
- Rust-serialized events parse in TypeScript and TypeScript fixture commands deserialize in Rust.
- Protocol subversion, command/event ordering, required fields, strict command unknown-field handling, safe-number limits, and credential-shaped field rejection are tested.
- Removing or renaming a required field makes at least one Rust and one TypeScript contract test fail.

### TASK-03 — Implement normalization and aggregation

In the gateway library, implement timestamp and price validation, source-schema selection, UTC resolution bucketing, duplicate replacement, OHLCV aggregation, and the three gap policies. Keep raw `i64/u64` values through the domain layer and convert only at the transport boundary.

Acceptance criteria:

- All unit/property cases in test-strategy sections 1 and 5 that do not need network/session state pass.
- Every supported resolution maps to the exact source schema in the design; all other strings are rejected.
- Duplicate components are idempotent and replacement never double-counts volume.
- A deliberate bucket off-by-one and duplicate-volume fault each cause focused tests to fail before being reverted.
- No TypeScript file contains Databento fixed-price or nanosecond conversion logic.

### TASK-04 — Implement historical data and symbology

Define `HistoricalSource` around the official client. Implement range validation, Historical calls, interval-aware mapping, normalized/aggregated output, cancellation, and the three HTTP resources for bars, symbol resolution/search, and dataset metadata. Use a fake boundary for mandatory tests.

Acceptance criteria:

- HTTP tests cover successful, empty, invalid, cancelled, forbidden, unavailable, duplicate, and mapping-changing scenarios with exact status/error codes.
- Resolve/search/metadata tests use the exact `from`/`to`, `SymbolMapping`, `SymbolSearchResult`, and `DatasetMetadata` wire shapes, including deterministic sort and configured result caps.
- Requests and results use start-inclusive/end-exclusive UTC ranges and return sorted unique target times.
- The configured history interval bound is enforced before an upstream call.
- Same ticker text in two datasets or two mapping intervals cannot cross-route records.
- Captured logs and bodies contain no fake credential sentinel.

### TASK-05 — Implement the dataset session manager

Define `LiveSource` around the official client. Implement one live client per active dataset, canonical request reference counts, upstream subscription sharing, live mapping updates, immediate downstream stop-forwarding, and graceful dataset-session stop at zero references. Do not add an upstream unsubscribe abstraction.

Acceptance criteria:

- Multiple downstream requests for one canonical key share one upstream subscription; different datasets use different sessions.
- Unsubscribe removes callbacks immediately, is idempotent, and does not invoke a nonexistent upstream unsubscribe.
- Removing one symbol while another remains does not restart the dataset session; unused records are counted and discarded.
- Removing the final reference stops the official client gracefully and removes session state.
- Configured client/subscription/dataset-session limits return typed errors without partial registration.

### TASK-06 — Implement coordinated handoff

Implement `HandoffCoordinator` exactly as the design's eight-step algorithm: align boundary, subscribe with one-source-interval replay overlap, buffer, query history, wait for replay completion, merge by base-bar key, emit one snapshot, drain, and enter live.

Acceptance criteria:

- Every handoff scenario table row passes with the exact event sequence and bar values.
- No test output contains duplicate target timestamps in the snapshot or a decreasing live timestamp.
- Buffer overflow and replay ineligibility produce typed terminal failures before a snapshot.
- A deliberate `bar`-before-`snapshot` fault makes the protocol/handoff suite fail before being reverted.

### TASK-07 — Implement reconnect, liveness, and backpressure

Add gateway upstream state transitions, configured bounded backoff/jitter/retry budget, active subscription restoration, replay overlap after reconnect, downstream heartbeat, all bounded queues, same-bucket coalescing, `slow_consumer`, and upstream-overflow failure/replay.

Acceptance criteria:

- Retryable disconnect follows `live -> reconnecting -> replaying -> live`; non-retryable errors go directly to `failed`.
- Tests use deterministic clock/randomness injection and assert the configured retry sequence without sleeping in real time.
- Same-time coalescing preserves the latest candle; interval transitions, mappings, errors, acknowledgements, and states are never silently lost.
- Queue exhaustion is observable and produces the designed failure/close behavior.
- Reconnect overlap produces the same final chart points as uninterrupted delivery.

### TASK-08 — Implement accepted product policies

Implement `DEC-014`, `DEC-015`, and `DEC-016` consistently in public types, protocol fixtures, gateway state, provider behavior, and demo workflow. Requested and resolved symbols remain observable; mapping changes never merge instruments; unsupported behavior fails explicitly; stale historical data is never presented as continuously handed off live data.

Acceptance criteria:

- Session-pinned tests prove the resolved instrument stays fixed in a session, unchanged re-resolution resumes, changed re-resolution emits terminal `resolved_instrument_changed`, and a new explicit subscription can obtain the new mapping.
- Symbol resolution returns every parent child; all parent bar methods return `unsupported_parent_series` before an upstream bar request; the demo requires one selected child and submits `instrument_id`.
- Browser tests prove advance reset notice, a fresh bounded live-edge `openBars`, atomic snapshot replacement, history retention on typed failure, and no stale/live continuity claim.

### TASK-09 — Implement the TypeScript provider

Implement HTTP/WS clients, strict runtime decoding, history windowing, `AbortSignal` with protocol `cancel`/`cancelled`, callback isolation, provider states, disposable subscriptions, standalone `getBars`/`subscribeBars`, coordinated `openBars`, and downstream WebSocket recovery using stable subscription IDs plus `resume_bars`. Declare Lightweight Charts as a peer dependency and keep UI/chart instances outside the package.

Acceptance criteria:

- All TypeScript unit cases in the test strategy pass in UTC and a non-UTC test timezone.
- `openBars` resolves only after `subscribed` and `snapshot`, then routes equal/newer updates correctly.
- Aborting `openBars` cancels gateway work, rejects the pending promise with the typed cancelled error, and produces no later callback.
- Multiple symbols and sockets cannot cross-call callbacks.
- Gateway restart after live delivery follows the configured deterministic retry sequence, resumes every active subscription from its last emitted target time, and yields the same final bars as uninterrupted delivery.
- `dispose` is idempotent, releases every subscription, and prevents callbacks after resolution.
- The built package contains no Databento credential string, Rust/DBN structure, Node-only WebSocket dependency, or UI framework dependency.

### TASK-10 — Complete gateway transport, security, and observability

Wire domain services to Axum HTTP/WS routes. Enforce protocol negotiation, origins, loopback-only development mode, non-loopback authentication integration flag, body/frame/range/session/queue limits, allowlists, stable error sanitization, structured spans, and metrics.

Acceptance criteria:

- Real HTTP/WS integration tests pass for all routes, commands/events, orderings, close paths, and two-client isolation.
- Startup fails for non-loopback unauthenticated mode, missing required bounds, invalid origins, or empty dataset allowlist.
- Credential-sentinel scans pass across assets, bodies, frames, errors, and logs.
- Metrics/log tests observe each required state and redaction without per-record default logs or high-cardinality symbol labels.
- Health liveness/readiness behavior matches the technical design.

### TASK-11 — Build the demo, documentation, and release evidence

Create the minimal Vite demo and user documentation: architecture, installation, configuration, quick start, historical and realtime examples, symbol resolution, resolutions, gaps, reconnection, continuous-symbol choice, security, and real-provider qualification. Implement the offline browser scenario and release-evidence capture.

Acceptance criteria:

- The demo exposes and displays every control/state in `REQ-F-012` without embedding provider logic in UI components.
- Candlesticks render in pane 0 and volume in a separate resizable pane 1; add/remove/resize uses public pane APIs.
- Earlier history is triggered through `barsInLogicalRange`, is single-flight, and preserves the prior logical viewport.
- The demo owns exactly one chart, cleans it with `chart.remove()`, and contains no `lightweight-chart-react` or private chart-model access.
- Toolbar, status, legend, and tooltip controls meet the accessibility and boundary-handling criteria in the tutorial knowledge artifact.
- The entire offline browser scenario passes in a real browser and asserts data/state rather than screenshots.
- All mandatory release gates pass with zero warnings; the receipt includes toolchains and lock revisions.
- The optional zero-argument Databento metadata inventory distinguishes global catalogs, account-aware historical ranges, and live licensing; an access error is not reported as a functional pass.
- Public package/API documentation matches exported signatures and protocol v1.

### TASK-12 — Provision and qualify dedicated Live connectivity

Select DC3, NY4, or NY5; select a colocation provider or managed services provider; obtain explicit commercial approval; arrange the Databento 10G or 25G Live Raw API TCP cross-connect; and collect the circuit-specific latency receipt defined in `docs/dedicated-connectivity-plan.md`.

Acceptance criteria:

- Databento confirms the installed circuit, site, port speed, Live service, and TCP Raw API handoff.
- A dated measurement tied to that circuit reports the exact Databento boundary-switch-to-cross-connect boundary at the 90th percentile.
- The measured value is strictly below 50 microseconds.
- Gateway/application latency is measured separately and never substituted for the cross-connect result.
- No commercial order, site selection, or infrastructure mutation occurs without explicit user authorization.

## Concrete commands

Run from the repository root after the relevant tasks create the scripts:

```sh
pnpm install --frozen-lockfile
cargo fetch --locked
pnpm format:check
pnpm lint
pnpm typecheck
pnpm test
pnpm build
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo deny check
pnpm test:e2e:offline
prek run --all-files
```

Expected final shape is that every command exits `0`, test commands report no failed tests, Clippy reports no warnings, and the browser test reports all fourteen offline steps passed. Do not invent exact test-case counts in advance; record actual totals in release evidence.

Run optional provider tests only with explicit authorization and credentials:

```sh
DATABENTO_REAL_TESTS=1 cargo test --workspace --all-features --test real_databento -- --ignored
```

The test code must read `DATABENTO_API_KEY` from the environment and must not print it.

## Idempotence and recovery

Build, format, lint, audit, and offline test commands are repeatable. Workspace bootstrap must not overwrite an existing lockfile without reviewing the diff. Fake-source tests use ephemeral ports and temporary directories and clean them on success or failure. If a handoff or reconnect test fails, preserve the deterministic scenario seed and concise event trace in the failure output; do not retain credentials or raw unrestricted data. A failed optional provider test stops its own dataset session gracefully and leaves the offline gates runnable.

## Surprises and discoveries

- Databento Live has no unsubscribe method. The downstream disposable therefore means stop-forwarding/reference removal, with upstream session reclamation only when the dataset has no remaining consumers.
- Databento live sessions are dataset-scoped and can combine subscriptions, which makes one-session-per-dataset the natural ownership boundary.
- Databento replay is limited to the most recent 24 hours. Continuous handoff cannot be claimed when that replay is unavailable.
- Existing live continuous-contract subscriptions do not automatically remap; `DEC-014` therefore pins one resolved instrument per session.
- Parent symbology groups related instruments, but a candlestick series needs one value per time; `DEC-015` prevents an accidental cross-contract merge.
- Lightweight Charts 5.2 recommends `update` for current/new points and warns that `setData` replaces all data, so snapshot and steady-state APIs are deliberately separate.

Add new discoveries with evidence and update affected tasks before continuing.

## Decision log

The detailed source-backed log is in `docs/decision-log.md`. During implementation, duplicate the short decision and rationale here when it changes task order or acceptance:

- Decision: Use TypeScript in the browser and Rust in the gateway.
  Rationale: The repository is empty and this follows the brief's preferred browser/security boundary and official Rust client path.
- Decision: WebSocket for live commands/events; HTTP for history and symbology.
  Rationale: One multiplexed bidirectional channel preserves acknowledgement, unsubscribe, handoff, and symbol-switch ordering.
- Decision: Gateway-coordinated `openBars` is the guaranteed handoff API.
  Rationale: Independent HTTP history and live subscription cannot close their race without server-side buffering and replay.
- Decision: Use native OHLCV only in version 1 and compose custom intervals from the largest exact divisor.
  Rationale: It satisfies the required bars and keeps trade-condition/correction semantics outside version 1.

## Outcomes and retrospective

Not yet implemented. At each milestone, record what works, the exact proof, remaining tasks, and any design change. At final completion, summarize user-visible behavior, release-gate evidence, optional real-provider status, known limits, and lessons that should change a later version.
