/**
 * TR-07-007 — Environment configuration.
 *
 * All runtime configuration is supplied via `VITE_*` variables. A missing
 * *required* variable fails fast with a clear, aggregated error so a
 * mis-deployed build surfaces the problem immediately instead of failing
 * obscurely later (e.g. a blank OIDC redirect).
 *
 * `loadConfig` takes the raw env record as an argument (rather than reading
 * `import.meta.env` directly) so it is fully unit-testable without a bundler.
 */

/** OIDC (Rauthy) client settings for the SPA. */
export interface OidcConfig {
  readonly issuer: string;
  readonly clientId: string;
  readonly redirectUri: string;
  readonly scope: string;
  readonly postLogoutRedirectUri: string;
}

/** Fully-resolved, validated frontend configuration. */
export interface AppConfig {
  readonly apiBaseUrl: string;
  readonly oidc: OidcConfig;
}

/** Raw environment shape (subset of `import.meta.env`). */
export type RawEnv = Record<string, string | undefined>;

/** Thrown when one or more required `VITE_*` variables are missing. */
export class ConfigError extends Error {
  readonly missing: readonly string[];
  constructor(missing: readonly string[]) {
    super(
      `Missing required environment variable(s): ${missing.join(", ")}. ` +
        `Set them as VITE_* variables (see .env.example).`,
    );
    this.name = "ConfigError";
    this.missing = missing;
  }
}

const REQUIRED = [
  "VITE_API_BASE_URL",
  "VITE_OIDC_ISSUER",
  "VITE_OIDC_CLIENT_ID",
  "VITE_OIDC_REDIRECT_URI",
] as const;

function stripTrailingSlash(value: string): string {
  return value.replace(/\/+$/, "");
}

/**
 * Validate and materialize the {@link AppConfig} from a raw env record.
 * @throws {ConfigError} if any required variable is absent or blank.
 */
export function loadConfig(env: RawEnv): AppConfig {
  const missing = REQUIRED.filter((key) => {
    const v = env[key];
    return v === undefined || v.trim() === "";
  });
  if (missing.length > 0) {
    throw new ConfigError(missing);
  }

  return {
    apiBaseUrl: stripTrailingSlash(env.VITE_API_BASE_URL!),
    oidc: {
      issuer: stripTrailingSlash(env.VITE_OIDC_ISSUER!),
      clientId: env.VITE_OIDC_CLIENT_ID!,
      redirectUri: env.VITE_OIDC_REDIRECT_URI!,
      scope: env.VITE_OIDC_SCOPE?.trim() || "openid profile email",
      postLogoutRedirectUri:
        env.VITE_OIDC_POST_LOGOUT_REDIRECT_URI?.trim() ||
        env.VITE_OIDC_REDIRECT_URI!.replace(/\/auth\/callback\/?$/, "/"),
    },
  };
}

let cached: AppConfig | undefined;

/** Lazily-resolved singleton config from `import.meta.env`. */
export function getConfig(): AppConfig {
  if (!cached) {
    cached = loadConfig(import.meta.env as unknown as RawEnv);
  }
  return cached;
}
