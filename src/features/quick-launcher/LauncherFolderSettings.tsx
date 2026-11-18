import { useState, type Dispatch, type SetStateAction } from "react";
import { X } from "lucide-react";
import { SettingsInput } from "../../components/settings/SettingsInput";
import { SettingsLinkButton } from "../../components/settings/SettingsLinkButton";
import { SettingsRow } from "../../components/settings/SettingsRow";
import { useTranslation } from "../../i18n";
import type { LauncherSettings } from "./storage";
import { effectiveLauncherFolders, useLauncherDefaultFolders } from "./useLauncherFolders";
import "./LauncherFolderSettings.css";

/**
 * The folders searched for files. The defaults (Desktop, Documents, Downloads) are
 * shown until one is removed or another added, which switches the list to a saved
 * custom one; restoring brings the defaults back without saving their paths.
 */
export function LauncherFolderSettings({
  settings,
  onSettingsChange,
}: {
  settings: LauncherSettings;
  onSettingsChange: Dispatch<SetStateAction<LauncherSettings>>;
}) {
  const t = useTranslation().settings.launcher;
  const defaults = useLauncherDefaultFolders();
  const folders = effectiveLauncherFolders(settings, defaults);
  const [draft, setDraft] = useState("");

  const saveFolders = (next: string[]) =>
    onSettingsChange((current) => ({ ...current, folderMode: "custom", folders: next }));

  const add = () => {
    const path = draft.trim();
    if (!path) return;
    if (!folders.some((folder) => folder.toLocaleLowerCase() === path.toLocaleLowerCase())) {
      saveFolders([...folders, path]);
    }
    setDraft("");
  };

  return (
    <SettingsRow title={t.folders} description={t.foldersHint}>
      <div className="launcher-folders">
        {folders.length === 0 ? (
          <p className="launcher-folders__empty">{t.foldersEmpty}</p>
        ) : (
          <ul className="launcher-folders__list">
            {folders.map((folder) => (
              <li key={folder} className="launcher-folders__item">
                <span>{folder}</span>
                <button
                  className="launcher-folders__remove"
                  type="button"
                  aria-label={t.removeFolder(folder)}
                  onClick={() => saveFolders(folders.filter((current) => current !== folder))}
                >
                  <X size={12} strokeWidth={2.2} aria-hidden="true" />
                </button>
              </li>
            ))}
          </ul>
        )}
        <div className="launcher-folders__add">
          <SettingsInput
            value={draft}
            placeholder={t.folderPlaceholder}
            aria-label={t.folders}
            onChange={(event) => setDraft(event.currentTarget.value)}
            onKeyDown={(event) => {
              if (event.key === "Enter") {
                event.preventDefault();
                add();
              }
            }}
          />
          <SettingsLinkButton disabled={!draft.trim()} onClick={add}>{t.addFolder}</SettingsLinkButton>
          <SettingsLinkButton
            disabled={settings.folderMode === "default" && settings.folders.length === 0}
            onClick={() => onSettingsChange((current) => ({ ...current, folderMode: "default", folders: [] }))}
          >
            {t.useDefaultFolders}
          </SettingsLinkButton>
        </div>
      </div>
    </SettingsRow>
  );
}
