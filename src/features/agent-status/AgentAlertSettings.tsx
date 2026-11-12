import { SettingsGroup } from "../../components/settings/SettingsGroup";
import { SettingsToggleRow } from "../../components/settings/SettingsToggleRow";
import { useTranslation } from "../../i18n";
import type { IslandSettings } from "../appearance/types";
import { useAgentSoundSettings } from "./sounds/useAgentSoundSettings";

/**
 * Settings → AI Agent → Sessions & alerts: when the Agent page pops up, and sounds, one switch
 * each. How long a pop-up stays, reminders, the sound pack, volume, events and quiet hours keep
 * their defaults.
 */
export function AgentAlertSettings({
  settings,
  onSettingsChange,
}: {
  settings: IslandSettings;
  onSettingsChange: (settings: IslandSettings) => void;
}) {
  const t = useTranslation();
  const text = t.settings.agent;
  const update = (patch: Partial<IslandSettings>) => onSettingsChange({ ...settings, ...patch });
  const [sounds, setSounds] = useAgentSoundSettings();

  return (
    <SettingsGroup title={text.alerts.title} footer={text.alerts.hint}>
      <SettingsToggleRow title={text.alerts.waiting} checked={settings.agentAlertWaiting} onChange={(agentAlertWaiting) => update({ agentAlertWaiting })} />
      <SettingsToggleRow title={text.alerts.completed} checked={settings.agentAlertCompleted} onChange={(agentAlertCompleted) => update({ agentAlertCompleted })} />
      <SettingsToggleRow
        title={t.settings.sounds.enabled}
        description={t.settings.sounds.enabledHint}
        checked={sounds.enabled}
        onChange={(enabled) => setSounds((current) => ({ ...current, enabled }))}
      />
    </SettingsGroup>
  );
}
