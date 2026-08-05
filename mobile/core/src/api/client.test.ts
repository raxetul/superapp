/** TR-08-002 — typed API client: parse success envelope, surface RFC 9457. */
import { ApiClient, ApiError, type TokenProvider } from './client';

function jsonResponse(body: unknown, init: { status?: number; contentType?: string } = {}): Response {
  const status = init.status ?? 200;
  return new Response(body === undefined ? '' : JSON.stringify(body), {
    status,
    headers: { 'content-type': init.contentType ?? 'application/json' },
  });
}

describe('ApiClient (TR-08-002)', () => {
  it('unwraps data from the house success envelope', async () => {
    const fetchImpl = jest.fn(async () =>
      jsonResponse({ success: true, data: { id: 7, name: 'ok' } }),
    );
    const client = new ApiClient({ baseUrl: 'https://api.test', fetchImpl: fetchImpl as unknown as typeof fetch });

    const data = await client.get<{ id: number; name: string }>('/widgets/7');
    expect(data).toEqual({ id: 7, name: 'ok' });
    expect(fetchImpl).toHaveBeenCalledWith(
      'https://api.test/widgets/7',
      expect.objectContaining({ method: 'GET' }),
    );
  });

  it('exposes pagination via requestEnvelope', async () => {
    const fetchImpl = jest.fn(async () =>
      jsonResponse({
        success: true,
        data: [1, 2, 3],
        pagination: { page: 1, per_page: 3, total_items: 9, total_pages: 3 },
      }),
    );
    const client = new ApiClient({ baseUrl: 'https://api.test', fetchImpl: fetchImpl as unknown as typeof fetch });
    const env = await client.requestEnvelope<number[]>('GET', '/items');
    expect(env.pagination?.total_pages).toBe(3);
    expect(env.data).toEqual([1, 2, 3]);
  });

  it('throws a typed ApiError from an RFC 9457 problem+json body', async () => {
    const problem = {
      type: 'https://superapp/errors/validation',
      title: 'Unprocessable Entity',
      status: 422,
      detail: 'validation failed',
      errors: [{ pointer: '/email', detail: 'must be a valid email' }],
    };
    const fetchImpl = jest.fn(async () =>
      jsonResponse(problem, { status: 422, contentType: 'application/problem+json' }),
    );
    const client = new ApiClient({ baseUrl: 'https://api.test', fetchImpl: fetchImpl as unknown as typeof fetch });

    await expect(client.get('/x')).rejects.toBeInstanceOf(ApiError);
    try {
      await client.get('/x');
    } catch (e) {
      const err = e as ApiError;
      expect(err.httpStatus).toBe(422);
      expect(err.problem.type).toBe('https://superapp/errors/validation');
      expect(err.fieldErrors).toEqual([{ pointer: '/email', detail: 'must be a valid email' }]);
      expect(err.message).toBe('validation failed');
    }
  });

  it('synthesizes a problem when the error body is not problem+json', async () => {
    const fetchImpl = jest.fn(async () => new Response('boom', { status: 500, statusText: 'Internal Server Error' }));
    const client = new ApiClient({ baseUrl: 'https://api.test', fetchImpl: fetchImpl as unknown as typeof fetch });
    await expect(client.get('/x')).rejects.toMatchObject({ httpStatus: 500 });
  });

  it('attaches the bearer token from the token provider', async () => {
    const fetchImpl = jest.fn(async (_url: string, _init: RequestInit) =>
      jsonResponse({ success: true, data: null }),
    );
    const tokenProvider: TokenProvider = { getAccessToken: async () => 'tok-123' };
    const client = new ApiClient({
      baseUrl: 'https://api.test',
      fetchImpl: fetchImpl as unknown as typeof fetch,
      tokenProvider,
    });

    await client.get('/me');
    const headers = (fetchImpl.mock.calls[0][1] as RequestInit).headers as Record<string, string>;
    expect(headers.Authorization).toBe('Bearer tok-123');
  });

  it('refreshes once on 401 then retries with the new token (TR-08-003)', async () => {
    const responses = [
      jsonResponse({ title: 'Unauthorized', status: 401 }, { status: 401, contentType: 'application/problem+json' }),
      jsonResponse({ success: true, data: { ok: true } }),
    ];
    const fetchImpl = jest.fn(async (_url: string, _init: RequestInit) => responses.shift()!);
    const refresh = jest.fn(async () => 'fresh-token');
    let token = 'stale';
    const tokenProvider: TokenProvider = {
      getAccessToken: async () => token,
      refresh: async () => {
        token = await refresh();
        return token;
      },
    };
    const client = new ApiClient({
      baseUrl: 'https://api.test',
      fetchImpl: fetchImpl as unknown as typeof fetch,
      tokenProvider,
    });

    const data = await client.get<{ ok: boolean }>('/secure');
    expect(data).toEqual({ ok: true });
    expect(refresh).toHaveBeenCalledTimes(1);
    expect(fetchImpl).toHaveBeenCalledTimes(2);
    const retryHeaders = (fetchImpl.mock.calls[1][1] as RequestInit).headers as Record<string, string>;
    expect(retryHeaders.Authorization).toBe('Bearer fresh-token');
  });

  it('does not loop when refresh fails', async () => {
    const fetchImpl = jest.fn(async () =>
      jsonResponse({ title: 'Unauthorized', status: 401 }, { status: 401, contentType: 'application/problem+json' }),
    );
    const tokenProvider: TokenProvider = {
      getAccessToken: async () => 'stale',
      refresh: async () => null,
    };
    const client = new ApiClient({
      baseUrl: 'https://api.test',
      fetchImpl: fetchImpl as unknown as typeof fetch,
      tokenProvider,
    });
    await expect(client.get('/secure')).rejects.toBeInstanceOf(ApiError);
    expect(fetchImpl).toHaveBeenCalledTimes(1);
  });
});
