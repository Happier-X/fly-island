import { useEffect, useState } from "react";
import {
  LAUNCHER_STORAGE_KEY,
  loadLauncherSettings,
  saveLauncherSettings,
  type LauncherSettings,
} from "./storage.ts";

/** Told to the other copies in this window when one copy saves a change. */
const SAVED_IN_THIS_WINDOW = "focusd-island-launcher-settings-saved";

/**
 * Launcher settings, saved on every change and kept in sync between the settings
 * window, the launcher panel (which reads them on every open) and copies inside one
 * window (the shortcuts page and the launcher page both edit them).
 */
export function useLauncherSettings() {
  const [settings, setSettings] = useState<LauncherSettings>(loadLauncherSettings);

  useEffect(() => {
    const before = localStorage.getItem(LAUNCHER_STORAGE_KEY);
    saveLauncherSettings(settings);
    if (localStorage.getItem(LAUNCHER_STORAGE_KEY) !== before) {
      window.dispatchEvent(new Event(SAVED_IN_THIS_WINDOW));
    }
  }, [settings]);

  useEffect(() => {
    const reload = () => setSettings(loadLauncherSettings());
    const syncFromOtherWindow = (event: StorageEvent) => {
      if (event.key === LAUNCHER_STORAGE_KEY) reload();
    };
    window.addEventListener("storage", syncFromOtherWindow);
    window.addEventListener(SAVED_IN_THIS_WINDOW, reload);
    return () => {
      window.removeEventListener("storage", syncFromOtherWindow);
      window.removeEventListener(SAVED_IN_THIS_WINDOW, reload);
    };
  }, []);

  return [settings, setSettings] as const;
}
