import { useEffect, useState } from "react";
import { getLauncherDefaultFolders } from "./api.ts";
import type { LauncherSettings } from "./storage.ts";

let cachedDefaults: Promise<string[]> | null = null;

/** Desktop, Documents and Downloads from Windows; remembered for the session. */
export function loadLauncherDefaultFolders() {
  cachedDefaults ??= getLauncherDefaultFolders().catch((error) => {
    console.error("Failed to read the default launcher folders", error);
    return [];
  });
  return cachedDefaults;
}

export function useLauncherDefaultFolders() {
  const [folders, setFolders] = useState<string[]>([]);
  useEffect(() => {
    let didCancel = false;
    void loadLauncherDefaultFolders().then((next) => {
      if (!didCancel) setFolders(next);
    });
    return () => {
      didCancel = true;
    };
  }, []);
  return folders;
}

/** The folders searched now: the saved list in custom mode, Windows' folders otherwise. */
export function effectiveLauncherFolders(settings: LauncherSettings, defaults: readonly string[]) {
  return settings.folderMode === "custom" ? settings.folders : [...defaults];
}
