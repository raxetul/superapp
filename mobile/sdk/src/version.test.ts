import { describe, expect, it } from "vitest";
import { isCompatible, SDK_VERSION, SUPPORTED_SDK_MAJOR } from "./version";

describe("TR-09-005 SDK version compatibility", () => {
  it("this SDK's own version is compatible with its supported major", () => {
    expect(isCompatible(SDK_VERSION)).toBe(true);
  });

  it("a matching major version is compatible", () => {
    expect(isCompatible("1.4.2")).toBe(true);
  });

  it("a different major version is rejected", () => {
    expect(isCompatible("2.0.0")).toBe(false);
  });

  it("no declared version is treated as compatible (legacy modules)", () => {
    expect(isCompatible(undefined)).toBe(true);
  });

  it("supports checking against an explicit supported major", () => {
    expect(isCompatible("2.0.0", 2)).toBe(true);
    expect(SUPPORTED_SDK_MAJOR).toBe(1);
  });
});
