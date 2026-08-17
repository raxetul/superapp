#!/usr/bin/env node
/**
 * Module packaging + signing (TR-09-006): given a manifest JSON file and an
 * ed25519 private key (PEM), produce a **signed** manifest — the signature
 * array (TR-05-002) the backend's `signing::verify` accepts, appended
 * alongside any existing signatures (never replacing them).
 *
 * OCI packaging (building/pushing the module's Docker image to the private
 * registry, TR-09-009) is **not** performed by this script — no Docker
 * daemon is available in this environment. See docs/module-authoring.md for
 * the documented `docker build && docker push` steps a CI pipeline runs
 * after this script signs the manifest.
 *
 * Usage:
 *   node package-module.mjs generate-key --out signer.pem
 *   node package-module.mjs sign --manifest manifest.json --key signer.pem \
 *     --signer my-ci [--out signed-manifest.json]
 */
import { generateKeyPairSync, sign as cryptoSign } from "node:crypto";
import { readFileSync, writeFileSync } from "node:fs";
import { codeArtifactBytes } from "./canonical.mjs";

function parseArgs(argv) {
  const args = { _: [] };
  for (let i = 0; i < argv.length; i++) {
    const a = argv[i];
    if (a.startsWith("--")) {
      args[a.slice(2)] = argv[i + 1];
      i++;
    } else {
      args._.push(a);
    }
  }
  return args;
}

/**
 * The raw 32-byte ed25519 public key, base64-encoded — the shape
 * `TrustStore::add_base64` (backend/core/src/modules/signing.rs) expects.
 * An SPKI DER-encoded Ed25519 key is always a fixed 12-byte algorithm header
 * followed by the 32 raw key bytes, so the raw key is simply the DER's last
 * 32 bytes.
 */
export function rawPublicKeyBase64(publicKey) {
  const der = publicKey.export({ type: "spki", format: "der" });
  return der.subarray(der.length - 32).toString("base64");
}

function generateKey(args) {
  const { publicKey, privateKey } = generateKeyPairSync("ed25519");
  const pem = privateKey.export({ type: "pkcs8", format: "pem" });
  writeFileSync(args.out ?? "signer.pem", pem);
  console.log(`private key written to ${args.out ?? "signer.pem"}`);
  console.log("public key, base64 (share with the core operator for modules.trusted_signers):");
  console.log(rawPublicKeyBase64(publicKey));
}

/** Sign `manifest`'s code artifact with `privateKeyPem`, appending the signature. */
export function signManifest(manifest, privateKeyPem, signerId) {
  const message = codeArtifactBytes(manifest);
  const signature = cryptoSign(null, message, privateKeyPem);
  const signatures = [...(manifest.signatures ?? [])];
  signatures.push({
    signer: signerId,
    algorithm: "ed25519",
    value: signature.toString("base64"),
  });
  return { ...manifest, signatures };
}

function sign(args) {
  if (!args.manifest || !args.key || !args.signer) {
    throw new Error("--manifest, --key, and --signer are required");
  }
  const manifest = JSON.parse(readFileSync(args.manifest, "utf-8"));
  const privateKeyPem = readFileSync(args.key, "utf-8");
  const signed = signManifest(manifest, privateKeyPem, args.signer);
  const out = JSON.stringify(signed, null, 2);
  if (args.out) {
    writeFileSync(args.out, out);
    console.log(`signed manifest written to ${args.out}`);
  } else {
    console.log(out);
  }
}

function main() {
  const [command, ...rest] = process.argv.slice(2);
  const args = parseArgs(rest);
  if (command === "generate-key") return generateKey(args);
  if (command === "sign") return sign(args);
  console.error("usage: package-module.mjs <generate-key|sign> [...args]");
  process.exitCode = 1;
}

if (import.meta.url === `file://${process.argv[1]}`) {
  main();
}
