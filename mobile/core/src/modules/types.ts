/**
 * Module contract for the mobile module host (TR-08-005).
 *
 * A module contributes screens and components, declares the permissions it
 * needs, and has `initialize`/`cleanup` lifecycle hooks. This mirrors the
 * backend's out-of-process module model at the UI layer: the core owns the
 * host, modules plug into it.
 */
import type { ComponentType } from 'react';
import type { ApiClient } from '@/api/client';

/** Services the core exposes to a module during its lifecycle. */
export interface ModuleContext {
  api: ApiClient;
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
  initialize?: (ctx: ModuleContext) => void | Promise<void>;
  cleanup?: () => void | Promise<void>;
}

/** A screen resolved against a specific granted-permission set. */
export interface ResolvedScreen {
  moduleId: string;
  screen: ModuleScreen;
}
