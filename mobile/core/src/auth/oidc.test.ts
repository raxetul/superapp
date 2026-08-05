/** TR-08-003 / FR-08-001 — OIDC Auth Code + PKCE flow and refresh. */
import type { OidcConfig } from '@/config/env';
import {
  OidcClient,
  OidcError,
  tokenResponseToStored,
  type AuthSessionPort,
  type AuthSessionResultLike,
} from './oidc';

const config: OidcConfig = {
  issuer: 'https://idp.test',
  clientId: 'superapp-mobile',
  scheme: 'superapp',
  redirectPath: 'oauthredirect',
  scopes: ['openid', 'profile', 'email', 'offline_access'],
};

function makePort(overrides: Partial<AuthSessionPort> = {}): {
  port: AuthSessionPort;
  promptResult: { current: AuthSessionResultLike };
  exchange: jest.Mock;
  refresh: jest.Mock;
} {
  const promptResult = {
    current: { type: 'success', params: { code: 'auth-code-123' } } as AuthSessionResultLike,
  };
  const exchange = jest.fn(async () => ({
    accessToken: 'access-1',
    refreshToken: 'refresh-1',
    idToken: 'id-1',
    issuedAt: 1_000,
    expiresIn: 3_600,
  }));
  const refresh = jest.fn(async () => ({
    accessToken: 'access-2',
    issuedAt: 2_000,
    expiresIn: 3_600,
  }));

  const port: AuthSessionPort = {
    makeRedirectUri: ({ scheme, path }) => `${scheme}://${path}`,
    fetchDiscoveryAsync: async (issuer) => ({ tokenEndpoint: `${issuer}/token` }),
    createAuthRequest: () => ({
      codeVerifier: 'verifier-xyz',
      promptAsync: async () => promptResult.current,
    }),
    exchangeCodeAsync: exchange,
    refreshAsync: refresh,
    ...overrides,
  };
  return { port, promptResult, exchange, refresh };
}

describe('OidcClient (TR-08-003, FR-08-001)', () => {
  it('derives the deep-link redirect URI from scheme + path', () => {
    const { port } = makePort();
    expect(new OidcClient(config, port).redirectUri()).toBe('superapp://oauthredirect');
  });

  it('runs Auth Code + PKCE and exchanges the code with the verifier', async () => {
    const { port, exchange } = makePort();
    const tokens = await new OidcClient(config, port).login();

    expect(exchange).toHaveBeenCalledWith(
      expect.objectContaining({
        clientId: 'superapp-mobile',
        code: 'auth-code-123',
        redirectUri: 'superapp://oauthredirect',
        extraParams: { code_verifier: 'verifier-xyz' },
      }),
      expect.anything(),
    );
    expect(tokens).toEqual({
      accessToken: 'access-1',
      refreshToken: 'refresh-1',
      idToken: 'id-1',
      expiresAt: (1_000 + 3_600) * 1000,
    });
  });

  it('throws OidcError when the browser flow is cancelled', async () => {
    const { port, promptResult } = makePort();
    promptResult.current = { type: 'cancel' };
    await expect(new OidcClient(config, port).login()).rejects.toBeInstanceOf(OidcError);
    await expect(new OidcClient(config, port).login()).rejects.toMatchObject({ reason: 'cancel' });
  });

  it('refreshes tokens and preserves the old refresh token when a new one is absent', async () => {
    const { port } = makePort();
    const tokens = await new OidcClient(config, port).refresh('refresh-1');
    expect(tokens.accessToken).toBe('access-2');
    expect(tokens.refreshToken).toBe('refresh-1'); // carried forward
    expect(tokens.expiresAt).toBe((2_000 + 3_600) * 1000);
  });
});

describe('tokenResponseToStored', () => {
  it('omits expiry when the response lacks timing', () => {
    expect(tokenResponseToStored({ accessToken: 'a' })).toEqual({
      accessToken: 'a',
      refreshToken: undefined,
      idToken: undefined,
      expiresAt: undefined,
    });
  });
});
