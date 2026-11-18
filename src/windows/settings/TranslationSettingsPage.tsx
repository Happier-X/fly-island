import type { IslandSettings } from "../../features/appearance/types";
import { TranslationSettingsSection } from "../../features/translation/TranslationSettingsSection";
import { useTranslation } from "../../i18n";
import { ModuleSwitchSection } from "./ModuleSwitchSection";
import { TranslationShortcutRows } from "../../features/translation/TranslationShortcutRows";
import { useTranslationSettings } from "../../features/translation/useTranslationSettings";
import { useClipboardSettings } from "../../features/clipboard/useClipboardSettings";
import { useLauncherSettings } from "../../features/quick-launcher/useLauncherSettings";
import { findShortcutOwner } from "./shortcutConflicts";
import { SettingsGroup } from "../../components/settings/SettingsGroup";

export function TranslationSettingsPage({
  settings,
  onSettingsChange,
}: {
  settings: IslandSettings;
  onSettingsChange: (settings: IslandSettings) => void;
}) {
  const t = useTranslation();
  const [translationSettings, setTranslationSettings] = useTranslationSettings();
  const [clipboardSettings] = useClipboardSettings();
  const [launcherSettings] = useLauncherSettings();
  // A global hotkey swallows its keys everywhere, so it may not reuse another page's.
  const takenBy = (shortcut: string) => findShortcutOwner(
    shortcut,
    [
      ...(clipboardSettings.enabled && clipboardSettings.shortcutEnabled
        ? [{ id: "clipboard", label: t.settings.clipboard.shortcut, shortcut: clipboardSettings.shortcut }]
        : []),
      ...(launcherSettings.enabled && launcherSettings.triggerMode === "shortcut"
        ? [{ id: "launcher", label: t.settings.launcher.shortcut, shortcut: launcherSettings.shortcut }]
        : []),
    ],
    "translation",
  );

  return (
    <>
      <ModuleSwitchSection
        label={t.settings.modules.translation}
        checked={settings.translationEnabled}
        onChange={(translationEnabled) => onSettingsChange({ ...settings, translationEnabled })}
      />
      {settings.translationEnabled && <TranslationSettingsSection />}
      {settings.translationEnabled && (
        <SettingsGroup title={t.settings.categories.shortcuts} footer={t.settings.shortcuts.globalHint}>
          <TranslationShortcutRows settings={translationSettings} onSettingsChange={setTranslationSettings} takenBy={takenBy} />
        </SettingsGroup>
      )}
    </>
  );
}
