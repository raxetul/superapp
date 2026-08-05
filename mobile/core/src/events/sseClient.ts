/**
 * Foreground SSE client for `GET /api/v1/events/stream` (TR-08-006).
 *
 * React Native has no native `EventSource`, and there is no single blessed
 * streaming primitive, so the transport is injected: the client owns the
 * *policy* (bearer auth, `Last-Event-ID` resume, exponential-backoff reconnect
 * on drop, and immediate reconnect on app-foreground) while the transport owns
 * the *mechanism* (the actual long-lived HTTP/streaming connection). This keeps
 * the reconnect/resume logic fully unit-testable without a live backend.
 */

/** Server event envelope, mirroring `events::EventEnvelope` on the backend. */
export interface ServerEvent<T = unknown> {
  type: string;
  data: T;
  timestamp: string;
  user_id?: string | null;
}

/** A raw SSE frame as delivered by the transport. */
export interface RawSseFrame {
  id?: string;
  event?: string;
  data: string;
}

export interface SseConnection {
  close(): void;
}

export interface SseConnectParams {
  url: string;
  headers: Record<string, string>;
  lastEventId?: string;
  onOpen: () => void;
  onFrame: (frame: RawSseFrame) => void;
  onError: (error: unknown) => void;
}

/** The connection mechanism (a real one would use fetch/XHR streaming). */
export interface SseTransport {
  connect(params: SseConnectParams): SseConnection;
}

export type SseState = 'idle' | 'connecting' | 'open' | 'reconnecting' | 'closed';

export interface SseClientOptions {
  url: string;
  transport: SseTransport;
  getAccessToken: () => Promise<string | null>;
  onEvent: (event: ServerEvent) => void;
  onStateChange?: (state: SseState) => void;
  /** Backoff schedule (ms). The last value repeats. */
  backoffMs?: number[];
  /** Injectable timer; returns a cancel fn. Defaults to setTimeout. */
  schedule?: (fn: () => void, delayMs: number) => () => void;
}

const DEFAULT_BACKOFF = [1_000, 2_000, 5_000, 10_000, 30_000];

function defaultSchedule(fn: () => void, delayMs: number): () => void {
  const handle = setTimeout(fn, delayMs);
  return () => clearTimeout(handle);
}

export class SseClient {
  private readonly backoff: number[];
  private readonly schedule: (fn: () => void, delayMs: number) => () => void;

  private connection?: SseConnection;
  private lastEventId?: string;
  private attempt = 0;
  private stopped = true;
  private cancelTimer?: () => void;
  private state: SseState = 'idle';

  constructor(private readonly options: SseClientOptions) {
    this.backoff = options.backoffMs ?? DEFAULT_BACKOFF;
    this.schedule = options.schedule ?? defaultSchedule;
  }

  get currentState(): SseState {
    return this.state;
  }

  /** The last event id seen, used for `Last-Event-ID` resume. */
  get lastId(): string | undefined {
    return this.lastEventId;
  }

  /** Open the stream and keep it alive (reconnecting on drop). */
  async start(): Promise<void> {
    this.stopped = false;
    this.attempt = 0;
    await this.open();
  }

  /** Stop the stream and cancel any pending reconnect. */
  stop(): void {
    this.stopped = true;
    this.cancelTimer?.();
    this.cancelTimer = undefined;
    this.connection?.close();
    this.connection = undefined;
    this.setState('closed');
  }

  /**
   * Called when the app returns to the foreground. If the stream is not
   * currently open, reconnect immediately (resetting backoff) and resume from
   * the last event id.
   */
  onForeground(): void {
    if (this.stopped) return;
    if (this.state === 'open' || this.state === 'connecting') return;
    this.cancelTimer?.();
    this.cancelTimer = undefined;
    this.attempt = 0;
    void this.open();
  }

  private setState(state: SseState): void {
    this.state = state;
    this.options.onStateChange?.(state);
  }

  private async open(): Promise<void> {
    if (this.stopped) return;
    this.setState(this.attempt > 0 ? 'reconnecting' : 'connecting');

    const token = await this.getToken();
    if (this.stopped) return;

    const headers: Record<string, string> = { Accept: 'text/event-stream' };
    if (token) headers.Authorization = `Bearer ${token}`;
    if (this.lastEventId) headers['Last-Event-ID'] = this.lastEventId;

    this.connection = this.options.transport.connect({
      url: this.options.url,
      headers,
      lastEventId: this.lastEventId,
      onOpen: this.handleOpen,
      onFrame: this.handleFrame,
      onError: this.handleError,
    });
  }

  private async getToken(): Promise<string | null> {
    try {
      return await this.options.getAccessToken();
    } catch {
      return null;
    }
  }

  private handleOpen = (): void => {
    this.attempt = 0;
    this.setState('open');
  };

  private handleFrame = (frame: RawSseFrame): void => {
    if (frame.id) this.lastEventId = frame.id;
    if (!frame.data) return;
    try {
      this.options.onEvent(JSON.parse(frame.data) as ServerEvent);
    } catch {
      // Ignore malformed frames rather than tearing the stream down.
    }
  };

  private handleError = (): void => {
    this.connection?.close();
    this.connection = undefined;
    if (this.stopped) {
      this.setState('closed');
      return;
    }
    const delay = this.backoff[Math.min(this.attempt, this.backoff.length - 1)];
    this.attempt += 1;
    this.setState('reconnecting');
    this.cancelTimer = this.schedule(() => {
      void this.open();
    }, delay);
  };
}
