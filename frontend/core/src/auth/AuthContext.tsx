/**
 * FR-07-001 / FR-07-002 / TR-07-003 — Auth session context.
 *
 * Owns the token set, the current user, and public capabilities; exposes
 * `login` (redirect to Rauthy), `completeLogin` (callback code exchange) and
 * `logout` (clear session + RP-initiated logout). All external seams — the
 * `fetch` transport, token `Storage`, the browser `redirect`, and the clock —
 * are injectable so the provider is unit-testable without a real IdP.
 */
import * as React from "react";
import { ApiClient, type FetchLike } from "@/api/client";
import { createApi, type SuperappApi } from "@/api/endpoints";
import type { AppConfig } from "@/config/env";
import type { Capabilities, CurrentUser } from "@/api/types";
import {
  beginLogin,
  buildLogoutUrl,
  exchangeCode,
  refreshTokens,
} from "./oidc";
import { isExpired, TokenStore, type TokenSet } from "./tokenStore";

export type AuthStatus = "loading" | "authenticated" | "unauthenticated";

export interface AuthContextValue {
  status: AuthStatus;
  user: CurrentUser | null;
  capabilities: Capabilities | null;
  isAdmin: boolean;
  /** Token-injecting API facade (shared with the auth session). */
  api: SuperappApi;
  login: (returnTo?: string) => Promise<void>;
  logout: () => Promise<void>;
  completeLogin: (search: string) => Promise<string>;
  reloadUser: () => Promise<void>;
}

const AuthContext = React.createContext<AuthContextValue | null>(null);

export interface AuthProviderProps {
  config: AppConfig;
  children: React.ReactNode;
  /** Test seams. */
  fetchImpl?: FetchLike;
  storage?: Storage;
  redirect?: (url: string) => void;
  now?: () => number;
  /** Inject a stub API in tests. */
  apiOverride?: SuperappApi;
}

const defaultRedirect = (url: string) => {
  window.location.assign(url);
};

export function AuthProvider({
  config,
  children,
  fetchImpl,
  storage,
  redirect = defaultRedirect,
  now = () => Date.now(),
  apiOverride,
}: AuthProviderProps): React.JSX.Element {
  const store = React.useMemo(() => new TokenStore(storage), [storage]);
  const tokensRef = React.useRef<TokenSet | null>(store.load());

  const [status, setStatus] = React.useState<AuthStatus>("loading");
  const [user, setUser] = React.useState<CurrentUser | null>(null);
  const [capabilities, setCapabilities] = React.useState<Capabilities | null>(
    null,
  );

  const api = React.useMemo<SuperappApi>(() => {
    if (apiOverride) return apiOverride;
    const client = new ApiClient({
      baseUrl: config.apiBaseUrl,
      fetchImpl,
      getToken: () => tokensRef.current?.accessToken ?? null,
      onUnauthorized: async () => {
        const rt = tokensRef.current?.refreshToken;
        if (!rt) return false;
        try {
          const next = await refreshTokens(
            config.oidc,
            rt,
            fetchImpl ?? ((i, init) => fetch(i, init)),
            now(),
          );
          tokensRef.current = next;
          store.save(next);
          return true;
        } catch {
          tokensRef.current = null;
          store.clear();
          return false;
        }
      },
    });
    return createApi(client);
  }, [apiOverride, config, fetchImpl, now, store]);

  const reloadUser = React.useCallback(async () => {
    const me = await api.getMe();
    setUser(me);
  }, [api]);

  // Bootstrap: load capabilities (public) + restore session if tokens exist.
  React.useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const caps = await api.getCapabilities();
        if (!cancelled) setCapabilities(caps);
      } catch {
        /* capabilities are best-effort */
      }

      const tokens = tokensRef.current;
      if (!tokens) {
        if (!cancelled) setStatus("unauthenticated");
        return;
      }
      try {
        if (isExpired(tokens, now()) && tokens.refreshToken) {
          const next = await refreshTokens(
            config.oidc,
            tokens.refreshToken,
            fetchImpl ?? ((i, init) => fetch(i, init)),
            now(),
          );
          tokensRef.current = next;
          store.save(next);
        }
        const me = await api.getMe();
        if (!cancelled) {
          setUser(me);
          setStatus("authenticated");
        }
      } catch {
        tokensRef.current = null;
        store.clear();
        if (!cancelled) setStatus("unauthenticated");
      }
    })();
    return () => {
      cancelled = true;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [api]);

  const login = React.useCallback(
    async (returnTo?: string) => {
      const { authorizeUrl, pkce } = await beginLogin(config.oidc, {
        returnTo,
      });
      store.savePkce(pkce);
      redirect(authorizeUrl);
    },
    [config, redirect, store],
  );

  const completeLogin = React.useCallback(
    async (search: string): Promise<string> => {
      const params = new URLSearchParams(search);
      const pkce = store.loadPkce();
      const tokens = await exchangeCode(
        config.oidc,
        {
          code: params.get("code"),
          state: params.get("state"),
          error: params.get("error"),
        },
        pkce,
        fetchImpl ?? ((i, init) => fetch(i, init)),
        now(),
      );
      tokensRef.current = tokens;
      store.save(tokens);
      store.clearPkce();
      await reloadUser();
      setStatus("authenticated");
      return pkce?.returnTo ?? "/";
    },
    [config, fetchImpl, now, reloadUser, store],
  );

  const logout = React.useCallback(async () => {
    const idToken = tokensRef.current?.idToken;
    tokensRef.current = null;
    store.clear();
    setUser(null);
    setStatus("unauthenticated");
    redirect(buildLogoutUrl(config.oidc, idToken));
  }, [config, redirect, store]);

  const value = React.useMemo<AuthContextValue>(
    () => ({
      status,
      user,
      capabilities,
      isAdmin: user?.role === "admin",
      api,
      login,
      logout,
      completeLogin,
      reloadUser,
    }),
    [status, user, capabilities, api, login, logout, completeLogin, reloadUser],
  );

  return <AuthContext.Provider value={value}>{children}</AuthContext.Provider>;
}

/** Access the auth session. Throws if used outside {@link AuthProvider}. */
export function useAuth(): AuthContextValue {
  const ctx = React.useContext(AuthContext);
  if (!ctx) {
    throw new Error("useAuth must be used within an AuthProvider");
  }
  return ctx;
}
