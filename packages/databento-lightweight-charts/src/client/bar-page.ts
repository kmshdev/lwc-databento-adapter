import type { UTCTimestamp } from 'lightweight-charts';
import type { BarPage } from '../types/index.js';

interface WireBarMetadata {
  time: number;
  dataset: string;
  requestedSymbol: string;
  resolvedSymbol: string;
  instrumentId: number;
  sourceSchema: 'ohlcv-1s' | 'ohlcv-1m' | 'ohlcv-1h' | 'ohlcv-1d';
  synthetic: boolean;
}

interface WireBarPage {
  bars: unknown;
  volumes: unknown;
  metadata: readonly WireBarMetadata[];
}

export function barPageFromWire(value: WireBarPage): BarPage {
  const metadata = new Map<
    UTCTimestamp,
    BarPage['metadata'] extends ReadonlyMap<UTCTimestamp, infer M> ? M : never
  >();
  for (const item of value.metadata)
    metadata.set(item.time as UTCTimestamp, {
      dataset: item.dataset,
      requestedSymbol: item.requestedSymbol,
      resolvedSymbol: item.resolvedSymbol,
      instrumentId: item.instrumentId,
      sourceSchema: item.sourceSchema,
      synthetic: item.synthetic,
    });
  return {
    bars: value.bars as BarPage['bars'],
    volumes: value.volumes as BarPage['volumes'],
    metadata,
  };
}
