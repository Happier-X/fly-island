import { useEffect, useState, type Dispatch, type SetStateAction } from "react";
import { ShortcutResetButton } from "../../components/controls/ShortcutResetButton";
import { ShortcutRecordButton } from "../../components/controls/ShortcutRecordButton";
import { SettingsRow } from "../../components/settings/SettingsRow";
import { SettingsSelect } from "../../components/settings/SettingsSelect";
import { useTranslation } from "../../i18n";
import { buildShortcutFromEvent } from "../clipboard/shortcut";
import { setLauncherShortcut } from "./api";
import { DEFAULT_LAUNCHER_SETTINGS, launcherTrigger, type LauncherSettings, type LauncherTriggerMode } from "./storage";
import "../clipboard/ClipboardSettingsSection.css";

export function LauncherShortcutRow({ settings, onSettingsChange, takenBy }: {
  settings: LauncherSettings;
  onSettingsChange: Dispatch<SetStateAction<LauncherSettings>>;
  takenBy: (shortcut: string) => string | null;
}) {
  const t = useTranslation();
  const text = t.settings.launcher;
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    let active = true;
    void setLauncherShortcut(launcherTrigger(settings)).catch(() => {
      if (active) {
        setError(settings.triggerMode === "doubleCtrl" ? text.doubleCtrlFailed : t.settings.shortcuts.registerFailed(settings.shortcut));
      }
    });
    return () => { active = false; };
    // Opening the page reports startup registration failures. Each edit reports its own result.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const apply = (shortcut: string, triggerMode: LauncherTriggerMode = "shortcut") => {
    if (busy) return;
    const owner = triggerMode === "shortcut" ? takenBy(shortcut) : null;
    if (owner) {
      setError(t.settings.shortcuts.taken(shortcut, owner));
      return;
    }
    setBusy(true);
    void setLauncherShortcut(launcherTrigger({ ...settings, shortcut, triggerMode }))
      .then((registered) => {
        onSettingsChange((current) => ({
          ...current,
          triggerMode,
          shortcut: triggerMode === "shortcut" ? registered : current.shortcut,
        }));
        setError("");
      })
      .catch(() => setError(triggerMode === "doubleCtrl" ? text.doubleCtrlFailed : t.settings.shortcuts.registerFailed(shortcut)))
      .finally(() => setBusy(false));
  };

  const fallback = DEFAULT_LAUNCHER_SETTINGS.shortcut;
  return (
    <>
      <SettingsRow title={text.trigger} control={
        <SettingsSelect<LauncherTriggerMode>
          ariaLabel={text.trigger}
          value={settings.triggerMode}
          disabled={busy}
          options={[
            { value: "shortcut", label: text.combination },
            { value: "doubleCtrl", label: text.doubleCtrl },
          ]}
          onChange={(mode) => apply(settings.shortcut, mode)}
        />
      } />
      {(settings.triggerMode === "shortcut" || error) && (
        <div className="shortcut-row">
          {settings.triggerMode === "shortcut" && (
            <div className="shortcut-control">
              <div className="shortcut-control__meta">
                <span>{text.shortcut}</span>
                <strong>{settings.shortcut}</strong>
              </div>
              <div className="shortcut-control__actions">
                <ShortcutResetButton
                  label={t.settings.shortcuts.reset(fallback)}
                  isDefault={settings.shortcut === fallback}
                  disabled={busy}
                  onReset={() => apply(fallback)}
                />
                <ShortcutRecordButton
                  shortcut={settings.shortcut}
                  pressKeysLabel={text.pressKeys}
                  disabled={busy}
                  toShortcut={buildShortcutFromEvent}
                  onRecord={apply}
                  onInterrupted={() => setError(t.settings.shortcuts.takenElsewhere)}
                />
              </div>
            </div>
          )}
          {error && <p className="shortcut-control__error" role="alert">{error}</p>}
        </div>
      )}
    </>
  );
}
