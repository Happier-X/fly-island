import { invoke } from "@tauri-apps/api/core";
import { listen, type EventCallback } from "@tauri-apps/api/event";
import type { LauncherEntry } from "./types.ts";

// 与 src-tauri/src/quick_launcher/mod.rs 中的 OPENED_EVENT、INDEX_READY_EVENT 一致。
export const LAUNCHER_OPENED_EVENT = "quick-launcher-opened";
export const LAUNCHER_INDEX_READY_EVENT = "quick-launcher-index-ready";

// 与 quick_launcher/mod.rs 中同名结构对应。
export type LauncherIndex = {
  apps: LauncherEntry[];
  files: LauncherEntry[];
  /** False while the first application enumeration is still running. */
  appsReady: boolean;
};

export type LauncherScope = {
  includeApps: boolean;
  includeFiles: boolean;
  folders: string[];
};

export function getLauncherIndex(scope: LauncherScope) {
  return invoke<LauncherIndex>("get_launcher_index", scope);
}

export function getLauncherDefaultFolders() {
  return invoke<string[]>("get_launcher_default_folders");
}

export function getLauncherIcon(path: string, kind: "app" | "file") {
  return invoke<string | null>("get_launcher_icon", { path, kind });
}

export function openLauncherItem(target: string) {
  return invoke<void>("open_launcher_item", { target });
}

export function openLauncherUrl(url: string) {
  return invoke<void>("open_launcher_url", { url });
}

export function hideQuickLauncher() {
  return invoke<void>("hide_quick_launcher");
}

export function setLauncherShortcut(shortcut: string) {
  return invoke<string>("set_launcher_shortcut", { shortcut });
}

/** Off releases the shortcut and hides the panel; on registers the saved trigger. */
export function setLauncherEnabled(enabled: boolean, shortcut: string) {
  return invoke<void>("set_launcher_enabled", { enabled, shortcut });
}

export function listenLauncherOpened(handler: EventCallback<unknown>) {
  return listen(LAUNCHER_OPENED_EVENT, handler);
}

export function listenLauncherIndexReady(handler: EventCallback<unknown>) {
  return listen(LAUNCHER_INDEX_READY_EVENT, handler);
}
