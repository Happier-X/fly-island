import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "../../i18n";
import { SettingsGroup } from "../../components/settings/SettingsGroup";
import { SettingsRow } from "../../components/settings/SettingsRow";
import { SettingsToggleRow } from "../../components/settings/SettingsToggleRow";
import { SelectControl } from "../../components/controls/SelectControl";
import { ShortcutRecordButton, type ShortcutKeys } from "../../components/controls/ShortcutRecordButton";
import { ShortcutResetButton } from "../../components/controls/ShortcutResetButton";
import { buildShortcutFromEvent } from "../clipboard/shortcut";
import {
  captureScreenshot,
  DEFAULT_COLORS,
  DEFAULT_SCREENSHOT_SHORTCUT,
  getScreenshotOcrLanguages,
  getScreenshotToolbarGroups,
  OCR_LANG_AUTO,
  pickScreenshotDirectory,
  UI_SCALES,
  type ScreenshotSettings,
  type ToolbarGroupInfo,
} from "./api";
import { ScreenshotOverlayKeys } from "./ScreenshotOverlayKeys";
import { normalizeScreenshotShortcut } from "./shortcut";
import "./ScreenshotSettingsSection.css";

/**
 * The screenshot settings: capture shortcut, where and in which format files are saved,
 * the five annotation colors the overlay toolbar offers, the overlay's own keys, which
 * toolbar items are drawn, and the OCR language and overlay scale.
 *
 * Everything here is owned by fly-island (`screenshot.json`) and pushed to the AirSnip
 * overlay thread on save; AirSnip's own `config.toml` is not read any more.
 */
export function ScreenshotSettingsSection({
  settings,
  onSettingsChange,
  takenBy,
}: {
  settings: ScreenshotSettings;
  onSettingsChange: (settings: ScreenshotSettings) => Promise<void>;
  /** Another page row already using those keys, by its label. */
  takenBy: (shortcut: string) => string | null;
}) {
  const t = useTranslation();
  const [error, setError] = useState("");
  const [captureError, setCaptureError] = useState("");
  const [toolbarGroups, setToolbarGroups] = useState<ToolbarGroupInfo[]>([]);
  const [ocrLanguages, setOcrLanguages] = useState<string[]>([]);
  const [ocrLanguageFailed, setOcrLanguageFailed] = useState(false);
  const shortcut = normalizeScreenshotShortcut(settings.shortcut);
  const text = t.settings.screenshot;

  const apply = async (next: string) => {
    const owner = takenBy(next);
    if (owner) {
      setError(t.settings.shortcuts.taken(next, owner));
      return;
    }
    setError("");
    try {
      await onSettingsChange({ ...settings, shortcut: next });
    } catch (failure) {
      setError(String(failure));
    }
  };

  const save = (patch: Partial<ScreenshotSettings>) => {
    void onSettingsChange({ ...settings, ...patch });
  };

  // The toolbar's structure lives in AirSnip; only the labels are translated here.
  useEffect(() => {
    void getScreenshotToolbarGroups()
      .then(setToolbarGroups)
      .catch((failure) => console.error("Failed to read the screenshot toolbar", failure));
    void getScreenshotOcrLanguages()
      .then(setOcrLanguages)
      .catch((failure) => {
        console.error("Failed to read the OCR languages", failure);
        setOcrLanguageFailed(true);
      });
  }, []);

  const slotLabel = (key: string, fallback: string) => text.slotLabels[key] ?? fallback;

  const toggleToolbarSlot = (key: string) => {
    const hidden = settings.hiddenToolbar.includes(key)
      ? settings.hiddenToolbar.filter((kept) => kept !== key)
      : [...settings.hiddenToolbar, key];
    save({ hiddenToolbar: hidden });
  };

  const chooseDirectory = async () => {
    try {
      const picked = await pickScreenshotDirectory(settings.saveDir);
      if (picked) save({ saveDir: picked });
    } catch (failure) {
      console.error("Failed to pick a screenshot directory", failure);
    }
  };

  // A language pack that was uninstalled since would otherwise show as an empty select.
  const ocrOptions = useMemo(() => {
    const installed = ocrLanguages.filter((tag) => tag !== OCR_LANG_AUTO);
    const options = [
      { value: OCR_LANG_AUTO, label: text.ocrLanguageAuto },
      ...installed.map((tag) => ({ value: tag, label: tag })),
    ];
    const current = settings.ocrLanguage;
    if (current !== OCR_LANG_AUTO && !installed.includes(current)) {
      options.push({ value: current, label: current });
    }
    return options;
  }, [ocrLanguages, settings.ocrLanguage, text.ocrLanguageAuto]);

  const scaleOptions = UI_SCALES.map((value) => ({
    value,
    label: value === "auto" ? text.uiScaleAuto : `${value}%`,
  }));

  return (
    <>
      <SettingsGroup title={text.title} footer={t.settings.shortcuts.globalHint}>
        <div className="screenshot-shortcut-row">
          <div className="screenshot-shortcut-row__meta">
            <span>{text.shortcut}</span>
            <strong>{shortcut}</strong>
          </div>
          <div className="screenshot-shortcut-row__actions">
            <ShortcutResetButton
              label={t.settings.shortcuts.reset(DEFAULT_SCREENSHOT_SHORTCUT)}
              isDefault={shortcut === DEFAULT_SCREENSHOT_SHORTCUT}
              disabled={!settings.enabled}
              onReset={() => void apply(DEFAULT_SCREENSHOT_SHORTCUT)}
            />
            <ShortcutRecordButton
              shortcut={shortcut}
              pressKeysLabel={text.pressKeys}
              disabled={!settings.enabled}
              toShortcut={(keys: ShortcutKeys) => buildShortcutFromEvent(keys)}
              onRecord={(next) => void apply(next)}
              onInterrupted={() => setError(t.settings.shortcuts.takenElsewhere)}
            />
          </div>
        </div>
        {error && <p className="screenshot-shortcut-row__error" role="alert">{error}</p>}
      </SettingsGroup>

      <ScreenshotOverlayKeys settings={settings} onChange={save} />

      <SettingsGroup title={text.saveTitle} footer={text.saveHint}>
        <SettingsRow
          title={text.saveFormat}
          control={
            <div className="screenshot-format-control">
              <SelectControl
                value={settings.saveFormat}
                ariaLabel={text.saveFormat}
                options={[
                  { value: "png", label: text.formatPng },
                  { value: "jpeg", label: text.formatJpeg },
                  { value: "webp", label: text.formatWebp },
                ]}
                disabled={!settings.enabled}
                onChange={(saveFormat) => save({ saveFormat })}
              />
            </div>
          }
        />
        <SettingsRow
          title={text.saveDir}
          description={settings.saveDir || text.saveDirDefault}
          control={
            <div className="screenshot-row-actions">
              {settings.saveDir ? (
                <button
                  type="button"
                  className="screenshot-ghost-button"
                  disabled={!settings.enabled}
                  onClick={() => save({ saveDir: "" })}
                >
                  {text.saveDirReset}
                </button>
              ) : null}
              <button
                type="button"
                className="screenshot-try-button"
                disabled={!settings.enabled}
                onClick={() => void chooseDirectory()}
              >
                {text.chooseDir}
              </button>
            </div>
          }
        />
        <SettingsToggleRow
          title={text.autoSave}
          description={text.autoSaveHint}
          checked={settings.autoSave}
          onChange={(autoSave) => {
            if (settings.enabled) save({ autoSave });
          }}
        />
      </SettingsGroup>

      <SettingsGroup title={text.colorsTitle} footer={text.colorsHint}>
        <SettingsRow
          title={text.colors}
          control={
            <div className="screenshot-colors">
              {settings.colors.map((color, index) => (
                <input
                  key={index}
                  type="color"
                  className="screenshot-color"
                  value={color}
                  aria-label={text.colorLabel(index + 1)}
                  disabled={!settings.enabled}
                  onChange={(event) => {
                    const colors = [...settings.colors];
                    colors[index] = event.target.value.toUpperCase();
                    save({ colors });
                  }}
                />
              ))}
            </div>
          }
        >
          <button
            type="button"
            className="screenshot-ghost-button screenshot-colors-reset"
            disabled={!settings.enabled || settings.colors.every((color, index) => color.toUpperCase() === DEFAULT_COLORS[index])}
            onClick={() => save({ colors: [...DEFAULT_COLORS] })}
          >
            {text.colorsReset}
          </button>
        </SettingsRow>
      </SettingsGroup>

      {/* The rows come from the overlay, so there is nothing to show until they arrive. */}
      {toolbarGroups.length > 0 && (
        <SettingsGroup title={text.toolbarTitle} footer={text.toolbarHint}>
          {toolbarGroups.map((group) => (
            <SettingsRow key={group.id} title={text.groupLabels[group.id] ?? group.title}>
              <div className="screenshot-toolbar-chips">
                {group.slots.map((slot) => {
                  const visible = !settings.hiddenToolbar.includes(slot.key);
                  const label = slotLabel(slot.key, slot.label);
                  return (
                    <button
                      key={slot.key}
                      type="button"
                      className={`screenshot-toolbar-chip${visible ? " screenshot-toolbar-chip--on" : ""}`}
                      aria-pressed={visible}
                      aria-label={text.toolbarToggle(label, visible)}
                      disabled={!settings.enabled}
                      onClick={() => toggleToolbarSlot(slot.key)}
                    >
                      {label}
                    </button>
                  );
                })}
              </div>
            </SettingsRow>
          ))}
        </SettingsGroup>
      )}

      <SettingsGroup title={text.interfaceTitle}>
        <SettingsRow
          title={text.ocrLanguage}
          description={ocrLanguageFailed ? text.ocrLanguageFailed : undefined}
          control={
            <div className="screenshot-select-control">
              <SelectControl
                value={settings.ocrLanguage}
                ariaLabel={text.ocrLanguage}
                options={ocrOptions}
                disabled={!settings.enabled}
                onChange={(ocrLanguage) => save({ ocrLanguage })}
              />
            </div>
          }
        />
        <SettingsRow
          title={text.uiScale}
          control={
            <div className="screenshot-select-control">
              <SelectControl
                value={settings.uiScale}
                ariaLabel={text.uiScale}
                options={scaleOptions}
                disabled={!settings.enabled}
                onChange={(uiScale) => save({ uiScale })}
              />
            </div>
          }
        />
      </SettingsGroup>

      <SettingsGroup title={text.tryTitle}>
        <SettingsRow
          title={text.tryNow}
          description={text.tryNowHint}
          control={
            <button
              type="button"
              className="screenshot-try-button"
              disabled={!settings.enabled}
              onClick={() => {
                setCaptureError("");
                void captureScreenshot().catch((failure) => setCaptureError(String(failure)));
              }}
            >
              {text.tryNowAction}
            </button>
          }
        >
          {captureError && <p className="screenshot-shortcut-row__error" role="alert">{captureError}</p>}
        </SettingsRow>
      </SettingsGroup>
    </>
  );
}
