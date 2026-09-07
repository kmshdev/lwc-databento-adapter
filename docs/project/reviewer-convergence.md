# Independent reviewer convergence

## Documentation-readiness review method

Two fresh-context, read-only reviewers independently read the complete contract and knowledge set before implementation. Each reconstructed the components, data model, dependencies, and definition of done, then identified implementation-divergent interpretations. After documentation repairs, both performed a second pass. Documentation result: **AGREE**. This result established an implementable contract; it did not claim that the later runtime already met that contract.

## Materially agreed system

### Components and ownership

- The Rust gateway alone owns credentials, official Databento clients, DBN decoding, symbology, normalization, aggregation, replay/handoff, upstream dataset sessions, security/capacity enforcement, fan-out, and server observability.
- The framework-neutral TypeScript package owns browser HTTP/WebSocket transport, runtime protocol validation, cancellation, reconnect-visible state, subscription handles, and mapping safe JSON into Lightweight Charts data types. It owns no market-data rules or chart instance.
- Shared protocol documents and fixtures enforce the Rust/TypeScript boundary in both directions.
- The demo owns one Lightweight Charts instance, panes, application DOM, accessibility behavior, and browser qualification. It imports `lightweight-charts` directly and removes the chart on unmount.

### Data model and invariants

The agreed flow is `RequestedStreamKey -> ResolvedStreamKey -> BaseBar -> TargetBar`. Routing occurs only after dataset-qualified symbology resolution. One emitted candle never mixes instruments. Historical ranges are integer-second, start-inclusive/end-exclusive, with incomplete leading source intervals skipped. UTC epoch bucketing, fixed-price validation, duplicate base-bar replacement, nondecreasing target timestamps, equal-time replacement, and greater-time append are normative.

### Dependencies and toolchain

- Browser peer boundary: pinned-compatible Lightweight Charts 5.2.x; no `lightweight-chart-react` or private chart-model imports.
- Server boundary: official `databento` Rust client plus the selected Axum/Tokio/Serde/Tracing stack, all reverified and locked by `TASK-00`.
- Tooling boundary: pnpm and the declared TypeScript gates; Cargo formatting, Clippy, tests, and dependency audit; mise owns committed tool/task configuration while ignored `mise.local.toml` owns local secrets.
- Real Databento qualification is manual and bounded. The default verification graph is offline.

### Definition of done

Every requirement has an automated traceability proof; every task acceptance criterion is met; all mandatory offline gates pass without warnings; the fourteen-step browser scenario passes; security/dependency scans find no key, wrapper, or private imports; mandatory performance invariants pass; documentation matches shipped interfaces; optional real-provider status is honestly recorded as passed, not run, or inconclusive.

## Gaps repaired during convergence

- Brought the review, core/tutorial knowledge, and mise artifacts into the normative contract gate.
- Fixed the separate resizable volume pane, `barsInLogicalRange` single-flight history trigger, viewport preservation, wrapper removal, and cleanup contract.
- Defined `historyChunkIntervals` in target-interval units.
- Added a normative gateway configuration schema with stable names and units.
- Defined non-aligned historical `from` behavior.
- Aligned mandatory performance invariants with optional user thresholds.
- Aligned the implementation plan with the fourteen-step browser scenario.
- Separated UI-only states from provider protocol states.
- Removed the hidden assumption that every demo history workflow always calls `openBars`.

## Remaining blockers

The user selected session-pinned continuous symbols (`DEC-014`), parent resolution followed by explicit child selection (`DEC-015`), and an explicit atomic Go-live reset (`DEC-016`). The requirements, design, task acceptance criteria, and traceability now encode those choices consistently. There is no remaining documentation-level product fork blocking implementation; compiled compatibility and runtime evidence remain milestone gates.

## Implementation review round 1

On 2026-08-30, two new fresh-context reviewers independently inspected the implemented workspace and reran representative gates. They materially agreed that the component boundaries and data-model intent matched the contract, while the runtime did not yet meet the release definition of done. Both identified the same high-confidence blockers:

- the executable gateway selected seeded `FakeHistorySource` rather than an official Databento source;
- live subscription state and coordinated handoff did not implement real upstream replay/streaming semantics;
- browser reconnect/resume initially depended on state local to the lost WebSocket;
- capacity, origin, frame, and unauthenticated-bind controls were incomplete;
- the mandatory browser proof did not cover every required live/reconnect/gap behavior against the Axum gateway.

One reviewer additionally identified paging/whitespace rendering and fabricated fallback symbology metadata. Both reviewers cleared the bounded-provider-script and disconnect-race findings after targeted fixes and a fresh passing `mise run check && prek run --all-files`. At the end of round 1 their converged verdict was **NOT READY** pending runtime repairs.

## Final implementation convergence

After successive repairs, both independent reviewers returned **READY** on 2026-08-31. They verified official Historical/Live composition, one physical dataset actor, canonical fan-out/refcounts, `subscribeBars`, replay-before-history handoff, wider live aggregation, cross-socket resume, parent selection, atomic Go-live reset, configuration bounds, paging/whitespace behavior, and the official development command.

The last disagreement concerned a stale `ReplayCompleted` marker in a shared canonical replay tail. The final design retains only bar events, fans lifecycle events out without retention, and appends one fresh boundary per resumed downstream. Focused regressions prove older overlap is filtered, equal-time data replaces, newer data remains ordered, and a stale boundary cannot truncate catch-up. Both reviewers inspected that code and its tests, then materially agreed that no implementation blocker remains for the local-beta definition of done.
