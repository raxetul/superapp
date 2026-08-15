/**
 * SuperApp frontend module SDK (TR-09-002).
 *
 * Exposes the web module host interface (routes, components, permissions,
 * `initialize`/`cleanup`) and the canonical manifest + SDK version types a
 * frontend module author needs to target `frontend/core`'s module host
 * (TR-07-005).
 */
export type {
  FrontendModule,
  ModuleContext,
  ModuleNavItem,
  ModuleRoute,
} from "./types";
export type { Manifest, ManifestEndpoint, ManifestSignature, FieldError } from "./manifest";
export { isValidManifest, validationErrors } from "./manifest";
export { SDK_VERSION, SUPPORTED_SDK_MAJOR, isCompatible } from "./version";
