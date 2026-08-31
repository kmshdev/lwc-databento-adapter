import { cancelledError, DatabentoProviderError, protocolError } from '../errors/index.js';
import type {
  BarHandlers,
  BarRequest,
  HistoryRequest,
  ProviderState,
  Subscription,
} from '../types/index.js';

export type LiveRequest = BarRequest | HistoryRequest;
export type SubscriptionMode = 'subscribe_bars' | 'open_bars';

export class ManagedSubscription implements Subscription {
  private currentState: ProviderState = 'idle';
  private resolveDone!: () => void;
  private readonly done = new Promise<void>((resolve) => {
    this.resolveDone = resolve;
  });
  private unsubscribing?: Promise<void>;
  private lastTime?: number;
  private active = true;

  public constructor(
    public readonly id: string,
    public readonly request: LiveRequest,
    public readonly mode: SubscriptionMode,
    public readonly handlers: BarHandlers,
    private readonly requestUnsubscribe: (subscription: ManagedSubscription) => Promise<void>,
  ) {}

  public get state(): ProviderState {
    return this.currentState;
  }
  public get latestTime(): number | undefined {
    return this.lastTime;
  }
  public get isActive(): boolean {
    return this.active;
  }

  public setState(state: ProviderState): void {
    if (!this.active || this.currentState === state) return;
    this.currentState = state;
    try {
      this.handlers.onState?.(state);
    } catch {
      // Consumer callbacks must not terminate the transport state machine.
    }
  }

  public emitBar(time: number, callback: () => void): void {
    if (!this.active) return;
    if (this.lastTime !== undefined && time < this.lastTime) {
      this.fail(protocolError('Gateway emitted a decreasing bar timestamp', this.id));
      return;
    }
    this.lastTime = time;
    try {
      callback();
    } catch (cause) {
      this.fail(
        new DatabentoProviderError('internal', 'Bar handler failed', false, undefined, this.id, {
          cause: String(cause),
        }),
      );
    }
  }

  public emitError(error: DatabentoProviderError): void {
    if (!this.active) return;
    try {
      this.handlers.onError?.(error);
    } catch {
      /* callback isolation */
    }
  }

  public emitMapping(callback: () => void): void {
    if (!this.active) return;
    try {
      callback();
    } catch {
      /* callback isolation */
    }
  }

  public fail(error: DatabentoProviderError): void {
    if (!this.active) return;
    this.emitError(error);
    this.currentState = 'failed';
    this.active = false;
    try {
      this.handlers.onState?.('failed');
    } catch {
      // Consumer callbacks must not terminate the transport state machine.
    }
    this.resolveDone();
  }

  public finishClosed(): void {
    if (!this.active) return;
    this.active = false;
    this.currentState = 'closed';
    try {
      this.handlers.onState?.('closed');
    } catch {
      // Consumer callbacks must not terminate the transport state machine.
    }
    this.resolveDone();
  }

  public async unsubscribe(): Promise<void> {
    this.unsubscribing ??= this.requestUnsubscribe(this);
    return this.unsubscribing;
  }

  public async dispose(): Promise<void> {
    return this.unsubscribe();
  }
  public async waitForTermination(): Promise<void> {
    return this.done;
  }
  public cancel(): void {
    this.fail(cancelledError(this.id));
  }
}
