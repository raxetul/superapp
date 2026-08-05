import { describe, it, expect, beforeEach } from "vitest";
import { TokenStore, isExpired, EXPIRY_SKEW_MS } from "./tokenStore";

class MemoryStorage implements Storage {
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

describe("TR-07-003 token store", () => {
  let store: TokenStore;

  beforeEach(() => {
    store = new TokenStore(new MemoryStorage());
  });

  it("round-trips a token set and clears it", () => {
    store.save({ accessToken: "a", refreshToken: "r", expiresAt: 123 });
    expect(store.load()).toEqual({
      accessToken: "a",
      refreshToken: "r",
      expiresAt: 123,
    });
    store.clear();
    expect(store.load()).toBeNull();
  });

  it("persists and clears PKCE state independently", () => {
    store.savePkce({ codeVerifier: "v", state: "s", returnTo: "/admin" });
    expect(store.loadPkce()?.state).toBe("s");
    store.clearPkce();
    expect(store.loadPkce()).toBeNull();
  });

  it("treats null / near-expiry tokens as expired (with skew)", () => {
    expect(isExpired(null)).toBe(true);
    const now = 1_000_000;
    expect(isExpired({ accessToken: "a", expiresAt: now + 60_000 }, now)).toBe(
      false,
    );
    // within the skew window -> considered expired so we refresh early
    expect(
      isExpired({ accessToken: "a", expiresAt: now + EXPIRY_SKEW_MS - 1 }, now),
    ).toBe(true);
  });
});
