import { useState } from "react";
import { SettingsGroup } from "../../components/settings/SettingsGroup";
import { SettingsToggleRow } from "../../components/settings/SettingsToggleRow";
import type { IslandSettings } from "../../features/appearance/types";
import { useClipboardSettings } from "../../features/clipboard/useClipboardSettings";
import { setLauncherEnabled } from "../../features/quick-launcher/api";
import { LauncherFolderSettings } from "../../features/quick-launcher/LauncherFolderSettings";
import { LauncherSearchSettings } from "../../features/quick-launcher/LauncherSearchSettings";
import { LauncherShortcutRow } from "../../features/quick-launcher/LauncherShortcutRow";
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
  const [registerToken, setRegisterToken] = useState(0);

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
    setLauncherSettings((current) => ({ ...current, enabled }));
    // The shortcut can only be registered once the native side is on.
    void setLauncherEnabled(enabled)
      .then(() => {
        if (enabled) setRegisterToken((value) => value + 1);
      })
      .catch((error) => {
        console.error("Failed to switch the quick launcher", error);
        setLauncherSettings((current) => ({ ...current, enabled: !enabled }));
      });
  };

  return (
    <>
      <SettingsGroup>
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
            enabled={launcherSettings.enabled}
            registerToken={registerToken}
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
