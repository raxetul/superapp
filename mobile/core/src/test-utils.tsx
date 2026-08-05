/* istanbul ignore file */
/** Shared test helpers: provider wrapper + fakes for auth/config. */
import React from 'react';
import { render } from '@testing-library/react-native';
import { ApiClient } from '@/api/client';
import type { AppConfig } from '@/config/env';
import { OidcClient, type AuthSessionPort, type AuthSessionResultLike } from '@/auth/oidc';
import type { TokenStore } from '@/auth/AuthContext';
import type { StoredTokens } from '@/auth/tokenStorage';
import type { Role } from '@/api/endpoints';
import { UiProviders } from '@/ui/providers';

export const testConfig: AppConfig = {
  apiBaseUrl: 'https://api.test',
  oidc: {
    issuer: 'https://idp.test',
    clientId: 'superapp-mobile',
    scheme: 'superapp',
    redirectPath: 'oauthredirect',
    scopes: ['openid', 'profile', 'email', 'offline_access'],
  },
};

export function renderWithProviders(ui: React.ReactElement) {
  return render(<UiProviders>{ui}</UiProviders>);
}

/** In-memory token store. */
export function makeMemoryStore(initial: StoredTokens | null = null): TokenStore & { current: StoredTokens | null } {
  const store = {
    current: initial,
    async save(tokens: StoredTokens) {
      store.current = tokens;
    },
    async load() {
      return store.current;
    },
    async clear() {
      store.current = null;
    },
  };
  return store;
}

/** OidcClient wired to a scripted browser result. */
export function makeFakeOidc(
  config: AppConfig = testConfig,
  result: AuthSessionResultLike = { type: 'success', params: { code: 'code-1' } },
): OidcClient {
  const port: AuthSessionPort = {
    makeRedirectUri: ({ scheme, path }) => `${scheme}://${path}`,
    fetchDiscoveryAsync: async (issuer) => ({ tokenEndpoint: `${issuer}/token` }),
    createAuthRequest: () => ({ codeVerifier: 'verifier', promptAsync: async () => result }),
    exchangeCodeAsync: async () => ({
      accessToken: 'access-1',
      refreshToken: 'refresh-1',
      issuedAt: Math.floor(Date.now() / 1000),
      expiresIn: 3_600,
    }),
    refreshAsync: async () => ({
      accessToken: 'access-2',
      issuedAt: Math.floor(Date.now() / 1000),
      expiresIn: 3_600,
    }),
  };
  return new OidcClient(config.oidc, port);
}

/** Build an ApiClient whose fetch answers `/auth/me` with the given role. */
export function makeMeApiClientFactory(role: Role = 'user', name = 'Ada Lovelace') {
  return (tokenProvider: { getAccessToken(): Promise<string | null> }) =>
    new ApiClient({
      baseUrl: testConfig.apiBaseUrl,
      tokenProvider,
      fetchImpl: (async (url: string) => {
        const body =
          typeof url === 'string' && url.endsWith('/api/v1/auth/me')
            ? { success: true, data: { pid: 'p1', email: 'ada@company.com', name, role } }
            : { success: true, data: null };
        return new Response(JSON.stringify(body), {
          status: 200,
          headers: { 'content-type': 'application/json' },
        });
      }) as unknown as typeof fetch,
    });
}
