import { useEffect, useMemo, useRef, useState } from "react";
import { getLauncherIndex, listenLauncherIndexReady } from "./api.ts";
import type { LauncherSettings } from "./storage.ts";
import type { LauncherEntry } from "./types.ts";
import { useLauncherDefaultFolders } from "./useLauncherFolders.ts";

/**
 * Applications and files for the panel. Applications come from the native cache and
 * are ready almost at once; files are scanned in the background by the native side.
 * `openCount` reloads the index each time the panel opens.
 */
export function useLauncherIndex(settings: LauncherSettings, openCount: number) {
  const defaults = useLauncherDefaultFolders();
  const folders = useMemo(
    () => (settings.folderMode === "custom" ? settings.folders : [...defaults]),
    [settings.folderMode, settings.folders, defaults],
  );
  const [apps, setApps] = useState<LauncherEntry[]>([]);
  const [files, setFiles] = useState<LauncherEntry[]>([]);
  const [appsReady, setAppsReady] = useState(false);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");
  const [revision, setRevision] = useState(0);
  const requestId = useRef(0);

  useEffect(() => {
    // The window is preloaded hidden; its index is read when it first opens, not at start-up.
    if (openCount === 0) return;
    if (!settings.enabled || (!settings.searchApps && !settings.searchFiles)) {
      setApps([]);
      setFiles([]);
      setLoading(false);
      return;
    }
    const id = ++requestId.current;
    setLoading(true);
    void getLauncherIndex({
      includeApps: settings.searchApps,
      includeFiles: settings.searchFiles,
      folders,
    })
      .then((next) => {
        if (id !== requestId.current) return;
        setApps(next.apps);
        setFiles(next.files);
        setAppsReady(next.appsReady);
        setError("");
        setLoading(false);
      })
      .catch((failure) => {
        if (id !== requestId.current) return;
        console.error("Failed to load the launcher index", failure);
        setError(String(failure));
        setLoading(false);
      });
  }, [openCount, revision, settings.enabled, settings.searchApps, settings.searchFiles, folders]);

  // The first application enumeration runs in the background after start-up.
  useEffect(() => {
    let didCancel = false;
    let unlisten: (() => void) | null = null;
    void listenLauncherIndexReady(() => setRevision((value) => value + 1))
      .then((nextUnlisten) => {
        if (didCancel) nextUnlisten();
        else unlisten = nextUnlisten;
      })
      .catch((failure) => console.error("Failed to listen for the launcher index", failure));
    return () => {
      didCancel = true;
      unlisten?.();
    };
  }, []);

  return { apps, files, appsReady, loading, error };
}
