import { CircleAlert, CircleCheck, CircleSlash } from "lucide-react";
import { SettingsGroup } from "../../components/settings/SettingsGroup";
import { SettingsLinkButton } from "../../components/settings/SettingsLinkButton";
import { SettingsRow } from "../../components/settings/SettingsRow";
import { SettingsToggleRow } from "../../components/settings/SettingsToggleRow";
import { useLocale, useTranslation } from "../../i18n";
import type { IslandSettings } from "../appearance/types";
import type { CindyNotificationSnapshot } from "./api";
import { useNotificationSource } from "./useNotificationSource";
import { useSoundSettings } from "./sounds/useSoundSettings";
import "./NotificationSettings.css";

const TONES = {
  ready: { icon: CircleCheck, className: "notification-source__state" },
  missing: { icon: CircleSlash, className: "notification-source__state notification-source__state--quiet" },
  failed: { icon: CircleAlert, className: "notification-source__state notification-source__state--attention" },
} as const;

type SourceState = keyof typeof TONES;

function sourceState(snapshot: CindyNotificationSnapshot | null, error: string | null): SourceState {
  if (error || snapshot?.error) return "failed";
  if (!snapshot) return "missing";
  return snapshot.databasePath ? "ready" : "missing";
}

/**
 * Settings → Cindy notifications: the module switch, the four events worth a badge, whether a
 * sound plays, and where the events come from. One page, no session lists.
 */
export function NotificationSettings({
  settings,
  onSettingsChange,
}: {
  settings: IslandSettings;
  onSettingsChange: (settings: IslandSettings) => void;
}) {
  const t = useTranslation();
  const text = t.settings.notifications;
  const { locale } = useLocale();
  const update = (patch: Partial<IslandSettings>) => onSettingsChange({ ...settings, ...patch });
  const [sounds, setSounds] = useSoundSettings();
  const { snapshot, error, refreshing, refresh } = useNotificationSource();
  const state = sourceState(snapshot, error);
  const { icon: Icon, className } = TONES[state];
  const readAt = snapshot && snapshot.checkedAt > 0 ? new Date(snapshot.checkedAt).toLocaleTimeString(locale) : null;
  const status = state === "failed"
    ? text.source.failed
    : state === "missing"
      ? text.source.missing
      : text.source.reading(snapshot?.items.length ?? 0, readAt ?? "");

  return (
    <>
      <SettingsGroup>
        <SettingsToggleRow
          title={text.enable}
          description={t.settings.modules.hint}
          checked={settings.cindyNotificationsEnabled}
          onChange={(cindyNotificationsEnabled) => update({ cindyNotificationsEnabled })}
        />
      </SettingsGroup>
      <SettingsGroup title={text.events.title} footer={text.events.hint}>
        <SettingsToggleRow title={text.events.done} checked={settings.cindyNotifyDone} onChange={(cindyNotifyDone) => update({ cindyNotifyDone })} />
        <SettingsToggleRow title={text.events.question} checked={settings.cindyNotifyQuestion} onChange={(cindyNotifyQuestion) => update({ cindyNotifyQuestion })} />
        <SettingsToggleRow title={text.events.scheduled} checked={settings.cindyNotifyScheduled} onChange={(cindyNotifyScheduled) => update({ cindyNotifyScheduled })} />
        <SettingsToggleRow title={text.events.error} checked={settings.cindyNotifyError} onChange={(cindyNotifyError) => update({ cindyNotifyError })} />
      </SettingsGroup>
      <SettingsGroup title={text.sound.title}>
        <SettingsToggleRow
          title={text.sound.enabled}
          description={text.sound.hint}
          checked={sounds.enabled}
          onChange={(enabled) => setSounds((current) => ({ ...current, enabled }))}
        />
      </SettingsGroup>
      <SettingsGroup title={text.source.title} footer={text.source.hint}>
        <SettingsRow
          title={
            <span className={className}>
              <Icon className="notification-source__icon" size={13} strokeWidth={2.4} aria-hidden="true" />
              {status}
            </span>
          }
          description={state === "failed" ? (error ?? snapshot?.error ?? undefined) : snapshot?.databasePath || undefined}
          control={
            <SettingsLinkButton disabled={refreshing} onClick={() => void refresh()}>
              {text.source.refresh}
            </SettingsLinkButton>
          }
        />
      </SettingsGroup>
    </>
  );
}
