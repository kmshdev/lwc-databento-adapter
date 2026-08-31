import { protocolError } from '../errors/index.js';
import { serverEventSchema, type ClientCommand, type ServerEvent } from './schemas.js';
import type { ProviderConfig } from '../types/index.js';

export interface BrowserSocket {
  readyState: number;
  send(data: string): void;
  close(code?: number, reason?: string): void;
  addEventListener(type: 'open' | 'error', listener: (event: Event) => void): void;
  addEventListener(type: 'message', listener: (event: MessageEvent) => void): void;
  addEventListener(type: 'close', listener: (event: CloseEvent) => void): void;
}

export type SocketFactory = (url: string, protocols: string | string[]) => BrowserSocket;

const defaultSocketFactory: SocketFactory = (url, protocols) => new WebSocket(url, protocols);

export class LiveSocket {
  private socket: BrowserSocket | undefined;
  private connecting: Promise<void> | undefined;
  private disposed = false;

  public constructor(
    private readonly gatewayUrl: URL,
    private readonly onEvent: (event: ServerEvent) => void,
    private readonly onClose: (unexpected: boolean) => void,
    private readonly onFault: (error: Error) => void,
    private readonly factory: SocketFactory = defaultSocketFactory,
  ) {}

  public connect(): Promise<void> {
    if (this.disposed) return Promise.reject(protocolError('Provider is disposed'));
    if (this.socket?.readyState === WebSocket.OPEN) return Promise.resolve();
    this.connecting ??= new Promise<void>((resolve, reject) => {
      const url = new URL('/v1/live', this.gatewayUrl);
      url.protocol = url.protocol === 'https:' ? 'wss:' : 'ws:';
      let opened = false;
      const socket = this.factory(url.toString(), 'databento-lwc.v1');
      this.socket = socket;
      socket.addEventListener('open', () => {
        opened = true;
        this.connecting = undefined;
        resolve();
      });
      socket.addEventListener('message', (event) => {
        if (typeof event.data !== 'string') {
          this.onFault(protocolError('Gateway sent a binary WebSocket frame'));
          return;
        }
        let decoded: unknown;
        try {
          decoded = JSON.parse(event.data);
        } catch {
          this.onFault(protocolError('Gateway sent malformed JSON'));
          return;
        }
        const parsed = serverEventSchema.safeParse(decoded);
        if (!parsed.success) {
          this.onFault(protocolError('Gateway sent an invalid protocol event'));
          return;
        }
        this.onEvent(parsed.data);
      });
      socket.addEventListener('error', () => {
        if (!opened) {
          this.connecting = undefined;
          reject(protocolError('WebSocket connection failed'));
        }
      });
      socket.addEventListener('close', () => {
        if (!opened) {
          this.connecting = undefined;
          reject(protocolError('WebSocket closed before opening'));
        }
        this.socket = undefined;
        this.onClose(!this.disposed);
      });
    });
    return this.connecting;
  }

  public async send(command: ClientCommand): Promise<void> {
    await this.connect();
    if (this.socket?.readyState !== WebSocket.OPEN) throw protocolError('WebSocket is not open');
    this.socket.send(JSON.stringify(command));
  }

  public close(): void {
    this.disposed = true;
    this.socket?.close(1000, 'provider disposed');
    this.socket = undefined;
  }
}

export const reconnectDelay = (config: ProviderConfig['reconnect'], attempt: number): number => {
  const unclamped = config.baseDelayMs * 2 ** Math.max(0, attempt - 1);
  const base = Math.min(config.maxDelayMs, unclamped);
  const offset = base * config.jitterRatio * (Math.random() * 2 - 1);
  return Math.max(0, Math.round(base + offset));
};
