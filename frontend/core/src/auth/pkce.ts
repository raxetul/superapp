/**
 * TR-07-003 — PKCE (RFC 7636) helpers for the OIDC Authorization Code flow.
 *
 * Uses the Web Crypto API (`crypto.getRandomValues` / `crypto.subtle`), which
 * is available in browsers and in the Node/jsdom test runtime.
 */

const CHARS =
  "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-._~";

/** base64url-encode raw bytes (no padding), per RFC 7636. */
export function base64UrlEncode(bytes: ArrayBuffer | Uint8Array): string {
  const view = bytes instanceof Uint8Array ? bytes : new Uint8Array(bytes);
  let binary = "";
  for (const b of view) binary += String.fromCharCode(b);
  return btoa(binary).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
}

/** Generate a high-entropy PKCE `code_verifier` (43–128 chars). */
export function generateCodeVerifier(length = 64): string {
  const clamped = Math.min(128, Math.max(43, length));
  const random = new Uint8Array(clamped);
  crypto.getRandomValues(random);
  let out = "";
  for (const n of random) out += CHARS[n % CHARS.length];
  return out;
}

/** Compute the S256 `code_challenge` for a verifier. */
export async function codeChallengeS256(verifier: string): Promise<string> {
  const data = new TextEncoder().encode(verifier);
  const digest = await crypto.subtle.digest("SHA-256", data);
  return base64UrlEncode(digest);
}

/** Generate a random URL-safe opaque value (for `state` / `nonce`). */
export function randomState(bytes = 16): string {
  const buf = new Uint8Array(bytes);
  crypto.getRandomValues(buf);
  return base64UrlEncode(buf);
}
