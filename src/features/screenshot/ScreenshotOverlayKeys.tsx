import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "../../i18n";
import { SettingsGroup } from "../../components/settings/SettingsGroup";
import { DEFAULT_OVERLAY_KEYS, screenshotKeyNames, type OverlayKeys, type ScreenshotSettings } from "./api";
import { KeyRecordButton } from "./KeyRecordButton";

/** The overlay's tool keys, in the order the toolbar shows the tools. */
const TOOL_KEY_FIELDS = [
  { field: "rect", slot: "tool.rect" },
  { field: "arrow", slot: "tool.arrow" },
  { field: "text", slot: "tool.text" },
  { field: "blur", slot: "tool.blur" },
  { field: "highlight", slot: "tool.highlight" },
  { field: "glow", slot: "tool.glow" },
  { field: "number", slot: "tool.number" },
  { field: "line", slot: "tool.line" },
  { field: "ellipse", slot: "tool.ellipse" },
  { field: "freehand", slot: "tool.freehand" },
] as const satisfies readonly { field: keyof OverlayKeys; slot: string }[];

/** The overlay's action keys, which are not tools. */
const ACTION_KEY_FIELDS = [
  { field: "longshot", slot: "output.longshot" },
  { field: "ocr", slot: "output.ocr" },
] as const satisfies readonly { field: keyof OverlayKeys; slot: string }[];

/**
 * The keys the capture overlay answers to. They only work while it is open, so they are
 * kept apart from the global capture shortcut above.
 */
export function ScreenshotOverlayKeys({
  settings,
  onChange,
}: {
  settings: ScreenshotSettings;
  onChange: (patch: Partial<ScreenshotSettings>) => void;
}) {
  const t = useTranslation();
  const text = t.settings.screenshot;
  const [keyNames, setKeyNames] = useState<Record<number, string>>({});

  // Key names are Win32 names ("1", "F1", "未设置"), so they come from the overlay's side.
  const codes = useMemo(() => Object.values(settings.overlayKeys), [settings.overlayKeys]);
  useEffect(() => {
    let cancelled = false;
    void screenshotKeyNames(codes)
      .then((names) => {
        if (cancelled) return;
        const resolved: Record<number, string> = {};
        codes.forEach((vk, index) => {
          resolved[vk] = names[index] ?? "";
        });
        setKeyNames(resolved);
      })
      .catch((failure) => console.error("Failed to name the overlay keys", failure));
    return () => {
      cancelled = true;
    };
  }, [codes]);

  const slotLabel = (key: string, fallback: string) => text.slotLabels[key] ?? fallback;
  const isDefault = (Object.keys(DEFAULT_OVERLAY_KEYS) as (keyof OverlayKeys)[]).every(
    (field) => settings.overlayKeys[field] === DEFAULT_OVERLAY_KEYS[field],
  );

  const cells = (fields: readonly { field: keyof OverlayKeys; slot: string }[]) =>
    fields.map(({ field, slot }) => (
      <div key={field} className="screenshot-key-cell">
        <span className="screenshot-key-cell__label">{slotLabel(slot, "")}</span>
        <KeyRecordButton
          name={keyNames[settings.overlayKeys[field]] ?? ""}
          recordingLabel={text.pressKey}
          unboundLabel={text.keyUnbound}
          label={slotLabel(slot, slot)}
          disabled={!settings.enabled}
          onRecord={(vk) => onChange({ overlayKeys: { ...settings.overlayKeys, [field]: vk } })}
        />
      </div>
    ));

  return (
    <SettingsGroup
      title={text.overlayKeysTitle}
      footer={text.overlayKeysHint}
      action={
        <button
          type="button"
          className="screenshot-ghost-button screenshot-group-action"
          disabled={!settings.enabled || isDefault}
          onClick={() => onChange({ overlayKeys: { ...DEFAULT_OVERLAY_KEYS } })}
        >
          {text.keysReset}
        </button>
      }
    >
      <div className="screenshot-key-block">
        <span className="screenshot-key-block__title">{text.toolKeys}</span>
        <div className="screenshot-key-grid">{cells(TOOL_KEY_FIELDS)}</div>
      </div>
      <div className="screenshot-key-block">
        <span className="screenshot-key-block__title">{text.functionKeys}</span>
        <div className="screenshot-key-grid">{cells(ACTION_KEY_FIELDS)}</div>
      </div>
    </SettingsGroup>
  );
}
