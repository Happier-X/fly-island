import { NotificationSettings } from "../../features/cindy-notifications/NotificationSettings";
import type { IslandSettings } from "../../features/appearance/types";

/**
 * Settings → Cindy notifications: the module switch, the four events worth a badge, the sound
 * and where the events come from, on one page.
 */
export function NotificationsSettingsPage({
  settings,
  onSettingsChange,
}: {
  settings: IslandSettings;
  onSettingsChange: (settings: IslandSettings) => void;
}) {
  return <NotificationSettings settings={settings} onSettingsChange={onSettingsChange} />;
}
