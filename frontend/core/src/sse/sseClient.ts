/**
 * TR-07-006 — SSE real-time client for `GET /api/v1/events/stream`.
 *
 * Wraps an EventSource-like transport with automatic reconnect (exponential
 * backoff) and resume via `Last-Event-ID`. The transport is injected via a
 * factory so the client is unit-testable and so callers can plug an
 * EventSource polyfill that attaches the `Authorization: Bearer` header (the
 * native `EventSource` cannot set headers).
 */

/** A server event, per the backend contract. */
export interface SseEvent {
  type: string;
  data: unknown;
  timestamp?: string;
  user_id?: string;
}

/** Minimal surface of the `EventSource` we depend on. */
export interface EventSourceLike {
  addEventListener(
    type: string,
    listener: (ev: MessageEvent) => void,
  ): void;
  close(): void;
  onopen: ((ev: Event) => void) | null;
  onerror: ((ev: Event) => void) | null;
}

export interface ConnectContext {
  /** The last event id seen, for resume; undefined on the first connect. */
  lastEventId?: string;
  /** Current access token (or null). */
  token: string | null;
}

export type EventSourceFactory = (
  url: string,
  ctx: ConnectContext,
) => EventSourceLike;

export interface SseClientOptions {
  url: string;
  factory: EventSourceFactory;
  onEvent: (ev: SseEvent) => void;
  getToken?: () => string | null;
  onError?: (ev: Event) => void;
  onOpen?: () => void;
  /** Base reconnect delay (ms). */
  reconnectBaseMs?: number;
  /** Maximum reconnect delay (ms). */
  reconnectMaxMs?: number;
}

export class SseClient {
  private es: EventSourceLike | null = null;
  private lastEventId: string | undefined;
  private attempt = 0;
  private timer: ReturnType<typeof setTimeout> | null = null;
  private closed = false;

  constructor(private readonly opts: SseClientOptions) {}

  /** The last event id observed (drives resume). */
  get currentLastEventId(): string | undefined {
    return this.lastEventId;
  }

  /** Open the stream (idempotent while a connection is live). */
  connect(): void {
    this.closed = false;
    this.open();
  }

  private open(): void {
    if (this.closed) return;
    const token = this.opts.getToken?.() ?? null;
    const es = this.opts.factory(this.opts.url, {
      lastEventId: this.lastEventId,
      token,
    });
    this.es = es;

    es.onopen = () => {
      this.attempt = 0;
      this.opts.onOpen?.();
    };

    es.addEventListener("message", (ev) => {
      if (ev.lastEventId) this.lastEventId = ev.lastEventId;
      const parsed = this.parse(ev.data);
      if (parsed) this.opts.onEvent(parsed);
    });

    es.onerror = (ev) => {
      this.opts.onError?.(ev);
      this.scheduleReconnect();
    };
  }

  private parse(data: unknown): SseEvent | null {
    if (typeof data !== "string") return null;
    try {
      const obj = JSON.parse(data) as Partial<SseEvent>;
      if (obj && typeof obj.type === "string") {
        return {
          type: obj.type,
          data: obj.data,
          timestamp: obj.timestamp,
          user_id: obj.user_id,
        };
      }
    } catch {
      /* ignore malformed frames */
    }
    return null;
  }

  private scheduleReconnect(): void {
    this.es?.close();
    this.es = null;
    if (this.closed) return;
    const base = this.opts.reconnectBaseMs ?? 1000;
    const max = this.opts.reconnectMaxMs ?? 30_000;
    const delay = Math.min(max, base * 2 ** this.attempt);
    this.attempt += 1;
    this.timer = setTimeout(() => this.open(), delay);
  }

  /** Permanently close the stream and cancel any pending reconnect. */
  close(): void {
    this.closed = true;
    if (this.timer) {
      clearTimeout(this.timer);
      this.timer = null;
    }
    this.es?.close();
    this.es = null;
  }
}
