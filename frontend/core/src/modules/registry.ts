/**
 * TR-07-005 — Module registry / host.
 *
 * Dynamically registers frontend modules, drives their lifecycle
 * (`initialize`/`cleanup`), and filters contributed routes and nav by the
 * caller's permission set so a user never sees a route they lack permission
 * for.
 */
import type {
  FrontendModule,
  ModuleContext,
  ModuleNavItem,
  ModuleRoute,
} from "./types";

/** True if `required` is satisfied by `granted` (undefined ⇒ always visible). */
export function hasPermission(
  granted: ReadonlySet<string> | readonly string[],
  required?: string,
): boolean {
  if (!required) return true;
  const set = granted instanceof Set ? granted : new Set(granted);
  return set.has(required);
}

/** The SDK major version this host currently supports (TR-09-005). */
export const SUPPORTED_SDK_MAJOR = 1;

/** True if a module-declared `sdkVersion` is compatible (missing ⇒ true). */
export function isSdkVersionCompatible(sdkVersion?: string): boolean {
  if (sdkVersion == null) return true;
  const major = Number.parseInt(sdkVersion.split(".")[0] ?? "", 10);
  return !Number.isNaN(major) && major === SUPPORTED_SDK_MAJOR;
}

export class ModuleRegistry {
  private readonly modules = new Map<string, FrontendModule>();

  /**
   * Register and initialize a module. Throws on duplicate id, or on an
   * incompatible declared `sdkVersion` (TR-09-005) — before `initialize` ever
   * runs.
   */
  async register(module: FrontendModule, ctx: ModuleContext): Promise<void> {
    if (this.modules.has(module.id)) {
      throw new Error(`module already registered: ${module.id}`);
    }
    if (!isSdkVersionCompatible(module.sdkVersion)) {
      throw new Error(
        `module "${module.id}" was built against an incompatible SDK version ` +
          `(${module.sdkVersion}); this host supports major version ${SUPPORTED_SDK_MAJOR}`,
      );
    }
    this.modules.set(module.id, module);
    await module.initialize?.(ctx);
  }

  /** Unregister and clean up a module. */
  async unregister(id: string): Promise<void> {
    const module = this.modules.get(id);
    if (!module) return;
    await module.cleanup?.();
    this.modules.delete(id);
  }

  list(): FrontendModule[] {
    return [...this.modules.values()];
  }

  /** All contributed routes visible to a user with `permissions`. */
  visibleRoutes(
    permissions: readonly string[] = [],
  ): (ModuleRoute & { moduleId: string })[] {
    const set = new Set(permissions);
    const out: (ModuleRoute & { moduleId: string })[] = [];
    for (const m of this.modules.values()) {
      for (const r of m.routes ?? []) {
        if (hasPermission(set, r.permission)) {
          out.push({ ...r, moduleId: m.id });
        }
      }
    }
    return out;
  }

  /** All contributed nav entries visible to a user with `permissions`. */
  visibleNav(permissions: readonly string[] = []): ModuleNavItem[] {
    const set = new Set(permissions);
    const out: ModuleNavItem[] = [];
    for (const m of this.modules.values()) {
      for (const n of m.nav ?? []) {
        if (hasPermission(set, n.permission)) out.push(n);
      }
    }
    return out;
  }
}
