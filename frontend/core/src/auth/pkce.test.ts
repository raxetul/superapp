import { describe, it, expect } from "vitest";
import {
  base64UrlEncode,
  codeChallengeS256,
  generateCodeVerifier,
  randomState,
} from "./pkce";

describe("TR-07-003 PKCE helpers", () => {
  it("base64url-encodes without padding or +/ characters", () => {
    const enc = base64UrlEncode(new Uint8Array([255, 254, 253, 0, 1]));
    expect(enc).not.toMatch(/[+/=]/);
  });

  it("generates a verifier of RFC 7636 length using the unreserved charset", () => {
    const v = generateCodeVerifier(64);
    expect(v.length).toBe(64);
    expect(v).toMatch(/^[A-Za-z0-9\-._~]+$/);
    expect(generateCodeVerifier(10).length).toBe(43); // clamped up to minimum
  });

  it("produces distinct verifiers and states across calls", () => {
    expect(generateCodeVerifier()).not.toBe(generateCodeVerifier());
    expect(randomState()).not.toBe(randomState());
  });

  it("computes a known S256 challenge (RFC 7636 Appendix B vector)", async () => {
    // The spec's example verifier and its expected S256 challenge.
    const verifier = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
    const challenge = await codeChallengeS256(verifier);
    expect(challenge).toBe("E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM");
  });
});
