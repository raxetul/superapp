/**
 * Wire types mirroring the backend response contract
 * (`backend/core/src/response.rs`).
 */

/** Pagination metadata for collection responses. */
export interface Pagination {
  page: number;
  per_page: number;
  total_items: number;
  total_pages: number;
}

/** The house success envelope for every 2xx response. */
export interface SuccessEnvelope<T> {
  success: true;
  data: T;
  message?: string;
  pagination?: Pagination;
}

/** User roles recognised by the UI (authorization stays server-side). */
export type Role = "admin" | "user";

/** Public auth capabilities: `GET /api/v1/auth/capabilities`. */
export interface Capabilities {
  self_registration_enabled: boolean;
  oidc_configured: boolean;
}

/** Current authenticated principal: `GET /api/v1/auth/me`. */
export interface CurrentUser {
  pid: string;
  email: string;
  name: string;
  role: Role;
}

/** An allow-listed user record: `GET /api/v1/admin/allowlist`. */
export interface AllowlistEntry {
  email: string;
  role: Role;
}
