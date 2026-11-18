import { readJson, writeJson } from "../../lib/storage.ts";
import type { SearchEngine } from "./types.ts";

export const LAUNCHER_STORAGE_KEY = "focusd-island-launcher-settings";

export const SEARCH_ENGINES: readonly SearchEngine[] = [
  "bing",
  "google",
  "baidu",
  "duckduckgo",
];

/** `default` searches Desktop, Documents and Downloads; `custom` uses `folders` as they are. */
export type FolderMode = "default" | "custom";

export type LauncherTriggerMode = "shortcut" | "doubleCtrl";
export const DOUBLE_CTRL_TRIGGER = "DoubleCtrl";

export type LauncherSettings = {
  /** The module switch; off releases the shortcut and hides the panel. */
  enabled: boolean;
  shortcut: string;
  triggerMode: LauncherTriggerMode;
  searchApps: boolean;
  searchFiles: boolean;
  searchWeb: boolean;
  searchEngine: SearchEngine;
  folderMode: FolderMode;
  folders: string[];
};

export const DEFAULT_LAUNCHER_SETTINGS: LauncherSettings = {
  enabled: true,
  shortcut: "Alt+Space",
  triggerMode: "shortcut",
  searchApps: true,
  searchFiles: true,
  searchWeb: true,
  searchEngine: "bing",
  folderMode: "default",
  folders: [],
};

function normalizeFolders(value: unknown): string[] {
  if (!Array.isArray(value)) return [];
  return [...new Set(
    value
      .filter((folder): folder is string => typeof folder === "string")
      .map((folder) => folder.trim())
      .filter(Boolean),
  )];
}

function normalizeShortcut(value: unknown): string {
  if (typeof value !== "string") return DEFAULT_LAUNCHER_SETTINGS.shortcut;
  const shortcut = value.trim();
  return shortcut.split("+").filter(Boolean).length >= 2
    ? shortcut
    : DEFAULT_LAUNCHER_SETTINGS.shortcut;
}

export function normalizeLauncherSettings(stored: unknown): LauncherSettings {
  if (typeof stored !== "object" || stored === null) return DEFAULT_LAUNCHER_SETTINGS;
  const value = stored as Record<string, unknown>;
  return {
    enabled: typeof value.enabled === "boolean" ? value.enabled : DEFAULT_LAUNCHER_SETTINGS.enabled,
    shortcut: normalizeShortcut(value.shortcut),
    triggerMode: value.triggerMode === "doubleCtrl" ? "doubleCtrl" : "shortcut",
    searchApps: typeof value.searchApps === "boolean" ? value.searchApps : DEFAULT_LAUNCHER_SETTINGS.searchApps,
    searchFiles: typeof value.searchFiles === "boolean" ? value.searchFiles : DEFAULT_LAUNCHER_SETTINGS.searchFiles,
    searchWeb: typeof value.searchWeb === "boolean" ? value.searchWeb : DEFAULT_LAUNCHER_SETTINGS.searchWeb,
    searchEngine: SEARCH_ENGINES.includes(value.searchEngine as SearchEngine)
      ? (value.searchEngine as SearchEngine)
      : DEFAULT_LAUNCHER_SETTINGS.searchEngine,
    folderMode: value.folderMode === "custom" ? "custom" : "default",
    folders: normalizeFolders(value.folders),
  };
}

export function loadLauncherSettings(): LauncherSettings {
  return normalizeLauncherSettings(readJson(LAUNCHER_STORAGE_KEY));
}

export function saveLauncherSettings(settings: LauncherSettings) {
  writeJson(LAUNCHER_STORAGE_KEY, settings);
}

export function launcherTrigger(settings: LauncherSettings) {
  return settings.triggerMode === "doubleCtrl" ? DOUBLE_CTRL_TRIGGER : settings.shortcut;
}
