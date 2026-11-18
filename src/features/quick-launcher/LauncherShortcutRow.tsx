import { useEffect, useState, type Dispatch, type SetStateAction } from "react";
import { ShortcutResetButton } from "../../components/controls/ShortcutResetButton";
import { ShortcutRecordButton } from "../../components/controls/ShortcutRecordButton";
import { useTranslation } from "../../i18n";
import { buildShortcutFromEvent } from "../clipboard/shortcut";
import { setLauncherShortcut } from "./api";
import { DEFAULT_LAUNCHER_SETTINGS, type LauncherSettings } from "./storage";
import "../clipboard/ClipboardSettingsSection.css";

/**
 * The global shortcut that opens the launcher. Same control as the other shortcut
 * rows: keys another island shortcut uses are refused before registration, and keys
 * Windows refuses keep the previous ones and show why. `registerToken` asks again
 * after the module is switched back on.
 */
export function LauncherShortcutRow({
  settings,
  onSettingsChange,
  takenBy,
  enabled,
  registerToken,
}: {
  settings: LauncherSettings;
  onSettingsChange: Dispatch<SetStateAction<LauncherSettings>>;
  takenBy: (shortcut: string) => string | null;
  enabled: boolean;
  registerToken: number;
}) {
  const t = useTranslation();
  const [error, setError] = useState("");

  useEffect(() => {
    if (!enabled) {
      setError("");
      return;
    }
    let didCancel = false;
    void setLauncherShortcut(settings.shortcut)
      .then(() => {
        if (!didCancel) setError("");
      })
      .catch((failure) => {
        console.warn("Launcher shortcut not registered:", failure);
        if (!didCancel) setError(t.settings.shortcuts.registerFailed(settings.shortcut));
      });
    return () => {
      didCancel = true;
    };
    // Registers on open and whenever the page asks again; recording reports its own result.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [registerToken, enabled]);

  const apply = (shortcut: string) => {
    const owner = takenBy(shortcut);
    if (owner) {
      setError(t.settings.shortcuts.taken(shortcut, owner));
      return;
    }
    void setLauncherShortcut(shortcut)
      .then((registered) => {
        onSettingsChange((current) => ({ ...current, shortcut: registered }));
        setError("");
      })
      .catch((failure) => {
        console.warn("Launcher shortcut not registered:", failure);
        setError(t.settings.shortcuts.registerFailed(shortcut, settings.shortcut));
      });
  };

  const fallback = DEFAULT_LAUNCHER_SETTINGS.shortcut;
  return (
    <div className="shortcut-row">
      <div className={`shortcut-control${enabled ? "" : " shortcut-control--disabled"}`}>
        <div className="shortcut-control__meta">
          <span>{t.settings.launcher.shortcut}</span>
          <strong>{settings.shortcut}</strong>
        </div>
        <div className="shortcut-control__actions">
          <ShortcutResetButton
            label={t.settings.shortcuts.reset(fallback)}
            isDefault={settings.shortcut === fallback}
            disabled={!enabled}
            onReset={() => apply(fallback)}
          />
          <ShortcutRecordButton
            shortcut={settings.shortcut}
            pressKeysLabel={t.settings.launcher.pressKeys}
            disabled={!enabled}
            toShortcut={buildShortcutFromEvent}
            onRecord={apply}
            onInterrupted={() => setError(t.settings.shortcuts.takenElsewhere)}
          />
        </div>
      </div>
      {error && <p className="shortcut-control__error" role="alert">{error}</p>}
    </div>
  );
}
