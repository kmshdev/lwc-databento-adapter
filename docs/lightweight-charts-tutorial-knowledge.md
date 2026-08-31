# Lightweight Charts tutorial knowledge

## Scope and exclusions

The review covered the full tutorial tree at the pinned upstream revision, excluding only the requested Vue and Web Components integration guides. Non-`how_to` material was analyzed first; `how_to` examples were then classified as built-in chart behavior or custom application glue. React examples were included as lifecycle evidence, not as a required chart-wrapper dependency.

This is a source review, not a claim that the upstream Docusaurus site passed in a browser. The runtime matrix remains pending because the checkout lacks installed dependencies and built bundles.

## Route and workflow families

| Family | Representative route | User-visible workflow |
| --- | --- | --- |
| Customization guide | `/tutorials/customization/intro` | Ten-step downloadable standalone chart from scaffold through series, scales, crosshair, colors, and typography |
| Accessibility guide | `/tutorials/a11y/intro` | Keyboard operation, readable modes, screen-reader description, reset, and randomized data |
| React guide | `/tutorials/react/simple` | Responsive mount/update/unmount patterns and an advanced interval/second-series workflow |
| How To | sidebar routes | Focused built-in features plus custom legends/tooltips |
| Examples and demos | sidebar routes | Range switching, locale/font changes, realtime return, infinite history, indicators, whitespace, and yield curves |
| Analysis indicators | `/tutorials/analysis-indicators` | Reactive `apply…` helpers and pure `calculate…` functions from upstream examples |

## Non-How-To findings

The customization sequence establishes a finite composition pattern: create a time chart, add candlesticks, configure colors and price formatting, set price/time scale behavior, configure crosshair, overlay a second series, apply per-point style overrides, and finish typography. The adapter demo should keep this separation between data transport, chart construction, and visual options.

The accessibility example exposes the richest interaction contract:

- buttons: Randomise Data and Reset Chart;
- checkboxes: higher contrast and larger font;
- keyboard: help, horizontal navigation, zoom, and visible-range description;
- focus state: concise help tooltip;
- assistive state: ARIA label, hidden instructions, and alert narration.

The React examples demonstrate lifecycle ownership with public APIs: responsive create/resize/remove, start/stop timed updates, and conditional add/remove of a second series. They do not justify a `lightweight-chart-react` dependency.

Indicator examples are reusable candidates, but adoption requires the same dependency and license review as any public package or copied source. Prefer the reactive helper surface when indicator results must follow series data updates; use pure calculation only when application code explicitly owns recomputation.

## Planned adapter demo surface

No application exists yet, so this inventory defines the required demo rather than claiming current routes.

### Route `/`

Role: local developer or evaluator. Authentication is out of scope; the gateway remains the secret boundary.

Inputs:

- dataset, symbol, input symbology, interval, start, and end;
- empty-interval policy;
- optional display locale/time zone formatter;
- explicit live-connect toggle after historical load.

Buttons:

- Load history;
- Connect live / Disconnect;
- Fit content;
- Go to realtime;
- Add/remove or show/hide volume pane;
- Reset chart.

Visible states:

- idle, validating, loading history, history ready, connecting, replaying, live, reconnecting, disconnected, exhausted history, empty result, recoverable error, and terminal error;
- connection and freshness indicator;
- current symbol, resolved instrument identity, interval, visible time range, and last update time;
- accessible live-region summary of state changes.

Chart workflows:

- candlestick main series plus histogram volume pane;
- pan left to request earlier history while preserving logical viewport;
- return to live edge;
- crosshair legend and boundary-safe tooltip;
- resize, pane height, and cleanup behavior;
- optional markers/watermark through first-party helpers.

Modals are not required for version 1. Errors use an inline alert with retry only when the operation is safe and idempotent.

## Built-in behavior versus custom application glue

Use built-in APIs for series, price/time scales, panes, price lines, crosshair position, markers, watermarks, logical-range subscriptions, and primitives.

Build application DOM for toolbar, form controls, connection state, accessible instructions, legend, and tooltip. Legends and tooltips are not built into Lightweight Charts; they subscribe to crosshair events and must hide or reposition on invalid/out-of-bounds coordinates.

Use upstream example source as reference, not a runtime import from the Docusaurus website. Site React examples depend on injected theme constants and are not package APIs.

## Pixel-perfect rendering rules

`CanvasRenderingTarget2D` offers two scopes:

- bitmap space for physical-pixel-aligned drawing;
- media space for CSS-pixel drawing without manual pixel-ratio conversion.

Both scopes save and restore canvas state. Nested helpers that call `ctx.save()` must restore it with `try/finally`.

Finite width invariants:

- bitmap positions and dimensions are integers;
- crosshair/grid width is `max(1, floor(pixelRatio))`;
- full-bar width fills the slot without gaps;
- histogram columns apply spacing and alignment correction, with an in-place form for hot paths;
- candlestick bodies may overlap at tight spacing and approach roughly 80 percent of the slot above spacing four.

Custom primitives must reuse the upstream position/width formulas or prove equivalent results with golden images across device-pixel ratios.

## Acceptance criteria and finite edge cases

| Surface | Acceptance criteria | Edge cases |
| --- | --- | --- |
| Load form | Valid bounded request loads ordered history exactly once | empty symbol, invalid interval, end before start, range over limit, unresolved symbol, empty result |
| Live toggle | Connect follows coordinated handoff; disconnect releases downstream reference | double click, connect during load, disconnect during replay, reconnect exhaustion |
| Infinite history | Left threshold requests one earlier page and preserves logical viewport | repeated threshold events, no earlier data, page overlap, request failure |
| Go to realtime | CTA appears only off the live edge and scrolls to latest | no data, live disconnected, resize during scroll |
| Legend/tooltip | Crosshair values update; invalid coordinates hide the overlay; keyboard users receive equivalent text | whitespace bar, pointer outside pane, left/right/top/bottom collision, multiple series |
| Panes | Volume pane can be added, resized, moved, and removed through public APIs | minimum 30-pixel height, removal with series, narrow viewport |
| Accessibility | All controls have names, visible focus, keyboard operation, and status announcements | high contrast, 200 percent zoom, reduced motion, screen reader, chart canvas unavailable |
| Resize | Chart follows container without observer loops and cleans observers on unmount | missing ResizeObserver, zero-size container, rapid DPR change, hidden then shown |
| Pixel rendering | Golden images match at DPR 1, 1.25, 1.5, 2, and 3 | fractional coordinates, very narrow bar spacing, high-density histogram |

## Source findings to track separately

These are upstream documentation findings, not bugs in this adapter:

1. High confidence: the tutorial-index Analysis indicators link appears to use an incorrect relative path.
2. High confidence: the image-watermark resource bullet links to the text-watermark API page.
3. High confidence: `how_to/no-time-scale.js` appears orphaned from the user-facing MDX routes.
4. Medium confidence: several example comments and pagination metadata are stale or copied from different pages.

Reproduction evidence is source-path based in this round. Browser reproduction is pending the upstream bootstrap gate and must be logged before proposing upstream fixes.

## Browser test matrix

- Tutorial index: every in-scope card/link resolves; known link findings reproduce or are cleared.
- Customization: each step iframe/new-window/download loads and selected visual deltas appear.
- Accessibility: focus help, help toggle, arrow/zoom keys, description key, randomize, reset, contrast, font, and announcements work.
- React lifecycle: resize and theme update; advanced start/stop and add/remove series clean up correctly.
- Demos: font, locale, range, realtime, infinite-history, whitespace, and yield-curve marker workflows work.
- How To: legends, tooltip boundary handling, crosshair sync/touch tracking, two scales, overlay volume, panes, price lines, markers, inversion, watermarks, and custom horizontal scale render as documented.
