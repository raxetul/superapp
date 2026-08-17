/**
 * TR-07-005 — Frontend module host contract.
 *
 * A frontend module contributes routes, navigation entries, permissions, and
 * lifecycle hooks. The host dynamically registers modules, runs their
 * `initialize`/`cleanup`, and exposes only the routes/nav the current user is
 * permitted to see (authorization itself stays server-side — this is UI
 * reflection only, TR-07-004).
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
   * `"1.0.0"` (see `@superapp/module-sdk-web`'s `SDK_VERSION`). Omit for
   * pre-SDK modules — `ModuleRegistry.register` treats a missing version as
   * compatible; an incompatible major version is rejected with a clear error.
   */
  sdkVersion?: string;
  initialize?: (ctx: ModuleContext) => void | Promise<void>;
  cleanup?: () => void | Promise<void>;
}
