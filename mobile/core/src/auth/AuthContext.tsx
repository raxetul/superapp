/**
 * Authentication state (FR-08-001 login/logout, FR-08-002 role exposure).
 *
 * Holds the session, drives the OIDC login/logout lifecycle, persists tokens
 * to secure storage, and exposes an {@link ApiClient} whose bearer token is
 * always the current access token (refreshed transparently on 401 / expiry).
 */
import React, { createContext, useCallback, useContext, useEffect, useMemo, useRef, useState } from 'react';
import { ApiClient, type TokenProvider } from '@/api/client';
import { AuthApi, isAdminRole, type Me } from '@/api/endpoints';
import { loadConfig, type AppConfig } from '@/config/env';
import { OidcClient } from './oidc';
import { isAccessTokenExpired, tokenStorage, type StoredTokens } from './tokenStorage';

export type AuthStatus = 'loading' | 'authenticated' | 'unauthenticated';

export interface AuthContextValue {
  status: AuthStatus;
  user: Me | null;
  isAdmin: boolean;
  error: string | null;
  login: () => Promise<void>;
  logout: () => Promise<void>;
  /** Shared API client carrying the live bearer token. */
  api: ApiClient;
}

export interface TokenStore {
  save(tokens: StoredTokens): Promise<void>;
  load(): Promise<StoredTokens | null>;
  clear(): Promise<void>;
}

export interface AuthProviderProps {
  children: React.ReactNode;
  /** Overridable for tests; defaults to {@link loadConfig}. */
  config?: AppConfig;
  oidcClient?: OidcClient;
  storage?: TokenStore;
  /** Build the AuthApi from a token provider (overridable for tests). */
  createApiClient?: (tokenProvider: TokenProvider, config: AppConfig) => ApiClient;
}

const AuthContext = createContext<AuthContextValue | null>(null);

function defaultCreateApiClient(tokenProvider: TokenProvider, config: AppConfig): ApiClient {
  return new ApiClient({ baseUrl: config.apiBaseUrl, tokenProvider });
}

export function AuthProvider({
  children,
  config,
  oidcClient,
  storage = tokenStorage,
  createApiClient = defaultCreateApiClient,
}: AuthProviderProps) {
  const appConfig = useMemo(() => config ?? loadConfig(), [config]);
  const oidc = useMemo(() => oidcClient ?? new OidcClient(appConfig.oidc), [oidcClient, appConfig]);

  const [status, setStatus] = useState<AuthStatus>('loading');
  const [user, setUser] = useState<Me | null>(null);
  const [error, setError] = useState<string | null>(null);

  // Tokens are held in a ref so the TokenProvider closure always reads the
  // latest value without rebuilding the ApiClient.
  const tokensRef = useRef<StoredTokens | null>(null);

  const doRefresh = useCallback(async (): Promise<string | null> => {
    const current = tokensRef.current;
    if (!current?.refreshToken) return null;
    try {
      const next = await oidc.refresh(current.refreshToken);
      tokensRef.current = next;
      await storage.save(next);
      return next.accessToken;
    } catch {
      return null;
    }
  }, [oidc, storage]);

  const tokenProvider = useMemo<TokenProvider>(
    () => ({
      getAccessToken: async () => {
        const current = tokensRef.current;
        if (!current) return null;
        if (isAccessTokenExpired(current) && current.refreshToken) {
          return doRefresh();
        }
        return current.accessToken;
      },
      refresh: doRefresh,
    }),
    [doRefresh],
  );

  const api = useMemo(() => createApiClient(tokenProvider, appConfig), [createApiClient, tokenProvider, appConfig]);
  const authApi = useMemo(() => new AuthApi(api), [api]);

  const hydrate = useCallback(async () => {
    const tokens = await storage.load();
    if (!tokens) {
      tokensRef.current = null;
      setUser(null);
      setStatus('unauthenticated');
      return;
    }
    tokensRef.current = tokens;
    try {
      const me = await authApi.me();
      setUser(me);
      setStatus('authenticated');
    } catch {
      // Token no longer valid — drop it and fall back to unauthenticated.
      tokensRef.current = null;
      await storage.clear();
      setUser(null);
      setStatus('unauthenticated');
    }
  }, [authApi, storage]);

  useEffect(() => {
    void hydrate();
  }, [hydrate]);

  const login = useCallback(async () => {
    setError(null);
    try {
      const tokens = await oidc.login();
      tokensRef.current = tokens;
      await storage.save(tokens);
      const me = await authApi.me();
      setUser(me);
      setStatus('authenticated');
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Login failed');
      tokensRef.current = null;
      await storage.clear();
      setUser(null);
      setStatus('unauthenticated');
      throw e;
    }
  }, [oidc, storage, authApi]);

  const logout = useCallback(async () => {
    tokensRef.current = null;
    await storage.clear();
    setUser(null);
    setError(null);
    setStatus('unauthenticated');
  }, [storage]);

  const value = useMemo<AuthContextValue>(
    () => ({
      status,
      user,
      isAdmin: isAdminRole(user?.role),
      error,
      login,
      logout,
      api,
    }),
    [status, user, error, login, logout, api],
  );

  return <AuthContext.Provider value={value}>{children}</AuthContext.Provider>;
}

export function useAuth(): AuthContextValue {
  const ctx = useContext(AuthContext);
  if (!ctx) {
    throw new Error('useAuth must be used within an <AuthProvider>');
  }
  return ctx;
}
