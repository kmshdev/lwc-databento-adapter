import { describe, expect, it, vi } from 'vitest';
import type { UTCTimestamp } from 'lightweight-charts';
import { bindSeries, toBarFeed } from '../../src/feed/index.js';
import type {
  BarMetadata,
  BarPage,
  DatabentoDataProvider,
  HistoryRequest,
  OpenBarsResult,
  ProviderState,
  Subscription,
} from '../../src/types/index.js';

const meta: BarMetadata = {
  dataset: 'GLBX.MDP3',
  requestedSymbol: 'ESZ4',
  resolvedSymbol: 'ESZ4',
  instrumentId: 123,
  sourceSchema: 'ohlcv-1m',
  synthetic: false,
};

const initialPage: BarPage = {
  bars: [{ time: 0 as UTCTimestamp, open: 1, high: 2, low: 0.5, close: 1.5 }],
  volumes: [],
  metadata: new Map(),
};

const request: HistoryRequest = {
  dataset: 'GLBX.MDP3',
  symbol: 'ESZ4',
  stypeIn: 'raw_symbol',
  resolution: '1m',
  from: 0 as UTCTimestamp,
  to: 60 as UTCTimestamp,
};

function fakeSubscription(): Subscription {
  return {
    id: 'sub-1',
    state: 'live' as ProviderState,
    unsubscribe: vi.fn().mockResolvedValue(undefined),
    dispose: vi.fn().mockResolvedValue(undefined),
  };
}

function fakeProvider(
  onOpen: (handlers: Parameters<DatabentoDataProvider['openBars']>[1]) => void,
): DatabentoDataProvider {
  return {
    getBars: vi.fn().mockResolvedValue(initialPage),
    openBars: vi.fn(async (_req, handlers) => {
      onOpen(handlers);
      const result: OpenBarsResult = { initial: initialPage, subscription: fakeSubscription() };
      return result;
    }),
    subscribeBars: vi.fn().mockResolvedValue(fakeSubscription()),
    resolveSymbol: vi.fn().mockResolvedValue([]),
    searchSymbols: vi.fn().mockResolvedValue([]),
    getDatasetMetadata: vi.fn().mockResolvedValue({ dataset: 'GLBX.MDP3', schemas: [], publishers: [] }),
    dispose: vi.fn().mockResolvedValue(undefined),
  };
}

describe('toBarFeed', () => {
  it('delegates getBars unchanged', async () => {
    const provider = fakeProvider(() => {});
    const feed = toBarFeed(provider);
    await expect(feed.getBars(request)).resolves.toBe(initialPage);
  });

  it('wraps provider onBar into a BarSink BarEvent without a historicalUpdate flag', async () => {
    const provider = fakeProvider((handlers) => {
      handlers.onBar(initialPage.bars[0]!, meta);
    });
    const feed = toBarFeed(provider);
    const onBar = vi.fn();
    await feed.openBars(request, { onBar });
    expect(onBar).toHaveBeenCalledWith({ bar: initialPage.bars[0], meta });
  });

  it('only forwards sink callbacks the caller actually supplied', async () => {
    let capturedHandlers: Parameters<DatabentoDataProvider['openBars']>[1] | undefined;
    const provider = fakeProvider((handlers) => {
      capturedHandlers = handlers;
    });
    const feed = toBarFeed(provider);
    await feed.openBars(request, { onBar: vi.fn() });
    expect(capturedHandlers?.onState).toBeUndefined();
    expect(capturedHandlers?.onError).toBeUndefined();
    expect(capturedHandlers?.onSymbolMapping).toBeUndefined();
  });
});

describe('bindSeries', () => {
  it('feeds the initial page to setData and each bar event to update', async () => {
    const provider = fakeProvider((handlers) => {
      handlers.onBar({ time: 60 as UTCTimestamp, open: 2, high: 3, low: 1.5, close: 2.5 }, meta);
    });
    const feed = toBarFeed(provider);
    const setData = vi.fn();
    const update = vi.fn();

    const subscription = await bindSeries({ setData, update }, feed, request);

    expect(setData).toHaveBeenCalledWith(initialPage.bars);
    expect(update).toHaveBeenCalledWith(
      { time: 60 as UTCTimestamp, open: 2, high: 3, low: 1.5, close: 2.5 },
      undefined,
    );
    expect(subscription.id).toBe('sub-1');
  });

  it('passes historicalUpdate through to series.update for revised bars', async () => {
    const provider: DatabentoDataProvider = {
      ...fakeProvider(() => {}),
      openBars: vi.fn(async (_req, handlers) => {
        handlers.onBar(initialPage.bars[0]!, meta);
        return { initial: initialPage, subscription: fakeSubscription() };
      }),
    };
    const feed: ReturnType<typeof toBarFeed> = {
      ...toBarFeed(provider),
      openBars: async (_req, sink) => {
        sink.onBar({ bar: initialPage.bars[0]!, meta, historicalUpdate: true });
        return { initial: initialPage, subscription: fakeSubscription() };
      },
    };
    const update = vi.fn();

    await bindSeries({ setData: vi.fn(), update }, feed, request);

    expect(update).toHaveBeenCalledWith(initialPage.bars[0], true);
  });

  it('forwards onState/onError/onSymbolMapping handlers when supplied', async () => {
    const provider = fakeProvider((handlers) => {
      handlers.onState?.('live' as ProviderState);
    });
    const feed = toBarFeed(provider);
    const onState = vi.fn();

    await bindSeries({ setData: vi.fn(), update: vi.fn() }, feed, request, { onState });

    expect(onState).toHaveBeenCalledWith('live');
  });
});
