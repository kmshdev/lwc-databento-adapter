# `@lwc-databento/adapter`

Framework-neutral Databento historical and live OHLCV provider for Lightweight Charts 5.2.

The browser package connects to the companion Rust gateway. Keep `DATABENTO_API_KEY` only in the gateway environment; never place it in frontend code or build-time browser variables.

## Install

After the first registry release:

```sh
pnpm add @lwc-databento/adapter lightweight-charts
```

Before registry release, create and install a local tarball:

```sh
pnpm --filter @lwc-databento/adapter pack
pnpm add ./lwc-databento-adapter-0.1.0.tgz lightweight-charts
```

## History

```ts
import { CandlestickSeries, createChart, type UTCTimestamp } from 'lightweight-charts';
import { createDatabentoDataProvider } from '@lwc-databento/adapter';

const chart = createChart(document.querySelector('#chart')!);
const candles = chart.addSeries(CandlestickSeries);
const provider = createDatabentoDataProvider({
  gatewayUrl: 'https://market-data.example.com',
  historyChunkIntervals: 500,
  reconnect: {
    baseDelayMs: 250,
    maxDelayMs: 8_000,
    maxAttempts: 8,
    jitterRatio: 0.2,
  },
});

const page = await provider.getBars({
  dataset: 'GLBX.MDP3',
  symbol: 'ES.c.0',
  stypeIn: 'continuous',
  resolution: '1m',
  from: 1_788_092_880 as UTCTimestamp,
  to: 1_788_122_880 as UTCTimestamp,
});
candles.setData(page.bars);
```

## Coordinated history-to-live handoff

```ts
const live = await provider.openBars(
  {
    dataset: 'GLBX.MDP3',
    symbol: 'ES.c.0',
    stypeIn: 'continuous',
    resolution: '1m',
    from: 1_788_092_880 as UTCTimestamp,
    to: 1_788_122_880 as UTCTimestamp,
  },
  {
    onBar: (bar) => candles.update(bar),
    onState: (state) => console.info('Databento state', state),
    onError: (error) => console.error(error.code, error.message),
  },
);

candles.setData(live.initial.bars);

// Component teardown:
await live.subscription.dispose();
await provider.dispose();
chart.remove();
```

Use `resolveSymbol` before requesting bars for a parent symbol, select one returned child, then submit that child with `stypeIn: 'instrument_id'`. A live continuous symbol remains pinned to its resolved instrument for the session; a reconnect that resolves differently terminates with `resolved_instrument_changed`.

The public surface also includes `subscribeBars`, `searchSymbols`, `getDatasetMetadata`, cancellation through `AbortSignal`, and idempotent disposal. It imports Lightweight Charts types directly and has no UI-framework dependency.

## Wiring a series directly: `BarFeed` and `bindSeries`

`toBarFeed`/`bindSeries` are a thin, additive layer over `openBars` for the common case of
feeding one Lightweight Charts series without hand-writing the `setData`/`update` glue:

```ts
import { createDatabentoDataProvider, toBarFeed, bindSeries } from '@lwc-databento/adapter';

const provider = createDatabentoDataProvider({ /* ... */ });
const feed = toBarFeed(provider);
const series = chart.addSeries(CandlestickSeries);

const live = await bindSeries(feed, series, {
  dataset: 'GLBX.MDP3',
  symbol: 'ES.c.0',
  stypeIn: 'continuous',
  resolution: '1m',
  from: 1_788_092_880 as UTCTimestamp,
  to: 1_788_122_880 as UTCTimestamp,
});

// Component teardown:
await live.subscription.dispose();
await provider.dispose();
```

`bindSeries` calls `series.setData(...)` with the initial page and `series.update(bar, historicalUpdate)`
for each subsequent event, forwarding any `onState`/`onError`/`onSymbolMapping` handlers supplied.
`series` only needs to satisfy the structural `ISeriesApiLike` interface, not the full Lightweight
Charts `ISeriesApi`. `BarEvent.historicalUpdate` is currently always `undefined` from `toBarFeed`
because the wire protocol has no revision signal yet (see `DEC-020` in `docs/decision-log.md`);
the field exists so `bindSeries` is ready for that extension without a breaking change. Consumers
who need `onState`/`onError`/custom batching should keep using `provider.openBars` directly — the
wrapper is optional, not a replacement for the provider's public API.

## Reference-data models

`referenceDataEnumNames` and `ReferenceDataEnumTable` model every table in Databento's
[reference-data enum catalog](https://databento.com/docs/standards-and-conventions/reference-data-enums).
Most tables use `{ value, description }` rows; `EVENTSUBTYPE` adds its parent `event`, while
`EXCHANGE` uses `{ name, country, exchange, mic }`. The package models the official table
structure without embedding a stale copy of Databento's catalog values.
