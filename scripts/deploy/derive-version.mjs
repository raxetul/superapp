#!/usr/bin/env node
/**
 * Image-tag/version derivation (TR-10-006, TR-00-001).
 *
 * A tag push `refs/tags/vX.Y.Z` publishes as `X.Y.Z`; a tag that isn't
 * semver-shaped publishes verbatim (minus the `refs/tags/` prefix). Anything
 * else (a branch push) publishes as `<sanitized-branch>-<short-sha>`, so
 * every `main`/`master` push still gets a distinct, traceable image tag
 * without colliding with a previous build.
 *
 * CLI: reads `GITHUB_REF`/`GITHUB_REF_NAME`/`GITHUB_SHA` and prints the
 * derived version to stdout (used by CI to tag/push images).
 */

const SEMVER_TAG_RE = /^v(\d+\.\d+\.\d+(?:-[0-9A-Za-z-.]+)?)$/;

/** Derive a publishable version string from a git ref + commit sha. */
export function deriveVersion({ ref, refName, sha }) {
  if (ref && ref.startsWith("refs/tags/")) {
    const tag = refName ?? ref.slice("refs/tags/".length);
    const m = SEMVER_TAG_RE.exec(tag);
    return m ? m[1] : tag;
  }

  if (!refName || !sha) {
    throw new Error("deriveVersion: a branch build needs both refName and sha");
  }
  const branch = refName.replace(/[^a-zA-Z0-9._-]+/g, "-");
  const shortSha = sha.slice(0, 12);
  if (shortSha.length < 7) {
    throw new Error(`deriveVersion: sha too short to be unique: "${sha}"`);
  }
  return `${branch}-${shortSha}`;
}

function main() {
  const version = deriveVersion({
    ref: process.env.GITHUB_REF,
    refName: process.env.GITHUB_REF_NAME,
    sha: process.env.GITHUB_SHA,
  });
  process.stdout.write(`${version}\n`);
}

if (import.meta.url === `file://${process.argv[1]}`) {
  main();
}
