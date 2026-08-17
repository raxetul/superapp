import { describe, expect, it } from "vitest";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import path from "node:path";
import { isValidManifest, validationErrors, type Manifest } from "./manifest";

const here = path.dirname(fileURLToPath(import.meta.url));
const schemaPath = path.resolve(here, "../../../schemas/module-manifest.schema.json");

function sample(): Manifest {
  return {
    name: "reference",
    version: "1.0.0",
    endpoints: [{ method: "GET", path: "/items", permission: "reference:read" }],
    permissions: ["reference:read"],
    config_schema: { type: "object" },
    signatures: [],
  };
}

describe("TR-09-004 canonical manifest", () => {
  it("accepts a well-formed manifest", () => {
    expect(isValidManifest(sample())).toBe(true);
  });

  it("reports a missing name and an unsupported method", () => {
    const m = sample();
    m.name = "  ";
    m.endpoints![0].method = "FETCH";
    const errors = validationErrors(m);
    const pointers = errors.map((e) => e.pointer);
    expect(pointers).toContain("/name");
    expect(pointers).toContain("/endpoints/0/method");
  });

  it("matches the canonical schema shared with the backend and mobile SDKs", () => {
    const schema = JSON.parse(readFileSync(schemaPath, "utf-8"));
    const schemaProps = Object.keys(schema.properties);
    for (const field of Object.keys(sample())) {
      expect(schemaProps).toContain(field);
    }
    // Endpoint/signature sub-shapes match too.
    expect(Object.keys(schema.definitions.endpoint.properties)).toEqual(
      expect.arrayContaining(["method", "path", "permission"]),
    );
    expect(Object.keys(schema.definitions.signature.properties)).toEqual(
      expect.arrayContaining(["signer", "algorithm", "value"]),
    );
  });
});
