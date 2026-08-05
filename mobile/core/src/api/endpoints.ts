/**
 * Typed backend endpoints (API v1). Thin, typed wrappers over {@link ApiClient}
 * that return already-unwrapped payloads.
 */
import type { ApiClient } from './client';

export type Role = 'admin' | 'user' | (string & {});

/** `GET /api/v1/auth/capabilities` */
export interface Capabilities {
  self_registration_enabled: boolean;
  oidc_configured: boolean;
}

/** `GET /api/v1/auth/me` */
export interface Me {
  pid: string;
  email: string;
  name: string;
  role: Role;
}

/** An allow-listed email entry. */
export interface AllowlistEntry {
  email: string;
  role?: Role;
}

/** Normalizes a role string and reports admin status case-insensitively. */
export function isAdminRole(role: Role | null | undefined): boolean {
  return typeof role === 'string' && role.trim().toLowerCase() === 'admin';
}

export class AuthApi {
  constructor(private readonly client: ApiClient) {}

  /** Public capabilities probe (no auth required). */
  capabilities(): Promise<Capabilities> {
    return this.client.get<Capabilities>('/api/v1/auth/capabilities', { auth: false });
  }

  /** Current authenticated user (Bearer). */
  me(): Promise<Me> {
    return this.client.get<Me>('/api/v1/auth/me');
  }
}

export class AdminApi {
  constructor(private readonly client: ApiClient) {}

  /** `GET /api/v1/admin/allowlist` */
  listAllowlist(): Promise<AllowlistEntry[]> {
    return this.client.get<AllowlistEntry[]>('/api/v1/admin/allowlist');
  }

  /** `POST /api/v1/admin/allowlist` */
  addAllowlist(email: string, role?: Role): Promise<AllowlistEntry> {
    return this.client.post<AllowlistEntry>('/api/v1/admin/allowlist', { email, role });
  }

  /** `PUT /api/v1/admin/users/role` */
  setUserRole(email: string, role: Role): Promise<Me> {
    return this.client.put<Me>('/api/v1/admin/users/role', { email, role });
  }
}
