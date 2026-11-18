import { useSyncExternalStore } from "react";
import { SettingsGroup } from "../../components/settings/SettingsGroup";
import { SettingsToggleRow } from "../../components/settings/SettingsToggleRow";
import { useTranslation } from "../../i18n";
import { AUTO_UPDATE_INTERVALS_HOURS, type AutoUpdateIntervalHours } from "./autoUpdateSettings";
import { SettingsSelectNumber } from "../../components/settings/SettingsSelectNumber";
import { getAutoUpdateSettings, subscribeToAutoUpdate, updateAutoUpdateSettings } from "./updaterController";

/** Settings → About: scheduled checks and the optional install-and-restart after download. */
export function AutoUpdateSettingsSection() {
  const t = useTranslation().settings.project.update;
  const settings = useSyncExternalStore(subscribeToAutoUpdate, getAutoUpdateSettings, getAutoUpdateSettings);
  const update = (patch: Partial<typeof settings>) => updateAutoUpdateSettings(patch);

  return (
    <SettingsGroup title={t.autoUpdateTitle} footer={t.autoUpdateFooter}>
      <SettingsToggleRow
        title={t.autoUpdateEnabled}
        description={t.autoUpdateEnabledHint}
        checked={settings.enabled}
        onChange={(enabled) => update({ enabled })}
      />
      {settings.enabled && (
        <>
          <SettingsToggleRow
            title={t.autoInstall}
            description={t.autoInstallHint}
            checked={settings.autoInstall}
            indent
            onChange={(autoInstall) => update({ autoInstall })}
          />
          <div className="settings-row settings-row--indent">
            <div className="settings-row__main">
              <div className="settings-row__text">
                <span className="settings-row__title">{t.checkInterval}</span>
                <span className="settings-row__description">{t.checkIntervalHint}</span>
              </div>
              <div className="settings-row__control">
                <SettingsSelectNumber<AutoUpdateIntervalHours>
                  value={settings.intervalHours}
                  options={AUTO_UPDATE_INTERVALS_HOURS.map((hours) => ({
                    value: hours,
                    label: t.intervalLabel(hours),
                  }))}
                  ariaLabel={t.checkInterval}
                  onChange={(intervalHours) => update({ intervalHours })}
                />
              </div>
            </div>
          </div>
        </>
      )}
    </SettingsGroup>
  );
}
