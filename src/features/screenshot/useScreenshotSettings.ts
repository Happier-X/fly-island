import { useCallback, useEffect, useState } from "react";
import {
  DEFAULT_OVERLAY_KEYS,
  DEFAULT_SCREENSHOT_SETTINGS,
  getScreenshotSettings,
  OCR_LANG_AUTO,
  setScreenshotSettings,
  UI_SCALES,
  type OverlayKeys,
  type ScreenshotSettings,
} from "./api";
import { normalizeScreenshotShortcut } from "./shortcut";

/** Keeps every field of a stored setting usable, whatever the file on disk says. */
function normalize(settings: ScreenshotSettings): ScreenshotSettings {
  return {
    ...settings,
    shortcut: normalizeScreenshotShortcut(settings.shortcut),
    saveFormat: ["png", "jpeg", "webp"].includes(settings.saveFormat) ? settings.saveFormat : "png",
    saveDir: settings.saveDir ?? "",
    colors: Array.isArray(settings.colors) && settings.colors.length > 0
      ? settings.colors
      : [...DEFAULT_SCREENSHOT_SETTINGS.colors],
    overlayKeys: normalizeOverlayKeys(settings.overlayKeys),
    hiddenToolbar: Array.isArray(settings.hiddenToolbar) ? settings.hiddenToolbar : [],
    ocrLanguage: settings.ocrLanguage?.trim() || OCR_LANG_AUTO,
    uiScale: UI_SCALES.includes(settings.uiScale as (typeof UI_SCALES)[number]) ? settings.uiScale : "auto",
    autoSave: settings.autoSave === true,
    enabled: settings.enabled ?? true,
  };
}

function normalizeOverlayKeys(keys: Partial<OverlayKeys> | undefined): OverlayKeys {
  const resolved = { ...DEFAULT_OVERLAY_KEYS };
  for (const field of Object.keys(resolved) as (keyof OverlayKeys)[]) {
    const value = keys?.[field];
    if (typeof value === "number" && Number.isInteger(value) && value >= 0 && value <= 0xff) {
      resolved[field] = value;
    }
  }
  return resolved;
}

/** Screenshot settings for the settings window, kept in sync with the native store. */
export function useScreenshotSettings() {
  const [settings, setSettings] = useState<ScreenshotSettings>(DEFAULT_SCREENSHOT_SETTINGS);

  const refresh = useCallback(async () => {
    try {
      setSettings(normalize(await getScreenshotSettings()));
    } catch (error) {
      console.error("Failed to read screenshot settings", error);
    }
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const update = useCallback(
    async (next: ScreenshotSettings) => {
      const normalized = normalize(next);
      setSettings(normalized);
      try {
        await setScreenshotSettings(normalized);
      } catch (error) {
        console.error("Failed to update screenshot settings", error);
        void refresh();
      }
    },
    [refresh],
  );

  return [settings, update] as const;
}
