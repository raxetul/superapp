import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import {
  SseClient,
  type ConnectContext,
  type EventSourceLike,
  type SseEvent,
} from "./sseClient";

class FakeEventSource implements EventSourceLike {
  onopen: ((ev: Event) => void) | null = null;
  onerror: ((ev: Event) => void) | null = null;
  closed = false;
  private listeners = new Map<string, (ev: MessageEvent) => void>();

  addEventListener(type: string, cb: (ev: MessageEvent) => void) {
    this.listeners.set(type, cb);
  }
  close() {
    this.closed = true;
  }
  emitOpen() {
    this.onopen?.(new Event("open"));
  }
  emitMessage(data: unknown, lastEventId = "") {
    this.listeners.get("message")?.({ data, lastEventId } as MessageEvent);
  }
  emitError() {
    this.onerror?.(new Event("error"));
  }
}

describe("TR-07-006 SSE client", () => {
  beforeEach(() => vi.useFakeTimers());
  afterEach(() => vi.useRealTimers());

  it("parses typed events and forwards them to the consumer", () => {
    const es = new FakeEventSource();
    const events: SseEvent[] = [];
    const client = new SseClient({
      url: "http://api/api/v1/events/stream",
      factory: () => es,
      onEvent: (e) => events.push(e),
      getToken: () => "tok",
    });
    client.connect();
    es.emitMessage(
      JSON.stringify({
        type: "message.created",
        data: { id: 7 },
        timestamp: "2026-07-11T00:00:00Z",
        user_id: "u1",
      }),
      "42",
    );
    expect(events).toHaveLength(1);
    expect(events[0].type).toBe("message.created");
    expect(events[0].user_id).toBe("u1");
    expect(client.currentLastEventId).toBe("42");
  });

  it("ignores malformed frames without throwing", () => {
    const es = new FakeEventSource();
    const onEvent = vi.fn();
    const client = new SseClient({
      url: "u",
      factory: () => es,
      onEvent,
    });
    client.connect();
    es.emitMessage("not json {");
    es.emitMessage(JSON.stringify({ nope: true }));
    expect(onEvent).not.toHaveBeenCalled();
  });

  it("reconnects on error and resumes with the last event id", () => {
    const created: { url: string; ctx: ConnectContext; es: FakeEventSource }[] =
      [];
    const factory = (url: string, ctx: ConnectContext) => {
      const es = new FakeEventSource();
      created.push({ url, ctx, es });
      return es;
    };
    const client = new SseClient({
      url: "http://api/stream",
      factory,
      onEvent: vi.fn(),
      getToken: () => "tok",
      reconnectBaseMs: 1000,
    });
    client.connect();
    expect(created).toHaveLength(1);
    expect(created[0].ctx.lastEventId).toBeUndefined();
    expect(created[0].ctx.token).toBe("tok");

    // Receive an event (advances resume cursor), then the stream errors.
    created[0].es.emitMessage(JSON.stringify({ type: "ping", data: 1 }), "99");
    created[0].es.emitError();
    expect(created[0].es.closed).toBe(true);

    // Backoff elapses -> a new connection resumes from Last-Event-ID 99.
    vi.advanceTimersByTime(1000);
    expect(created).toHaveLength(2);
    expect(created[1].ctx.lastEventId).toBe("99");
  });

  it("applies exponential backoff and stops after close()", () => {
    let count = 0;
    const sources: FakeEventSource[] = [];
    const factory = () => {
      count += 1;
      const es = new FakeEventSource();
      sources.push(es);
      return es;
    };
    const client = new SseClient({
      url: "u",
      factory,
      onEvent: vi.fn(),
      reconnectBaseMs: 1000,
      reconnectMaxMs: 30_000,
    });
    client.connect();
    sources[0].emitError();
    // first backoff is 1000ms; 999ms is not enough
    vi.advanceTimersByTime(999);
    expect(count).toBe(1);
    vi.advanceTimersByTime(1);
    expect(count).toBe(2);

    sources[1].emitError();
    // second backoff doubles to 2000ms
    vi.advanceTimersByTime(2000);
    expect(count).toBe(3);

    client.close();
    sources[2].emitError();
    vi.advanceTimersByTime(60_000);
    expect(count).toBe(3); // no further reconnects after close()
  });

  it("resets backoff after a successful open", () => {
    const sources: FakeEventSource[] = [];
    let count = 0;
    const client = new SseClient({
      url: "u",
      factory: () => {
        count += 1;
        const es = new FakeEventSource();
        sources.push(es);
        return es;
      },
      onEvent: vi.fn(),
      reconnectBaseMs: 1000,
    });
    client.connect();
    sources[0].emitError();
    vi.advanceTimersByTime(1000);
    expect(count).toBe(2);
    sources[1].emitOpen(); // success resets attempt counter
    sources[1].emitError();
    vi.advanceTimersByTime(1000); // back to base delay
    expect(count).toBe(3);
  });
});
