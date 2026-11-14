import { useCallback, useMemo } from "react";
import type { IslandSettings } from "../../features/appearance/types";
import { focusAgentWindow } from "../../features/cindy-notifications/api";
import { useCindyNotifications } from "../../features/cindy-notifications/useCindyNotifications";
import { useNotificationSounds } from "../../features/cindy-notifications/sounds/useNotificationSounds";

/**
 * Cindy notifications on the island: how many are unhandled (null while the module is off), a
 * sound for new ones, and bringing Cindy forward when the badge is clicked. Game mode keeps the
 * sound quiet; the badge itself is passive and stays.
 */
export function useIslandNotifications(settings: IslandSettings, quiet: boolean) {
  const kinds = useMemo(
    () => ({
      done: settings.cindyNotifyDone,
      question: settings.cindyNotifyQuestion,
      scheduled: settings.cindyNotifyScheduled,
      error: settings.cindyNotifyError,
    }),
    [settings.cindyNotifyDone, settings.cindyNotifyError, settings.cindyNotifyQuestion, settings.cindyNotifyScheduled],
  );
  const enabled = settings.cindyNotificationsEnabled;
  const { unseen, markSeen } = useCindyNotifications({ enabled, kinds });
  useNotificationSounds(unseen, enabled && !quiet);

  const openCindy = useCallback(async () => {
    const opened = await focusAgentWindow().catch((error: unknown) => {
      console.error("Failed to bring Cindy to the front", error);
      return false;
    });
    // 没找到 Cindy 的窗口就不算看过：它打开后这些通知仍会显示。
    if (opened) markSeen();
  }, [markSeen]);

  return { notificationCount: enabled ? unseen.length : null, openCindy };
}
