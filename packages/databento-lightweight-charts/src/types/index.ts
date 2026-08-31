import type {
  CandlestickData,
  HistogramData,
  UTCTimestamp,
  WhitespaceData,
} from 'lightweight-charts';

export type SymbolType = 'raw_symbol' | 'instrument_id' | 'parent' | 'continuous';
export type Resolution =
  | '1s'
  | '5s'
  | '15s'
  | '30s'
  | '1m'
  | '5m'
  | '15m'
  | '30m'
  | '1h'
  | '2h'
  | '4h'
  | '1d';
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
  | 'initial_connect'
  | 'handoff_replay'
  | 'replay_completed'
  | 'upstream_disconnect'
  | 'downstream_disconnect'
  | 'retry_scheduled'
  | 'retry_exhausted'
  | 'client_unsubscribe'
  | 'server_shutdown'
  | 'slow_consumer';

export type ProviderErrorCode =
  | 'invalid_request'
  | 'invalid_range'
  | 'range_too_large'
  | 'origin_forbidden'
  | 'dataset_forbidden'
  | 'unsupported_dataset'
  | 'unsupported_schema'
  | 'unsupported_resolution'
  | 'symbol_not_found'
  | 'symbol_mapping_failed'
  | 'unsupported_parent_series'
  | 'unsupported_live_symbology'
  | 'resolved_instrument_changed'
  | 'access_denied'
  | 'quota_exceeded'
  | 'slow_consumer'
  | 'replay_unavailable'
  | 'upstream_unavailable'
  | 'cancelled'
  | 'protocol_error'
  | 'internal';

export type ChartBar = CandlestickData<UTCTimestamp> | WhitespaceData<UTCTimestamp>;

export interface BarPage {
  bars: ChartBar[];
  volumes: HistogramData<UTCTimestamp>[];
  metadata: ReadonlyMap<UTCTimestamp, BarMetadata>;
}

export type ProviderState =
  | 'idle'
  | 'connecting'
  | 'replaying'
  | 'live'
  | 'reconnecting'
  | 'failed'
  | 'closed';

export interface BarHandlers {
  onBar(bar: ChartBar, meta: BarMetadata): void;
  onVolume?(volume: HistogramData<UTCTimestamp>, meta: BarMetadata): void;
  onState?(state: ProviderState): void;
  onError?(error: import('../errors/index.js').DatabentoProviderError): void;
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
