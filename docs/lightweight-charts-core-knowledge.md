# Lightweight Charts core knowledge

## Source pin and scope

This artifact records implementation constraints verified against Lightweight Charts `5.2.1` at commit `65e78a0d61e086aeceee15eda32be1614d16c246`. The authoritative public surface for this checkout is `src/index.ts`, the `src/api` interfaces, and the documentation. Generated `dist/typings.d.ts` is absent, so `TASK-00` must build and compare generated declarations before freezing compatibility.

Requested documentation-plugin paths resolve under `website/plugins`, not the repository root. First-party runtime plugins resolve under `src/plugins`.

## Public entry surface

`src/index.ts` exports:

- constructors: `createChart`, `createChartEx`, `createYieldCurveChart`, and `createOptionsChart`;
- horizontal-scale support: `defaultHorzScaleBehavior`;
- built-in series definitions: `LineSeries`, `AreaSeries`, `BarSeries`, `CandlestickSeries`, `HistogramSeries`, and `BaselineSeries`;
- first-party helpers: text/image watermarks, series markers, and up/down markers.

`src/standalone.ts` does not define a second chart runtime. It attaches the same module exports to `window.LightweightCharts`. The adapter package should import the module API; the no-bundler demo and browser fixture may consume the standalone window export.

## Chart constructors

| Constructor | Horizontal domain | Intended use |
| --- | --- | --- |
| `createChart` | time | Market bars and the adapter's version 1 demo |
| `createChartEx` | custom behavior | A deliberately custom horizontal scale; not needed for standard market time |
| `createOptionsChart` | numeric/price | Option-chain style views |
| `createYieldCurveChart` | duration/months | Yield curves; Area and Line series only |

Version 5 uses series definitions: `chart.addSeries(CandlestickSeries, options, paneIndex)`. Legacy methods such as `addCandlestickSeries()` are not the target contract.

## Series data and update semantics

- Intraday `UTCTimestamp` values are Unix seconds, not JavaScript milliseconds.
- `Time` may also be a `BusinessDay` object or ISO business-day string.
- `setData()` is for ordered full replacement/backfill. Input times must be strictly ascending and unique.
- `update()` is for replacing the latest point or appending a newer point.
- Historical updates have different failure modes for old or missing points and require explicit tests.
- Whitespace records preserve horizontal spacing without inventing price values.

Adapter rule: normalize nanoseconds once at the transport boundary, validate exact second alignment where the requested schema requires it, use `setData()` for the initial ordered page, then use `update()` for the live edge.

## Pane lifecycle

Use the built-in APIs rather than a parallel pane model:

- chart: `addPane`, `panes`, `removePane`, `swapPanes`, and `paneSize`;
- series: create with `paneIndex` and move with `moveToPane`;
- pane: `getHeight`, `setHeight`, `setStretchFactor`, `moveTo`, `paneIndex`, `getSeries`, and `priceScale`.

Removing a pane removes its series. The documented minimum pane height is 30 pixels. The demo must confirm volume-pane creation, movement, resize, swap, and removal as observable workflows.

One upstream behavior needs runtime qualification: `IPaneApi.addCustomSeries()` is documented as pane-local, but the inspected implementation appears to forward a default pane index of zero while `addSeries()` uses the current pane index. Until verified, create custom series through the chart with an explicit pane index.

## Time scale

The public time-scale API already supports the adapter's viewport needs:

- get/set visible time and logical ranges;
- `fitContent` and scroll operations;
- time/coordinate and logical/coordinate conversion;
- subscriptions for visible-range and size changes;
- series `barsInLogicalRange()` for incremental history triggers.

History loading must preserve the user's logical viewport when prepending data and must not call private model methods.

## Price scale

Use the public pane-aware `priceScale(id, paneIndex)` and series `priceScale()` APIs. They provide options, visible-range control, autoscale control, and width inspection. Overlay scale width is not a layout measurement: only default left/right scales return a non-zero width.

Tests cover normal, logarithmic, percentage, and indexed-to-100 modes when the demo exposes them, plus scale margins, inversion, autoscale, and overlay scales.

## Time zones and localization

Lightweight Charts intentionally has no native time-zone option. Internally it processes time in UTC. Formatting hooks include chart localization `timeFormatter` and time-scale `tickMarkFormatter`.

Version 1 policy: canonical bar timestamps remain UTC seconds. Display localization may format labels without altering bar identity or ordering. Pre-shifting stored timestamps is prohibited because it changes the represented instant and creates daylight-saving discontinuities. Business-day data remains date-only.

## Primitives and first-party helpers

Series primitives are the extension point for chart-bound overlays. They may provide pane and axis views, autoscale information, lifecycle hooks, hit testing, and z-order. Pane primitives provide pane-level views and the same attachment lifecycle. Renderers receive `CanvasRenderingTarget2D` and must use the appropriate media- or bitmap-coordinate scope.

Prefer first-party helpers before custom rendering:

- `createSeriesMarkers` or `createUpDownMarkers` for marker workflows;
- text/image watermark helpers for watermark workflows;
- primitives for overlays that must follow chart coordinates, autoscale, hover, or pane lifecycle;
- application DOM for toolbars, sidebars, forms, and modals that are not part of the canvas.

`autoscaleInfo()` is hot-path code and must be bounded, cached where appropriate, and covered by performance tests.

## Resize and embedding

`autoSize` defaults to false. When enabled it depends on `ResizeObserver`, and explicit `resize()` calls are ignored while autosize owns sizing. The demo must have one clear owner:

- preferred: `autoSize: true` when `ResizeObserver` exists;
- controlled fallback: application observer/manual `resize()` when it does not.

The documentation plugin's chart builder uses the same imperative chart APIs and a small resize binding; no React chart wrapper is required.

## Wrapper-removal contract

The implementation must not introduce `lightweight-chart-react`. A mounted chart component owns exactly one chart instance, stores public chart/series references, subscribes and unsubscribes public events, and calls `chart.remove()` on unmount. It must not reach into wrapper-private or library-private DOM/model state.

## Acceptance criteria and finite edge cases

1. Mount/unmount: exactly one chart is created and removed; remount does not duplicate canvases or subscriptions.
2. Data: ordered unique seconds render; milliseconds, duplicate times, descending input, and malformed OHLC fail before reaching the chart.
3. Live edge: same-bucket updates replace the current bar; newer buckets append; older/missing historical updates take the documented error path.
4. Panes: add, move, height/stretch, swap, and remove behaviors use public APIs and preserve surviving series ownership.
5. Scales: logical range is preserved on prepend; overlay price-scale width is never used as a non-zero layout invariant.
6. Time zones: canonical timestamps do not change when display locale/time zone changes; daylight-saving transitions preserve order.
7. Primitives: attach/detach cleans external listeners; hit testing and z-order are deterministic; autoscale work is bounded.
8. Resize: both native autosize and the fallback pass; ResizeObserver loop errors are not globally suppressed by production code.

## Open runtime risks

- Build/generated-type parity is unverified until the upstream checkout or installed package is built.
- The pane-local custom-series behavior requires a focused browser test.
- The upstream Puppeteer runner cannot currently execute because dependencies and standalone bundles are absent.
