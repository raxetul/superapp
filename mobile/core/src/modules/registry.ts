/**
 * Module registry / host (TR-08-005).
 *
 * Dynamically loads module definitions, runs their `initialize`/`cleanup`
 * lifecycle, and resolves which screens are visible for a given granted
 * permission set — screens whose `requiredPermission` is not granted are
 * hidden.
 */
import type { ModuleContext, ModuleDefinition, ModuleScreen, ResolvedScreen } from './types';

function hasPermission(granted: ReadonlySet<string>, required: string | undefined): boolean {
  return required == null || granted.has(required);
}

/** The SDK major version this host currently supports (TR-09-005). */
export const SUPPORTED_SDK_MAJOR = 1;

/** True if a module-declared `sdkVersion` is compatible (missing ⇒ true). */
export function isSdkVersionCompatible(sdkVersion?: string): boolean {
  if (sdkVersion == null) return true;
  const major = Number.parseInt(sdkVersion.split('.')[0] ?? '', 10);
  return !Number.isNaN(major) && major === SUPPORTED_SDK_MAJOR;
}

export class ModuleRegistry {
  private readonly modules = new Map<string, ModuleDefinition>();

  /**
   * Register (and initialize) a module. Throws on duplicate id, or on an
   * incompatible declared `sdkVersion` (TR-09-005) — before `initialize` ever
   * runs.
   */
  async register(module: ModuleDefinition, ctx: ModuleContext): Promise<void> {
    if (this.modules.has(module.id)) {
      throw new Error(`Module "${module.id}" is already registered`);
    }
    if (!isSdkVersionCompatible(module.sdkVersion)) {
      throw new Error(
        `Module "${module.id}" was built against an incompatible SDK version ` +
          `(${module.sdkVersion}); this host supports major version ${SUPPORTED_SDK_MAJOR}`,
      );
    }
    await module.initialize?.(ctx);
    this.modules.set(module.id, module);
  }

  /** Unregister (and clean up) a module. No-op if not present. */
  async unregister(id: string): Promise<void> {
    const module = this.modules.get(id);
    if (!module) return;
    await module.cleanup?.();
    this.modules.delete(id);
  }

  has(id: string): boolean {
    return this.modules.has(id);
  }

  get(id: string): ModuleDefinition | undefined {
    return this.modules.get(id);
  }

  list(): ModuleDefinition[] {
    return [...this.modules.values()];
  }

  /** Screens of one module visible for the granted permissions. */
  screensFor(moduleId: string, granted: Iterable<string>): ModuleScreen[] {
    const module = this.modules.get(moduleId);
    if (!module) return [];
    const set = new Set(granted);
    return module.screens.filter((s) => hasPermission(set, s.requiredPermission));
  }

  /** All visible screens across every registered module. */
  visibleScreens(granted: Iterable<string>): ResolvedScreen[] {
    const set = new Set(granted);
    const resolved: ResolvedScreen[] = [];
    for (const module of this.modules.values()) {
      for (const screen of module.screens) {
        if (hasPermission(set, screen.requiredPermission)) {
          resolved.push({ moduleId: module.id, screen });
        }
      }
    }
    return resolved;
  }

  /** Resolve a named component contributed by a module. */
  component(moduleId: string, name: string) {
    return this.modules.get(moduleId)?.components?.[name];
  }
}
