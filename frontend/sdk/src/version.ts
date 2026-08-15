/**
 * TR-09-005 — SDK version + compatibility.
 *
 * The same major-version rule as `backend/sdk/src/version.rs` and
 * `mobile/sdk/src/version.ts`: a module is compatible with a host iff its
 * declared major version matches the host's supported major.
 */

export const SDK_VERSION = "1.0.0";

/** The web host's currently supported SDK major version. */
export const SUPPORTED_SDK_MAJOR = 1;

function majorVersion(version: string): number | undefined {
  const major = Number.parseInt(version.split(".")[0] ?? "", 10);
  return Number.isNaN(major) ? undefined : major;
}

/**
 * Check a module-declared SDK `version` against `supportedMajor` (defaults to
 * this package's own [`SUPPORTED_SDK_MAJOR`]). `undefined` (no declared
 * version) is always compatible — legacy/non-SDK modules.
 */
export function isCompatible(
  version: string | undefined,
  supportedMajor: number = SUPPORTED_SDK_MAJOR,
): boolean {
  if (version == null) return true;
  const major = majorVersion(version);
  return major !== undefined && major === supportedMajor;
}
