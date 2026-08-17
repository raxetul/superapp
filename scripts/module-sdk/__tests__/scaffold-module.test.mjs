import { test } from "node:test";
import assert from "node:assert/strict";
import { existsSync, mkdtempSync, readFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { scaffold } from "../scaffold-module.mjs";

test("scaffolds a buildable cross-platform skeleton wired to the SDKs", () => {
  const root = mkdtempSync(path.join(tmpdir(), "scaffold-test-"));
  try {
    scaffold("widget", "widget:read", root);

    const backendLib = readFileSync(path.join(root, "backend/modules/widget/src/lib.rs"), "utf-8");
    assert.match(backendLib, /superapp_module_sdk::\{Manifest, ModuleServer\}/);
    assert.match(backendLib, /pub const READ_PERMISSION: &str = "widget:read";/);
    assert.ok(existsSync(path.join(root, "backend/modules/widget/Cargo.toml")));

    const webModule = readFileSync(path.join(root, "frontend/modules/widget/src/module.ts"), "utf-8");
    assert.match(webModule, /from "\.\.\/\.\.\/\.\.\/sdk\/src\/types"/);
    assert.match(webModule, /READ_PERMISSION = "widget:read"/);

    const mobileModule = readFileSync(path.join(root, "mobile/modules/widget/src/module.ts"), "utf-8");
    assert.match(mobileModule, /ModuleDefinition/);
    assert.match(mobileModule, /READ_PERMISSION = "widget:read"/);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

test("refuses to overwrite an existing module directory", () => {
  const root = mkdtempSync(path.join(tmpdir(), "scaffold-test-"));
  try {
    scaffold("widget", "widget:read", root);
    assert.throws(() => scaffold("widget", "widget:read", root), /refusing to overwrite/);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});
