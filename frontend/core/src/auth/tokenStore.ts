/**
 * TR-07-003 — Token persistence.
 *
 * Stores the OIDC token set behind an injectable `Storage` (defaults to
 * `localStorage`) so it survives reloads and is unit-testable with an
 * in-memory stub.
 */

export interface TokenSet {
  accessToken: string;
  refreshToken?: string;
  idToken?: string;
  /** Absolute expiry in epoch milliseconds. */
  expiresAt: number;
}

/** Transient state carried across the authorize redirect. */
export interface PkceState {
  codeVerifier: string;
  state: string;
  /** Where to send the user after a successful callback. */
  returnTo?: string;
}

const TOKENS_KEY = "superapp.tokens";
const PKCE_KEY = "superapp.pkce";
/** Refresh a little before real expiry to avoid mid-flight 401s. */
export const EXPIRY_SKEW_MS = 30_000;

export class TokenStore {
  constructor(private readonly storage: Storage = localStorage) {}

  save(tokens: TokenSet): void {
    this.storage.setItem(TOKENS_KEY, JSON.stringify(tokens));
  }

  load(): TokenSet | null {
    const raw = this.storage.getItem(TOKENS_KEY);
    if (!raw) return null;
    try {
      return JSON.parse(raw) as TokenSet;
    } catch {
      return null;
    }
  }

  clear(): void {
    this.storage.removeItem(TOKENS_KEY);
    this.storage.removeItem(PKCE_KEY);
  }

  savePkce(state: PkceState): void {
    this.storage.setItem(PKCE_KEY, JSON.stringify(state));
  }

  loadPkce(): PkceState | null {
    const raw = this.storage.getItem(PKCE_KEY);
    if (!raw) return null;
    try {
      return JSON.parse(raw) as PkceState;
    } catch {
      return null;
    }
  }

  clearPkce(): void {
    this.storage.removeItem(PKCE_KEY);
  }
}

/** True when the token set is absent or within the refresh skew of expiry. */
export function isExpired(
  tokens: TokenSet | null,
  now: number = Date.now(),
  skewMs: number = EXPIRY_SKEW_MS,
): boolean {
  if (!tokens) return true;
  return now >= tokens.expiresAt - skewMs;
}
