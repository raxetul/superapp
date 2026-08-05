/** FR-08-001 (login/logout) + FR-08-002 (role) — AuthContext behaviour. */
import React from 'react';
import { act, renderHook, waitFor } from '@testing-library/react-native';
import { AuthProvider, useAuth } from './AuthContext';
import {
  makeFakeOidc,
  makeMeApiClientFactory,
  makeMemoryStore,
  testConfig,
} from '@/test-utils';
import type { Role } from '@/api/endpoints';
import type { StoredTokens } from './tokenStorage';

function wrapperFor(opts: {
  role?: Role;
  store?: ReturnType<typeof makeMemoryStore>;
  oidc?: ReturnType<typeof makeFakeOidc>;
}) {
  const store = opts.store ?? makeMemoryStore();
  const oidc = opts.oidc ?? makeFakeOidc();
  const createApiClient = makeMeApiClientFactory(opts.role ?? 'user');
  const wrapper = ({ children }: { children: React.ReactNode }) => (
    <AuthProvider config={testConfig} oidcClient={oidc} storage={store} createApiClient={createApiClient}>
      {children}
    </AuthProvider>
  );
  return { wrapper, store };
}

describe('AuthContext (FR-08-001, FR-08-002)', () => {
  it('starts unauthenticated when no tokens are stored', async () => {
    const { wrapper } = wrapperFor({});
    const { result } = await renderHook(() => useAuth(), { wrapper });
    await waitFor(() => expect(result.current.status).toBe('unauthenticated'));
    expect(result.current.user).toBeNull();
    expect(result.current.isAdmin).toBe(false);
  });

  it('hydrates an authenticated session from stored tokens', async () => {
    const store = makeMemoryStore({ accessToken: 'a', refreshToken: 'r' } as StoredTokens);
    const { wrapper } = wrapperFor({ role: 'admin', store });
    const { result } = await renderHook(() => useAuth(), { wrapper });
    await waitFor(() => expect(result.current.status).toBe('authenticated'));
    expect(result.current.user?.email).toBe('ada@company.com');
    expect(result.current.isAdmin).toBe(true);
  });

  it('logs in via OIDC, persists tokens and exposes the user role (FR-08-001)', async () => {
    const { wrapper, store } = wrapperFor({ role: 'user' });
    const { result } = await renderHook(() => useAuth(), { wrapper });
    await waitFor(() => expect(result.current.status).toBe('unauthenticated'));

    await act(async () => {
      await result.current.login();
    });

    expect(result.current.status).toBe('authenticated');
    expect(result.current.user?.role).toBe('user');
    expect(result.current.isAdmin).toBe(false);
    // Tokens landed in secure storage.
    expect(store.current?.accessToken).toBe('access-1');
  });

  it('logs out, clearing session and stored tokens (FR-08-001)', async () => {
    const store = makeMemoryStore({ accessToken: 'a', refreshToken: 'r' } as StoredTokens);
    const { wrapper } = wrapperFor({ role: 'admin', store });
    const { result } = await renderHook(() => useAuth(), { wrapper });
    await waitFor(() => expect(result.current.status).toBe('authenticated'));

    await act(async () => {
      await result.current.logout();
    });

    expect(result.current.status).toBe('unauthenticated');
    expect(result.current.user).toBeNull();
    expect(store.current).toBeNull();
  });

  it('surfaces a login failure and stays unauthenticated', async () => {
    const oidc = makeFakeOidc(testConfig, { type: 'cancel' });
    const { wrapper } = wrapperFor({ oidc });
    const { result } = await renderHook(() => useAuth(), { wrapper });
    await waitFor(() => expect(result.current.status).toBe('unauthenticated'));

    await act(async () => {
      await expect(result.current.login()).rejects.toBeTruthy();
    });
    expect(result.current.status).toBe('unauthenticated');
    expect(result.current.error).toBeTruthy();
  });
});
