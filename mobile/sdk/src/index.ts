/**
 * SuperApp mobile module SDK (TR-09-003).
 *
 * Exposes the mobile module host interface (screens, components,
 * permissions, `initialize`/`cleanup`) and the canonical manifest + SDK
 * version types a mobile module author needs to target `mobile/core`'s
 * module host (TR-08-005).
 */
export type {
  ModuleContext,
  ModuleDefinition,
  ModuleScreen,
  ResolvedScreen,
} from "./types";
export type { Manifest, ManifestEndpoint, ManifestSignature, FieldError } from "./manifest";
export { isValidManifest, validationErrors } from "./manifest";
export { SDK_VERSION, SUPPORTED_SDK_MAJOR, isCompatible } from "./version";
