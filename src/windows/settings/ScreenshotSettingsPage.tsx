import { ScreenshotSettingsSection } from "../../features/screenshot/ScreenshotSettingsSection";
import { useScreenshotSettings } from "../../features/screenshot/useScreenshotSettings";
import type { IslandSettings } from "../../features/appearance/types";
import { useTranslation } from "../../i18n";
import { useTranslationSettings } from "../../features/translation/useTranslationSettings";
import { useClipboardSettings } from "../../features/clipboard/useClipboardSettings";
import { ModuleSwitchSection } from "./ModuleSwitchSection";
import { findShortcutOwner } from "./shortcutConflicts";

/** Settings → Screenshots: the module switch, the capture shortcut and a way to try it. */
export function ScreenshotSettingsPage({ settings }: { settings: IslandSettings }) {
  const t = useTranslation();
  const [screenshotSettings, updateScreenshotSettings] = useScreenshotSettings();
  const [translationSettings] = useTranslationSettings();
  const [clipboardSettings] = useClipboardSettings();

  // A global hotkey swallows its keys everywhere, so it may not reuse another page's.
  const takenBy = (shortcut: string) =>
    findShortcutOwner(
      shortcut,
      [
        ...(settings.translationEnabled
          ? [{ id: "translation", label: t.settings.shortcuts.openTranslation, shortcut: translationSettings.shortcut }]
          : []),
        ...(clipboardSettings.shortcutEnabled
          ? [{ id: "clipboard", label: t.settings.categories.clipboard, shortcut: clipboardSettings.shortcut }]
          : []),
      ],
      "screenshot",
    );

  return (
    <>
      <ModuleSwitchSection
        label={t.settings.modules.screenshot}
        checked={screenshotSettings.enabled}
        onChange={(enabled) => {
          if (enabled !== screenshotSettings.enabled) void updateScreenshotSettings({ ...screenshotSettings, enabled });
        }}
      />
      {screenshotSettings.enabled && (
        <ScreenshotSettingsSection
          settings={screenshotSettings}
          onSettingsChange={updateScreenshotSettings}
          takenBy={takenBy}
        />
      )}
    </>
  );
}
