# Databento Lightweight Charts protocol v1

This contract is the shared boundary between the Rust gateway and the browser package. All JSON objects use `v: 1`. Unknown command fields are rejected. Protocol errors are stable, sanitized, and never include upstream response bodies or credentials.

## HTTP

All JSON endpoints are under `/v1`. A request ID is accepted in `X-Request-Id` and returned in that header and response body.

| Endpoint | Request | Success response |
| --- | --- | --- |
| `POST /history/bars` | `dataset`, `symbol`, `stypeIn`, `resolution`, `from`, `to`, optional `gapPolicy` | `{v,requestId,bars,volumes,metadata}` |
| `POST /symbols/resolve` | `dataset`, `symbols`, `stypeIn`, `from`, `to` | `{v,requestId,mappings}` |
| `POST /symbols/search` | `dataset`, `query`, `stypeIn` | `{v,requestId,results}` |
| `GET /datasets/{dataset}` | none | `{v,requestId,metadata}` |

`from` and `to` are safe integer UTC seconds and all ranges are `[from,to)`. Bar operations reject `stypeIn: "parent"` with `unsupported_parent_series`; callers resolve a parent then use one returned child with `stypeIn: "instrument_id"`.

Non-success responses have `{v,requestId,error:{code,message,retryable,details}}`. `code` is one of the public TypeScript `ProviderErrorCode` values.

## WebSocket

`GET /v1/live` requires subprotocol `databento-lwc.v1`. Every command has `v`, `type`, and `commandId`; subscription commands additionally have a client-generated stable `subscriptionId`.

| Command | Required fields | Meaning |
| --- | --- | --- |
| `subscribe_bars` | `commandId`, `subscriptionId`, `request: BarRequest` | Subscribe without a snapshot. |
| `open_bars` | `commandId`, `subscriptionId`, `request: HistoryRequest` | Gateway-coordinated history/replay/live handoff. |
| `resume_bars` | `commandId`, `subscriptionId`, `resumeFrom`, `request: BarRequest` | Resume a disconnected stable subscription. |
| `unsubscribe` | `commandId`, `subscriptionId` | Remove the downstream reference only. |
| `cancel` | `commandId`, `targetCommandId`, `subscriptionId` | Cancel pending/active work; terminal. |

Events are `subscribed`, `snapshot`, `bar`, `status`, `symbol_mapping`, `unsubscribed`, `cancelled`, `error`, and `heartbeat`. `open_bars` must order `subscribed -> snapshot -> bar/status`; resume never emits snapshot. A `bar` contains `data`, optional `volume`, and `meta`; when volume is present its time equals `data.time`. Event timestamps must not decrease per subscription.

Continuous symbols remain session-pinned. During reconnect the gateway resolves again: unchanged resolution resumes; a changed instrument ends the subscription with `resolved_instrument_changed`, and the caller explicitly creates a new subscription.

`unsubscribed`, `cancelled`, and terminal `error` prevent later events for the subscription. Browser unsubscribe does not imply an upstream Databento unsubscribe.

## Fixture format

Fixtures are JSON documents with `valid`, `direction`, and `payload`. `direction` is `http-response`, `client-command`, or `server-event`. Fixtures in `valid` must parse under the TypeScript Zod schemas; fixtures in `invalid` must not parse. They contain only sanitized synthetic market data.
