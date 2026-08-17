import { describe, expect, it } from "vitest";
import { isValidManifest } from "../../../sdk/src/manifest";
import { isCompatible } from "../../../sdk/src/version";
import { READ_PERMISSION, referenceModule } from "./module";
import { ReferenceScreen } from "./ReferenceScreen";

describe("TR-09-007 reference module (web)", () => {
  it("declares the read permission and a gated route + nav entry", () => {
    expect(referenceModule.permissions).toContain(READ_PERMISSION);
    expect(referenceModule.routes?.[0].permission).toBe(READ_PERMISSION);
    expect(referenceModule.nav?.[0].permission).toBe(READ_PERMISSION);
  });

  it("declares an SDK version compatible with this SDK's own rule", () => {
    expect(isCompatible(referenceModule.sdkVersion)).toBe(true);
  });

  it("the route wires the same component the module exports", () => {
    expect(referenceModule.routes?.[0].component).toBe(ReferenceScreen);
  });

  it("the screen component renders the expected element", () => {
    const element = ReferenceScreen();
    expect(element.props["data-testid"]).toBe("reference-screen");
    expect(element.props.children).toBe("hello from the reference module");
  });
});

// Sanity: the SDK's own manifest validator is usable from this package too
// (a module author reaches for both the frontend types and, where relevant,
// the shared manifest helpers from the same SDK).
describe("SDK manifest validator is reachable", () => {
  it("accepts a minimal valid manifest", () => {
    expect(isValidManifest({ name: "reference", version: "1.0.0" })).toBe(true);
  });
});
