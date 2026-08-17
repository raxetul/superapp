/**
 * TR-09-004 — the canonical module manifest, mirrored field-for-field from
 * `backend/core/src/modules/manifest.rs::Manifest` and the shared JSON Schema
 * at `schemas/module-manifest.schema.json`. One shape, three independent
 * declarations (backend Rust, this package, `mobile/sdk`) — cross-checked
 * against the same schema file in each package's tests so they can't
 * silently drift.
 */

const ALLOWED_METHODS = new Set([
  "GET",
  "POST",
  "PUT",
  "PATCH",
  "DELETE",
  "HEAD",
  "OPTIONS",
]);

export interface ManifestEndpoint {
  method: string;
  path: string;
  permission?: string | null;
}

export interface ManifestSignature {
  signer: string;
  algorithm: string;
  value: string;
}

export interface Manifest {
  name: string;
  version: string;
  endpoints?: ManifestEndpoint[];
  permissions?: string[];
  config_schema?: unknown;
  signatures?: ManifestSignature[];
}

export interface FieldError {
  pointer: string;
  detail: string;
}

/** The same structural rules the core applies at `/modules/register`. */
export function validationErrors(manifest: Manifest): FieldError[] {
  const errors: FieldError[] = [];
  if (!manifest.name?.trim()) {
    errors.push({ pointer: "/name", detail: "name is required" });
  }
  if (!manifest.version?.trim()) {
    errors.push({ pointer: "/version", detail: "version is required" });
  }
  (manifest.endpoints ?? []).forEach((ep, i) => {
    if (!ALLOWED_METHODS.has(ep.method.toUpperCase())) {
      errors.push({
        pointer: `/endpoints/${i}/method`,
        detail: `unsupported HTTP method \`${ep.method}\``,
      });
    }
    if (!ep.path.startsWith("/")) {
      errors.push({
        pointer: `/endpoints/${i}/path`,
        detail: "path must start with `/`",
      });
    }
  });
  return errors;
}

export function isValidManifest(manifest: Manifest): boolean {
  return validationErrors(manifest).length === 0;
}
