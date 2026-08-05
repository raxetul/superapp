/**
 * Endpoint wiring + role helper.
 * Supports FR-08-002 (role), FR-08-003 (admin allowlist/roles), FR-08-004
 * (capabilities), TR-08-003 (me).
 */
import { ApiClient } from './client';
import { AdminApi, AuthApi, isAdminRole } from './endpoints';

function makeClient() {
  const calls: Array<{ url: string; init: RequestInit }> = [];
  const fetchImpl = jest.fn(async (url: string, init: RequestInit) => {
    calls.push({ url, init });
    // Echo a benign success payload for every call.
    return new Response(JSON.stringify({ success: true, data: {} }), {
      status: 200,
      headers: { 'content-type': 'application/json' },
    });
  });
  const client = new ApiClient({
    baseUrl: 'https://api.test',
    fetchImpl: fetchImpl as unknown as typeof fetch,
    tokenProvider: { getAccessToken: async () => 'tok' },
  });
  return { client, calls };
}

describe('isAdminRole (FR-08-002)', () => {
  it('recognizes admin case-insensitively', () => {
    expect(isAdminRole('admin')).toBe(true);
    expect(isAdminRole('Admin')).toBe(true);
    expect(isAdminRole('ADMIN')).toBe(true);
  });
  it('rejects non-admin / empty', () => {
    expect(isAdminRole('user')).toBe(false);
    expect(isAdminRole(null)).toBe(false);
    expect(isAdminRole(undefined)).toBe(false);
  });
});

describe('AuthApi', () => {
  it('probes capabilities without auth (FR-08-004)', async () => {
    const { client, calls } = makeClient();
    await new AuthApi(client).capabilities();
    expect(calls[0].url).toBe('https://api.test/api/v1/auth/capabilities');
    const headers = calls[0].init.headers as Record<string, string>;
    expect(headers.Authorization).toBeUndefined();
  });

  it('fetches the current user with bearer auth (TR-08-003)', async () => {
    const { client, calls } = makeClient();
    await new AuthApi(client).me();
    expect(calls[0].url).toBe('https://api.test/api/v1/auth/me');
    expect((calls[0].init.headers as Record<string, string>).Authorization).toBe('Bearer tok');
  });
});

describe('AdminApi (FR-08-003)', () => {
  it('lists the allowlist', async () => {
    const { client, calls } = makeClient();
    await new AdminApi(client).listAllowlist();
    expect(calls[0].url).toBe('https://api.test/api/v1/admin/allowlist');
    expect(calls[0].init.method).toBe('GET');
  });

  it('adds an allowlist entry by email', async () => {
    const { client, calls } = makeClient();
    await new AdminApi(client).addAllowlist('a@b.com');
    expect(calls[0].init.method).toBe('POST');
    expect(calls[0].url).toBe('https://api.test/api/v1/admin/allowlist');
    expect(JSON.parse(calls[0].init.body as string)).toMatchObject({ email: 'a@b.com' });
  });

  it('updates a user role', async () => {
    const { client, calls } = makeClient();
    await new AdminApi(client).setUserRole('a@b.com', 'admin');
    expect(calls[0].init.method).toBe('PUT');
    expect(calls[0].url).toBe('https://api.test/api/v1/admin/users/role');
    expect(JSON.parse(calls[0].init.body as string)).toEqual({ email: 'a@b.com', role: 'admin' });
  });
});
