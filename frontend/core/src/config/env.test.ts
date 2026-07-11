import { describe, it, expect } from "vitest";
import { loadConfig, ConfigError, type RawEnv } from "./env";

const fullEnv: RawEnv = {
  VITE_API_BASE_URL: "http://localhost:5150/",
  VITE_OIDC_ISSUER: "http://localhost:8080/auth/v1/",
  VITE_OIDC_CLIENT_ID: "superapp-web",
  VITE_OIDC_REDIRECT_URI: "http://localhost:5173/auth/callback",
};

describe("TR-07-007 environment configuration", () => {
  it("resolves a typed config from VITE_* variables", () => {
    const cfg = loadConfig(fullEnv);
    expect(cfg.apiBaseUrl).toBe("http://localhost:5150");
    expect(cfg.oidc.issuer).toBe("http://localhost:8080/auth/v1");
    expect(cfg.oidc.clientId).toBe("superapp-web");
    expect(cfg.oidc.redirectUri).toBe("http://localhost:5173/auth/callback");
  });

  it("applies defaults for optional scope and post-logout uri", () => {
    const cfg = loadConfig(fullEnv);
    expect(cfg.oidc.scope).toBe("openid profile email");
    expect(cfg.oidc.postLogoutRedirectUri).toBe("http://localhost:5173/");
  });

  it("honours explicit optional overrides", () => {
    const cfg = loadConfig({
      ...fullEnv,
      VITE_OIDC_SCOPE: "openid email module:read",
      VITE_OIDC_POST_LOGOUT_REDIRECT_URI: "http://localhost:5173/bye",
    });
    expect(cfg.oidc.scope).toBe("openid email module:read");
    expect(cfg.oidc.postLogoutRedirectUri).toBe("http://localhost:5173/bye");
  });

  it("fails with a clear error naming every missing required variable", () => {
    expect(() => loadConfig({})).toThrow(ConfigError);
    try {
      loadConfig({ VITE_API_BASE_URL: "http://x" });
    } catch (e) {
      const err = e as ConfigError;
      expect(err.missing).toEqual([
        "VITE_OIDC_ISSUER",
        "VITE_OIDC_CLIENT_ID",
        "VITE_OIDC_REDIRECT_URI",
      ]);
      expect(err.message).toContain("VITE_OIDC_ISSUER");
      expect(err.message).toContain(".env.example");
    }
  });

  it("treats a blank/whitespace variable as missing", () => {
    expect(() => loadConfig({ ...fullEnv, VITE_OIDC_CLIENT_ID: "   " })).toThrow(
      /VITE_OIDC_CLIENT_ID/,
    );
  });
});
