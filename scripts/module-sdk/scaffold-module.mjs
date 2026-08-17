#!/usr/bin/env node
/**
 * Module scaffolding generator (TR-09-006, SHOULD): produces a buildable
 * cross-platform module skeleton — `backend/modules/<name>`,
 * `frontend/modules/<name>`, `mobile/modules/<name>` — wired to the SDKs,
 * matching the reference module's own structure (see `docs/module-authoring.md`).
 *
 * Usage:
 *   node scaffold-module.mjs <module-name> [--permission <name>:<action>]
 *
 * Refuses to overwrite an existing module directory.
 */
import { existsSync, mkdirSync, writeFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const REPO_ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");

function pascalCase(name) {
  return name
    .split(/[-_]/)
    .map((s) => s.charAt(0).toUpperCase() + s.slice(1))
    .join("");
}

function write(filePath, content) {
  mkdirSync(path.dirname(filePath), { recursive: true });
  writeFileSync(filePath, content);
}

function backendFiles(name, permission) {
  const crateName = `${name}-module`;
  const libName = crateName.replace(/-/g, "_");
  return {
    "Cargo.toml": `[workspace]

[package]
name = "${crateName}"
version = "1.0.0"
edition = "2021"
rust-version = "1.85"
publish = false
description = "\${name} module, built with the backend module SDK."

[lib]
name = "${libName}"
path = "src/lib.rs"

[[bin]]
name = "${crateName}"
path = "src/main.rs"

[dependencies]
superapp-module-sdk = { path = "../../sdk" }
serde_json = { version = "1" }
axum = { version = "0.8" }
tokio = { version = "1.45", default-features = false, features = ["net", "rt-multi-thread", "macros"] }
`.replace("\\${name}", name),
    "src/lib.rs": `//! ${pascalCase(name)} module, scaffolded from the reference module.

use axum::routing::get;
use axum::{Json, Router};
use serde_json::{json, Value};
use superapp_module_sdk::{Manifest, ModuleServer};

pub const NAME: &str = "${name}";
pub const VERSION: &str = "1.0.0";
pub const READ_PERMISSION: &str = "${permission}";

#[must_use]
pub fn manifest() -> Manifest {
    Manifest::new(NAME, VERSION)
        .endpoint("GET", "/items", Some(READ_PERMISSION))
        .permission(READ_PERMISSION)
        .config_schema(config_schema())
}

#[must_use]
pub fn config_schema() -> Value {
    json!({ "type": "object", "properties": {} })
}

async fn items() -> Json<Value> {
    Json(json!({ "items": [] }))
}

#[must_use = "build the router and pass it to axum::serve"]
pub fn router() -> Router {
    ModuleServer::new(manifest())
        .merge(Router::new().route("/items", get(items)))
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_is_valid() {
        assert!(manifest().is_valid());
    }
}
`,
    "src/main.rs": `#[tokio::main]
async fn main() {
    let port: u16 = std::env::var("PORT").ok().and_then(|p| p.parse().ok()).unwrap_or(8080);
    let listener = tokio::net::TcpListener::bind(("0.0.0.0", port)).await.expect("bind module port");
    println!("${name} module listening on :{port}");
    axum::serve(listener, ${libName}::router()).await.expect("serve ${name} module");
}
`,
  };
}

function frontendFiles(name, permission) {
  const componentName = `${pascalCase(name)}Screen`;
  return {
    "package.json": `${JSON.stringify(
      {
        name: `@superapp/module-${name}-web`,
        private: true,
        version: "1.0.0",
        type: "module",
        description: `${name} module (web), scaffolded from the reference module.`,
        scripts: { build: "tsc -b", typecheck: "tsc --noEmit", test: "vitest run" },
        devDependencies: {
          "@types/react": "^18.3.12",
          react: "^18.3.1",
          typescript: "^5.6.3",
          vitest: "^2.1.5",
        },
      },
      null,
      2,
    )}\n`,
    "tsconfig.json": `${JSON.stringify(
      {
        compilerOptions: {
          target: "ES2022",
          lib: ["ES2022", "DOM"],
          module: "ESNext",
          moduleResolution: "Bundler",
          jsx: "react-jsx",
          strict: true,
          noEmit: true,
          esModuleInterop: true,
          skipLibCheck: true,
        },
        include: ["src/**/*.ts", "src/**/*.tsx"],
      },
      null,
      2,
    )}\n`,
    [`src/${componentName}.tsx`]: `import * as React from "react";

export function ${componentName}(): React.JSX.Element {
  return React.createElement("div", { "data-testid": "${name}-screen" }, "${name} module");
}
`,
    "src/module.ts": `import type { FrontendModule } from "../../../sdk/src/types";
import { SDK_VERSION } from "../../../sdk/src/version";
import { ${componentName} } from "./${componentName}";

export const MODULE_ID = "${name}";
export const READ_PERMISSION = "${permission}";

export const ${name.replace(/-/g, "_")}Module: FrontendModule = {
  id: MODULE_ID,
  name: "${pascalCase(name)}",
  permissions: [READ_PERMISSION],
  sdkVersion: SDK_VERSION,
  routes: [{ path: "/${name}", component: ${componentName}, permission: READ_PERMISSION }],
  nav: [{ label: "${pascalCase(name)}", to: "/${name}", permission: READ_PERMISSION }],
};
`,
  };
}

function mobileFiles(name, permission) {
  const componentName = `${pascalCase(name)}Screen`;
  return {
    "package.json": `${JSON.stringify(
      {
        name: `@superapp/module-${name}-mobile`,
        private: true,
        version: "1.0.0",
        description: `${name} module (mobile), scaffolded from the reference module.`,
        scripts: { build: "tsc -b", typecheck: "tsc --noEmit", test: "vitest run" },
        dependencies: { "@babel/runtime": "^7.28.4" },
        devDependencies: {
          "@types/react": "~19.2.2",
          react: "19.2.3",
          typescript: "~6.0.3",
          vitest: "^2.1.5",
        },
      },
      null,
      2,
    )}\n`,
    "tsconfig.json": `${JSON.stringify(
      {
        compilerOptions: {
          target: "ES2022",
          lib: ["ES2022"],
          module: "ESNext",
          moduleResolution: "Bundler",
          jsx: "react-jsx",
          strict: true,
          noEmit: true,
          esModuleInterop: true,
          skipLibCheck: true,
        },
        include: ["src/**/*.ts", "src/**/*.tsx"],
      },
      null,
      2,
    )}\n`,
    [`src/${componentName}.tsx`]: `import * as React from "react";

function Label(props: { testID?: string; children?: React.ReactNode }): null {
  void props;
  return null;
}

export function ${componentName}(): React.ReactElement {
  return React.createElement(Label, { testID: "${name}-screen" }, "${name} module");
}
`,
    "src/module.ts": `import type { ModuleDefinition } from "../../../sdk/src/types";
import { SDK_VERSION } from "../../../sdk/src/version";
import { ${componentName} } from "./${componentName}";

export const MODULE_ID = "${name}";
export const READ_PERMISSION = "${permission}";

export const ${name.replace(/-/g, "_")}Module: ModuleDefinition = {
  id: MODULE_ID,
  title: "${pascalCase(name)}",
  permissions: [READ_PERMISSION],
  sdkVersion: SDK_VERSION,
  screens: [{ name: "${pascalCase(name)}", component: ${componentName}, requiredPermission: READ_PERMISSION }],
};
`,
  };
}

function scaffold(name, permission, root = REPO_ROOT) {
  const targets = [
    [path.join(root, "backend/modules", name), backendFiles(name, permission)],
    [path.join(root, "frontend/modules", name), frontendFiles(name, permission)],
    [path.join(root, "mobile/modules", name), mobileFiles(name, permission)],
  ];
  for (const [dir] of targets) {
    if (existsSync(dir)) {
      throw new Error(`refusing to overwrite existing module directory: ${dir}`);
    }
  }
  for (const [dir, files] of targets) {
    for (const [rel, content] of Object.entries(files)) {
      write(path.join(dir, rel), content);
    }
    console.log(`scaffolded ${dir}`);
  }
}

function main() {
  const argv = process.argv.slice(2);
  const name = argv[0];
  if (!name || name.startsWith("--")) {
    console.error("usage: scaffold-module.mjs <module-name> [--permission <name>:<action>]");
    process.exitCode = 1;
    return;
  }
  const permIdx = argv.indexOf("--permission");
  const permission = permIdx !== -1 ? argv[permIdx + 1] : `${name}:read`;
  scaffold(name, permission);
}

if (import.meta.url === `file://${process.argv[1]}`) {
  main();
}

export { scaffold };
