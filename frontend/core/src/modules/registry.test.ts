import { describe, it, expect, vi } from "vitest";
import { ModuleRegistry, hasPermission, isSdkVersionCompatible } from "./registry";
import type { FrontendModule, ModuleContext } from "./types";

const ctx: ModuleContext = {
  apiBaseUrl: "http://api",
  getToken: () => "tok",
};

function makeModule(over: Partial<FrontendModule> = {}): FrontendModule {
  const Comp = () => null;
  return {
    id: "fleet",
    name: "Fleet",
    permissions: ["fleet:view", "fleet:admin"],
    routes: [
      { path: "/fleet", component: Comp },
      { path: "/fleet/admin", component: Comp, permission: "fleet:admin" },
    ],
    nav: [
      { label: "Fleet", to: "/fleet" },
      { label: "Fleet Admin", to: "/fleet/admin", permission: "fleet:admin" },
    ],
    ...over,
  };
}

describe("TR-07-005 module host", () => {
  it("runs initialize on register and cleanup on unregister", async () => {
    const initialize = vi.fn();
    const cleanup = vi.fn();
    const reg = new ModuleRegistry();
    await reg.register(makeModule({ initialize, cleanup }), ctx);
    expect(initialize).toHaveBeenCalledWith(ctx);
    expect(reg.list()).toHaveLength(1);
    await reg.unregister("fleet");
    expect(cleanup).toHaveBeenCalledOnce();
    expect(reg.list()).toHaveLength(0);
  });

  it("rejects duplicate module ids", async () => {
    const reg = new ModuleRegistry();
    await reg.register(makeModule(), ctx);
    await expect(reg.register(makeModule(), ctx)).rejects.toThrow(
      /already registered/,
    );
  });

  it("hides routes the user lacks permission for", async () => {
    const reg = new ModuleRegistry();
    await reg.register(makeModule(), ctx);

    const asUser = reg.visibleRoutes(["fleet:view"]).map((r) => r.path);
    expect(asUser).toEqual(["/fleet"]); // admin route hidden

    const asAdmin = reg
      .visibleRoutes(["fleet:view", "fleet:admin"])
      .map((r) => r.path);
    expect(asAdmin).toEqual(["/fleet", "/fleet/admin"]);
  });

  it("filters nav entries the same way", async () => {
    const reg = new ModuleRegistry();
    await reg.register(makeModule(), ctx);
    expect(reg.visibleNav([]).map((n) => n.to)).toEqual(["/fleet"]);
    expect(reg.visibleNav(["fleet:admin"]).map((n) => n.to)).toEqual([
      "/fleet",
      "/fleet/admin",
    ]);
  });

  it("hasPermission: undefined requirement is always visible", () => {
    expect(hasPermission([], undefined)).toBe(true);
    expect(hasPermission(["a"], "a")).toBe(true);
    expect(hasPermission(["a"], "b")).toBe(false);
  });
});

describe("TR-09-005 SDK version compatibility", () => {
  it("a module with a compatible SDK version registers normally", async () => {
    const reg = new ModuleRegistry();
    await reg.register(makeModule({ sdkVersion: "1.2.0" }), ctx);
    expect(reg.list()).toHaveLength(1);
  });

  it("a module with no declared SDK version registers normally", async () => {
    const reg = new ModuleRegistry();
    await reg.register(makeModule({ sdkVersion: undefined }), ctx);
    expect(reg.list()).toHaveLength(1);
  });

  it("a module with an incompatible SDK major version is rejected before initialize runs", async () => {
    const initialize = vi.fn();
    const reg = new ModuleRegistry();
    await expect(
      reg.register(makeModule({ sdkVersion: "2.0.0", initialize }), ctx),
    ).rejects.toThrow(/incompatible SDK version/);
    expect(initialize).not.toHaveBeenCalled();
    expect(reg.list()).toHaveLength(0);
  });

  it("isSdkVersionCompatible matches major-version rule", () => {
    expect(isSdkVersionCompatible(undefined)).toBe(true);
    expect(isSdkVersionCompatible("1.9.9")).toBe(true);
    expect(isSdkVersionCompatible("2.0.0")).toBe(false);
  });
});
