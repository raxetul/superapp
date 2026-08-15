/**
 * The canonical *code artifact* a module manifest's signatures cover
 * (TR-05-002 / TR-09-006), byte-identical to
 * `backend/core/src/modules/manifest.rs::Manifest::code_artifact_bytes` and
 * `backend/sdk/src/manifest.rs`'s copy of it: `name`, `version`, `endpoints`,
 * `permissions`, `config_schema` — with object keys recursively sorted —
 * serialized as **compact** JSON (no whitespace). Signatures and any runtime
 * config are excluded, so signing here produces a signature the backend's
 * `signing::verify` accepts.
 */

/** Recursively sort object keys so the encoding is deterministic. */
function canonicalize(value) {
  if (Array.isArray(value)) return value.map(canonicalize);
  if (value !== null && typeof value === "object") {
    const sorted = {};
    for (const key of Object.keys(value).sort()) {
      sorted[key] = canonicalize(value[key]);
    }
    return sorted;
  }
  return value;
}

/** The canonical code-artifact bytes for `manifest` (name/version/endpoints/permissions/config_schema only). */
export function codeArtifactBytes(manifest) {
  const artifact = {
    name: manifest.name,
    version: manifest.version,
    endpoints: manifest.endpoints ?? [],
    permissions: manifest.permissions ?? [],
    config_schema: manifest.config_schema ?? null,
  };
  return Buffer.from(JSON.stringify(canonicalize(artifact)), "utf-8");
}
