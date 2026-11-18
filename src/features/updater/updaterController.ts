import { getVersion } from "@tauri-apps/api/app";
import { isTauri } from "@tauri-apps/api/core";
import {
  checkUpdate,
  downloadAndInstallUpdate,
  downloadUpdate,
  getUpdaterProxy,
  isUpdaterConfigured,
  listenUpdateRequests,
  listenUpdateState,
  publishUpdateState,
  requestUpdateAction,
  type DownloadEvent,
} from "./api";
import {
  isSameAutoUpdateSettings,
  loadAutoUpdateSettings,
  saveAutoUpdateSettings,
  subscribeToAutoUpdateSettings,
  type AutoUpdateSettings,
} from "./autoUpdateSettings";

export type UpdateStage =
  | "idle"
  | "checking"
  | "latest"
  | "available"
  | "downloading"
  | "ready"
  | "installing"
  | "done"
  | "error"
  | "disabled";

export type UpdateState = {
  stage: UpdateStage;
  version: string;
  nextVersion: string;
  downloaded: number;
  total: number;
  detail: string;
  /** The user hid the update todo; the settings panel still offers the restart. */
  dismissed: boolean;
};

// Only the main window owns the update: the standalone settings window can be
// destroyed and recreated without interrupting the download or starting another
// one, so it mirrors the main window's state and forwards its actions there.
const windowKind = new URLSearchParams(window.location.search).get("window");
const remoteSettings = windowKind === "settings";
const initialState: UpdateState = {
  stage: "idle",
  version: "",
  nextVersion: "",
  downloaded: 0,
  total: 0,
  detail: "",
  dismissed: false,
};

let state: UpdateState = initialState;
const listeners = new Set<() => void>();
const autoListeners = new Set<() => void>();

let autoUpdate: AutoUpdateSettings = loadAutoUpdateSettings();
let proxy: string | null = null;
let initialized = false;
let updaterReady = false;
let startupCheckStarted = false;
let checkTimer: ReturnType<typeof setInterval> | null = null;
/** One flow at a time: startup, scheduled, manual and install all share this lock. */
let flowRunning = false;

// The translation window shares this bundle but stays out of the update flow,
// otherwise every startup would check and download twice.
function isMainWindow() {
  return windowKind === null || windowKind === "main";
}

function publish() {
  if (!isMainWindow() || !isTauri()) return;
  void publishUpdateState({ state, autoUpdate: { ...autoUpdate } })
    .catch((error) => console.error("Failed to share update state", error));
}

function setState(patch: Partial<UpdateState>) {
  state = { ...state, ...patch };
  for (const listener of listeners) listener();
  publish();
}

function applyAutoUpdate(next: AutoUpdateSettings, persist = false) {
  if (isSameAutoUpdateSettings(autoUpdate, next)) return;
  const wasEnabled = autoUpdate.enabled;
  const wasInterval = autoUpdate.intervalHours;
  autoUpdate = next;
  if (persist) saveAutoUpdateSettings(next);
  if (isMainWindow() && updaterReady && (wasEnabled !== next.enabled || wasInterval !== next.intervalHours)) refreshSchedule();
  for (const listener of autoListeners) listener();
  publish();
}

export function subscribeToUpdate(listener: () => void) {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}

export function subscribeToAutoUpdate(listener: () => void) {
  autoListeners.add(listener);
  return () => {
    autoListeners.delete(listener);
  };
}

export function getUpdateState() {
  return state;
}

export function getAutoUpdateSettings() {
  return autoUpdate;
}

/** Called from the settings UI in either window; storage events mirror it to the other one. */
export function updateAutoUpdateSettings(patch: Partial<AutoUpdateSettings>) {
  applyAutoUpdate({ ...autoUpdate, ...patch }, true);
}

export function errorReason(error: unknown) {
  const reason = error instanceof Error ? error.message : String(error);
  return reason.length > 240 ? `${reason.slice(0, 240)}…` : reason;
}

function busy() {
  return flowRunning || state.stage === "checking" || state.stage === "downloading" || state.stage === "installing";
}

function trackProgress(event: DownloadEvent) {
  if (event.event === "Started") setState({ total: event.data.contentLength ?? 0 });
  else if (event.event === "Progress") setState({ downloaded: state.downloaded + event.data.chunkLength });
}

function stopSchedule() {
  if (checkTimer !== null) {
    clearInterval(checkTimer);
    checkTimer = null;
  }
}

/** Rebuilt whenever the interval or the master switch changes; no timer while off. */
function refreshSchedule() {
  stopSchedule();
  if (!isMainWindow() || !updaterReady || !autoUpdate.enabled) return;
  checkTimer = setInterval(() => {
    void runScheduledCheck();
  }, autoUpdate.intervalHours * 60 * 60 * 1000);
}

async function initialize() {
  if (initialized) return;
  initialized = true;

  subscribeToAutoUpdateSettings((next) => applyAutoUpdate(next));

  if (!isTauri()) {
    setState({ stage: "disabled" });
    return;
  }

  if (remoteSettings) {
    await listenUpdateState((snapshot) => {
      state = snapshot.state;
      for (const listener of listeners) listener();
      applyAutoUpdate(snapshot.autoUpdate);
    });
    await requestUpdateAction({ action: "snapshot" });
    return;
  }

  if (!isMainWindow()) return;

  await listenUpdateRequests((request) => {
    if (request.action === "check") void checkForUpdate();
    else if (request.action === "install") void installUpdate();
    else if (request.action === "dismiss") dismissUpdatePrompt();
    else publish();
  });

  const [version, configured] = await Promise.all([
    getVersion().catch(() => ""),
    isUpdaterConfigured().catch(() => false),
  ]);
  proxy = await getUpdaterProxy().catch(() => null);

  setState({
    version,
    stage: configured ? state.stage : "disabled",
  });
  updaterReady = configured;
  refreshSchedule();

  if (configured) void runStartupUpdate();
}

async function findUpdate() {
  setState({ nextVersion: "", downloaded: 0, total: 0, detail: "", dismissed: false, stage: "checking" });
  try {
    const found = await checkUpdate(proxy);
    if (!found) {
      setState({ stage: "latest" });
      return false;
    }
    setState({ stage: "available", nextVersion: found.version });
    return true;
  } catch (error) {
    console.error("Failed to check for updates", error);
    setState({ stage: "error", detail: errorReason(error) });
    return false;
  }
}

/** Downloads the found update; with auto-install on the caller installs it immediately. */
async function downloadFound() {
  setState({ downloaded: 0, total: 0, detail: "", stage: "downloading" });
  try {
    await downloadUpdate(trackProgress);
    // A dismissed flag stays: hiding the todo mid-download must not bring it back.
    setState({ stage: "ready" });
    if (autoUpdate.enabled && autoUpdate.autoInstall) await installReady();
  } catch (error) {
    console.error("Failed to download the update", error);
    setState({ stage: "error", detail: errorReason(error) });
  }
}

/** Installs a ready update, reusing the package `downloadUpdate` already cached natively. */
async function installReady() {
  setState({ stage: "installing" });
  try {
    await downloadAndInstallUpdate((event) => {
      if (event.event === "Finished") setState({ stage: "installing" });
      else trackProgress(event);
    });
    setState({ stage: "done" });
  } catch (error) {
    console.error("Failed to install the update", error);
    setState({ stage: "error", detail: errorReason(error) });
  }
}

/** Checks once at startup and downloads quietly; the todo list offers the restart when auto-install is off. */
async function runStartupUpdate() {
  if (startupCheckStarted || !autoUpdate.enabled || state.stage !== "idle") return;
  startupCheckStarted = true;
  await runUpdateFlow();
}

async function runUpdateFlow() {
  if (busy() || state.stage === "disabled") return;
  flowRunning = true;
  try {
    if (await findUpdate()) await downloadFound();
  } finally {
    flowRunning = false;
  }
}

async function runScheduledCheck() {
  if (!autoUpdate.enabled) return;
  await runUpdateFlow();
}

export async function checkForUpdate() {
  if (remoteSettings) return requestUpdateAction({ action: "check" });
  await runUpdateFlow();
}

/** Hiding the todo row installs nothing and keeps the downloaded package. */
export function dismissUpdatePrompt() {
  if (remoteSettings) {
    void requestUpdateAction({ action: "dismiss" }).catch((error) => console.error("Failed to dismiss update prompt", error));
    return;
  }
  if (!state.dismissed) setState({ dismissed: true });
}

export async function installUpdate() {
  if (remoteSettings) return requestUpdateAction({ action: "install" });
  if (busy() || (state.stage !== "available" && state.stage !== "ready")) return;
  flowRunning = true;
  const downloadedAlready = state.stage === "ready";
  setState(downloadedAlready ? { stage: "installing" } : { downloaded: 0, total: 0, detail: "", stage: "downloading" });
  try {
    await downloadAndInstallUpdate((event) => {
      if (event.event === "Finished") setState({ stage: "installing" });
      else trackProgress(event);
    });
    setState({ stage: "done" });
  } catch (error) {
    console.error("Failed to install the update", error);
    setState({ stage: "error", detail: errorReason(error) });
  } finally {
    flowRunning = false;
  }
}

void initialize().catch((error) => {
  console.error("Failed to initialize updates", error);
  setState({ stage: "error", detail: errorReason(error) });
});
