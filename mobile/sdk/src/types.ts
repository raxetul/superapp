/**
 * TR-09-003 — the mobile module host interface.
 *
 * Mirrors `mobile/core/src/modules/types.ts` field-for-field: an object built
 * against these types satisfies `mobile/core`'s `ModuleRegistry` exactly (see
 * the integration test in `mobile/core/src/modules/`), so a module author
 * never has to depend on the host's own source tree.
 *
 * `ModuleContext.api` is intentionally typed `unknown` here rather than
 * `mobile/core`'s own `ApiClient` — the SDK doesn't depend on the host's
 * internal API client shape; a module receives it at `initialize` time and
 * narrows it as needed.
 */
import type { ComponentType } from "react";

/** Services the core exposes to a module during its lifecycle. */
export interface ModuleContext {
  api: unknown;
  /** Permissions granted to the current user. */
  grantedPermissions: string[];
}

export interface ModuleScreen {
  name: string;
  component: ComponentType<Record<string, unknown>>;
  title?: string;
  /** Permission required to see this screen; omitted ⇒ always visible. */
  requiredPermission?: string;
}

export interface ModuleDefinition {
  id: string;
  title?: string;
  /** Permissions this module declares/consumes. */
  permissions: string[];
  screens: ModuleScreen[];
  components?: Record<string, ComponentType<Record<string, unknown>>>;
  /**
   * SDK major version this module was built against (TR-09-005), e.g.
   * `"1.0.0"`. Omit for pre-SDK modules; when present, an incompatible major
   * version is rejected by the host's `ModuleRegistry.register`.
   */
  sdkVersion?: string;
  initialize?: (ctx: ModuleContext) => void | Promise<void>;
  cleanup?: () => void | Promise<void>;
}

/** A screen resolved against a specific granted-permission set. */
export interface ResolvedScreen {
  moduleId: string;
  screen: ModuleScreen;
}
