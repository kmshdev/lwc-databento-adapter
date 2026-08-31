import { describe, expect, it, vi } from 'vitest';
import { DatabentoProviderError } from '../../src/errors/index.js';
import { ManagedSubscription } from '../../src/subscriptions/subscription.js';

describe('ManagedSubscription', () => {
  it('terminates cleanly when the gateway fails a newly constructed subscription', async () => {
    const onError = vi.fn();
    const onState = vi.fn();
    const subscription = new ManagedSubscription(
      'sub-1',
      {
        dataset: 'GLBX.MDP3',
        symbol: 'ES.c.0',
        stypeIn: 'continuous',
        resolution: '1m',
      },
      'subscribe_bars',
      { onBar: vi.fn(), onError, onState },
      async () => undefined,
    );
    const error = new DatabentoProviderError(
      'symbol_mapping_failed',
      'Databento symbol resolution failed',
      false,
    );

    subscription.fail(error);
    await expect(subscription.waitForTermination()).resolves.toBeUndefined();

    expect(subscription.state).toBe('failed');
    expect(onError).toHaveBeenCalledWith(error);
    expect(onState).toHaveBeenCalledWith('failed');
  });
});
