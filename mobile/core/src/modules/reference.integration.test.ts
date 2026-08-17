/**
 * TR-09-003 / TR-09-007 (mobile half): the reference module — built with
 * `@superapp/module-sdk-mobile` under `mobile/modules/reference` — registers
 * and loads in this **real** `ModuleRegistry`, and its screen is hidden
 * without the required permission and visible with it.
 */
import { ModuleRegistry } from './registry';
import type { ModuleContext } from './types';
import { READ_PERMISSION, referenceModule } from '../../../modules/reference/src/module';
import { ReferenceScreen } from '../../../modules/reference/src/ReferenceScreen';

function ctx(grantedPermissions: string[] = []): ModuleContext {
  return { api: {} as ModuleContext['api'], grantedPermissions };
}

describe('TR-09-007 reference module loads in the mobile host', () => {
  it('registers, and its screen is hidden without the permission and visible with it', async () => {
    const registry = new ModuleRegistry();
    await registry.register(referenceModule, ctx());
    expect(registry.has('reference')).toBe(true);

    expect(registry.screensFor('reference', [])).toHaveLength(0);

    const visible = registry.screensFor('reference', [READ_PERMISSION]);
    expect(visible.map((s) => s.name)).toContain('Reference');
    expect(visible[0].component).toBe(ReferenceScreen);

    const all = registry.visibleScreens([READ_PERMISSION]);
    expect(all.some((r) => r.moduleId === 'reference' && r.screen.name === 'Reference')).toBe(true);
  });
});
