import type {
  CandlestickData,
  HistogramData,
  UTCTimestamp,
  WhitespaceData,
} from 'lightweight-charts';
import type {
  BarHandlers,
  BarMetadata,
  BarPage,
  BarRequest,
  DatabentoDataProvider,
  HistoryRequest,
  OpenBarsResult,
  ProviderState,
  Subscription,
  SymbolMapping,
} from '../types/index.js';
import type { DatabentoProviderError } from '../errors/index.js';

/**
 * The chart-facing bar shape: exactly what {@link ISeriesApiLike.setData} and
 * {@link ISeriesApiLike.update} accept for a candlestick series. This is the
 * package's existing `ChartBar` type re-exported under a feed-facing name —
 * a feed never invents its own payload vocabulary; Lightweight Charts
 * remains the only type authority.
 */
export type FeedBar = CandlestickData<UTCTimestamp> | WhitespaceData<UTCTimestamp>;

/** A `bar` event's correction flag, mapped onto `ISeriesApi.update(bar, historicalUpdate)`. */
export interface BarEvent {
  bar: FeedBar;
  volume?: HistogramData<UTCTimestamp>;
  meta: BarMetadata;
  /** True when this event revises an existing (non-latest) bar rather than extending the series. */
  historicalUpdate?: boolean;
}

/** Callbacks a feed consumer supplies to receive bars without touching provider-specific vocabulary. */
export interface BarSink {
  onBar(event: BarEvent): void;
  onState?(state: ProviderState): void;
  onError?(error: DatabentoProviderError): void;
  onSymbolMapping?(mapping: SymbolMapping): void;
}

/**
 * Provider-neutral bar feed. Every method and payload type here is either a
 * Lightweight Charts type or a small opaque envelope around it — never
 * Databento vocabulary (`dataset`, `stypeIn`, DBN concepts). This is the
 * additive DEC-020 wrapper: it delegates to an existing
 * {@link DatabentoDataProvider} rather than replacing it, so the provider's
 * own public API is unchanged and non-Databento-aware call sites only ever
 * see this surface.
 */
export interface BarFeed {
  getBars(request: HistoryRequest): Promise<BarPage>;
  openBars(request: HistoryRequest, sink: BarSink): Promise<OpenBarsResult>;
  subscribeBars(request: BarRequest, sink: BarSink): Promise<Subscription>;
}

/**
 * The minimal structural shape of `ISeriesApi<'Candlestick'>` a feed needs.
 * Consumers pass their real `ISeriesApi` (or a compatible stand-in for
 * tests); this package never imports or retains `ISeriesApi` itself, keeping
 * the "do not hold a chart handle" rule intact while still offering
 * one-line wiring.
 */
export interface ISeriesApiLike {
  setData(data: FeedBar[]): void;
  update(bar: FeedBar, historicalUpdate?: boolean): void;
}

/**
 * Bridges a {@link BarSink} to the provider's {@link BarHandlers}. The
 * provider dispatches `onBar` and then, synchronously and only when volume
 * data exists for that same bar, `onVolume` for the identical `meta`
 * reference (see `emitBar` in `provider/index.ts`) — there is no combined
 * "bar with volume" event on the wire. A bar is therefore queued via a
 * microtask rather than forwarded to `sink.onBar` immediately, so a
 * same-tick `onVolume` call has a chance to attach its volume first; the
 * microtask always runs after that synchronous pair completes.
 */
function toBarHandlers(sink: BarSink): BarHandlers {
  let pendingBar: FeedBar | undefined;
  let pendingMeta: BarMetadata | undefined;
  let pendingVolume: HistogramData<UTCTimestamp> | undefined;

  const flush = () => {
    if (pendingBar === undefined || pendingMeta === undefined) return;
    sink.onBar({
      bar: pendingBar,
      meta: pendingMeta,
      ...(pendingVolume !== undefined ? { volume: pendingVolume } : {}),
    });
    pendingBar = undefined;
    pendingMeta = undefined;
    pendingVolume = undefined;
  };

  return {
    onBar: (bar, meta) => {
      flush();
      pendingBar = bar;
      pendingMeta = meta;
      pendingVolume = undefined;
      queueMicrotask(flush);
    },
    onVolume: (volume, meta) => {
      if (pendingMeta === meta) pendingVolume = volume;
    },
    ...(sink.onState ? { onState: sink.onState } : {}),
    ...(sink.onError ? { onError: sink.onError } : {}),
    ...(sink.onSymbolMapping ? { onSymbolMapping: sink.onSymbolMapping } : {}),
  };
}

/** Wraps an existing {@link DatabentoDataProvider} as a {@link BarFeed}. */
export function toBarFeed(provider: DatabentoDataProvider): BarFeed {
  return {
    getBars: (request) => provider.getBars(request),
    openBars: (request, sink) => provider.openBars(request, toBarHandlers(sink)),
    subscribeBars: (request, sink) => provider.subscribeBars(request, toBarHandlers(sink)),
  };
}

/**
 * Binds a {@link BarFeed} to a chart series: an initial `getBars` page feeds
 * `series.setData`, and every subsequent `BarEvent` feeds
 * `series.update(bar, historicalUpdate)`. Returns the {@link Subscription}
 * for the caller to unsubscribe/dispose; this helper never retains the
 * series beyond the synchronous calls it makes into it.
 */
export async function bindSeries(
  series: ISeriesApiLike,
  feed: BarFeed,
  request: HistoryRequest,
  handlers: Pick<BarSink, 'onState' | 'onError' | 'onSymbolMapping'> = {},
): Promise<Subscription> {
  const { initial, subscription } = await feed.openBars(request, {
    onBar: (event) => series.update(event.bar, event.historicalUpdate),
    ...handlers,
  });
  try {
    series.setData(initial.bars);
  } catch (error) {
    await subscription.dispose();
    throw error;
  }
  return subscription;
}
