import type { ProviderErrorCode } from '../types/index.js';

export class DatabentoProviderError extends Error {
  public override readonly name = 'DatabentoProviderError';

  public constructor(
    public readonly code: ProviderErrorCode,
    message: string,
    public readonly retryable: boolean,
    public readonly requestId?: string,
    public readonly subscriptionId?: string,
    public readonly details: Readonly<Record<string, unknown>> = {},
  ) {
    super(message);
  }
}

export const cancelledError = (subscriptionId?: string): DatabentoProviderError =>
  new DatabentoProviderError(
    'cancelled',
    'Operation was cancelled',
    false,
    undefined,
    subscriptionId,
  );

export const protocolError = (message: string, subscriptionId?: string): DatabentoProviderError =>
  new DatabentoProviderError('protocol_error', message, false, undefined, subscriptionId);
