import {
  CandlestickSeries,
  ColorType,
  createChart,
  CrosshairMode,
  HistogramSeries,
  type CandlestickData,
  type HistogramData,
  type IChartApi,
  type ISeriesApi,
  type LogicalRange,
  type UTCTimestamp,
} from 'lightweight-charts';
import {
  createDatabentoDataProvider,
  type BarMetadata,
  type BarPage,
  type ChartBar,
  type DatabentoDataProvider,
  type DatabentoProviderError,
  type OpenBarsResult,
  type ProviderState,
  type ResolvedSymbol,
  type Resolution,
  type Subscription,
  type SymbolType,
} from '@lwc-databento/adapter';

const PAGE_THRESHOLD = 30;
const PAGE_INTERVALS = 300;
const LIVE_LOOKBACK_INTERVALS = 500;

type Candle = CandlestickData<UTCTimestamp>;
type SeriesBar = ChartBar;
type Volume = HistogramData<UTCTimestamp>;

interface FormValues {
  dataset: string;
  symbol: string;
  stypeIn: SymbolType;
  resolution: Resolution;
  from: UTCTimestamp;
  to: UTCTimestamp;
}

function element<T extends HTMLElement>(document: Document, id: string): T {
  const value = document.getElementById(id);
  if (!(value instanceof HTMLElement)) throw new Error(`Missing #${id}`);
  return value as T;
}

function utcSeconds(input: string): UTCTimestamp {
  const milliseconds = Date.parse(input);
  if (!Number.isFinite(milliseconds)) throw new Error('Enter a valid UTC date and time.');
  return Math.floor(milliseconds / 1000) as UTCTimestamp;
}

function toLocalInput(seconds: number): string {
  const date = new Date(seconds * 1000);
  const offset = date.getTimezoneOffset() * 60_000;
  return new Date(date.getTime() - offset).toISOString().slice(0, 16);
}

function resolutionSeconds(resolution: Resolution): number {
  const lookup: Readonly<Record<Resolution, number>> = {
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
    '4h': 14_400,
    '1d': 86_400,
  };
  return lookup[resolution] ?? 1;
}

function barTime(bar: SeriesBar): UTCTimestamp | undefined {
  return typeof bar.time === 'number' ? (bar.time as UTCTimestamp) : undefined;
}

function isCandle(bar: SeriesBar): bar is Candle {
  return 'open' in bar;
}

function mergeByTime<T extends { time: UTCTimestamp }>(
  current: readonly T[],
  incoming: readonly T[],
): T[] {
  const values = new Map<UTCTimestamp, T>();
  for (const point of current) values.set(point.time, point);
  for (const point of incoming) values.set(point.time, point);
  return [...values.values()].toSorted((left, right) => Number(left.time) - Number(right.time));
}

export class DemoApp {
  private readonly form: HTMLFormElement;
  private readonly dataset: HTMLInputElement;
  private readonly symbol: HTMLInputElement;
  private readonly stype: HTMLSelectElement;
  private readonly resolution: HTMLSelectElement;
  private readonly from: HTMLInputElement;
  private readonly to: HTMLInputElement;
  private readonly goLive: HTMLButtonElement;
  private readonly volumeToggle: HTMLButtonElement;
  private readonly parentPicker: HTMLElement;
  private readonly childSymbol: HTMLSelectElement;
  private readonly error: HTMLElement;
  private readonly notice: HTMLElement;
  private readonly chartContainer: HTMLElement;
  private readonly legend: HTMLElement;
  private readonly tooltip: HTMLElement;
  private readonly provider: DatabentoDataProvider;
  private chart?: IChartApi;
  private candles?: ISeriesApi<'Candlestick'>;
  private volume?: ISeriesApi<'Histogram'>;
  private subscription?: Subscription;
  private resizeObserver?: ResizeObserver;
  private abort?: AbortController;
  private bars: SeriesBar[] = [];
  private volumes: Volume[] = [];
  private metadata = new Map<UTCTimestamp, BarMetadata>();
  private pageLoading = false;
  private historyExhausted = false;
  private activeRequest?: FormValues;
  private selectedChildren: ResolvedSymbol[] = [];
  private viewRevision = 0;
  private requestRevision = 0;
  private disconnected = false;

  public constructor(private readonly document: Document) {
    this.form = element(document, 'history-form');
    this.dataset = element(document, 'dataset');
    this.symbol = element(document, 'symbol');
    this.stype = element(document, 'stype');
    this.resolution = element(document, 'resolution');
    this.from = element(document, 'from');
    this.to = element(document, 'to');
    this.goLive = element(document, 'go-live');
    this.volumeToggle = element(document, 'toggle-volume');
    this.parentPicker = element(document, 'parent-picker');
    this.childSymbol = element(document, 'child-symbol');
    this.error = element(document, 'error');
    this.notice = element(document, 'notice');
    this.chartContainer = element(document, 'chart');
    this.legend = element(document, 'legend');
    this.tooltip = element(document, 'tooltip');
    this.provider = createDatabentoDataProvider({
      gatewayUrl: import.meta.env.VITE_GATEWAY_URL ?? 'http://127.0.0.1:8080',
      historyChunkIntervals: 500,
      reconnect: { baseDelayMs: 250, maxDelayMs: 4_000, maxAttempts: 5, jitterRatio: 0 },
    });
  }

  public mount(): void {
    const now = Math.floor(Date.now() / 1000);
    this.from.value = toLocalInput(now - 86_400 * 3);
    this.to.value = toLocalInput(now);
    this.createChart();
    this.form.addEventListener('submit', this.onHistorySubmit);
    this.goLive.addEventListener('click', this.onGoLive);
    element<HTMLButtonElement>(this.document, 'disconnect').addEventListener('click', () => {
      void this.disconnect();
    });
    element<HTMLButtonElement>(this.document, 'fit-content').addEventListener('click', () =>
      this.chart?.timeScale().fitContent(),
    );
    this.volumeToggle.addEventListener('click', () => this.toggleVolumePane());
    element<HTMLButtonElement>(this.document, 'use-child').addEventListener('click', () => {
      void this.loadSelectedChild();
    });
  }

  public async dispose(): Promise<void> {
    this.abort?.abort();
    await this.subscription?.dispose();
    this.subscription = undefined;
    this.resizeObserver?.disconnect();
    window.removeEventListener('resize', this.resizeChart);
    this.chart?.remove();
    this.chart = undefined;
    await this.provider.dispose();
  }

  private createChart(): void {
    this.chart = createChart(this.chartContainer, {
      width: this.chartContainer.clientWidth,
      height: this.chartContainer.clientHeight,
      layout: { background: { type: ColorType.Solid, color: '#151d27' }, textColor: '#d8e2ee' },
      grid: { vertLines: { color: '#202c3a' }, horzLines: { color: '#202c3a' } },
      crosshair: { mode: CrosshairMode.Normal },
      timeScale: { timeVisible: true, secondsVisible: false },
    });
    this.candles = this.chart.addSeries(
      CandlestickSeries,
      {
        upColor: '#3fb950',
        downColor: '#f85149',
        borderVisible: false,
        wickUpColor: '#3fb950',
        wickDownColor: '#f85149',
      },
      0,
    );
    const pane = this.chart.addPane(true);
    pane.setHeight(150);
    this.volume = this.chart.addSeries(
      HistogramSeries,
      { priceFormat: { type: 'volume' }, priceScaleId: '', lastValueVisible: false },
      1,
    );
    this.chart.subscribeCrosshairMove((event) => this.renderCrosshair(event));
    this.chart.timeScale().subscribeVisibleLogicalRangeChange((range) => {
      void this.loadOlderIfNeeded(range);
    });
    if (typeof ResizeObserver !== 'undefined') {
      this.resizeObserver = new ResizeObserver(() => this.resizeChart());
      this.resizeObserver.observe(this.chartContainer);
    } else {
      window.addEventListener('resize', this.resizeChart);
      this.setNotice('ResizeObserver is unavailable; using the window resize fallback.');
    }
  }

  private readonly resizeChart = (): void => {
    if (this.chartContainer.clientWidth > 0 && this.chartContainer.clientHeight > 0) {
      this.chart?.applyOptions({
        width: this.chartContainer.clientWidth,
        height: this.chartContainer.clientHeight,
      });
    }
  };

  private readonly onHistorySubmit = (event: SubmitEvent): void => {
    event.preventDefault();
    void this.loadHistory();
  };

  private readonly onGoLive = (): void => {
    void this.openLive();
  };

  private values(): FormValues {
    return {
      dataset: this.dataset.value.trim(),
      symbol: this.symbol.value.trim(),
      stypeIn: this.stype.value as SymbolType,
      resolution: this.resolution.value as Resolution,
      from: utcSeconds(this.from.value),
      to: utcSeconds(this.to.value),
    };
  }

  private async loadHistory(): Promise<void> {
    const revision = ++this.requestRevision;
    try {
      const values = this.values();
      this.disconnected = false;
      this.viewRevision += 1;
      if (values.stypeIn === 'parent') {
        await this.resolveParent(values, revision);
        return;
      }
      this.setBusy(true, 'Loading historical bars…');
      await this.closeSubscription();
      if (!this.isCurrentRequest(revision)) return;
      this.abort?.abort();
      const controller = new AbortController();
      this.abort = controller;
      const page = await this.provider.getBars({ ...values, signal: controller.signal });
      if (!this.isCurrentRequest(revision) || controller.signal.aborted) return;
      this.activeRequest = values;
      this.replaceData(page);
      this.historyExhausted = page.bars.length === 0;
      this.setNotice(
        page.bars.length === 0 ? 'No historical bars found.' : `Loaded ${page.bars.length} bars.`,
      );
    } catch (caught) {
      if (this.isCurrentRequest(revision)) this.showError(caught);
    } finally {
      if (revision === this.requestRevision) this.setBusy(false);
    }
  }

  private async resolveParent(values: FormValues, revision: number): Promise<void> {
    this.setBusy(true, 'Resolving parent symbol…');
    try {
      const children = await this.provider.resolveSymbol({
        dataset: values.dataset,
        symbols: [values.symbol],
        stypeIn: 'parent',
        from: values.from,
        to: values.to,
      });
      if (!this.isCurrentRequest(revision)) return;
      this.selectedChildren = children;
      this.childSymbol.replaceChildren(
        ...this.selectedChildren.map((child) => {
          const option = this.document.createElement('option');
          option.value = String(child.instrumentId);
          option.textContent = `${child.resolvedSymbol} (${child.instrumentId})`;
          return option;
        }),
      );
      this.parentPicker.hidden = this.selectedChildren.length === 0;
      this.setNotice(
        this.selectedChildren.length === 0
          ? 'No child instruments were resolved.'
          : 'Choose one resolved instrument before loading bars.',
      );
    } catch (caught) {
      if (this.isCurrentRequest(revision)) this.showError(caught);
    } finally {
      if (revision === this.requestRevision) this.setBusy(false);
    }
  }

  private async loadSelectedChild(): Promise<void> {
    const child = this.selectedChildren.find(
      (candidate) => String(candidate.instrumentId) === this.childSymbol.value,
    );
    if (child === undefined) return;
    this.symbol.value = String(child.instrumentId);
    this.stype.value = 'instrument_id';
    this.parentPicker.hidden = true;
    await this.loadHistory();
  }

  private async openLive(): Promise<void> {
    const revision = ++this.requestRevision;
    try {
      const values = this.values();
      this.disconnected = false;
      this.viewRevision += 1;
      if (values.stypeIn === 'parent') {
        await this.resolveParent(values, revision);
        return;
      }
      this.setBusy(
        true,
        'Preparing a live-edge snapshot. The current historical view remains visible until it succeeds…',
      );
      await this.closeSubscription();
      if (!this.isCurrentRequest(revision)) return;
      const interval = resolutionSeconds(values.resolution);
      const now = Math.floor(Date.now() / 1000);
      const liveEdge = Math.floor(now / interval) * interval;
      this.abort?.abort();
      const controller = new AbortController();
      this.abort = controller;
      const activeRequest = {
        ...values,
        from: (liveEdge - interval * LIVE_LOOKBACK_INTERVALS) as UTCTimestamp,
        to: now as UTCTimestamp,
      };
      const result = await this.provider.openBars(
        {
          ...activeRequest,
          signal: controller.signal,
        },
        this.handlers(revision),
      );
      if (!this.isCurrentRequest(revision) || controller.signal.aborted) {
        await result.subscription.dispose();
        return;
      }
      this.activeRequest = activeRequest;
      this.applyOpenResult(result);
      this.setNotice(
        'Live snapshot loaded. Incoming updates now replace the current interval or append the next one.',
      );
    } catch (caught) {
      if (this.isCurrentRequest(revision)) this.showError(caught);
    } finally {
      if (revision === this.requestRevision) this.setBusy(false);
    }
  }

  private handlers(revision: number) {
    return {
      onBar: (bar: ChartBar) => {
        if (this.isCurrentRequest(revision)) this.updateCandle(bar);
      },
      onVolume: (volume: Volume) => {
        if (this.isCurrentRequest(revision)) this.updateVolume(volume);
      },
      onState: (state: ProviderState) => {
        if (this.isCurrentRequest(revision) && state !== 'closed')
          this.setNotice(`Connection state: ${state}.`);
      },
      onError: (error: DatabentoProviderError) => {
        if (this.isCurrentRequest(revision)) this.showError(error);
      },
      onSymbolMapping: (mapping: ResolvedSymbol) => {
        if (this.isCurrentRequest(revision))
          this.setNotice(
            `Session-pinned instrument: ${mapping.resolvedSymbol} (${mapping.instrumentId}).`,
          );
      },
    };
  }

  private isCurrentRequest(revision: number): boolean {
    return revision === this.requestRevision && !this.disconnected;
  }

  private applyOpenResult(result: OpenBarsResult): void {
    this.replaceData(result.initial);
    this.subscription = result.subscription;
    this.historyExhausted = false;
  }

  private replaceData(page: BarPage): void {
    this.viewRevision += 1;
    this.bars = [...page.bars];
    this.volumes = [...page.volumes];
    this.metadata = new Map();
    for (const [time, metadata] of page.metadata) this.metadata.set(time, metadata);
    this.candles?.setData(this.bars);
    this.volume?.setData(this.volumes);
    this.chart?.timeScale().fitContent();
  }

  private updateCandle(bar: SeriesBar): void {
    const time = barTime(bar);
    if (time === undefined) return;
    const previous = this.bars.at(-1);
    const previousTime = previous === undefined ? undefined : barTime(previous);
    if (previousTime !== undefined && Number(time) < Number(previousTime)) {
      this.showError(new Error('Rejected decreasing timestamp from the provider.'));
      return;
    }
    this.bars = mergeByTime(this.bars, [bar]);
    this.candles?.update(bar);
  }

  private updateVolume(volume: Volume): void {
    const previous = this.volumes.at(-1);
    if (previous !== undefined && Number(volume.time) < Number(previous.time)) {
      this.showError(new Error('Rejected decreasing volume timestamp from the provider.'));
      return;
    }
    this.volumes = mergeByTime(this.volumes, [volume]);
    this.volume?.update(volume);
  }

  private async loadOlderIfNeeded(range: LogicalRange | null): Promise<void> {
    if (range === null || this.pageLoading || this.historyExhausted || this.bars.length === 0)
      return;
    const barsInRange = this.candles?.barsInLogicalRange(range);
    if (
      barsInRange === null ||
      barsInRange === undefined ||
      barsInRange.barsBefore > PAGE_THRESHOLD
    )
      return;
    const earliestBar = this.bars.at(0);
    if (earliestBar === undefined) return;
    const earliest = barTime(earliestBar);
    if (earliest === undefined) return;
    const values = this.activeRequest;
    if (values === undefined) return;
    const width = resolutionSeconds(values.resolution) * PAGE_INTERVALS;
    this.pageLoading = true;
    const before = this.chart?.timeScale().getVisibleLogicalRange() ?? null;
    const revision = this.viewRevision;
    try {
      const page = await this.provider.getBars({
        ...values,
        from: (Number(earliest) - width) as UTCTimestamp,
        to: earliest,
        signal: this.abort?.signal,
      });
      if (revision !== this.viewRevision || this.disconnected) return;
      if (page.bars.length === 0) {
        this.historyExhausted = true;
        this.setNotice('No earlier history is available.');
        return;
      }
      const newBars = [...page.bars];
      this.bars = mergeByTime(newBars, this.bars);
      this.volumes = mergeByTime(page.volumes, this.volumes);
      for (const [time, metadata] of page.metadata) this.metadata.set(time, metadata);
      this.candles?.setData(this.bars);
      this.volume?.setData(this.volumes);
      if (before !== null)
        this.chart?.timeScale().setVisibleLogicalRange({
          from: before.from + newBars.length,
          to: before.to + newBars.length,
        });
      this.setNotice(`Loaded ${newBars.length} earlier bars.`);
    } catch (caught) {
      if (!this.disconnected && !this.abort?.signal.aborted) this.showError(caught);
    } finally {
      this.pageLoading = false;
    }
  }

  private toggleVolumePane(): void {
    if (this.chart === undefined) return;
    if (this.volume !== undefined) {
      this.chart.removeSeries(this.volume);
      this.chart.removePane(1);
      this.volume = undefined;
      this.volumeToggle.textContent = 'Show volume pane';
      this.volumeToggle.setAttribute('aria-pressed', 'false');
      return;
    }
    const pane = this.chart.addPane(true);
    pane.setHeight(150);
    this.volume = this.chart.addSeries(
      HistogramSeries,
      { priceFormat: { type: 'volume' }, priceScaleId: '', lastValueVisible: false },
      1,
    );
    this.volume.setData(this.volumes);
    this.volumeToggle.textContent = 'Hide volume pane';
    this.volumeToggle.setAttribute('aria-pressed', 'true');
  }

  private renderCrosshair(
    event: Parameters<NonNullable<IChartApi['subscribeCrosshairMove']>>[0] extends (
      parameter: infer P,
    ) => unknown
      ? P
      : never,
  ): void {
    const seriesBar =
      this.candles === undefined
        ? undefined
        : (event.seriesData.get(this.candles) as SeriesBar | undefined);
    const point = event.point;
    if (
      seriesBar === undefined ||
      !isCandle(seriesBar) ||
      point === undefined ||
      point.x < 0 ||
      point.y < 0 ||
      point.x > this.chartContainer.clientWidth ||
      point.y > this.chartContainer.clientHeight
    ) {
      this.legend.hidden = true;
      this.tooltip.hidden = true;
      return;
    }
    const metadata = this.metadata.get(seriesBar.time as UTCTimestamp);
    this.legend.replaceChildren(
      ...[
        ['Open', String(seriesBar.open)],
        ['High', String(seriesBar.high)],
        ['Low', String(seriesBar.low)],
        ['Close', String(seriesBar.close)],
        ['Instrument', metadata === undefined ? 'Unknown' : String(metadata.instrumentId)],
      ].flatMap((entry) => {
        const term = entry[0] ?? '';
        const value = entry[1] ?? '';
        const dt = this.document.createElement('dt');
        dt.textContent = term;
        const dd = this.document.createElement('dd');
        dd.textContent = value;
        dd.style.margin = '0';
        return [dt, dd];
      }),
    );
    this.legend.hidden = false;
    this.tooltip.textContent = `O ${seriesBar.open} H ${seriesBar.high} L ${seriesBar.low} C ${seriesBar.close}`;
    this.tooltip.style.left = `${Math.min(point.x + 12, this.chartContainer.clientWidth - 150)}px`;
    this.tooltip.style.top = `${Math.min(point.y + 12, this.chartContainer.clientHeight - 42)}px`;
    this.tooltip.hidden = false;
  }

  private async closeSubscription(): Promise<void> {
    await this.subscription?.dispose();
    this.subscription = undefined;
  }

  private async disconnect(): Promise<void> {
    this.disconnected = true;
    this.requestRevision += 1;
    const controller = this.abort;
    this.abort = undefined;
    this.historyExhausted = true;
    this.viewRevision += 1;
    this.pageLoading = false;
    this.setBusy(false);
    this.setNotice('Disconnected. Historical chart data remains visible.');
    await this.closeSubscription();
    controller?.abort();
  }

  private setBusy(busy: boolean, message?: string): void {
    for (const control of this.form.querySelectorAll('button, input, select')) {
      if (
        control instanceof HTMLButtonElement ||
        control instanceof HTMLInputElement ||
        control instanceof HTMLSelectElement
      )
        control.disabled = busy;
    }
    if (message !== undefined) this.setNotice(message);
  }

  private setNotice(message: string): void {
    if (this.disconnected && message !== 'Disconnected. Historical chart data remains visible.')
      return;
    this.error.hidden = true;
    this.notice.textContent = message;
  }

  private showError(caught: unknown): void {
    const message =
      caught instanceof Error ? caught.message : 'The gateway returned an unknown error.';
    this.error.textContent = message;
    this.error.hidden = false;
    this.notice.textContent = 'The existing chart data was preserved.';
  }
}
