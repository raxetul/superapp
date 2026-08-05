/** TR-08-006 — foreground SSE client: parse, reconnect on drop, resume, foreground. */
import { SseClient, type SseConnectParams, type SseConnection, type SseTransport } from './sseClient';

const tick = () => new Promise<void>((resolve) => setImmediate(resolve));

interface FakeConn extends SseConnection {
  closed: boolean;
  params: SseConnectParams;
}

function makeTransport() {
  const connections: FakeConn[] = [];
  const transport: SseTransport = {
    connect(params) {
      const conn: FakeConn = {
        closed: false,
        params,
        close() {
          this.closed = true;
        },
      };
      connections.push(conn);
      return conn;
    },
  };
  return { transport, connections };
}

function makeScheduler() {
  const jobs: Array<{ fn: () => void; cancelled: boolean }> = [];
  const schedule = (fn: () => void) => {
    const job = { fn, cancelled: false };
    jobs.push(job);
    return () => {
      job.cancelled = true;
    };
  };
  const flush = () => {
    const pending = jobs.filter((j) => !j.cancelled);
    jobs.length = 0;
    pending.forEach((j) => j.fn());
  };
  return { schedule, flush, jobs };
}

describe('SseClient (TR-08-006)', () => {
  it('connects with bearer auth and marks the stream open', async () => {
    const { transport, connections } = makeTransport();
    const client = new SseClient({
      url: 'https://api.test/api/v1/events/stream',
      transport,
      getAccessToken: async () => 'tok',
      onEvent: jest.fn(),
    });

    await client.start();
    expect(connections).toHaveLength(1);
    expect(connections[0].params.headers.Authorization).toBe('Bearer tok');
    expect(connections[0].params.headers.Accept).toBe('text/event-stream');

    connections[0].params.onOpen();
    expect(client.currentState).toBe('open');
  });

  it('parses event frames and tracks the last event id', async () => {
    const { transport, connections } = makeTransport();
    const onEvent = jest.fn();
    const client = new SseClient({
      url: 'u',
      transport,
      getAccessToken: async () => null,
      onEvent,
    });
    await client.start();
    connections[0].params.onFrame({
      id: '42',
      data: JSON.stringify({ type: 'message.created', data: { id: 1 }, timestamp: 't', user_id: 'u1' }),
    });
    expect(onEvent).toHaveBeenCalledWith({
      type: 'message.created',
      data: { id: 1 },
      timestamp: 't',
      user_id: 'u1',
    });
    expect(client.lastId).toBe('42');
  });

  it('reconnects on drop and resumes via Last-Event-ID', async () => {
    const { transport, connections } = makeTransport();
    const { schedule, flush } = makeScheduler();
    const client = new SseClient({
      url: 'u',
      transport,
      getAccessToken: async () => 'tok',
      onEvent: jest.fn(),
      backoffMs: [100, 200],
      schedule,
    });

    await client.start();
    connections[0].params.onOpen();
    connections[0].params.onFrame({ id: '7', data: '{"type":"x","data":{},"timestamp":"t"}' });

    connections[0].params.onError(new Error('network drop'));
    expect(connections[0].closed).toBe(true);
    expect(client.currentState).toBe('reconnecting');

    flush();
    await tick();

    expect(connections).toHaveLength(2);
    expect(connections[1].params.headers['Last-Event-ID']).toBe('7');
    expect(connections[1].params.lastEventId).toBe('7');
  });

  it('reconnects immediately on app-foreground when the stream is down', async () => {
    const { transport, connections } = makeTransport();
    const { schedule } = makeScheduler();
    const client = new SseClient({
      url: 'u',
      transport,
      getAccessToken: async () => 'tok',
      onEvent: jest.fn(),
      schedule,
    });

    await client.start();
    connections[0].params.onError(new Error('drop')); // now waiting on backoff timer
    expect(client.currentState).toBe('reconnecting');

    client.onForeground(); // should not wait for the timer
    await tick();
    expect(connections).toHaveLength(2);
  });

  it('stop() cancels reconnection and closes the connection', async () => {
    const { transport, connections } = makeTransport();
    const { schedule, jobs } = makeScheduler();
    const client = new SseClient({
      url: 'u',
      transport,
      getAccessToken: async () => 'tok',
      onEvent: jest.fn(),
      schedule,
    });
    await client.start();
    client.stop();
    expect(connections[0].closed).toBe(true);
    expect(client.currentState).toBe('closed');
    // A foreground event after stop must not reconnect.
    client.onForeground();
    await tick();
    expect(connections).toHaveLength(1);
    expect(jobs.filter((j) => !j.cancelled)).toHaveLength(0);
  });
});
