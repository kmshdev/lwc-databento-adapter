import { z } from 'zod';

const safeInteger = z.number().int().safe();
const finiteNumber = z.number().finite();

export const protocolVersionSchema = z.literal(1);
export const symbolTypeSchema = z.enum(['raw_symbol', 'instrument_id', 'parent', 'continuous']);
export const resolutionSchema = z.enum([
  '1s',
  '5s',
  '15s',
  '30s',
  '1m',
  '5m',
  '15m',
  '30m',
  '1h',
  '2h',
  '4h',
  '1d',
]);
export const gapPolicySchema = z.enum(['preserve-gaps', 'whitespace', 'carry-forward']);
export const providerStateSchema = z.enum([
  'idle',
  'connecting',
  'replaying',
  'live',
  'reconnecting',
  'failed',
  'closed',
]);
export const statusReasonSchema = z.enum([
  'initial_connect',
  'handoff_replay',
  'replay_completed',
  'upstream_disconnect',
  'downstream_disconnect',
  'retry_scheduled',
  'retry_exhausted',
  'client_unsubscribe',
  'server_shutdown',
  'slow_consumer',
]);
export const providerErrorCodeSchema = z.enum([
  'invalid_request',
  'invalid_range',
  'range_too_large',
  'origin_forbidden',
  'dataset_forbidden',
  'unsupported_dataset',
  'unsupported_schema',
  'unsupported_resolution',
  'symbol_not_found',
  'symbol_mapping_failed',
  'unsupported_parent_series',
  'unsupported_live_symbology',
  'resolved_instrument_changed',
  'access_denied',
  'quota_exceeded',
  'slow_consumer',
  'replay_unavailable',
  'upstream_unavailable',
  'cancelled',
  'protocol_error',
  'internal',
]);

export const barRequestSchema = z.strictObject({
  dataset: z.string().min(1),
  symbol: z.string().min(1),
  stypeIn: symbolTypeSchema,
  resolution: resolutionSchema,
  gapPolicy: gapPolicySchema.optional(),
});

export const historyRequestSchema = barRequestSchema
  .extend({
    from: safeInteger,
    to: safeInteger,
  })
  .strict()
  .refine((value) => value.from < value.to, {
    message: 'from must be less than to',
    path: ['to'],
  });

export const metadataSchema = z.strictObject({
  dataset: z.string().min(1),
  requestedSymbol: z.string().min(1),
  resolvedSymbol: z.string().min(1),
  instrumentId: safeInteger.nonnegative(),
  sourceSchema: z.enum(['ohlcv-1s', 'ohlcv-1m', 'ohlcv-1h', 'ohlcv-1d']),
  synthetic: z.boolean(),
});

export const mappingSchema = z
  .strictObject({
    dataset: z.string().min(1),
    requestedSymbol: z.string().min(1),
    resolvedSymbol: z.string().min(1),
    instrumentId: safeInteger.nonnegative(),
    effectiveFrom: safeInteger,
    effectiveTo: safeInteger.optional(),
  })
  .refine((value) => value.effectiveTo === undefined || value.effectiveTo > value.effectiveFrom, {
    message: 'effectiveTo must be greater than effectiveFrom',
    path: ['effectiveTo'],
  });

export const whitespaceBarSchema = z.strictObject({ time: safeInteger });
export const candleBarSchema = z
  .strictObject({
    time: safeInteger,
    open: finiteNumber,
    high: finiteNumber,
    low: finiteNumber,
    close: finiteNumber,
  })
  .refine(
    (value) =>
      value.low <= value.open &&
      value.open <= value.high &&
      value.low <= value.close &&
      value.close <= value.high,
    {
      message: 'OHLC values are inconsistent',
    },
  );
export const chartBarSchema = z.union([whitespaceBarSchema, candleBarSchema]);
export const volumeSchema = z.strictObject({
  time: safeInteger,
  value: finiteNumber.nonnegative(),
  color: z.string().optional(),
});

export const protocolErrorSchema = z.strictObject({
  code: providerErrorCodeSchema,
  message: z.string(),
  retryable: z.boolean(),
  details: z.record(z.string(), z.unknown()),
});
export const errorResponseSchema = z.strictObject({
  v: protocolVersionSchema,
  requestId: z.string().min(1),
  error: protocolErrorSchema,
});

export const barPageResponseSchema = z.strictObject({
  v: protocolVersionSchema,
  requestId: z.string().min(1),
  bars: z.array(chartBarSchema),
  volumes: z.array(volumeSchema),
  metadata: z.array(metadataSchema.extend({ time: safeInteger }).strict()),
});
export const resolveResponseSchema = z.strictObject({
  v: protocolVersionSchema,
  requestId: z.string().min(1),
  mappings: z.array(mappingSchema),
});
export const searchResultSchema = z.strictObject({
  dataset: z.string().min(1),
  symbol: z.string().min(1),
  stypeIn: symbolTypeSchema,
  description: z.string().optional(),
});
export const searchResponseSchema = z.strictObject({
  v: protocolVersionSchema,
  requestId: z.string().min(1),
  results: z.array(searchResultSchema),
});
export const datasetMetadataSchema = z.strictObject({
  dataset: z.string().min(1),
  schemas: z.array(z.string()),
  publishers: z.array(
    z.strictObject({
      publisherId: safeInteger.nonnegative(),
      name: z.string(),
      venue: z.string(),
    }),
  ),
  availableFrom: safeInteger.optional(),
  availableTo: safeInteger.optional(),
});
export const datasetResponseSchema = z.strictObject({
  v: protocolVersionSchema,
  requestId: z.string().min(1),
  metadata: datasetMetadataSchema,
});

const commandBase = { v: protocolVersionSchema, commandId: z.string().min(1) };
export const subscribeCommandSchema = z.strictObject({
  ...commandBase,
  type: z.literal('subscribe_bars'),
  subscriptionId: z.string().min(1),
  request: barRequestSchema,
});
export const openCommandSchema = z.strictObject({
  ...commandBase,
  type: z.literal('open_bars'),
  subscriptionId: z.string().min(1),
  request: historyRequestSchema,
});
export const resumeCommandSchema = z.strictObject({
  ...commandBase,
  type: z.literal('resume_bars'),
  subscriptionId: z.string().min(1),
  resumeFrom: safeInteger,
  request: barRequestSchema,
});
export const unsubscribeCommandSchema = z.strictObject({
  ...commandBase,
  type: z.literal('unsubscribe'),
  subscriptionId: z.string().min(1),
});
export const cancelCommandSchema = z.strictObject({
  ...commandBase,
  type: z.literal('cancel'),
  targetCommandId: z.string().min(1),
  subscriptionId: z.string().min(1),
});
export const clientCommandSchema = z.discriminatedUnion('type', [
  subscribeCommandSchema,
  openCommandSchema,
  resumeCommandSchema,
  unsubscribeCommandSchema,
  cancelCommandSchema,
]);

export const subscribedEventSchema = z.strictObject({
  v: protocolVersionSchema,
  type: z.literal('subscribed'),
  commandId: z.string().min(1),
  subscriptionId: z.string().min(1),
  state: providerStateSchema,
  resolvedSymbols: z.array(mappingSchema),
});
export const snapshotEventSchema = z.strictObject({
  v: protocolVersionSchema,
  type: z.literal('snapshot'),
  subscriptionId: z.string().min(1),
  bars: z.array(chartBarSchema),
  volumes: z.array(volumeSchema),
  metadata: z.array(metadataSchema.extend({ time: safeInteger }).strict()),
});
export const barEventSchema = z
  .strictObject({
    v: protocolVersionSchema,
    type: z.literal('bar'),
    subscriptionId: z.string().min(1),
    data: chartBarSchema,
    volume: volumeSchema.optional(),
    meta: metadataSchema,
  })
  .refine((event) => event.volume === undefined || event.volume.time === event.data.time, {
    message: 'volume time must match bar time',
    path: ['volume'],
  });
export const statusEventSchema = z.strictObject({
  v: protocolVersionSchema,
  type: z.literal('status'),
  subscriptionId: z.string().min(1),
  state: providerStateSchema,
  retryable: z.boolean(),
  reason: statusReasonSchema.optional(),
});
export const mappingEventSchema = z.strictObject({
  v: protocolVersionSchema,
  type: z.literal('symbol_mapping'),
  subscriptionId: z.string().min(1),
  requestedSymbol: z.string().min(1),
  resolvedSymbol: z.string().min(1),
  instrumentId: safeInteger.nonnegative(),
  effectiveFrom: safeInteger,
});
export const unsubscribedEventSchema = z.strictObject({
  v: protocolVersionSchema,
  type: z.literal('unsubscribed'),
  commandId: z.string().min(1),
  subscriptionId: z.string().min(1),
});
export const cancelledEventSchema = z.strictObject({
  v: protocolVersionSchema,
  type: z.literal('cancelled'),
  commandId: z.string().min(1),
  targetCommandId: z.string().min(1),
  subscriptionId: z.string().min(1),
});
export const eventErrorSchema = z.strictObject({
  v: protocolVersionSchema,
  type: z.literal('error'),
  commandId: z.string().min(1).optional(),
  subscriptionId: z.string().min(1).optional(),
  error: protocolErrorSchema,
});
export const heartbeatEventSchema = z.strictObject({
  v: protocolVersionSchema,
  type: z.literal('heartbeat'),
  serverTime: safeInteger,
});
export const serverEventSchema = z.discriminatedUnion('type', [
  subscribedEventSchema,
  snapshotEventSchema,
  barEventSchema,
  statusEventSchema,
  mappingEventSchema,
  unsubscribedEventSchema,
  cancelledEventSchema,
  eventErrorSchema,
  heartbeatEventSchema,
]);

export type ClientCommand = z.infer<typeof clientCommandSchema>;
export type ServerEvent = z.infer<typeof serverEventSchema>;
