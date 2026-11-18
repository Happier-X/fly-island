import { useState } from "react";
import { SettingsGroup } from "../../components/settings/SettingsGroup";
import { SettingsToggleRow } from "../../components/settings/SettingsToggleRow";
import type { IslandSettings } from "../../features/appearance/types";
import { useClipboardSettings } from "../../features/clipboard/useClipboardSettings";
import { setLauncherEnabled } from "../../features/quick-launcher/api";
import { LauncherFolderSettings } from "../../features/quick-launcher/LauncherFolderSettings";
import { LauncherSearchSettings } from "../../features/quick-launcher/LauncherSearchSettings";
import { LauncherShortcutRow } from "../../features/quick-launcher/LauncherShortcutRow";
import { launcherTrigger } from "../../features/quick-launcher/storage";
import { useLauncherSettings } from "../../features/quick-launcher/useLauncherSettings";
import { useScreenshotSettings } from "../../features/screenshot/useScreenshotSettings";
import { useTranslationSettings } from "../../features/translation/useTranslationSettings";
import { useTranslation } from "../../i18n";
import { findShortcutOwner } from "./shortcutConflicts";

/**
 * Settings → Quick launcher: the module switch, its global shortcut, what is searched
 * and the folders used for files. Every global shortcut here is mutually exclusive with
 * translation, clipboard history and screenshots.
 */
export function LauncherSettingsPage({ settings }: { settings: IslandSettings }) {
  const t = useTranslation();
  const [launcherSettings, setLauncherSettings] = useLauncherSettings();
  const [translationSettings] = useTranslationSettings();
  const [clipboardSettings] = useClipboardSettings();
  const [screenshotSettings] = useScreenshotSettings();
  const [switchError, setSwitchError] = useState("");
  const [switching, setSwitching] = useState(false);

  // A global hotkey swallows its keys everywhere, so it may not reuse another page's.
  const takenBy = (shortcut: string) =>
    findShortcutOwner(
      shortcut,
      [
        ...(settings.translationEnabled
          ? [{ id: "translation", label: t.settings.shortcuts.openTranslation, shortcut: translationSettings.shortcut }]
          : []),
        ...(clipboardSettings.enabled && clipboardSettings.shortcutEnabled
          ? [{ id: "clipboard", label: t.settings.categories.clipboard, shortcut: clipboardSettings.shortcut }]
          : []),
        ...(screenshotSettings.enabled
          ? [{ id: "screenshot", label: t.settings.categories.screenshot, shortcut: screenshotSettings.shortcut }]
          : []),
      ],
      "launcher",
    );

  const changeEnabled = (enabled: boolean) => {
    if (switching) return;
    const shortcut = launcherTrigger(launcherSettings);
    const owner = enabled && launcherSettings.triggerMode === "shortcut" ? takenBy(shortcut) : null;
    if (owner) {
      setSwitchError(t.settings.shortcuts.taken(shortcut, owner));
      return;
    }
    setSwitching(true);
    setSwitchError("");
    void setLauncherEnabled(enabled, shortcut)
      .then(() => setLauncherSettings((current) => ({ ...current, enabled })))
      .catch((error) => {
        console.error("Failed to switch the quick launcher", error);
        setSwitchError(!enabled ? t.settings.launcher.disableFailed
          : launcherSettings.triggerMode === "doubleCtrl" ? t.settings.launcher.doubleCtrlFailed
            : t.settings.shortcuts.registerFailed(shortcut));
      })
      .finally(() => setSwitching(false));
  };

  return (
    <>
      <SettingsGroup footer={switchError && <span role="alert">{switchError}</span>}>
        <SettingsToggleRow
          title={t.settings.launcher.enable}
          description={t.settings.launcher.enableHint}
          checked={launcherSettings.enabled}
          onChange={changeEnabled}
        />
      </SettingsGroup>
      {launcherSettings.enabled && (
        <SettingsGroup title={t.settings.categories.shortcuts} footer={t.settings.launcher.shortcutHint}>
          <LauncherShortcutRow
            settings={launcherSettings}
            onSettingsChange={setLauncherSettings}
            takenBy={takenBy}
          />
        </SettingsGroup>
      )}
      {launcherSettings.enabled && (
        <SettingsGroup title={t.settings.launcher.searchScope}>
          <LauncherSearchSettings settings={launcherSettings} onSettingsChange={setLauncherSettings} />
        </SettingsGroup>
      )}
      {launcherSettings.enabled && (
        <SettingsGroup>
          <LauncherFolderSettings settings={launcherSettings} onSettingsChange={setLauncherSettings} />
        </SettingsGroup>
      )}
    </>
  );
}
