/** TR-08-007 — configuration via EXPO_PUBLIC_* env vars; missing var fails clearly. */
import { loadConfig, MissingConfigError, REQUIRED_ENV_VARS } from './env';

const FULL_ENV = {
  EXPO_PUBLIC_API_BASE_URL: 'https://api.example.com/',
  EXPO_PUBLIC_OIDC_ISSUER: 'https://idp.example.com/',
  EXPO_PUBLIC_OIDC_CLIENT_ID: 'superapp-mobile',
  EXPO_PUBLIC_OIDC_SCHEME: 'superapp',
};

describe('loadConfig (TR-08-007)', () => {
  it('parses a complete environment', () => {
    const config = loadConfig(FULL_ENV);
    expect(config.apiBaseUrl).toBe('https://api.example.com'); // trailing slash stripped
    expect(config.oidc.issuer).toBe('https://idp.example.com');
    expect(config.oidc.clientId).toBe('superapp-mobile');
    expect(config.oidc.scheme).toBe('superapp');
    expect(config.oidc.redirectPath).toBe('oauthredirect');
    expect(config.oidc.scopes).toEqual(['openid', 'profile', 'email', 'offline_access']);
  });

  it('respects overrides for redirect path and scopes', () => {
    const config = loadConfig({
      ...FULL_ENV,
      EXPO_PUBLIC_OIDC_REDIRECT_PATH: 'cb',
      EXPO_PUBLIC_OIDC_SCOPES: 'openid email',
    });
    expect(config.oidc.redirectPath).toBe('cb');
    expect(config.oidc.scopes).toEqual(['openid', 'email']);
  });

  it('throws MissingConfigError listing every missing required var', () => {
    expect(() => loadConfig({})).toThrow(MissingConfigError);
    try {
      loadConfig({});
      throw new Error('should have thrown');
    } catch (e) {
      expect(e).toBeInstanceOf(MissingConfigError);
      expect((e as MissingConfigError).missing).toEqual([...REQUIRED_ENV_VARS]);
      expect((e as MissingConfigError).message).toContain('EXPO_PUBLIC_API_BASE_URL');
    }
  });

  it('treats blank/whitespace values as missing', () => {
    expect(() => loadConfig({ ...FULL_ENV, EXPO_PUBLIC_OIDC_CLIENT_ID: '   ' })).toThrow(
      /EXPO_PUBLIC_OIDC_CLIENT_ID/,
    );
  });
});
