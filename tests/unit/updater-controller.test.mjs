import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";
import { runInNewContext } from "node:vm";
import ts from "typescript";

const transpile = (path) => ts.transpileModule(
  readFileSync(new URL(path, import.meta.url), "utf8"),
  { compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2022 } },
).outputText;
const source = transpile("../../src/features/updater/updaterController.ts");
const apiSource = transpile("../../src/features/updater/api.ts");

class Channel {
  onmessage = () => {};
}

const settle = () => new Promise(setImmediate);

async function controller({
  configured = true,
  check = async () => null,
  download = async () => {},
  install = async () => {},
  version = "0.2.11",
  windowKind = null,
  autoUpdate = { enabled: true, autoInstall: true, intervalHours: 6 },
  timers = null,
} = {}) {
  const exports = {};
  const calls = [];
  const emitted = [];
  const stateListeners = [];
  const mocks = {
    "@tauri-apps/api/app": { getVersion: async () => version },
    "@tauri-apps/api/core": {
      Channel,
      isTauri: () => true,
      invoke: async (command, args) => {
        calls.push(command);
        const send = (event) => args.onEvent.onmessage(event);
        if (command === "updater_configured") return configured;
        if (command === "get_updater_proxy") return null;
        if (command === "updater_check") return check(args);
        if (command === "updater_download") return download(send);
        if (command === "updater_install") return install(send);
        throw new Error(`Unexpected command: ${command}`);
      },
    },
    "@tauri-apps/api/event": {
      listen: async (name, callback) => {
        stateListeners.push([name, callback]);
        return () => {};
      },
      emit: async (name, payload) => emitted.push({ name, payload }),
      emitTo: async (target, name, payload) => emitted.push({ target, name, payload }),
    },
    "./api": null,
    "./autoUpdateSettings": null,
  };
  const api = {};
  runInNewContext(apiSource, { exports: api, require: (name) => mocks[name] });
  mocks["./api"] = api;
  mocks["./autoUpdateSettings"] = {
    loadAutoUpdateSettings: () => ({ ...autoUpdate }),
    saveAutoUpdateSettings: (next) => { autoUpdate = { ...next }; return true; },
    isSameAutoUpdateSettings: (a, b) => a.enabled === b.enabled && a.autoInstall === b.autoInstall && a.intervalHours === b.intervalHours,
    subscribeToAutoUpdateSettings: () => () => {},
  };
  const sandboxTimers = timers ?? {
    setInterval: () => 1,
    clearInterval: () => {},
  };
  runInNewContext(source, {
    exports,
    window: { location: { search: windowKind ? `?window=${windowKind}` : "" } },
    URLSearchParams,
    require: (name) => {
      assert.ok(Object.hasOwn(mocks, name), `Unexpected import: ${name}`);
      return mocks[name];
    },
    console: { error() {} },
    setInterval: sandboxTimers.setInterval,
    clearInterval: sandboxTimers.clearInterval,
  });
  for (let i = 0; i < 5; i++) await settle();
  return Object.assign(exports, {
    calls, emitted,
    emitState(snapshot) {
      for (const [name, callback] of stateListeners) {
        if (name === "focusd-update-state") callback({ payload: snapshot });
      }
    },
  });
}

test("startup checks the stable channel and auto-installs by default", async () => {
  const seen = [];
  const installed = [];
  const updater = await controller({
    version: "0.3.0-beta.1",
    check: async (args) => { seen.push(args.channel); return { version: "0.3.0", body: null }; },
    install: async (send) => { installed.push("install"); send({ event: "Finished" }); },
  });
  assert.deepEqual(seen, ["stable"]);
  assert.deepEqual(installed, ["install"]);
  assert.equal(updater.getUpdateState().stage, "done");
});

test("turning auto-install off keeps the download and the manual restart", async () => {
  const installed = [];
  const updater = await controller({
    autoUpdate: { enabled: true, autoInstall: false, intervalHours: 6 },
    check: async () => ({ version: "0.3.0", body: null }),
    install: async (send) => { installed.push("install"); send({ event: "Finished" }); },
  });
  assert.equal(updater.getUpdateState().stage, "ready");
  assert.deepEqual(installed, []);
  await updater.installUpdate();
  assert.equal(updater.getUpdateState().stage, "done");
  assert.deepEqual(installed, ["install"]);
});

test("switching auto-install on during a download installs that same update once", async () => {
  let finishDownload = null;
  const installs = [];
  const updater = await controller({
    autoUpdate: { enabled: true, autoInstall: false, intervalHours: 6 },
    check: async () => ({ version: "0.3.0", body: null }),
    download: () => new Promise((resolve) => { finishDownload = resolve; }),
    install: async (send) => { installs.push("install"); send({ event: "Finished" }); },
  });
  assert.equal(updater.getUpdateState().stage, "downloading");
  updater.updateAutoUpdateSettings({ autoInstall: true });
  assert.equal(installs.length, 0);
  finishDownload();
  await settle();
  assert.deepEqual(installs, ["install"]);
  assert.equal(updater.getUpdateState().stage, "done");
  assert.equal(updater.calls.filter((call) => call === "updater_download").length, 1);
});

test("turning the master switch off during a download cancels the pending auto-install", async () => {
  let finishDownload = null;
  const installs = [];
  const updater = await controller({
    check: async () => ({ version: "0.3.0", body: null }),
    download: () => new Promise((resolve) => { finishDownload = resolve; }),
    install: async (send) => { installs.push("install"); send({ event: "Finished" }); },
  });
  assert.equal(updater.getUpdateState().stage, "downloading");
  updater.updateAutoUpdateSettings({ enabled: false });
  finishDownload();
  await settle();
  assert.deepEqual(installs, []);
  assert.equal(updater.getUpdateState().stage, "ready");
});

test("scheduled checks use the selected interval and stop while disabled", async () => {
  const intervals = [];
  const cleared = [];
  const ticks = [];
  const callbacks = [];
  const timers = {
    setInterval: (fn, ms) => { intervals.push(ms); callbacks.push(fn); return { fn, id: intervals.length }; },
    clearInterval: (timer) => { cleared.push(timer?.id); },
  };
  const updater = await controller({
    check: async () => { ticks.push("check"); return null; },
    timers,
  });
  assert.deepEqual(intervals, [6 * 60 * 60 * 1000]);
  ticks.length = 0;
  await callbacks[0]();
  assert.deepEqual(ticks, ["check"], "the timer checks on its interval");
  ticks.length = 0;
  updater.updateAutoUpdateSettings({ intervalHours: 12 });
  assert.equal(cleared[0], 1, `cleared timers: ${JSON.stringify(cleared)}`);
  assert.deepEqual(intervals, [6 * 60 * 60 * 1000, 12 * 60 * 60 * 1000]);
  updater.updateAutoUpdateSettings({ enabled: false });
  assert.equal(cleared.at(-1), 2);
  assert.equal(intervals.length, 2);
  ticks.length = 0;
  await updater.checkForUpdate();
  assert.equal(ticks.length, 0, "disabling stops automatic checks but not manual ones");
  await callbacks[1]();
  assert.equal(ticks.length, 0, "a stale scheduled tick is ignored while disabled");
});

test("a scheduled tick during an active flow does not start a second download", async () => {
  let finishCheck = null;
  let checkCount = 0;
  const downloads = [];
  let timer = null;
  let scheduled = false;
  const timers = {
    setInterval: (fn) => { timer = fn; scheduled = true; return { fn, id: 1 }; },
    clearInterval: () => { scheduled = false; },
  };
  const updater = await controller({
    autoUpdate: { enabled: true, autoInstall: true, intervalHours: 6 },
    check: async () => {
      checkCount++;
      if (checkCount === 1) return null;
      assert.equal(scheduled, true, "a scheduled tick comes from a live timer");
      if (checkCount === 2) return new Promise((resolve) => { finishCheck = resolve; });
      return { version: "0.3.1", body: null };
    },
    download: async () => { downloads.push("download"); },
    timers,
  });
  await timer();
  await timer();
  assert.equal(checkCount, 2, "the second tick starts, the third is skipped while it runs");
  finishCheck({ version: "0.3.1", body: null });
  await settle();
  assert.deepEqual(downloads, ["download"]);
});

test("a stale tick after disabling stops scheduling further checks", async () => {
  let timer = null;
  const ticks = [];
  let scheduled = false;
  const timers = {
    setInterval: (fn) => { timer = fn; scheduled = true; return 1; },
    clearInterval: () => { scheduled = false; },
  };
  const updater = await controller({
    autoUpdate: { enabled: true, autoInstall: true, intervalHours: 6 },
    check: async () => { ticks.push("check"); return null; },
    timers,
  });
  ticks.length = 0;
  updater.updateAutoUpdateSettings({ enabled: false });
  assert.equal(scheduled, false);
  await timer();
  assert.deepEqual(ticks, []);
});

test("manual checks work while automatic updates are disabled", async () => {
  const seen = [];
  const updater = await controller({
    autoUpdate: { enabled: false, autoInstall: false, intervalHours: 6 },
    check: async () => { seen.push("check"); return null; },
  });
  assert.deepEqual(seen, []);
  await updater.checkForUpdate();
  assert.deepEqual(seen, ["check"]);
  assert.equal(updater.getUpdateState().stage, "latest");
});

test("the settings window mirrors state and forwards actions to the main window", async () => {
  const updater = await controller({ windowKind: "settings" });
  await updater.checkForUpdate();
  assert.ok(!updater.calls.includes("updater_check"));
  assert.deepEqual(
    JSON.stringify(updater.emitted.filter((event) => event.target === "main").map((event) => event.payload)),
    JSON.stringify([{ action: "snapshot" }, { action: "check" }]),
  );
  updater.emitState({
    state: { ...updater.getUpdateState(), stage: "ready", nextVersion: "9.9.9" },
    autoUpdate: { enabled: false, autoInstall: false, intervalHours: 12 },
  });
  assert.equal(updater.getUpdateState().stage, "ready");
  assert.deepEqual(JSON.parse(JSON.stringify(updater.getAutoUpdateSettings())), {
    enabled: false, autoInstall: false, intervalHours: 12,
  });
});

test("hiding the update todo keeps the download", async () => {
  const updater = await controller({
    autoUpdate: { enabled: true, autoInstall: false, intervalHours: 6 },
    check: async () => ({ version: "0.3.0", body: null }),
  });
  updater.dismissUpdatePrompt();
  assert.equal(updater.getUpdateState().dismissed, true);
  assert.equal(updater.getUpdateState().stage, "ready");
});

test("an unconfigured build never checks", async () => {
  const updater = await controller({ configured: false });
  assert.equal(updater.getUpdateState().stage, "disabled");
  assert.ok(!updater.calls.includes("updater_check"));
});
