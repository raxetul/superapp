/**
 * Shared test harness (not a test suite). Renders children inside a real
 * {@link AuthProvider} driven by a stub API and a pre-seeded in-memory token
 * store, wrapped in a MemoryRouter. Lets tests assert real routing/guard
 * behaviour with deterministic auth state.
 */
import { render, type RenderResult } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { vi } from "vitest";
import { AuthProvider } from "@/auth/AuthContext";
import { loadConfig } from "@/config/env";
import { TokenStore } from "@/auth/tokenStore";
import type { SuperappApi } from "@/api/endpoints";
import type { Capabilities, CurrentUser } from "@/api/types";

export class MemoryStorage implements Storage {
  private map = new Map<string, string>();
  get length() {
    return this.map.size;
  }
  clear() {
    this.map.clear();
  }
  getItem(k: string) {
    return this.map.get(k) ?? null;
  }
  key(i: number) {
    return [...this.map.keys()][i] ?? null;
  }
  removeItem(k: string) {
    this.map.delete(k);
  }
  setItem(k: string, v: string) {
    this.map.set(k, v);
  }
}

export const testConfig = loadConfig({
  VITE_API_BASE_URL: "http://api",
  VITE_OIDC_ISSUER: "http://idp/auth/v1",
  VITE_OIDC_CLIENT_ID: "superapp-web",
  VITE_OIDC_REDIRECT_URI: "http://app/auth/callback",
});

export function stubApi(over: Partial<SuperappApi> = {}): SuperappApi {
  return {
    getCapabilities: async (): Promise<Capabilities> => ({
      self_registration_enabled: false,
      oidc_configured: true,
    }),
    getMe: async (): Promise<CurrentUser> => ({
      pid: "p1",
      email: "user@buyutech.com.tr",
      name: "Test User",
      role: "user",
    }),
    listAllowlist: async () => [],
    addToAllowlist: async (email) => ({ email, role: "user" }),
    setUserRole: async (email, role) => ({ email, role }),
    ...over,
  };
}

export interface RenderAppOptions {
  api?: SuperappApi;
  authenticated?: boolean;
  initialEntries?: string[];
  redirect?: (url: string) => void;
}

export function makeStorage(authenticated: boolean): MemoryStorage {
  const storage = new MemoryStorage();
  if (authenticated) {
    new TokenStore(storage).save({
      accessToken: "at",
      refreshToken: "rt",
      idToken: "it",
      expiresAt: Date.now() + 3_600_000,
    });
  }
  return storage;
}

export function renderWithProviders(
  ui: React.ReactNode,
  opts: RenderAppOptions = {},
): RenderResult & { redirect: (url: string) => void } {
  const redirect = opts.redirect ?? vi.fn();
  const storage = makeStorage(opts.authenticated ?? false);
  const result = render(
    <MemoryRouter initialEntries={opts.initialEntries ?? ["/"]}>
      <AuthProvider
        config={testConfig}
        storage={storage}
        redirect={redirect}
        apiOverride={opts.api ?? stubApi()}
      >
        {ui}
      </AuthProvider>
    </MemoryRouter>,
  );
  return { ...result, redirect };
}
