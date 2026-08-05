/**
 * Runtime configuration (TR-08-007).
 *
 * All configuration is supplied through `EXPO_PUBLIC_*` environment variables,
 * which Expo inlines into the JS bundle at build time. A missing *required*
 * variable is a hard, clearly-reported failure rather than a silent
 * `undefined` that surfaces later as an opaque network error.
 */

export interface OidcConfig {
  /** Rauthy issuer URL (OIDC discovery is derived from this). */
  issuer: string;
  /** Public OAuth client id registered in Rauthy for the mobile app. */
  clientId: string;
  /** Custom URL scheme used for the deep-link redirect (e.g. `superapp`). */
  scheme: string;
  /** Redirect path appended to the scheme (default `oauthredirect`). */
  redirectPath: string;
  /** OAuth scopes requested during the Authorization Code + PKCE flow. */
  scopes: string[];
}

export interface AppConfig {
  /** Base URL of the backend core API, without a trailing slash. */
  apiBaseUrl: string;
  oidc: OidcConfig;
}

/** The environment variables that must be present for the app to boot. */
export const REQUIRED_ENV_VARS = [
  'EXPO_PUBLIC_API_BASE_URL',
  'EXPO_PUBLIC_OIDC_ISSUER',
  'EXPO_PUBLIC_OIDC_CLIENT_ID',
  'EXPO_PUBLIC_OIDC_SCHEME',
] as const;

/** Thrown when one or more required `EXPO_PUBLIC_*` variables are absent. */
export class MissingConfigError extends Error {
  constructor(public readonly missing: string[]) {
    super(
      `Missing required environment variable(s): ${missing.join(', ')}. ` +
        'Set them in mobile/core/.env (see .env.example) or your build profile.',
    );
    this.name = 'MissingConfigError';
  }
}

type Env = Record<string, string | undefined>;

function isBlank(value: string | undefined): boolean {
  return value == null || value.trim() === '';
}

/**
 * Parse and validate configuration from an environment map (defaults to
 * `process.env`). Throws {@link MissingConfigError} listing every missing
 * required variable so the failure is actionable.
 */
export function loadConfig(env: Env = process.env): AppConfig {
  const missing = REQUIRED_ENV_VARS.filter((key) => isBlank(env[key]));
  if (missing.length > 0) {
    throw new MissingConfigError([...missing]);
  }

  const scopes = (env.EXPO_PUBLIC_OIDC_SCOPES ?? 'openid profile email offline_access')
    .split(/\s+/)
    .filter(Boolean);

  return {
    apiBaseUrl: env.EXPO_PUBLIC_API_BASE_URL!.replace(/\/+$/, ''),
    oidc: {
      issuer: env.EXPO_PUBLIC_OIDC_ISSUER!.replace(/\/+$/, ''),
      clientId: env.EXPO_PUBLIC_OIDC_CLIENT_ID!,
      scheme: env.EXPO_PUBLIC_OIDC_SCHEME!,
      redirectPath: env.EXPO_PUBLIC_OIDC_REDIRECT_PATH ?? 'oauthredirect',
      scopes,
    },
  };
}
