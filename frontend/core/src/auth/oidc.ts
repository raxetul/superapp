/**
 * TR-07-003 / FR-07-001 — OIDC Authorization Code + PKCE flow against Rauthy.
 *
 * The frontend is a *public* OIDC client (no secret). The endpoints are
 * derived from the configured issuer following Rauthy's layout
 * (`<issuer>/oidc/{authorize,token,logout}`, where the issuer is typically
 * `https://host/auth/v1`).
 *
 * NOTE (verify against your Rauthy deployment): if a deployment exposes an
 * OIDC discovery document, prefer `<issuer>/.well-known/openid-configuration`.
 * The derived paths below match Rauthy's documented defaults.
 */
import type { OidcConfig } from "@/config/env";
import type { FetchLike } from "@/api/client";
import { codeChallengeS256, generateCodeVerifier, randomState } from "./pkce";
import type { PkceState, TokenSet } from "./tokenStore";

export interface OidcEndpoints {
  authorize: string;
  token: string;
  logout: string;
}

export function oidcEndpoints(issuer: string): OidcEndpoints {
  const base = issuer.replace(/\/+$/, "");
  return {
    authorize: `${base}/oidc/authorize`,
    token: `${base}/oidc/token`,
    logout: `${base}/oidc/logout`,
  };
}

/** Raw OIDC token endpoint response. */
interface TokenResponse {
  access_token: string;
  refresh_token?: string;
  id_token?: string;
  token_type: string;
  expires_in: number;
}

function toTokenSet(r: TokenResponse, now: number = Date.now()): TokenSet {
  return {
    accessToken: r.access_token,
    refreshToken: r.refresh_token,
    idToken: r.id_token,
    expiresAt: now + r.expires_in * 1000,
  };
}

export interface BeginLoginResult {
  authorizeUrl: string;
  pkce: PkceState;
}

/**
 * Build the authorize redirect URL and the PKCE state to persist before
 * navigating the browser to Rauthy.
 */
export async function beginLogin(
  oidc: OidcConfig,
  opts: { returnTo?: string } = {},
): Promise<BeginLoginResult> {
  const codeVerifier = generateCodeVerifier();
  const challenge = await codeChallengeS256(codeVerifier);
  const state = randomState();

  const params = new URLSearchParams({
    response_type: "code",
    client_id: oidc.clientId,
    redirect_uri: oidc.redirectUri,
    scope: oidc.scope,
    state,
    code_challenge: challenge,
    code_challenge_method: "S256",
  });

  return {
    authorizeUrl: `${oidcEndpoints(oidc.issuer).authorize}?${params.toString()}`,
    pkce: { codeVerifier, state, returnTo: opts.returnTo },
  };
}

/** Thrown when the callback query does not match the stored PKCE state. */
export class OidcCallbackError extends Error {}

/**
 * Exchange an authorization `code` for tokens, validating `state` against the
 * persisted PKCE state (CSRF protection).
 */
export async function exchangeCode(
  oidc: OidcConfig,
  query: { code: string | null; state: string | null; error?: string | null },
  pkce: PkceState | null,
  fetchImpl: FetchLike,
  now: number = Date.now(),
): Promise<TokenSet> {
  if (query.error) {
    throw new OidcCallbackError(`authorization error: ${query.error}`);
  }
  if (!query.code) {
    throw new OidcCallbackError("missing authorization code");
  }
  if (!pkce) {
    throw new OidcCallbackError("missing PKCE state");
  }
  if (!query.state || query.state !== pkce.state) {
    throw new OidcCallbackError("state mismatch");
  }

  const body = new URLSearchParams({
    grant_type: "authorization_code",
    code: query.code,
    redirect_uri: oidc.redirectUri,
    client_id: oidc.clientId,
    code_verifier: pkce.codeVerifier,
  });

  const res = await fetchImpl(oidcEndpoints(oidc.issuer).token, {
    method: "POST",
    headers: { "Content-Type": "application/x-www-form-urlencoded" },
    body: body.toString(),
  });
  if (!res.ok) {
    throw new OidcCallbackError(`token exchange failed (${res.status})`);
  }
  return toTokenSet((await res.json()) as TokenResponse, now);
}

/** Redeem a refresh token for a fresh token set. */
export async function refreshTokens(
  oidc: OidcConfig,
  refreshToken: string,
  fetchImpl: FetchLike,
  now: number = Date.now(),
): Promise<TokenSet> {
  const body = new URLSearchParams({
    grant_type: "refresh_token",
    refresh_token: refreshToken,
    client_id: oidc.clientId,
  });
  const res = await fetchImpl(oidcEndpoints(oidc.issuer).token, {
    method: "POST",
    headers: { "Content-Type": "application/x-www-form-urlencoded" },
    body: body.toString(),
  });
  if (!res.ok) {
    throw new OidcCallbackError(`refresh failed (${res.status})`);
  }
  const next = toTokenSet((await res.json()) as TokenResponse, now);
  // Some providers omit refresh_token on refresh; keep the previous one.
  if (!next.refreshToken) next.refreshToken = refreshToken;
  return next;
}

/** Build the RP-initiated logout (end-session) URL. */
export function buildLogoutUrl(oidc: OidcConfig, idToken?: string): string {
  const params = new URLSearchParams({
    post_logout_redirect_uri: oidc.postLogoutRedirectUri,
    client_id: oidc.clientId,
  });
  if (idToken) params.set("id_token_hint", idToken);
  return `${oidcEndpoints(oidc.issuer).logout}?${params.toString()}`;
}
