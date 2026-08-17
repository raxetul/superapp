/**
 * TR-09-002 / TR-09-007 (web half): the reference module — built with
 * `@superapp/module-sdk-web` under `frontend/modules/reference` — registers
 * and loads in this **real** `ModuleRegistry`, and its route/nav are visible
 * only once the required permission is granted.
 */
import { describe, expect, it } from "vitest";
import { ModuleRegistry } from "./registry";
import type { ModuleContext } from "./types";
import { READ_PERMISSION, referenceModule } from "../../../modules/reference/src/module";
import { ReferenceScreen } from "../../../modules/reference/src/ReferenceScreen";

const ctx: ModuleContext = {
  apiBaseUrl: "http://api",
  getToken: () => "tok",
};

describe("TR-09-007 reference module loads in the web host", () => {
  it("registers, and its route/nav are hidden without the permission and visible with it", async () => {
    const reg = new ModuleRegistry();
    await reg.register(referenceModule, ctx);
    expect(reg.list().map((m) => m.id)).toContain("reference");

    expect(reg.visibleRoutes([]).map((r) => r.path)).not.toContain("/reference");
    expect(reg.visibleNav([]).map((n) => n.to)).not.toContain("/reference");

    const visible = reg.visibleRoutes([READ_PERMISSION]);
    expect(visible.map((r) => r.path)).toContain("/reference");
    expect(reg.visibleNav([READ_PERMISSION]).map((n) => n.to)).toContain("/reference");

    const route = visible.find((r) => r.path === "/reference")!;
    expect(route.component).toBe(ReferenceScreen);
    const element = ReferenceScreen();
    expect(element.props["data-testid"]).toBe("reference-screen");
  });
});
