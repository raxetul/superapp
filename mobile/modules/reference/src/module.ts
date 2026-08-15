/**
 * The reference module (TR-09-007, mobile half) — built against
 * `@superapp/module-sdk-mobile`'s types. No npm workspaces exist in this repo
 * yet (see the phase doc), so the SDK is reached via a relative import
 * rather than a package dependency; the shape is what matters for TR-09-003's
 * accept criterion ("imports the SDK types, builds, and loads in the mobile
 * module host"), and `mobile/core`'s integration test proves the "loads" half
 * against the real `ModuleRegistry`.
 */
import type { ModuleDefinition } from '../../../sdk/src/types';
import { SDK_VERSION } from '../../../sdk/src/version';
import { ReferenceScreen } from './ReferenceScreen';

export const REFERENCE_MODULE_ID = 'reference';
/** Matches the backend reference module's Cedar-gated permission exactly. */
export const READ_PERMISSION = 'reference:read';

export const referenceModule: ModuleDefinition = {
  id: REFERENCE_MODULE_ID,
  title: 'Reference',
  permissions: [READ_PERMISSION],
  sdkVersion: SDK_VERSION,
  screens: [{ name: 'Reference', component: ReferenceScreen, requiredPermission: READ_PERMISSION }],
};
