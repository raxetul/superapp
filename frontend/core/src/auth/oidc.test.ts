import { describe, it, expect, vi } from "vitest";
import {
  beginLogin,
  buildLogoutUrl,
  exchangeCode,
  OidcCallbackError,
  refreshTokens,
} from "./oidc";
import type { OidcConfig } from "@/config/env";
import type { FetchLike } from "@/api/client";

const oidc: OidcConfig = {
  issuer: "http://idp/auth/v1",
  clientId: "superapp-web",
  redirectUri: "http://app/auth/callback",
  scope: "openid profile email",
  postLogoutRedirectUri: "http://app/",
};

function tokenResponse(body: unknown, ok = true): Response {
  return new Response(JSON.stringify(body), {
    status: ok ? 200 : 400,
    headers: { "content-type": "application/json" },
  });
}

describe("TR-07-003 OIDC Auth Code + PKCE", () => {
  it("builds an authorize URL carrying PKCE + state (S256)", async () => {
    const { authorizeUrl, pkce } = await beginLogin(oidc, { returnTo: "/admin" });
    const url = new URL(authorizeUrl);
    expect(url.origin + url.pathname).toBe("http://idp/auth/v1/oidc/authorize");
    expect(url.searchParams.get("response_type")).toBe("code");
    expect(url.searchParams.get("client_id")).toBe("superapp-web");
    expect(url.searchParams.get("redirect_uri")).toBe("http://app/auth/callback");
    expect(url.searchParams.get("code_challenge_method")).toBe("S256");
    expect(url.searchParams.get("code_challenge")).toBeTruthy();
    expect(url.searchParams.get("state")).toBe(pkce.state);
    expect(pkce.returnTo).toBe("/admin");
    expect(pkce.codeVerifier.length).toBeGreaterThanOrEqual(43);
  });

  it("exchanges a code for tokens and computes absolute expiry", async () => {
    const fetchImpl = vi.fn<FetchLike>(async () =>
      tokenResponse({
        access_token: "at",
        refresh_token: "rt",
        id_token: "it",
        token_type: "Bearer",
        expires_in: 3600,
      }),
    );
    const tokens = await exchangeCode(
      oidc,
      { code: "the-code", state: "st" },
      { codeVerifier: "verif", state: "st" },
      fetchImpl,
      1_000,
    );
    expect(tokens.accessToken).toBe("at");
    expect(tokens.refreshToken).toBe("rt");
    expect(tokens.expiresAt).toBe(1_000 + 3600 * 1000);
    const [tokenUrl, init] = fetchImpl.mock.calls[0];
    expect(tokenUrl).toBe("http://idp/auth/v1/oidc/token");
    const sent = new URLSearchParams((init as RequestInit).body as string);
    expect(sent.get("grant_type")).toBe("authorization_code");
    expect(sent.get("code_verifier")).toBe("verif");
    expect(sent.get("code")).toBe("the-code");
  });

  it("rejects a callback whose state does not match the stored PKCE state", async () => {
    await expect(
      exchangeCode(
        oidc,
        { code: "c", state: "attacker" },
        { codeVerifier: "v", state: "expected" },
        vi.fn(),
      ),
    ).rejects.toBeInstanceOf(OidcCallbackError);
  });

  it("rejects a callback that carries an error param", async () => {
    await expect(
      exchangeCode(
        oidc,
        { code: null, state: null, error: "access_denied" },
        { codeVerifier: "v", state: "s" },
        vi.fn(),
      ),
    ).rejects.toThrow(/access_denied/);
  });

  it("refreshes tokens and preserves the refresh token when omitted", async () => {
    const fetchImpl = vi.fn<FetchLike>(async () =>
      tokenResponse({
        access_token: "at2",
        token_type: "Bearer",
        expires_in: 600,
      }),
    );
    const tokens = await refreshTokens(oidc, "old-rt", fetchImpl, 5_000);
    expect(tokens.accessToken).toBe("at2");
    expect(tokens.refreshToken).toBe("old-rt");
    expect(tokens.expiresAt).toBe(5_000 + 600 * 1000);
    const sent = new URLSearchParams(
      (fetchImpl.mock.calls[0][1] as RequestInit).body as string,
    );
    expect(sent.get("grant_type")).toBe("refresh_token");
  });

  it("throws when refresh fails", async () => {
    const fetchImpl = vi.fn(async () => tokenResponse({}, false));
    await expect(refreshTokens(oidc, "rt", fetchImpl)).rejects.toBeInstanceOf(
      OidcCallbackError,
    );
  });

  it("builds an RP-initiated logout URL with post-logout redirect", () => {
    const url = new URL(buildLogoutUrl(oidc, "id-tok"));
    expect(url.origin + url.pathname).toBe("http://idp/auth/v1/oidc/logout");
    expect(url.searchParams.get("post_logout_redirect_uri")).toBe("http://app/");
    expect(url.searchParams.get("id_token_hint")).toBe("id-tok");
  });
});
