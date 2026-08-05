/**
 * OIDC Authorization Code + PKCE via the system browser (TR-08-003, FR-08-001).
 *
 * The flow is: OIDC discovery → build an `AuthRequest` with PKCE → open the
 * system browser (`promptAsync`) → receive the authorization code on the
 * deep-link redirect → exchange the code (with the PKCE verifier) for tokens.
 * Refresh uses the OAuth refresh grant.
 *
 * Every call to `expo-auth-session` goes through the injectable
 * {@link AuthSessionPort} so the flow is unit-testable without a browser.
 */
import * as AuthSession from 'expo-auth-session';
import type { OidcConfig } from '@/config/env';
import { computeExpiresAt, type StoredTokens } from './tokenStorage';

export interface DiscoveryLike {
  authorizationEndpoint?: string;
  tokenEndpoint?: string;
  endSessionEndpoint?: string;
  revocationEndpoint?: string;
}

export type AuthResultType = 'success' | 'cancel' | 'dismiss' | 'error' | 'locked';

export interface AuthSessionResultLike {
  type: AuthResultType;
  params?: Record<string, string>;
  error?: unknown;
}

export interface AuthRequestLike {
  codeVerifier?: string;
  promptAsync(discovery: DiscoveryLike): Promise<AuthSessionResultLike>;
}

export interface TokenResponseLike {
  accessToken: string;
  refreshToken?: string;
  idToken?: string;
  expiresIn?: number;
  issuedAt?: number;
}

export interface CreateAuthRequestConfig {
  clientId: string;
  scopes: string[];
  redirectUri: string;
  usePKCE: boolean;
}

/** The subset of `expo-auth-session` the OIDC client depends on. */
export interface AuthSessionPort {
  makeRedirectUri(opts: { scheme: string; path?: string }): string;
  fetchDiscoveryAsync(issuer: string): Promise<DiscoveryLike>;
  createAuthRequest(config: CreateAuthRequestConfig): AuthRequestLike;
  exchangeCodeAsync(
    config: {
      clientId: string;
      code: string;
      redirectUri: string;
      extraParams?: Record<string, string>;
    },
    discovery: DiscoveryLike,
  ): Promise<TokenResponseLike>;
  refreshAsync(
    config: { clientId: string; refreshToken: string; scopes?: string[] },
    discovery: DiscoveryLike,
  ): Promise<TokenResponseLike>;
}

/** Default port backed by the real `expo-auth-session` module. */
export const defaultAuthSessionPort: AuthSessionPort = {
  makeRedirectUri: (opts) => AuthSession.makeRedirectUri(opts),
  fetchDiscoveryAsync: (issuer) => AuthSession.fetchDiscoveryAsync(issuer),
  createAuthRequest: (config) =>
    new AuthSession.AuthRequest({
      clientId: config.clientId,
      scopes: config.scopes,
      redirectUri: config.redirectUri,
      usePKCE: config.usePKCE,
      responseType: AuthSession.ResponseType.Code,
    }) as unknown as AuthRequestLike,
  exchangeCodeAsync: (config, discovery) =>
    AuthSession.exchangeCodeAsync(config, discovery) as unknown as Promise<TokenResponseLike>,
  refreshAsync: (config, discovery) =>
    AuthSession.refreshAsync(config, discovery) as unknown as Promise<TokenResponseLike>,
};

/** Raised when the browser flow does not complete successfully. */
export class OidcError extends Error {
  constructor(
    public readonly reason: AuthResultType,
    message?: string,
  ) {
    super(message ?? `OIDC flow did not succeed: ${reason}`);
    this.name = 'OidcError';
  }
}

/** Map an OAuth token response to our secure-storage shape. */
export function tokenResponseToStored(
  token: TokenResponseLike,
  fallbackRefreshToken?: string,
): StoredTokens {
  return {
    accessToken: token.accessToken,
    refreshToken: token.refreshToken ?? fallbackRefreshToken,
    idToken: token.idToken,
    expiresAt: computeExpiresAt(token.issuedAt, token.expiresIn),
  };
}

export class OidcClient {
  private discoveryPromise?: Promise<DiscoveryLike>;

  constructor(
    private readonly config: OidcConfig,
    private readonly port: AuthSessionPort = defaultAuthSessionPort,
  ) {}

  redirectUri(): string {
    return this.port.makeRedirectUri({
      scheme: this.config.scheme,
      path: this.config.redirectPath,
    });
  }

  /** Cached OIDC discovery document. */
  discovery(): Promise<DiscoveryLike> {
    this.discoveryPromise ??= this.port.fetchDiscoveryAsync(this.config.issuer);
    return this.discoveryPromise;
  }

  /** Run the interactive login (system browser + PKCE) and return tokens. */
  async login(): Promise<StoredTokens> {
    const discovery = await this.discovery();
    const redirectUri = this.redirectUri();
    const request = this.port.createAuthRequest({
      clientId: this.config.clientId,
      scopes: this.config.scopes,
      redirectUri,
      usePKCE: true,
    });

    const result = await request.promptAsync(discovery);
    if (result.type !== 'success' || !result.params?.code) {
      throw new OidcError(result.type);
    }

    const token = await this.port.exchangeCodeAsync(
      {
        clientId: this.config.clientId,
        code: result.params.code,
        redirectUri,
        extraParams: request.codeVerifier ? { code_verifier: request.codeVerifier } : undefined,
      },
      discovery,
    );

    return tokenResponseToStored(token);
  }

  /** Exchange a refresh token for a fresh access token. */
  async refresh(refreshToken: string): Promise<StoredTokens> {
    const discovery = await this.discovery();
    const token = await this.port.refreshAsync(
      { clientId: this.config.clientId, refreshToken, scopes: this.config.scopes },
      discovery,
    );
    return tokenResponseToStored(token, refreshToken);
  }
}
