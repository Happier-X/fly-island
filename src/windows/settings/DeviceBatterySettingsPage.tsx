import { SettingsGroup } from "../../components/settings/SettingsGroup";
import { SettingsRow } from "../../components/settings/SettingsRow";
import { SettingsSelectNumber } from "../../components/settings/SettingsSelectNumber";
import { BATTERY_REFRESH_SECONDS } from "../../features/appearance/settings";
import type { IslandSettings } from "../../features/appearance/types";
import { useTranslation } from "../../i18n";
import { ModuleSwitchSection } from "./ModuleSwitchSection";

/** Settings → Device battery: the module switch and how often an open page re-reads devices. */
export function DeviceBatterySettingsPage({
  settings,
  onSettingsChange,
}: {
  settings: IslandSettings;
  onSettingsChange: (settings: IslandSettings) => void;
}) {
  const t = useTranslation();
  const battery = t.settings.deviceBattery;

  return (
    <>
      <ModuleSwitchSection
        label={t.settings.modules.battery}
        checked={settings.batteryEnabled}
        onChange={(batteryEnabled) => onSettingsChange({ ...settings, batteryEnabled })}
      />
      {settings.batteryEnabled && (
        <SettingsGroup title={battery.title}>
          <SettingsRow
            title={battery.refresh}
            description={battery.refreshHint}
            control={
              <SettingsSelectNumber
                value={settings.batteryRefreshSeconds}
                options={BATTERY_REFRESH_SECONDS.map((seconds) => ({ value: seconds, label: battery.seconds(seconds) }))}
                ariaLabel={battery.refresh}
                onChange={(batteryRefreshSeconds) => onSettingsChange({ ...settings, batteryRefreshSeconds })}
              />
            }
          />
        </SettingsGroup>
      )}
    </>
  );
}
