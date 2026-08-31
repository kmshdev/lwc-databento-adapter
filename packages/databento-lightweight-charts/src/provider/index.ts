import type { UTCTimestamp } from 'lightweight-charts';
import { cancelledError, DatabentoProviderError, protocolError } from '../errors/index.js';
import { HttpClient } from '../client/http-client.js';
import { barPageFromWire } from '../client/bar-page.js';
import { LiveSocket, reconnectDelay } from '../client/live-socket.js';
import type { ClientCommand, ServerEvent } from '../client/schemas.js';
import { ManagedSubscription, type LiveRequest } from '../subscriptions/subscription.js';
import type {
  BarHandlers,
  BarPage,
  BarRequest,
  DatabentoDataProvider,
  HistoryRequest,
  OpenBarsResult,
  ProviderConfig,
  ProviderState,
  ResolveSymbolRequest,
  ResolvedSymbol,
  SearchSymbolsRequest,
  Subscription,
  SymbolSearchResult,
  DatasetMetadata,
} from '../types/index.js';

interface PendingCommand {
  subscription: ManagedSubscription;
  resolveSubscription?: (subscription: Subscription) => void;
  resolveOpen?: (result: OpenBarsResult) => void;
  reject: (error: DatabentoProviderError) => void;
  command: ClientCommand;
  acknowledged: boolean;
}

const resolutionSeconds: Record<BarRequest['resolution'], number> = {
  '1s': 1,
  '5s': 5,
  '15s': 15,
  '30s': 30,
  '1m': 60,
  '5m': 300,
  '15m': 900,
  '30m': 1800,
  '1h': 3600,
  '2h': 7200,
  '4h': 14400,
  '1d': 86400,
};

const identifier = (prefix: string): string =>
  `${prefix}-${globalThis.crypto?.randomUUID?.() ?? `${Date.now()}-${Math.random().toString(36).slice(2)}`}`;

const assertConfig = (config: ProviderConfig): void => {
  const numeric = [
    config.historyChunkIntervals,
    config.reconnect.baseDelayMs,
    config.reconnect.maxDelayMs,
    config.reconnect.maxAttempts,
  ];
  if (
    !config.gatewayUrl ||
    numeric.some((value) => !Number.isFinite(value) || value <= 0) ||
    !Number.isInteger(config.historyChunkIntervals) ||
    !Number.isInteger(config.reconnect.maxAttempts) ||
    config.reconnect.maxDelayMs < config.reconnect.baseDelayMs ||
    !Number.isFinite(config.reconnect.jitterRatio) ||
    config.reconnect.jitterRatio < 0 ||
    config.reconnect.jitterRatio > 1
  ) {
    throw new DatabentoProviderError('invalid_request', 'Provider configuration is invalid', false);
  }
};

class Provider implements DatabentoDataProvider {
  private readonly url: URL;
  private readonly http: HttpClient;
  private readonly live: LiveSocket;
  private readonly subscriptions = new Map<string, ManagedSubscription>();
  private readonly pending = new Map<string, PendingCommand>();
  private disposed = false;
  private reconnectAttempt = 0;
  private reconnectTimer?: ReturnType<typeof setTimeout>;

  public constructor(private readonly config: ProviderConfig) {
    assertConfig(config);
    this.url = new URL(config.gatewayUrl);
    this.http = new HttpClient(this.url);
    this.live = new LiveSocket(
      this.url,
      (event) => this.handleEvent(event),
      (unexpected) => this.handleClose(unexpected),
      (error) => this.handleFault(error),
    );
  }

  public async getBars(request: HistoryRequest): Promise<BarPage> {
    this.assertOpen();
    if (request.stypeIn === 'parent')
      throw new DatabentoProviderError(
        'unsupported_parent_series',
        'Select a resolved child instrument before requesting bars',
        false,
      );
    const parsed = this.validateHistory(request);
    const interval = resolutionSeconds[parsed.resolution];
    const chunkSeconds = interval * this.config.historyChunkIntervals;
    const pages: BarPage[] = [];
    for (
      let from: number = parsed.from as number;
      from < (parsed.to as number);
      from += chunkSeconds
    ) {
      if (request.signal?.aborted) throw cancelledError();
      const to = Math.min(parsed.to, from + chunkSeconds);
      pages.push(
        await this.http.getBars(
          { ...parsed, from: from as UTCTimestamp, to: to as UTCTimestamp },
          request.signal,
        ),
      );
    }
    return mergePages(pages);
  }

  public async openBars(request: HistoryRequest, handlers: BarHandlers): Promise<OpenBarsResult> {
    this.assertOpen();
    if (request.stypeIn === 'parent')
      throw new DatabentoProviderError(
        'unsupported_parent_series',
        'Select a resolved child instrument before opening bars',
        false,
      );
    const parsed = this.validateHistory(request);
    if (request.signal?.aborted) throw cancelledError();
    const subscription = this.newSubscription(parsed, 'open_bars', handlers);
    const commandId = identifier('cmd');
    const command: ClientCommand = {
      v: 1,
      type: 'open_bars',
      commandId,
      subscriptionId: subscription.id,
      request: parsed,
    };
    const result = new Promise<OpenBarsResult>((resolve, reject) =>
      this.pending.set(commandId, {
        subscription,
        resolveOpen: resolve,
        reject,
        command,
        acknowledged: false,
      }),
    );
    this.bindAbort(request.signal, commandId, subscription);
    await this.sendPending(commandId);
    return result;
  }

  public async subscribeBars(request: BarRequest, handlers: BarHandlers): Promise<Subscription> {
    this.assertOpen();
    if (request.stypeIn === 'parent')
      throw new DatabentoProviderError(
        'unsupported_parent_series',
        'Select a resolved child instrument before subscribing',
        false,
      );
    const parsed = this.validateBar(request);
    const subscription = this.newSubscription(parsed, 'subscribe_bars', handlers);
    const commandId = identifier('cmd');
    const command: ClientCommand = {
      v: 1,
      type: 'subscribe_bars',
      commandId,
      subscriptionId: subscription.id,
      request: parsed,
    };
    const result = new Promise<Subscription>((resolve, reject) =>
      this.pending.set(commandId, {
        subscription,
        resolveSubscription: resolve,
        reject,
        command,
        acknowledged: false,
      }),
    );
    await this.sendPending(commandId);
    return result;
  }

  public resolveSymbol(request: ResolveSymbolRequest): Promise<ResolvedSymbol[]> {
    this.assertOpen();
    return this.http.resolveSymbols(
      {
        dataset: request.dataset,
        symbols: request.symbols,
        stypeIn: request.stypeIn,
        from: request.from,
        to: request.to,
      },
      request.signal,
    );
  }

  public searchSymbols(request: SearchSymbolsRequest): Promise<SymbolSearchResult[]> {
    this.assertOpen();
    return this.http.searchSymbols(
      { dataset: request.dataset, query: request.query, stypeIn: request.stypeIn },
      request.signal,
    );
  }

  public getDatasetMetadata(dataset: string, signal?: AbortSignal): Promise<DatasetMetadata> {
    this.assertOpen();
    return this.http.getDatasetMetadata(dataset, signal);
  }

  public async dispose(): Promise<void> {
    if (this.disposed) return;
    this.disposed = true;
    if (this.reconnectTimer) clearTimeout(this.reconnectTimer);
    const active = [...this.subscriptions.values()].map((subscription) =>
      this.unsubscribe(subscription),
    );
    await Promise.allSettled(active);
    this.live.close();
    for (const pending of this.pending.values())
      pending.reject(protocolError('Provider was disposed', pending.subscription.id));
    this.pending.clear();
  }

  private newSubscription(
    request: LiveRequest,
    mode: 'subscribe_bars' | 'open_bars',
    handlers: BarHandlers,
  ): ManagedSubscription {
    const subscription = new ManagedSubscription(
      identifier('sub'),
      request,
      mode,
      handlers,
      (item) => this.unsubscribe(item),
    );
    this.subscriptions.set(subscription.id, subscription);
    return subscription;
  }

  private async unsubscribe(subscription: ManagedSubscription): Promise<void> {
    if (!subscription.isActive) return;
    const commandId = identifier('cmd');
    const command: ClientCommand = {
      v: 1,
      type: 'unsubscribe',
      commandId,
      subscriptionId: subscription.id,
    };
    const done = new Promise<void>((resolve, reject) =>
      this.pending.set(commandId, {
        subscription,
        reject,
        command,
        acknowledged: true,
        resolveSubscription: () => resolve(),
      }),
    );
    try {
      await this.sendPending(commandId);
    } catch (error) {
      this.pending.delete(commandId);
      subscription.finishClosed();
      this.subscriptions.delete(subscription.id);
      if (error instanceof DatabentoProviderError && error.code === 'protocol_error') return;
      throw error;
    }
    return done;
  }

  private async sendPending(commandId: string): Promise<void> {
    const pending = this.pending.get(commandId);
    if (!pending) return;
    try {
      await this.live.send(pending.command);
    } catch (cause) {
      this.handleSendFailure(commandId, cause);
    }
  }

  private handleEvent(event: ServerEvent): void {
    if (event.type === 'heartbeat') return;
    if (event.type === 'subscribed') {
      const pending = this.pending.get(event.commandId);
      const subscription = this.subscriptions.get(event.subscriptionId);
      if (!pending || !subscription || pending.subscription !== subscription)
        return this.handleFault(protocolError('Unexpected subscribed event', event.subscriptionId));
      pending.acknowledged = true;
      subscription.setState(event.state);
      for (const mapping of event.resolvedSymbols)
        subscription.emitMapping(() =>
          subscription.handlers.onSymbolMapping?.(mapping as unknown as ResolvedSymbol),
        );
      if (pending.resolveSubscription) {
        pending.resolveSubscription(subscription);
        this.pending.delete(event.commandId);
      }
      return;
    }
    const subscription =
      'subscriptionId' in event && event.subscriptionId !== undefined
        ? this.subscriptions.get(event.subscriptionId)
        : undefined;
    if (event.type === 'snapshot') {
      const pending = [...this.pending.values()].find(
        (item) =>
          item.subscription.id === event.subscriptionId && item.command.type === 'open_bars',
      );
      if (!pending || !pending.acknowledged || !pending.resolveOpen)
        return this.handleFault(protocolError('Unexpected snapshot event', event.subscriptionId));
      const page = pageFromSnapshot(event);
      pending.resolveOpen({ initial: page, subscription: pending.subscription });
      this.pending.delete(pending.command.commandId);
      return;
    }
    if (!subscription) return;
    if (event.type === 'bar') {
      subscription.emitBar(event.data.time, () => {
        subscription.handlers.onBar(event.data as never, event.meta as never);
        if (event.volume)
          subscription.handlers.onVolume?.(event.volume as never, event.meta as never);
      });
      return;
    }
    if (event.type === 'status') {
      subscription.setState(event.state as ProviderState);
      return;
    }
    if (event.type === 'symbol_mapping') {
      subscription.emitMapping(() =>
        subscription.handlers.onSymbolMapping?.({
          dataset: subscription.request.dataset,
          requestedSymbol: event.requestedSymbol,
          resolvedSymbol: event.resolvedSymbol,
          instrumentId: event.instrumentId,
          effectiveFrom: event.effectiveFrom as UTCTimestamp,
        }),
      );
      return;
    }
    if (event.type === 'unsubscribed' || event.type === 'cancelled') {
      subscription.finishClosed();
      this.subscriptions.delete(subscription.id);
      const pending = this.pending.get(event.commandId);
      pending?.resolveSubscription?.(subscription);
      this.pending.delete(event.commandId);
      return;
    }
    if (event.type === 'error') {
      const error = new DatabentoProviderError(
        event.error.code,
        event.error.message,
        event.error.retryable,
        undefined,
        event.subscriptionId,
        event.error.details,
      );
      subscription.fail(error);
      this.subscriptions.delete(subscription.id);
      if (event.commandId) {
        const pending = this.pending.get(event.commandId);
        pending?.reject(error);
        this.pending.delete(event.commandId);
      }
    }
  }

  private handleFault(error: Error): void {
    for (const subscription of this.subscriptions.values())
      subscription.fail(
        error instanceof DatabentoProviderError ? error : protocolError(error.message),
      );
    this.subscriptions.clear();
    for (const pending of this.pending.values())
      pending.reject(
        error instanceof DatabentoProviderError ? error : protocolError(error.message),
      );
    this.pending.clear();
  }

  private handleClose(unexpected: boolean): void {
    if (!unexpected || this.disposed || this.subscriptions.size === 0) return;
    this.reconnectAttempt += 1;
    if (this.reconnectAttempt > this.config.reconnect.maxAttempts) {
      this.handleFault(
        new DatabentoProviderError(
          'upstream_unavailable',
          'WebSocket reconnect budget exhausted',
          true,
        ),
      );
      return;
    }
    for (const subscription of this.subscriptions.values()) subscription.setState('reconnecting');
    this.reconnectTimer = setTimeout(
      () => {
        void this.resumeAll();
      },
      reconnectDelay(this.config.reconnect, this.reconnectAttempt),
    );
  }

  private async resumeAll(): Promise<void> {
    if (this.disposed) return;
    try {
      await this.live.connect();
      this.reconnectAttempt = 0;
      for (const subscription of this.subscriptions.values()) {
        const existing = [...this.pending.entries()].find(
          ([, pending]) => pending.subscription === subscription,
        );
        if (existing !== undefined) {
          await this.sendPending(existing[0]);
          continue;
        }
        const commandId = identifier('cmd');
        const command: ClientCommand =
          subscription.mode === 'open_bars' && subscription.latestTime === undefined
            ? {
                v: 1,
                type: 'open_bars',
                commandId,
                subscriptionId: subscription.id,
                request: subscription.request as HistoryRequest,
              }
            : {
                v: 1,
                type: 'resume_bars',
                commandId,
                subscriptionId: subscription.id,
                resumeFrom: subscription.latestTime ?? 0,
                request: stripHistory(subscription.request),
              };
        this.pending.set(commandId, {
          subscription,
          reject: (error) => subscription.fail(error),
          command,
          acknowledged: false,
        });
        await this.sendPending(commandId);
      }
    } catch {
      this.handleClose(true);
    }
  }

  private handleSendFailure(commandId: string, cause: unknown): void {
    const pending = this.pending.get(commandId);
    if (!pending) return;
    const error =
      cause instanceof DatabentoProviderError
        ? cause
        : new DatabentoProviderError('upstream_unavailable', 'WebSocket send failed', true);
    if (pending.command.type === 'open_bars' || pending.command.type === 'subscribe_bars') {
      this.handleClose(true);
      return;
    }
    pending.reject(error);
    this.pending.delete(commandId);
  }

  private bindAbort(
    signal: AbortSignal | undefined,
    commandId: string,
    subscription: ManagedSubscription,
  ): void {
    signal?.addEventListener(
      'abort',
      () => {
        if (!subscription.isActive) return;
        const cancelId = identifier('cmd');
        const command: ClientCommand = {
          v: 1,
          type: 'cancel',
          commandId: cancelId,
          targetCommandId: commandId,
          subscriptionId: subscription.id,
        };
        const pending = this.pending.get(commandId);
        pending?.reject(cancelledError(subscription.id));
        this.pending.delete(commandId);
        void this.live.send(command).catch(() => {
          this.subscriptions.delete(subscription.id);
          subscription.finishClosed();
        });
      },
      { once: true },
    );
  }

  private validateBar(request: BarRequest): BarRequest {
    if (!request.dataset || !request.symbol)
      throw new DatabentoProviderError('invalid_request', 'dataset and symbol are required', false);
    return { ...request };
  }

  private validateHistory(request: HistoryRequest): Omit<HistoryRequest, 'signal'> {
    this.validateBar(request);
    if (
      !Number.isSafeInteger(request.from) ||
      !Number.isSafeInteger(request.to) ||
      request.from >= request.to
    ) {
      throw new DatabentoProviderError(
        'invalid_range',
        'from and to must be ordered integer seconds',
        false,
      );
    }
    const { signal: _signal, ...value } = request;
    return value;
  }

  private assertOpen(): void {
    if (this.disposed)
      throw new DatabentoProviderError('protocol_error', 'Provider is disposed', false);
  }
}

const stripHistory = (request: LiveRequest): BarRequest => {
  const { dataset, symbol, stypeIn, resolution, gapPolicy } = request;
  return gapPolicy === undefined
    ? { dataset, symbol, stypeIn, resolution }
    : { dataset, symbol, stypeIn, resolution, gapPolicy };
};

const pageFromSnapshot = (event: Extract<ServerEvent, { type: 'snapshot' }>): BarPage => {
  return barPageFromWire(event);
};

const mergePages = (pages: readonly BarPage[]): BarPage => {
  const bars = new Map<number, BarPage['bars'][number]>();
  const volumes = new Map<number, BarPage['volumes'][number]>();
  const metadata = new Map<
    UTCTimestamp,
    BarPage['metadata'] extends ReadonlyMap<UTCTimestamp, infer M> ? M : never
  >();
  for (const page of pages) {
    for (const bar of page.bars) bars.set(bar.time as number, bar);
    for (const volume of page.volumes) volumes.set(volume.time as number, volume);
    for (const [time, meta] of page.metadata) metadata.set(time, meta);
  }
  return {
    bars: [...bars.values()].toSorted(
      (left, right) => (left.time as number) - (right.time as number),
    ),
    volumes: [...volumes.values()].toSorted(
      (left, right) => (left.time as number) - (right.time as number),
    ),
    metadata,
  };
};

export const createDatabentoDataProvider = (config: ProviderConfig): DatabentoDataProvider =>
  new Provider(config);
