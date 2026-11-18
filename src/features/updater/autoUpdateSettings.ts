import { readJson, writeJson } from "../../lib/storage.ts";

export const AUTO_UPDATE_STORAGE_KEY = "focusd-island-auto-update-settings";

/** Hours between two scheduled checks; `0` is only used by tests and never offered. */
export const AUTO_UPDATE_INTERVALS_HOURS = [1, 6, 12, 24] as const;
export type AutoUpdateIntervalHours = (typeof AUTO_UPDATE_INTERVALS_HOURS)[number];

export type AutoUpdateSettings = {
  /** Off stops scheduled checks and automatic installs; manual checks still work. */
  enabled: boolean;
  /** Install and restart as soon as a download finishes, without waiting for a click. */
  autoInstall: boolean;
  intervalHours: AutoUpdateIntervalHours;
};

export const DEFAULT_AUTO_UPDATE_SETTINGS: AutoUpdateSettings = {
  enabled: true,
  autoInstall: true,
  intervalHours: 6,
};

export function normalizeAutoUpdateSettings(stored: unknown): AutoUpdateSettings {
  if (typeof stored !== "object" || stored === null) return DEFAULT_AUTO_UPDATE_SETTINGS;
  const value = stored as Record<string, unknown>;
  return {
    enabled: typeof value.enabled === "boolean" ? value.enabled : DEFAULT_AUTO_UPDATE_SETTINGS.enabled,
    autoInstall: typeof value.autoInstall === "boolean" ? value.autoInstall : DEFAULT_AUTO_UPDATE_SETTINGS.autoInstall,
    intervalHours: AUTO_UPDATE_INTERVALS_HOURS.find((hours) => hours === value.intervalHours)
      ?? DEFAULT_AUTO_UPDATE_SETTINGS.intervalHours,
  };
}

export function loadAutoUpdateSettings(): AutoUpdateSettings {
  return normalizeAutoUpdateSettings(readJson(AUTO_UPDATE_STORAGE_KEY));
}

export function saveAutoUpdateSettings(settings: AutoUpdateSettings): boolean {
  return writeJson(AUTO_UPDATE_STORAGE_KEY, settings);
}

export function isSameAutoUpdateSettings(a: AutoUpdateSettings, b: AutoUpdateSettings) {
  return a.enabled === b.enabled && a.autoInstall === b.autoInstall && a.intervalHours === b.intervalHours;
}

/**
 * The settings window writes this key; both windows subscribe to its storage events so a
 * change takes effect immediately, without waiting for a restart.
 */
export function subscribeToAutoUpdateSettings(listener: (settings: AutoUpdateSettings) => void) {
  const onStorage = (event: StorageEvent) => {
    if (event.key === AUTO_UPDATE_STORAGE_KEY) listener(loadAutoUpdateSettings());
  };
  window.addEventListener("storage", onStorage);
  return () => window.removeEventListener("storage", onStorage);
}
