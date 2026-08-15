import { test } from "node:test";
import assert from "node:assert/strict";
import { codeArtifactBytes } from "../canonical.mjs";

function sample() {
  return {
    name: "reference",
    version: "1.0.0",
    endpoints: [{ method: "GET", path: "/items", permission: "reference:read" }],
    permissions: ["reference:read"],
    config_schema: { type: "object", properties: { greeting: { type: "string" } } },
    signatures: [],
  };
}

test("code artifact excludes signatures and is deterministic", () => {
  const a = codeArtifactBytes(sample());
  const withSignature = { ...sample(), signatures: [{ signer: "x", algorithm: "ed25519", value: "abc" }] };
  assert.deepEqual(a, codeArtifactBytes(withSignature));
});

test("changing a route changes the artifact", () => {
  const base = codeArtifactBytes(sample());
  const changed = sample();
  changed.endpoints[0].path = "/items/all";
  assert.notDeepEqual(base, codeArtifactBytes(changed));
});

test("matches the exact byte-for-byte fixture the Rust SDK's own test asserts", () => {
  // Keep this string in sync with backend/sdk/src/manifest.rs's
  // `matches_js_packaging_fixture` test — both sides assert the identical
  // canonical bytes for the identical fixture manifest, proving the two
  // independent implementations (Rust `code_artifact_bytes`, this JS
  // `codeArtifactBytes`) agree byte-for-byte (TR-09-006).
  const fixture = {
    name: "fixture",
    version: "1.0.0",
    endpoints: [{ method: "GET", path: "/items", permission: "fixture:read" }],
    permissions: ["fixture:read"],
    config_schema: { type: "object" },
  };
  const expected =
    '{"config_schema":{"type":"object"},"endpoints":[{"method":"GET","path":"/items","permission":"fixture:read"}],"name":"fixture","permissions":["fixture:read"],"version":"1.0.0"}';
  assert.equal(codeArtifactBytes(fixture).toString("utf-8"), expected);
});
