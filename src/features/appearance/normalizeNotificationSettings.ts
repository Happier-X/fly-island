// Reading saved settings, part 2 (normalizeSettings in settings.ts): Cindy notifications.
import type { IslandSettings, LegacyIslandSettings } from "./types";

/**
 * The notification switches. The master switch and each event default to on; an older version
 * that never saved them keeps the default.
 */
export function normalizeNotificationSettings(settings: LegacyIslandSettings | null | undefined) {
  return {
    cindyNotificationsEnabled: settings?.cindyNotificationsEnabled !== false,
    cindyNotifyDone: settings?.cindyNotifyDone !== false,
    cindyNotifyQuestion: settings?.cindyNotifyQuestion !== false,
    cindyNotifyScheduled: settings?.cindyNotifyScheduled !== false,
    cindyNotifyError: settings?.cindyNotifyError !== false,
  } satisfies Partial<IslandSettings>;
}
