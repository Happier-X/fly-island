import type { Dispatch, SetStateAction } from "react";
import { SettingsRow } from "../../components/settings/SettingsRow";
import { SettingsSelect } from "../../components/settings/SettingsSelect";
import { SettingsToggleRow } from "../../components/settings/SettingsToggleRow";
import { useTranslation } from "../../i18n";
import { SEARCH_ENGINES, type LauncherSettings } from "./storage";
import type { SearchEngine } from "./types";

/** Search scope and the engine used for the web fallback. */
export function LauncherSearchSettings({
  settings,
  onSettingsChange,
}: {
  settings: LauncherSettings;
  onSettingsChange: Dispatch<SetStateAction<LauncherSettings>>;
}) {
  const t = useTranslation().settings.launcher;

  return (
    <>
      <SettingsToggleRow
        title={t.apps}
        checked={settings.searchApps}
        onChange={(searchApps) => onSettingsChange((current) => ({ ...current, searchApps }))}
      />
      <SettingsToggleRow
        title={t.files}
        checked={settings.searchFiles}
        onChange={(searchFiles) => onSettingsChange((current) => ({ ...current, searchFiles }))}
      />
      <SettingsToggleRow
        title={t.web}
        checked={settings.searchWeb}
        onChange={(searchWeb) => onSettingsChange((current) => ({ ...current, searchWeb }))}
      />
      <SettingsRow
        title={t.engine}
        description={t.engineHint}
        control={
          <SettingsSelect
            value={settings.searchEngine}
            options={SEARCH_ENGINES.map((engine: SearchEngine) => ({ value: engine, label: t.engines[engine] }))}
            ariaLabel={t.engine}
            onChange={(searchEngine) => onSettingsChange((current) => ({ ...current, searchEngine }))}
          />
        }
      />
    </>
  );
}
