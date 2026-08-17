/**
 * TR-09-002 — the web module host interface.
 *
 * Mirrors `frontend/core/src/modules/types.ts` field-for-field: an object
 * built against these types satisfies `frontend/core`'s `ModuleRegistry`
 * exactly (see the integration test in `frontend/core/src/modules/`), so a
 * module author never has to depend on the host's own source tree.
 */
import type { ComponentType } from "react";

/** Context handed to a module at initialization. */
export interface ModuleContext {
  /** API base URL for the module to call the core backend. */
  apiBaseUrl: string;
  /** Returns the current access token, or null. */
  getToken: () => string | null;
}

/** A route contributed by a module. */
export interface ModuleRoute {
  path: string;
  component: ComponentType;
  /** Permission required to see/reach this route; omit for always-visible. */
  permission?: string;
}

/** A navigation entry contributed by a module. */
export interface ModuleNavItem {
  label: string;
  to: string;
  permission?: string;
}

export interface FrontendModule {
  id: string;
  name: string;
  /** Permissions this module declares/consumes. */
  permissions?: string[];
  routes?: ModuleRoute[];
  nav?: ModuleNavItem[];
  /**
   * SDK major version this module was built against (TR-09-005), e.g.
   * `"1.0.0"`. Omit for pre-SDK modules; when present, an incompatible major
   * version is rejected by the host's `ModuleRegistry.register`.
   */
  sdkVersion?: string;
  initialize?: (ctx: ModuleContext) => void | Promise<void>;
  cleanup?: () => void | Promise<void>;
}
