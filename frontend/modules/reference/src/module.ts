/**
 * The reference module (TR-09-007, web half) — built against
 * `@superapp/module-sdk-web`'s types. No npm workspaces exist in this repo
 * yet (see the phase doc), so the SDK is reached via a relative import
 * rather than a package dependency; the shape is what matters for TR-09-002's
 * accept criterion ("imports the SDK types, builds, and loads in the web
 * module host"), and `frontend/core`'s integration test proves the "loads"
 * half against the real `ModuleRegistry`.
 */
import type { FrontendModule } from "../../../sdk/src/types";
import { SDK_VERSION } from "../../../sdk/src/version";
import { ReferenceScreen } from "./ReferenceScreen";

export const REFERENCE_MODULE_ID = "reference";
/** Matches the backend reference module's Cedar-gated permission exactly. */
export const READ_PERMISSION = "reference:read";

export const referenceModule: FrontendModule = {
  id: REFERENCE_MODULE_ID,
  name: "Reference",
  permissions: [READ_PERMISSION],
  sdkVersion: SDK_VERSION,
  routes: [{ path: "/reference", component: ReferenceScreen, permission: READ_PERMISSION }],
  nav: [{ label: "Reference", to: "/reference", permission: READ_PERMISSION }],
};
