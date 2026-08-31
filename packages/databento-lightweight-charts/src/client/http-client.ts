import { DatabentoProviderError, protocolError } from '../errors/index.js';
import { barPageFromWire } from './bar-page.js';
import {
  barPageResponseSchema,
  datasetResponseSchema,
  errorResponseSchema,
  resolveResponseSchema,
  searchResponseSchema,
} from './schemas.js';
import type {
  BarPage,
  DatasetMetadata,
  HistoryRequest,
  ResolveSymbolRequest,
  ResolvedSymbol,
  SearchSymbolsRequest,
  SymbolSearchResult,
} from '../types/index.js';

const requestId = (): string =>
  globalThis.crypto?.randomUUID?.() ?? `req-${Date.now()}-${Math.random()}`;

export class HttpClient {
  public constructor(private readonly baseUrl: URL) {}

  public async getBars(
    request: Omit<HistoryRequest, 'signal'>,
    signal?: AbortSignal,
  ): Promise<BarPage> {
    const value = await this.request(
      '/v1/history/bars',
      withSignal({ method: 'POST', body: JSON.stringify({ v: 1, ...request }) }, signal),
      barPageResponseSchema,
    );
    return barPageFromWire(value);
  }

  public async resolveSymbols(
    request: Omit<ResolveSymbolRequest, 'signal'>,
    signal?: AbortSignal,
  ): Promise<ResolvedSymbol[]> {
    const value = await this.request(
      '/v1/symbols/resolve',
      withSignal({ method: 'POST', body: JSON.stringify({ v: 1, ...request }) }, signal),
      resolveResponseSchema,
    );
    return value.mappings as ResolvedSymbol[];
  }

  public async searchSymbols(
    request: Omit<SearchSymbolsRequest, 'signal'>,
    signal?: AbortSignal,
  ): Promise<SymbolSearchResult[]> {
    const value = await this.request(
      '/v1/symbols/search',
      withSignal({ method: 'POST', body: JSON.stringify({ v: 1, ...request }) }, signal),
      searchResponseSchema,
    );
    return value.results as SymbolSearchResult[];
  }

  public async getDatasetMetadata(dataset: string, signal?: AbortSignal): Promise<DatasetMetadata> {
    const value = await this.request(
      `/v1/datasets/${encodeURIComponent(dataset)}`,
      withSignal({ method: 'GET' }, signal),
      datasetResponseSchema,
    );
    return value.metadata as unknown as DatasetMetadata;
  }

  private async request<T extends { requestId: string }>(
    path: string,
    init: RequestInit,
    schema: { safeParse(value: unknown): { success: boolean; data?: T } },
  ): Promise<T> {
    const response = await fetch(new URL(path, this.baseUrl), {
      ...init,
      headers: { 'content-type': 'application/json', 'x-request-id': requestId() },
    }).catch((cause: unknown) => {
      if (init.signal?.aborted)
        throw new DatabentoProviderError('cancelled', 'Operation was cancelled', false);
      throw new DatabentoProviderError(
        'upstream_unavailable',
        'Gateway request failed',
        true,
        undefined,
        undefined,
        { cause: String(cause) },
      );
    });
    const body: unknown = await response.json().catch(() => undefined);
    if (!response.ok) {
      const parsed = errorResponseSchema.safeParse(body);
      if (parsed.success) {
        const error = parsed.data.error;
        throw new DatabentoProviderError(
          error.code,
          error.message,
          error.retryable,
          parsed.data.requestId,
          undefined,
          error.details,
        );
      }
      throw new DatabentoProviderError(
        'protocol_error',
        'Gateway returned an invalid error response',
        false,
      );
    }
    const parsed = schema.safeParse(body);
    if (!parsed.success || parsed.data === undefined)
      throw protocolError('Gateway returned an invalid response');
    return parsed.data;
  }
}

const withSignal = (init: RequestInit, signal: AbortSignal | undefined): RequestInit =>
  signal === undefined ? init : { ...init, signal };
