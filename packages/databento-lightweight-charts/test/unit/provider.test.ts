import { describe, expect, it, vi } from 'vitest';
import { createDatabentoDataProvider } from '../../src/index.js';

const config = {
  gatewayUrl: 'http://127.0.0.1:8080',
  historyChunkIntervals: 2,
  reconnect: { baseDelayMs: 1, maxDelayMs: 2, maxAttempts: 2, jitterRatio: 0 },
};

describe('Databento data provider', () => {
  it('forgets an aborted subscription when sending cancel fails', async () => {
    let sendCount = 0;
    class FailingCancelSocket {
      public static readonly OPEN = 1;
      public readyState = FailingCancelSocket.OPEN;

      public send(): void {
        sendCount += 1;
        if (sendCount > 1) throw new Error('cancel send failed');
      }

      public close(): void {
        this.readyState = 3;
      }

      public addEventListener(type: string, listener: (event: Event) => void): void {
        if (type === 'open') queueMicrotask(() => listener(new Event('open')));
      }
    }
    vi.stubGlobal('WebSocket', FailingCancelSocket);
    const provider = createDatabentoDataProvider(config);
    const controller = new AbortController();
    const opening = provider.openBars(
      {
        dataset: 'GLBX.MDP3',
        symbol: 'ESZ4',
        stypeIn: 'raw_symbol',
        resolution: '1m',
        from: 0,
        to: 60,
        signal: controller.signal,
      } as never,
      { onBar: vi.fn() },
    );
    await vi.waitFor(() => expect(sendCount).toBe(1));

    controller.abort();

    await expect(opening).rejects.toMatchObject({ code: 'cancelled' });
    await vi.waitFor(() =>
      expect(
        (provider as unknown as { subscriptions: Map<string, unknown> }).subscriptions.size,
      ).toBe(0),
    );
    await provider.dispose();
  });

  it('rejects parent bar series before transport', async () => {
    const provider = createDatabentoDataProvider(config);
    await expect(
      provider.getBars({
        dataset: 'GLBX.MDP3',
        symbol: 'ES.FUT',
        stypeIn: 'parent',
        resolution: '1m',
        from: 10,
        to: 20,
      } as never),
    ).rejects.toMatchObject({ code: 'unsupported_parent_series' });
    await provider.dispose();
  });

  it('splits adjacent history intervals sequentially and deduplicates boundaries', async () => {
    const fetchMock = vi.fn(async (_input: RequestInfo | URL, init?: RequestInit) => {
      const body = JSON.parse(String(init?.body)) as { from: number; to: number };
      return new Response(
        JSON.stringify({
          v: 1,
          requestId: `req-${body.from}`,
          bars: [{ time: body.from, open: 1, high: 1, low: 1, close: 1 }],
          volumes: [{ time: body.from, value: 1 }],
          metadata: [
            {
              time: body.from,
              dataset: 'GLBX.MDP3',
              requestedSymbol: 'ESZ4',
              resolvedSymbol: 'ESZ4',
              instrumentId: 1,
              sourceSchema: 'ohlcv-1m',
              synthetic: false,
            },
          ],
        }),
        { status: 200, headers: { 'content-type': 'application/json' } },
      );
    });
    vi.stubGlobal('fetch', fetchMock);
    const provider = createDatabentoDataProvider(config);
    const page = await provider.getBars({
      dataset: 'GLBX.MDP3',
      symbol: 'ESZ4',
      stypeIn: 'raw_symbol',
      resolution: '1m',
      from: 0,
      to: 300,
    } as never);
    expect(fetchMock).toHaveBeenCalledTimes(3);
    expect(page.bars.map((bar) => bar.time)).toEqual([0, 120, 240]);
    await provider.dispose();
  });

  it('rejects invalid configuration', () => {
    expect(() => createDatabentoDataProvider({ ...config, historyChunkIntervals: 0 })).toThrow(
      'Provider configuration is invalid',
    );
  });
});
