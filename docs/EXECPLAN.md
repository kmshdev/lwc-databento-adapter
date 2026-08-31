# Build and iterate the online-beta Databento adapter for Lightweight Charts

This ExecPlan is a living document governed by `/Users/kmsh/.codex/agent/PLANS.md`. Keep `Progress`, `Surprises & Discoveries`, `Decision Log`, and `Outcomes & Retrospective` current as implementation proceeds.

## Purpose / Big Picture

Build a mixed Rust and TypeScript monorepo that lets browser applications load Databento OHLCV history, render candlesticks and volume with Lightweight Charts, and explicitly reset to a coordinated live-edge stream without exposing the Databento key. The verified local workflow is the integration baseline; the active phase is online beta testing and rapid feature iteration across the full workflow, basic functions, execution pipelines, ToB/C interfaces, package consumption, and deployment to an identified beta target. Production hardening remains deferred.

## Progress

- [x] (2026-08-30) Prepared and independently reviewed requirements, design, tasks, tests, traceability, and knowledge artifacts.
- [x] (2026-08-30) Selected `DEC-014` session-pinned continuous symbols, `DEC-015` resolve-then-select parents, `DEC-016` explicit Go-live reset, and local-only beta scope.
- [x] (2026-08-30) Audited high-risk external claims against primary sources.
- [x] (2026-08-30) Persisted this ExecPlan.
- [x] (2026-08-30) Closed the selected decisions across every planning artifact and enforced the folder blueprint.
- [x] (2026-08-30) Bootstrapped the pinned mise, pnpm, Cargo, Vite, and Puppeteer workspace; compiled the Databento, Axum, and Lightweight Charts API proofs.
- [x] (2026-08-30) Froze protocol v1 and made TypeScript and Rust validate the shared valid/invalid fixtures.
- [x] (2026-08-30) Implemented and tested integer normalization, aggregation, fakeable history, symbology, parent rejection, and dataset isolation.
- [x] (2026-08-31) Completed the live-session lifecycle: one physical dataset actor, canonical fan-out/refcounts, replay-before-history, bounded handoff/deduplication, wider-resolution aggregation, graceful final close, and ordered shared-canonical resume.
- [x] (2026-08-30) Implemented the framework-neutral TypeScript provider, shared browser WebSocket, cancellation, runtime decoding, timestamp ordering, and idempotent disposal.
- [x] (2026-08-30) Built the accessible two-pane demo and passed its deterministic Puppeteer workflow plus a visible seeded gateway-to-browser workflow.
- [x] (2026-08-30) Passed `mise run check`, the self-starting Axum/Vite `test:e2e:local`, and `prek run --all-files` after the second repair round.
- [x] (2026-08-31) Added zero-argument account discovery; the inventory found 29 catalog datasets, 29 account-aware historical ranges, and zero unverified datasets.
- [x] (2026-08-31) Used the key for bounded `GLBX.MDP3` live qualification and passed direct subscription/replay/close, real gateway `openBars` and `subscribeBars` in the in-app browser, and isolated packed-package compilation against Lightweight Charts 5.2.1.
- [x] (2026-08-31) Advanced to online beta testing and rapid feature iteration; enabled npm-compatible packaging and expanded the isolated consumer proof across the complete public provider API.
- [x] (2026-08-31) Two independent final reviewers reconstructed the implementation, challenged reconnect and shared replay ordering, and both returned `READY` after the regressions passed.
- [x] (2026-08-31) Started the contract-led refactor loop, preserved the pre-existing tree as an uncommitted baseline on `agent/databento-contract-refactor`, and recorded all 17 requirement IDs in `/tmp/refactor-lwc-databento-adapter.md`.
- [x] (2026-08-31) Reproduced and repaired the deterministic browser test's fixed-port collision; the focused Puppeteer workflow now passes while an unrelated Vite process remains on port 5173.
- [x] (2026-08-31) Closed review blockers around active-request pagination, disconnect races, aborted-subscription cleanup, production-only test globals, `open_bars` live-edge validation, and DBN live mapping records with focused regressions.
- [x] (2026-08-31) Added public TypeScript models for all 26 tables on Databento's official reference-data enum page without embedding a stale copy of the external value catalog.
- [ ] Audit every local and supplied Databento requirement against current code, tests, configuration, documentation, and bounded runtime evidence; keep every ledger entry honestly classified.
- [x] (2026-08-31) Established the repeatable production-preview page-load gate. The only route, `/`, measured p95 20.6 ms, 19.1 ms, and 21.2 ms across three consecutive cache-disabled runs, below the 50 ms threshold each time.
- [x] (2026-08-31) Verified the dedicated-connectivity source and separated its published 42.4-microsecond 90th-percentile physical handoff from browser and gateway latency.
- [ ] Provision and qualify a 10G or 25G Databento Live Raw TCP cross-connect at DC3 or Equinix NY4/5; completion requires the circuit-specific sub-50-microsecond receipt in `docs/dedicated-connectivity-plan.md`.
- [x] (2026-08-31) Researched the current DC3 provider market and prepared a no-secrets RFQ. Beeks is the recommended small-footprint pilot candidate; Options Technology and Avelacom remain alternatives pending written Databento cross-connect confirmation.
- [ ] Run the complete offline and authorized live gates, review and simplify each significant slice, and commit each verified slice without including secrets or unrelated state.

## Surprises & Discoveries

- Observation: package-manager commands initially fail because `mise.local.toml` is untrusted.
  Evidence: mise 2026.8.8 reports that the project config is not trusted.
- Observation: the pinned Lightweight Charts checkout has source and Puppeteer configuration but no installed dependencies or standalone bundles.
  Evidence: its `node_modules/puppeteer` and `dist/lightweight-charts.standalone.*.js` paths are absent.
- Observation: a generated Context7 summary incorrectly listed a Databento live unsubscribe method.
  Evidence: the official Live API says there is no unsubscribe method.
- Observation: Graph Mode was unavailable.
  Consequence: dependency ordering is recorded explicitly in this plan and the folder blueprint.
- Observation: the first Rust contract command filtered to zero tests even though the aggregate check exited successfully.
  Evidence: the fixture tests did not contain the `protocol_contract` filter string.
  Resolution: renamed the tests, added an all-valid-fixtures parity test, and reran the gate with seven Rust and thirteen TypeScript contract tests.
- Observation: an in-flight older-history request could overwrite the demo's disconnected notice.
  Evidence: the Puppeteer teardown assertion observed `No earlier history is available.` after disconnect.
  Resolution: disconnect now invalidates the view, publishes its terminal notice before awaiting unsubscribe, and suppresses stale notices; the strict regression passes.
- Observation: the first provider task required the user to supply dataset, symbol, symbology, schema, range, and limit even though official metadata can discover account-aware historical availability.
  Resolution: the task now accepts no arguments, enumerates the global catalogs, probes `get_dataset_range` for each dataset, joins successful ranges to publisher venues, and requests no market-data records.
- Observation: the Databento API does not expose a bulk live-subscription entitlement list.
  Evidence: `list_datasets` lists valid dataset codes, `list_publishers` is a global mapping, and only `get_dataset_range` is documented as entitlement-aware; live licensing is managed in the portal or proved with a dataset-specific session.
  Consequence: the inventory reports `live_entitlements=not_inferred` instead of mislabeling historical metadata as live access.
- Observation: Databento's sub-50-microsecond figure is a physical handoff metric, not an application end-to-end measurement.
  Evidence: the dedicated-connectivity guide reports 42.4 microseconds at the 90th percentile from first byte entering Databento's boundary switch to last byte leaving onto the customer cross-connect.
  Consequence: repository benchmarks cannot close `REQ-Q-006`; a selected and installed DC3/NY4/NY5 10G or 25G circuit plus provider-linked evidence is required.
- Observation: the first real browser `openBars` request sent `Date.now()` milliseconds as protocol seconds.
  Evidence: the in-app browser captured `to=1788122761680`, followed by `symbol_mapping_failed`.
  Resolution: live-edge calculation now converts milliseconds to seconds, and the browser regression rejects 13-digit request times.
- Observation: a gateway error on a newly constructed browser subscription threw `this.resolveDone is not a function`.
  Resolution: promise resolver initialization now precedes promise construction, with a focused termination regression.
- Observation: Databento historical availability lagged the requested live edge, causing the history leg to fail after live subscription succeeded.
  Resolution: `openBars` clamps the historical leg to account-aware `available_to` and begins intraday replay one source interval before that boundary, preserving overlap through the requested live edge.
- Observation: real `subscribeBars` attempted symbology resolution over `[0, i64::MAX)` and could never qualify against Databento.
  Resolution: it now resolves over a finite current source interval; the in-app browser observed `live`, instrument `42140870`, and clean `closed` state.
- Observation: the public `BarPage` arrays were readonly while Lightweight Charts `setData` requires a mutable array type.
  Resolution: the public arrays are mutable, and a packed-tarball external consumer compiles direct `candles.setData(page.bars)` usage.
- Observation: the first runtime pass constructed `FakeHistorySource::demo` unconditionally and kept WebSocket state connection-local.
  Resolution: seeded mode is now explicit; opt-in historical mode wires official Databento history and live replay, and subscription state survives a browser socket reconnect. Deterministic local evidence still does not qualify paid-provider behavior.
- Observation: the first live-client implementation modeled one session runner per downstream subscription rather than one physical Databento client per dataset.
  Resolution: `DatabentoDatasetActor` now owns one client per dataset; `DatasetLiveRegistry` provides bounded canonical fan-out, reference counting, shared replay tails, and final-reference close.
- Observation: retaining an old `ReplayCompleted` marker in a shared canonical tail could end a resumed catch-up before later bars were emitted.
  Resolution: only bar events enter the retained tail; lifecycle markers fan out without retention, and every resumed downstream receives one fresh boundary. The exact stale-marker regression passes.
- Observation: the repository had no commits and every file was untracked on an unborn `main`, so there was no historical diff or known-good commit to use as audit evidence.
  Evidence: `git status --short --branch` reported `No commits yet on main` before the feature branch was created.
- Observation: the deterministic browser test accepted any HTTP 200 response on fixed port 5173 after ignoring Vite's early exit, so an unrelated application could satisfy readiness and produce a false DOM failure.
  Evidence: port 5173 served `anthera-design-system`; after dynamic test ports were introduced, the focused browser test exited 0 while that process remained listening.
- Observation: the default-feature gateway failed to compile even though the all-features gate passed because a transport helper import was incorrectly gated behind `databento-compat`; after the import was corrected, the default build exposed one feature-specific unused variable.
  Evidence: `RUSTFLAGS='-D warnings' cargo check -p databento-gateway` passes after making the helper import unconditional and explicitly consuming the feature-only value in the default build.
- Observation: the demo has one HTML entry, no router dependency or pathname state, and no alternate Vite build input, so the complete current page inventory is `/`.
  Evidence: three identical benchmark runs reported `/` p95 values of 20.6 ms, 19.1 ms, and 21.2 ms with maxima between 21.0 ms and 23.2 ms.
- Observation: `docs/implementation-plan.md` still described the pre-implementation skeleton as current state after the workspace, gateway, fixtures, demo, and acceptance tasks existed.
  Evidence: the statement contradicted the passing full gate and current repository tree; it now identifies the implemented system and current evidence boundaries.
- Observation: the passing gate did not prove `REQ-Q-003`; the gateway currently has no structured tracing calls, metrics recorder, or tests for the promised operational signals.
  Evidence: source searches for tracing macros, `TraceLayer`, metrics, latency, drop reasons, queue utilization, and redaction found only the internal active-client counter.
- Observation: `open_bars` accepted an ordered historical range whose `to` was an hour behind the current source bucket, then emitted `subscribed` and `snapshot` despite the accepted live-edge-only contract.
  Resolution: the gateway now rejects stale, future-bucket, and empty-history live ranges with `invalid_range` before resolution, subscription state, or upstream live work begins.
- Observation: the dataset actor ignored typed DBN `SymbolMappingMsg` records and routed bars only through the instrument identity resolved before the session began.
  Resolution: the actor now consumes mapping records before OHLCV records, emits a typed mapping event when the session-pinned identity is unchanged, and terminates with `resolved_instrument_changed` when it changes.
- Observation: editing form fields without submitting changed the request used for older-history pagination, and disconnect did not invalidate every delayed request/callback.
  Resolution: the demo stores the request that produced the visible chart, pins pagination to it, sequences unsubscribe before abort for an active stream, and rejects all late request results and callbacks by revision.

## Decision Log

- Decision: `DEC-014` makes continuous-symbol subscriptions session-pinned. On reconnect, unchanged resolution resumes; changed resolution terminates with `resolved_instrument_changed` and requires a new subscription.
  Rationale: one chart series must never silently mix instruments.
  Date/Author: 2026-08-30, user and Codex.
- Decision: `DEC-015` makes parent symbols resolve to children; bar methods reject `parent` until one instrument is selected.
  Rationale: one candle series represents one resolved instrument.
  Date/Author: 2026-08-30, user and Codex.
- Decision: `DEC-016` makes Go live explicitly reset an arbitrary historical view only after a fresh live-edge snapshot succeeds.
  Rationale: stale history must not be presented as a coordinated handoff.
  Date/Author: 2026-08-30, user and Codex.
- Decision: the earlier loopback-only delivery boundary is superseded by `DEC-017`; online beta testing, rapid feature iteration, reusable package consumption, and deployment to an identified beta target are active scope.
  Rationale: the user advanced the project phase after local workflow and real-provider qualification.
  Date/Author: 2026-08-31, user.
- Decision: researched dependency versions are initial pins; compatibility compilation is the blocking acceptance gate.
  Rationale: registry presence does not prove required APIs compile together.
  Date/Author: 2026-08-30, Codex.
- Decision: real-provider discovery is a zero-argument metadata inventory, not a paid record probe.
  Rationale: official account-aware range metadata can discover historical availability safely, while no bulk API can prove live venue licensing.
  Date/Author: 2026-08-31, user and Codex.
- Decision: the zero-argument provider task includes one fixed bounded live proof for `GLBX.MDP3`, `ES.FUT`, and `ohlcv-1m`.
- Decision: page-load performance and Databento transport latency are separate metrics. The user-supplied 50 ms page-load threshold is measured locally per demo route; Databento's dedicated-connectivity figures remain provider network measurements and are not treated as proof of browser page-load time.
  Rationale: the official dedicated-connectivity guide defines its latency from first byte at Databento's boundary switch to last byte read from the client socket, while the goal separately requires every page load to remain below 50 ms.
  Date/Author: 2026-08-31 / Codex.
  Rationale: it proves actual authentication and replay semantics without requiring user-supplied parameters or inferring every venue license.
  Date/Author: 2026-08-31, user and Codex.

## Outcomes & Retrospective

The monorepo, shared protocol, Rust gateway, TypeScript adapter, and accessible Lightweight Charts demo are observable and reproducible. On 2026-08-31, the final `mise run check` passed with 47 Rust tests, seven filtered Rust contract tests, thirteen TypeScript contract tests, twenty TypeScript tests, successful builds, Puppeteer behavior, dependency audit, and no-warning formatting/lint/type gates. The self-starting Axum/Vite `local-beta.e2e.ts`, `prek run --all-files`, structure/doc/credential checks, and both default/all-feature Rust builds also passed. The final production-preview run measured the only route, `/`, at 21.1 ms p95 across twenty cache-disabled navigations after three warmups.

Real-provider qualification passed on 2026-08-31. Metadata discovery returned `catalog_datasets=29`, `account_aware_ranges=29`, and `unverified=0`. The fixed `GLBX.MDP3` session observed `SubscriptionAck`, `ReplayCompleted`, eighteen normalized `ohlcv-1m` bars, latest-available continuous resolution, and graceful close. The browser-to-gateway provider flow reached `live`, retained instrument `42140870` while enriching it with raw symbol `ESU6`, exposed no browser credential, and closed cleanly in 23.8 seconds. The package now has npm-compatible public exports, declarations, peer metadata, prepack build, integration documentation, and isolated full-API consumer compilation. Registry publication still requires scope ownership, licensing, and release identity; online deployment still requires a named target and its application access boundary.

## Context and Orientation

The repository began as documentation plus a partial browser skeleton and now contains the implemented Rust gateway, framework-neutral TypeScript package, shared protocol fixtures, runnable Vite demo, and repeatable offline/live qualification tasks. The Rust gateway owns credentials, DBN decoding, symbology, normalization, aggregation, replay, reconnect, fan-out, and limits; the complete promised observability surface remains an explicit audit gap. The TypeScript package owns browser HTTP/WebSocket transport, runtime validation, cancellation, subscription handles, reference-data models, and conversion to Lightweight Charts data. The plain TypeScript demo owns chart instances, panes, controls, legends, tooltips, and accessibility.

The dependency direction is `demo -> TypeScript provider -> protocol fixtures` and `Rust gateway -> protocol fixtures -> official Databento client`. A `BaseBar` is one native Databento OHLCV record. A `TargetBar` is a native or aggregated candle sent to the browser. Routing follows `RequestedStreamKey -> ResolvedStreamKey -> BaseBar -> TargetBar`; ticker text alone never routes records.

## Plan of Work

Milestone 0 closes the three decisions in requirements, design, tasks, tests, traceability, and reviewer artifacts; changes the phase to local beta; creates `Project_Folders_Structure_Blueprint.md`; and adds structural enforcement.

Milestone 1 creates mise, pnpm, Cargo, strict tool configs, CI, lockfiles, and compatibility spikes for Databento 0.60.0, Axum 0.8.9, Lightweight Charts 5.2.1, and Puppeteer. Semantic incompatibility stops for design repair.

Milestone 2 creates protocol v1, shared valid/invalid fixtures, Rust Serde types, and TypeScript Zod schemas. Both languages must accept and reject the same fixtures.

Milestone 3 implements Rust normalization, aggregation, historical endpoints, symbol resolution/search, metadata, three gap policies, parent rejection, and fakeable sources.

Milestone 4 implements one Databento live client per dataset, reference-counted downstream subscriptions, coordinated history-to-live handoff, replay, reconnect, bounded queues, backpressure, and session-pinned continuous-symbol behavior.

Milestone 5 implements the browser provider: HTTP, one shared WebSocket, runtime decoding, history chunking, cancellation, reconnect/resume, callback isolation, and idempotent cleanup. It imports no UI framework, DBN conversion, chart wrapper, or private chart API.

Milestone 6 builds the local Vite demo at `127.0.0.1:5173` against the gateway at `127.0.0.1:8080`, with separate candle/volume panes, history paging, parent child-selection, explicit atomic Go-live reset, accessibility, resize fallback, and the fourteen-step Puppeteer workflow.

Milestone 7 runs all offline gates, starts the local beta, records bounded real-provider and external-consumer evidence, and obtains two fresh independent reviews.

## Concrete Steps

After reviewed `mise.toml` exists, explicitly run `mise trust`, `mise install`, and `prek install`. Use mise leaf tasks for documentation, structure, formatting, linting, typing, contracts, Rust tests, TypeScript tests, gateway tests, build, browser tests, audit, and development servers. The aggregate `mise run check` is offline and must not require `DATABENTO_API_KEY`.

The final gate is:

    mise run docs:check
    mise run structure:check
    mise run fmt:check
    mise run lint
    mise run typecheck
    mise run test:contracts
    mise run test:rust
    mise run test:typescript
    mise run test:gateway
    mise run build
    mise run test:browser
    mise run audit
    mise run check
    prek run --all-files

Start the local beta with `mise run dev:gateway` and `mise run dev:demo`. Run `mise run test:live-databento` for account discovery plus the fixed bounded live replay proof, `mise run test:live-integration` for the real gateway/browser workflow, and `mise run test:package-consumer` for isolated tarball integration. None may print credentials or infer untested live licenses.

## Validation and Acceptance

Completion requires structure/document consistency, zero-warning offline gates, matching Rust/TypeScript protocol fixtures, ordered instrument-isolated bars, no duplicate volume, explicit parent selection, terminal detection of changed continuous instruments, atomic Go-live reset, passing fourteen-step browser behavior, absence of credentials/wrappers/private imports, a visible local workflow, honest optional-provider status, and material agreement from two independent reviewers.

## Idempotence and Recovery

All setup and check tasks are rerunnable. Never rewrite golden fixtures during normal tests. Preserve unrelated user files and ignored `mise.local.toml`. Do not proceed past a compatibility contradiction, credential leak, protocol mismatch, unexplained continuity gap, or changed resolved instrument. No production deployment, destructive cleanup, external publication, or unbounded Databento request is authorized.

## Interfaces and Dependencies

The public TypeScript provider retains `getBars`, `openBars`, `subscribeBars`, `resolveSymbol`, `searchSymbols`, `getDatasetMetadata`, and `dispose`; `ProviderErrorCode` includes `resolved_instrument_changed`. HTTP owns history, symbology, metadata, and health. WebSocket subprotocol `databento-lwc.v1` owns subscribe, open, resume, unsubscribe, cancel, acknowledgement, snapshot, bar, status, mapping, error, cancellation, and heartbeat messages.

Initial pins are Node 24.20.0, pnpm 11.19.0, Rust 1.97.1, Lightweight Charts 5.2.1, TypeScript 7.0.2, Vite 8.2.2, Vitest 4.1.11, Zod 4.5.4, `ws` 8.21.3, Oxlint 1.80.0, Oxfmt 0.65.0, Puppeteer 24.6.1, Databento 0.60.0, Axum 0.8.9, Tokio 1.53.1, Serde 1.0.229, Tracing 0.1.44, and Tower HTTP 0.7.0.

Revision note (2026-08-31): this plan now also governs the contract-led refactor loop, records the no-history Git baseline and fixed-port browser-test repair, and separates the user-supplied page-load threshold from Databento network-latency evidence.
