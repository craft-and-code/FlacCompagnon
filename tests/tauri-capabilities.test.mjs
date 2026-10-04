import assert from "node:assert/strict";
import { readFile, readdir } from "node:fs/promises";
import test from "node:test";
import ts from "typescript";

const root = new URL("../", import.meta.url);
const capabilityDirectory = new URL("src-tauri/capabilities/", root);

// API contracts, not implementation text: listeners (including native drop)
// return an UnlistenFn, so their cleanup needs allow-unlisten too.
// https://v2.tauri.app/reference/javascript/api/namespaceevent/
// https://v2.tauri.app/reference/javascript/api/namespacewebview/
const apiPermissions = new Map([
  ["@tauri-apps/api/core:invoke", []],
  ["@tauri-apps/api/event:emit", ["core:event:allow-emit"]],
  ["@tauri-apps/api/event:listen", ["core:event:allow-listen", "core:event:allow-unlisten"]],
  ["@tauri-apps/api/window:getCurrentWindow", []],
  ["@tauri-apps/api/webview:getCurrentWebview", []],
  ["@tauri-apps/plugin-dialog:open", ["dialog:allow-open"]],
  ["@tauri-apps/plugin-dialog:save", ["dialog:allow-save"]],
]);
const factories = new Map([
  ["@tauri-apps/api/window:getCurrentWindow", "window"],
  ["@tauri-apps/api/webview:getCurrentWebview", "webview"],
]);
const methodPermissions = new Map([
  ["window:show", ["core:window:allow-show"]],
  ["window:setFocus", ["core:window:allow-set-focus"]],
  ["window:startDragging", ["core:window:allow-start-dragging"]],
  ["webview:onDragDropEvent", ["core:event:allow-listen", "core:event:allow-unlisten"]],
]);

async function frontendSources(directory = new URL("src/", root)) {
  const entries = await readdir(directory, { withFileTypes: true });
  const sources = [];
  for (const entry of entries) {
    const path = new URL(entry.name, directory);
    if (entry.isDirectory())
      sources.push(...(await frontendSources(new URL(`${entry.name}/`, directory))));
    else if (/\.tsx?$/.test(entry.name))
      sources.push([path.pathname, await readFile(path, "utf8")]);
  }
  return sources;
}

function requiredPermissions(sources) {
  const required = new Set();
  for (const [name, text] of sources) {
    const source = ts.createSourceFile(name, text, ts.ScriptTarget.Latest, true);
    const imports = new Map();
    const instances = new Map();
    const add = (permissions) => permissions.forEach((permission) => required.add(permission));
    for (const node of source.statements) {
      if (!ts.isImportDeclaration(node) || !ts.isStringLiteral(node.moduleSpecifier)) continue;
      const module = node.moduleSpecifier.text;
      if (!module.startsWith("@tauri-apps/")) continue;
      const clause = node.importClause;
      if (clause?.isTypeOnly) continue;
      assert.ok(
        !clause?.name && ts.isNamedImports(clause?.namedBindings),
        `${name}: inventory new Tauri imports`,
      );
      for (const binding of clause.namedBindings.elements) {
        if (binding.isTypeOnly) continue;
        const key = `${module}:${(binding.propertyName ?? binding.name).text}`;
        assert.ok(apiPermissions.has(key), `${name}: inventory ${key} before granting it`);
        imports.set(binding.name.text, key);
        add(apiPermissions.get(key));
      }
    }
    const domain = (expression) => {
      if (ts.isIdentifier(expression)) return instances.get(expression.text);
      if (ts.isCallExpression(expression) && ts.isIdentifier(expression.expression)) {
        return factories.get(imports.get(expression.expression.text));
      }
      return undefined;
    };
    const visit = (node) => {
      if (ts.isCallExpression(node) && node.expression.kind === ts.SyntaxKind.ImportKeyword) {
        const module = node.arguments[0];
        assert.ok(
          !ts.isStringLiteral(module) || !module.text.startsWith("@tauri-apps/"),
          `${name}: inventory dynamic Tauri imports`,
        );
      }
      if (ts.isVariableDeclaration(node) && node.initializer && ts.isIdentifier(node.name)) {
        const kind = domain(node.initializer);
        if (kind) instances.set(node.name.text, kind);
      }
      if (ts.isPropertyAccessExpression(node) || ts.isElementAccessExpression(node)) {
        const kind = domain(node.expression);
        if (kind) {
          assert.ok(
            ts.isPropertyAccessExpression(node),
            `${name}: inventory computed Tauri members`,
          );
          const key = `${kind}:${node.name.text}`;
          assert.ok(methodPermissions.has(key), `${name}: inventory ${key} before granting it`);
          add(methodPermissions.get(key));
        }
      }
      if (ts.isJsxAttribute(node) && node.name.getText(source) === "data-tauri-dragregion") {
        required.add("core:window:allow-start-dragging");
      }
      ts.forEachChild(node, visit);
    };
    visit(source);
  }
  return required;
}

function assertLeastPrivilege(capability, required) {
  assert.equal(capability.local, true);
  assert.equal(capability.remote, undefined);
  assert.deepEqual(capability.windows, ["main"]);
  assert.equal(capability.webviews, undefined);
  assert.equal(new Set(capability.permissions).size, capability.permissions.length);
  assert.ok(
    capability.permissions.every(
      (permission) => typeof permission === "string" && !permission.endsWith(":default"),
    ),
  );
  // Preserve the existing window-dragging contract alongside APIs found in
  // source. The current title bar is native, so it has no JS call to inventory.
  const expected = new Set([...required, "core:window:allow-start-dragging"]);
  assert.deepEqual([...capability.permissions].sort(), [...expected].sort());
}

test("main capability grants only the frontend's inventoried Tauri APIs", async () => {
  const capability = JSON.parse(
    await readFile(new URL("default.json", capabilityDirectory), "utf8"),
  );
  assertLeastPrivilege(capability, requiredPermissions(await frontendSources()));
});

test("capabilities cannot silently merge broader or remote grants into main", async () => {
  // Tauri merges all matching capability files by default, including TOML.
  // https://v2.tauri.app/security/capabilities/
  const files = (await readdir(capabilityDirectory)).filter((name) => /\.(json|toml)$/.test(name));
  assert.deepEqual(files, ["default.json"]);
  const config = JSON.parse(await readFile(new URL("src-tauri/tauri.conf.json", root), "utf8"));
  assert.ok(
    config.app.security.capabilities === undefined ||
      JSON.stringify(config.app.security.capabilities) === '["default"]',
    "review inline or additional capabilities before merging their permissions",
  );
  assert.notEqual(config.app.withGlobalTauri, true);
});

test("the inventory includes aliased listeners, drop cleanup and window methods", () => {
  const actual = requiredPermissions([
    [
      "fixture.ts",
      `
    import { listen as on } from "@tauri-apps/api/event";
    import { getCurrentWebview as currentView } from "@tauri-apps/api/webview";
    import { getCurrentWindow as currentWindow } from "@tauri-apps/api/window";
    on("menu://action", () => {});
    currentView().onDragDropEvent(() => {});
    const window = currentWindow();
    window.show();
    window.setFocus();
  `,
    ],
  ]);
  assert.deepEqual([...actual].sort(), [
    "core:event:allow-listen",
    "core:event:allow-unlisten",
    "core:window:allow-set-focus",
    "core:window:allow-show",
  ]);
});

test("missing cleanup permission and unused extra permissions are regressions", () => {
  const required = new Set(["core:event:allow-listen", "core:event:allow-unlisten"]);
  const capability = {
    local: true,
    windows: ["main"],
    permissions: [...required, "core:window:allow-start-dragging"],
  };
  assertLeastPrivilege(capability, required);
  assert.throws(() =>
    assertLeastPrivilege(
      {
        ...capability,
        permissions: capability.permissions.filter(
          (permission) => !permission.endsWith("allow-unlisten"),
        ),
      },
      required,
    ),
  );
  assert.throws(() =>
    assertLeastPrivilege(
      {
        ...capability,
        permissions: [...capability.permissions, "core:webview:allow-get-all-webviews"],
      },
      required,
    ),
  );
});

test("new unreviewed Tauri APIs fail the inventory instead of borrowing defaults", () => {
  for (const source of [
    'import { message } from "@tauri-apps/plugin-dialog";',
    'import { getCurrentWindow } from "@tauri-apps/api/window"; getCurrentWindow().close();',
    'import { getCurrentWindow } from "@tauri-apps/api/window"; getCurrentWindow()["hide"]();',
    'import("@tauri-apps/api/webview");',
  ]) {
    assert.throws(() => requiredPermissions([["unreviewed.ts", source]]), /inventory/);
  }
});
