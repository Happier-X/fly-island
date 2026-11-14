import { useEffect, useState } from "react";
import {
  NOTIFICATION_SOUNDS_STORAGE_KEY,
  loadNotificationSoundSettings,
  saveNotificationSoundSettings,
  type NotificationSoundSettings,
} from "./storage.ts";

/**
 * The sound switch, saved on every change and kept in sync between the settings window
 * (where it is edited) and the island (where it plays). Writing back a value that came from
 * the other window stores the same string, which fires no further storage event.
 */
export function useSoundSettings() {
  const [settings, setSettings] = useState<NotificationSoundSettings>(loadNotificationSoundSettings);

  useEffect(() => {
    saveNotificationSoundSettings(settings);
  }, [settings]);

  useEffect(() => {
    const syncFromOtherWindow = (event: StorageEvent) => {
      if (event.key === NOTIFICATION_SOUNDS_STORAGE_KEY) setSettings(loadNotificationSoundSettings());
    };
    window.addEventListener("storage", syncFromOtherWindow);
    return () => window.removeEventListener("storage", syncFromOtherWindow);
  }, []);

  return [settings, setSettings] as const;
}
