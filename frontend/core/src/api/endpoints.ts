/**
 * Typed facade over {@link ApiClient} for the SuperApp core endpoints
 * (backend contract, Phase 7 task brief).
 */
import type { ApiClient } from "./client";
import type {
  AllowlistEntry,
  Capabilities,
  CurrentUser,
  Role,
} from "./types";

export interface SuperappApi {
  getCapabilities(): Promise<Capabilities>;
  getMe(): Promise<CurrentUser>;
  listAllowlist(): Promise<AllowlistEntry[]>;
  addToAllowlist(email: string): Promise<AllowlistEntry>;
  setUserRole(email: string, role: Role): Promise<AllowlistEntry>;
}

export function createApi(client: ApiClient): SuperappApi {
  return {
    getCapabilities: () =>
      client.get<Capabilities>("/api/v1/auth/capabilities"),
    getMe: () => client.get<CurrentUser>("/api/v1/auth/me"),
    listAllowlist: () =>
      client.get<AllowlistEntry[]>("/api/v1/admin/allowlist"),
    addToAllowlist: (email: string) =>
      client.post<AllowlistEntry>("/api/v1/admin/allowlist", { email }),
    setUserRole: (email: string, role: Role) =>
      client.put<AllowlistEntry>("/api/v1/admin/users/role", { email, role }),
  };
}
