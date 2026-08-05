/** TR-08-005 — module host: dynamic load, lifecycle, permission-gated screens. */
import type { ComponentType } from 'react';
import { ModuleRegistry } from './registry';
import type { ModuleContext, ModuleDefinition } from './types';

const Dummy = (() => null) as unknown as ComponentType<Record<string, unknown>>;

function ctx(grantedPermissions: string[] = []): ModuleContext {
  return { api: {} as ModuleContext['api'], grantedPermissions };
}

function makeModule(overrides: Partial<ModuleDefinition> = {}): ModuleDefinition {
  return {
    id: 'reports',
    permissions: ['reports.view', 'reports.admin'],
    screens: [
      { name: 'ReportsList', component: Dummy, requiredPermission: 'reports.view' },
      { name: 'ReportsAdmin', component: Dummy, requiredPermission: 'reports.admin' },
      { name: 'ReportsAbout', component: Dummy }, // no permission required
    ],
    ...overrides,
  };
}

describe('ModuleRegistry (TR-08-005)', () => {
  it('runs initialize on register and cleanup on unregister', async () => {
    const initialize = jest.fn();
    const cleanup = jest.fn();
    const registry = new ModuleRegistry();
    const mod = makeModule({ initialize, cleanup });

    await registry.register(mod, ctx(['reports.view']));
    expect(initialize).toHaveBeenCalledTimes(1);
    expect(registry.has('reports')).toBe(true);

    await registry.unregister('reports');
    expect(cleanup).toHaveBeenCalledTimes(1);
    expect(registry.has('reports')).toBe(false);
  });

  it('rejects duplicate registration', async () => {
    const registry = new ModuleRegistry();
    await registry.register(makeModule(), ctx());
    await expect(registry.register(makeModule(), ctx())).rejects.toThrow(/already registered/);
  });

  it('hides screens whose required permission is not granted', async () => {
    const registry = new ModuleRegistry();
    await registry.register(makeModule(), ctx(['reports.view']));

    const visible = registry.visibleScreens(['reports.view']).map((r) => r.screen.name);
    expect(visible).toContain('ReportsList'); // granted
    expect(visible).toContain('ReportsAbout'); // no permission required
    expect(visible).not.toContain('ReportsAdmin'); // permission missing
  });

  it('reveals all screens when every permission is granted', async () => {
    const registry = new ModuleRegistry();
    await registry.register(makeModule(), ctx());
    const visible = registry.screensFor('reports', ['reports.view', 'reports.admin']).map((s) => s.name);
    expect(visible).toEqual(['ReportsList', 'ReportsAdmin', 'ReportsAbout']);
  });

  it('resolves module-contributed components', async () => {
    const registry = new ModuleRegistry();
    await registry.register(makeModule({ components: { Badge: Dummy } }), ctx());
    expect(registry.component('reports', 'Badge')).toBe(Dummy);
    expect(registry.component('reports', 'Missing')).toBeUndefined();
  });
});
