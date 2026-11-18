import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";
import { runInNewContext } from "node:vm";
import ts from "typescript";

const transpile = (path) => ts.transpileModule(
  readFileSync(new URL(path, import.meta.url), "utf8"),
  { compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2022 } },
).outputText;
const source = transpile("../../src/features/updater/autoUpdateSettings.ts");

function module(stored = undefined) {
  const exports = {};
  const data = new Map();
  if (stored !== undefined) data.set("focusd-island-auto-update-settings", stored);
  const listeners = [];
  runInNewContext(source, {
    exports,
    require: (name) => {
      if (name === "../../lib/storage.ts") {
        return {
          readJson: (key) => {
        const value = data.get(key);
        return typeof value === "string" ? JSON.parse(value) : value;
      },
          writeJson: (key, value) => { data.set(key, JSON.stringify(value)); return true; },
        };
      }
      throw new Error(`Unexpected import: ${name}`);
    },
    window: {
      addEventListener: (name, listener) => listeners.push([name, listener]),
    },
  });
  return Object.assign(exports, {
    data,
    fire(key) {
      for (const [name, listener] of listeners) {
        if (name === "storage") listener({ key });
      }
    },
  });
}

test("first launch enables both checks and automatic installs every six hours", () => {
  const settings = module();
  assert.deepEqual(JSON.parse(JSON.stringify(settings.loadAutoUpdateSettings())), {
    enabled: true, autoInstall: true, intervalHours: 6,
  });
});

test("saved choices are read back and bad or unsupported values fall back", () => {
  const saved = module(JSON.stringify({ enabled: false, autoInstall: false, intervalHours: 24 }));
  assert.deepEqual(JSON.parse(JSON.stringify(saved.loadAutoUpdateSettings())), {
    enabled: false, autoInstall: false, intervalHours: 24,
  });
  // readJson already returns undefined for corrupt data, just like the real helper.
  const bad = module(undefined);
  assert.equal(bad.loadAutoUpdateSettings().intervalHours, 6);
  const partial = module({ enabled: false, intervalHours: 100 });
  assert.deepEqual(JSON.parse(JSON.stringify(partial.loadAutoUpdateSettings())), {
    enabled: false, autoInstall: true, intervalHours: 6,
  });
});

test("saving writes the shared key and storage events deliver other windows' changes", () => {
  const settings = module();
  assert.equal(settings.saveAutoUpdateSettings({ enabled: false, autoInstall: false, intervalHours: 12 }), true);
  assert.deepEqual(JSON.parse(settings.data.get("focusd-island-auto-update-settings")), {
    enabled: false, autoInstall: false, intervalHours: 12,
  });
  let seen = null;
  settings.subscribeToAutoUpdateSettings((next) => { seen = next; });
  settings.data.set("focusd-island-auto-update-settings", { enabled: true, autoInstall: true, intervalHours: 1 });
  settings.fire("focusd-island-auto-update-settings");
  assert.deepEqual(JSON.parse(JSON.stringify(seen)), { enabled: true, autoInstall: true, intervalHours: 1 });
  settings.fire("focusd-island-settings");
  assert.deepEqual(JSON.parse(JSON.stringify(seen)), { enabled: true, autoInstall: true, intervalHours: 1 });
});

test("equality ignores no differences and catches every persisted field", () => {
  const settings = module();
  assert.equal(settings.isSameAutoUpdateSettings(
    { enabled: true, autoInstall: true, intervalHours: 6 },
    { enabled: true, autoInstall: true, intervalHours: 6 },
  ), true);
  assert.equal(settings.isSameAutoUpdateSettings(
    { enabled: true, autoInstall: true, intervalHours: 6 },
    { enabled: true, autoInstall: true, intervalHours: 12 },
  ), false);
  assert.equal(settings.isSameAutoUpdateSettings(
    { enabled: true, autoInstall: true, intervalHours: 6 },
    { enabled: true, autoInstall: false, intervalHours: 6 },
  ), false);
});
