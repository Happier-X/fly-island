import { useCallback, useEffect, useMemo, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  hideQuickLauncher,
  listenLauncherOpened,
  openLauncherItem,
  openLauncherUrl,
  setLauncherEnabled,
} from "../../features/quick-launcher/api";
import { LauncherView } from "../../features/quick-launcher/LauncherView";
import {
  launcherTrigger,
  loadLauncherSettings,
  type LauncherSettings,
} from "../../features/quick-launcher/storage";
import type { LauncherResult } from "../../features/quick-launcher/types";
import { useLauncherIcons } from "../../features/quick-launcher/useLauncherIcons";
import { useLauncherIndex } from "../../features/quick-launcher/useLauncherIndex";
import { DEFAULT_SETTINGS } from "../../features/appearance/settings";
import { STORAGE_KEY as ISLAND_SETTINGS_KEY } from "../../features/appearance/storage";
import { useTranslation } from "../../i18n";
import { readJson } from "../../lib/storage";

/** The island's background colour, read without the migration side effects of loadSettings. */
function loadIslandSurfaceColor() {
  const stored = readJson(ISLAND_SETTINGS_KEY);
  if (typeof stored !== "object" || stored === null) return DEFAULT_SETTINGS.islandBackgroundColor;
  const color = (stored as Record<string, unknown>).islandBackgroundColor;
  return typeof color === "string" ? color : DEFAULT_SETTINGS.islandBackgroundColor;
}

/**
 * The standalone quick launcher window: loads applications and files while hidden,
 * then clears the field, lists matches and opens the chosen row. Closing (Esc, losing
 * the focus, or the shortcut again) is handled here and in the native window.
 */
export function LauncherWindow() {
  const t = useTranslation();
  const [settings, setSettings] = useState<LauncherSettings>(loadLauncherSettings);
  const [openToken, setOpenToken] = useState(0);
  const [results, setResults] = useState<LauncherResult[]>([]);
  const [openError, setOpenError] = useState("");
  const [surfaceColor, setSurfaceColor] = useState(loadIslandSurfaceColor);

  const index = useLauncherIndex(settings, openToken);
  const entries = useMemo(() => [...index.apps, ...index.files], [index.apps, index.files]);
  const icons = useLauncherIcons(results);
  const loading = index.loading || (settings.searchApps && !index.appsReady && !index.error);

  // Registers the saved shortcut when the app starts and after the module is switched
  // on; switching it off releases the keys at once.
  useEffect(() => {
    const saved = loadLauncherSettings();
    setSettings(saved);
    void setLauncherEnabled(saved.enabled, launcherTrigger(saved))
      .catch((error) => console.error("Failed to apply the launcher shortcut", error));
  }, []);

  const handleOpened = useCallback(() => {
    setSettings(loadLauncherSettings());
    setSurfaceColor(loadIslandSurfaceColor());
    setOpenError("");
    setOpenToken((value) => value + 1);
  }, []);

  useEffect(() => {
    let didCancel = false;
    let unlisten: (() => void) | null = null;
    void listenLauncherOpened(handleOpened).then((nextUnlisten) => {
      if (didCancel) nextUnlisten();
      else unlisten = nextUnlisten;
    });
    // The event may have fired before this listener existed; a window already up opens now.
    void getCurrentWindow()
      .isVisible()
      .then((visible) => {
        if (!didCancel && visible) handleOpened();
      })
      .catch(() => {});
    return () => {
      didCancel = true;
      unlisten?.();
    };
  }, [handleOpened]);

  const close = useCallback(() => {
    void hideQuickLauncher().catch((error) => {
      console.error("Failed to hide the quick launcher", error);
    });
  }, []);

  // Escape works even after the focus moved off the field (for example after a click).
  useEffect(() => {
    const onKeyDown = (event: globalThis.KeyboardEvent) => {
      if (event.key === "Escape" && !event.defaultPrevented && !event.isComposing && event.keyCode !== 229) {
        event.preventDefault();
        close();
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [close]);

  const open = useCallback(
    (result: LauncherResult) => {
      setOpenError("");
      const opening =
        result.kind === "web"
          ? openLauncherUrl(result.target)
          : openLauncherItem(result.target);
      void opening
        .then(() => {
          // The item opened; failing to hide this panel must not look like a failed open.
          void hideQuickLauncher().catch((error) => {
            console.error("Failed to hide the quick launcher", error);
          });
        })
        .catch(() => {
          setOpenError(t.launcher.openFailed(result.name));
        });
    },
    [t],
  );

  return (
    <LauncherView
      entries={entries}
      settings={settings}
      icons={icons}
      loading={loading}
      openToken={openToken}
      openError={openError || (index.error ? t.launcher.indexFailed : "")}
      surfaceColor={surfaceColor}
      onOpen={open}
      onClose={close}
      onResultsChange={setResults}
    />
  );
}
