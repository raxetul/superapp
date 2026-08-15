import { test } from "node:test";
import assert from "node:assert/strict";
import { generateKeyPairSync, verify as cryptoVerify } from "node:crypto";
import { codeArtifactBytes } from "../canonical.mjs";
import { rawPublicKeyBase64, signManifest } from "../package-module.mjs";

function sample() {
  return {
    name: "reference",
    version: "1.0.0",
    endpoints: [{ method: "GET", path: "/items", permission: "reference:read" }],
    permissions: ["reference:read"],
    config_schema: { type: "object" },
    signatures: [],
  };
}

test("signs the manifest's code artifact with a valid ed25519 signature", () => {
  const { publicKey, privateKey } = generateKeyPairSync("ed25519");
  const privatePem = privateKey.export({ type: "pkcs8", format: "pem" });

  const signed = signManifest(sample(), privatePem, "ci-signer");

  assert.equal(signed.signatures.length, 1);
  const sig = signed.signatures[0];
  assert.equal(sig.signer, "ci-signer");
  assert.equal(sig.algorithm, "ed25519");

  const ok = cryptoVerify(null, codeArtifactBytes(sample()), publicKey, Buffer.from(sig.value, "base64"));
  assert.equal(ok, true, "the signature must validate over the exact code-artifact bytes the backend re-derives");
});

test("appends to (never replaces) existing signatures", () => {
  const withExisting = { ...sample(), signatures: [{ signer: "self", algorithm: "ed25519", value: "prior" }] };
  const { privateKey } = generateKeyPairSync("ed25519");
  const signed = signManifest(withExisting, privateKey.export({ type: "pkcs8", format: "pem" }), "ci-signer");
  assert.equal(signed.signatures.length, 2);
  assert.equal(signed.signatures[0].signer, "self");
  assert.equal(signed.signatures[1].signer, "ci-signer");
});

test("signing does not change the code artifact (config/data excluded, TR-05-002)", () => {
  const before = codeArtifactBytes(sample());
  const { privateKey } = generateKeyPairSync("ed25519");
  const signed = signManifest(sample(), privateKey.export({ type: "pkcs8", format: "pem" }), "ci-signer");
  assert.deepEqual(before, codeArtifactBytes(signed));
});

test("rawPublicKeyBase64 yields the 32-byte key TrustStore.add_base64 expects", () => {
  const { publicKey } = generateKeyPairSync("ed25519");
  const raw = rawPublicKeyBase64(publicKey);
  assert.equal(Buffer.from(raw, "base64").length, 32);
});
