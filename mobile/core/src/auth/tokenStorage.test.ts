/** TR-08-003 — tokens persisted in secure storage; expiry math. */
import * as SecureStore from 'expo-secure-store';
import { computeExpiresAt, isAccessTokenExpired, tokenStorage } from './tokenStorage';

const KEY = 'superapp.auth.tokens';

describe('tokenStorage (TR-08-003)', () => {
  it('saves tokens into expo-secure-store (Keychain/Keystore)', async () => {
    await tokenStorage.save({ accessToken: 'a', refreshToken: 'r', expiresAt: 123 });
    expect(SecureStore.setItemAsync).toHaveBeenCalledWith(KEY, expect.any(String));
    const stored = await tokenStorage.load();
    expect(stored).toEqual({ accessToken: 'a', refreshToken: 'r', expiresAt: 123 });
  });

  it('returns null when nothing is stored', async () => {
    expect(await tokenStorage.load()).toBeNull();
  });

  it('clears tokens', async () => {
    await tokenStorage.save({ accessToken: 'a' });
    await tokenStorage.clear();
    expect(SecureStore.deleteItemAsync).toHaveBeenCalledWith(KEY);
    expect(await tokenStorage.load()).toBeNull();
  });

  it('recovers from a corrupt entry', async () => {
    await SecureStore.setItemAsync(KEY, 'not-json');
    expect(await tokenStorage.load()).toBeNull();
  });
});

describe('expiry helpers', () => {
  it('computes absolute expiry from issuedAt + expiresIn (seconds → ms)', () => {
    expect(computeExpiresAt(1_000, 3_600)).toBe(4_600_000);
    expect(computeExpiresAt(undefined, 3_600)).toBeUndefined();
  });

  it('treats tokens within the skew window as expired', () => {
    const now = 1_000_000;
    expect(isAccessTokenExpired({ expiresAt: now + 60_000 }, now)).toBe(false);
    expect(isAccessTokenExpired({ expiresAt: now + 10_000 }, now)).toBe(true); // inside 30s skew
    expect(isAccessTokenExpired({ expiresAt: now - 1 }, now)).toBe(true);
    expect(isAccessTokenExpired({ expiresAt: undefined }, now)).toBe(false);
  });
});
