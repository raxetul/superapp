/**
 * Secure token storage (TR-08-003).
 *
 * Tokens live in the platform secure enclave via `expo-secure-store`
 * (Keychain on iOS, Keystore-backed encrypted storage on Android) — never in
 * AsyncStorage or plain files.
 */
import * as SecureStore from 'expo-secure-store';

const TOKENS_KEY = 'superapp.auth.tokens';

export interface StoredTokens {
  accessToken: string;
  refreshToken?: string;
  idToken?: string;
  /** Absolute expiry of the access token, epoch milliseconds. */
  expiresAt?: number;
}

/** Clock skew (ms) treated as "already expired" so refresh happens early. */
const EXPIRY_SKEW_MS = 30_000;

export const tokenStorage = {
  async save(tokens: StoredTokens): Promise<void> {
    await SecureStore.setItemAsync(TOKENS_KEY, JSON.stringify(tokens));
  },

  async load(): Promise<StoredTokens | null> {
    const raw = await SecureStore.getItemAsync(TOKENS_KEY);
    if (!raw) return null;
    try {
      return JSON.parse(raw) as StoredTokens;
    } catch {
      // Corrupt entry — treat as absent and clear it.
      await SecureStore.deleteItemAsync(TOKENS_KEY);
      return null;
    }
  },

  async clear(): Promise<void> {
    await SecureStore.deleteItemAsync(TOKENS_KEY);
  },
};

/** True when the access token is missing an expiry or is within the skew of it. */
export function isAccessTokenExpired(
  tokens: Pick<StoredTokens, 'expiresAt'>,
  now: number = Date.now(),
): boolean {
  if (tokens.expiresAt == null) return false;
  return now >= tokens.expiresAt - EXPIRY_SKEW_MS;
}

/** Compute an absolute expiry (ms) from OAuth `issuedAt`/`expiresIn` (seconds). */
export function computeExpiresAt(
  issuedAtSeconds: number | undefined,
  expiresInSeconds: number | undefined,
): number | undefined {
  if (issuedAtSeconds == null || expiresInSeconds == null) return undefined;
  return (issuedAtSeconds + expiresInSeconds) * 1000;
}
