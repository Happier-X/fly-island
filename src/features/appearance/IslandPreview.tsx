import type { CSSProperties } from "react";
import { Bell } from "lucide-react";
import { useTranslation } from "../../i18n";
import { inkOn } from "../../utils/color";
import { ExpandedPreview } from "./ExpandedPreview";
import { cornerShapeCss } from "./cornerShape";
import { getEffectiveAppearanceMode, hexToRgba, MAX_EXPANDED_SCALE } from "./settings";
import type { IslandSettings } from "./types";
import "./IslandPreview.css";

/**
 * Appearance, first: the island drawn from the current settings, collapsed and expanded, on a
 * colourful wallpaper so opacity and glass show. Collapsed shows the todo text and a Cindy
 * notification badge; expanded shows the todo page. A drawing with sample content, not the
 * island itself; the variables mirror windows/island/stageStyle.ts, which features cannot import.
 */
export function IslandPreview({ settings }: { settings: IslandSettings }) {
  const text = useTranslation().settings.appearance.preview;
  const glass = getEffectiveAppearanceMode(settings.appearanceMode) === "liquidGlass";
  const style = {
    "--preview-fill": hexToRgba(settings.islandBackgroundColor, settings.opacity / 100),
    "--preview-glass": settings.glassIntensity / 100,
    "--preview-radius": `${settings.islandCornerRadius}px`,
    "--preview-radius-px": settings.islandCornerRadius,
    "--preview-corner-shape": cornerShapeCss(settings.islandCornerShape),
    "--preview-pulse": settings.pulseColor,
    "--preview-pulse-glow": hexToRgba(settings.pulseColor, 0.72),
    "--preview-pulse-brightness": `${settings.pulseBrightness}%`,
    "--preview-text": settings.taskTextColor,
    "--preview-ink": inkOn(settings.islandBackgroundColor),
    "--preview-collapsed-scale": settings.collapsedScale,
    "--preview-expanded-scale": settings.expandedScale,
    "--preview-expanded-max-scale": MAX_EXPANDED_SCALE,
  } as CSSProperties;
  const islandClass = [
    "island-preview__island",
    glass && "island-preview__island--glass",
  ].filter(Boolean).join(" ");

  return (
    <figure
      className={`island-preview island-preview--font-${settings.fontFamily}`}
      style={style}
      aria-label={text.label}
    >
      <div className="island-preview__desktop">
        <div className={`${islandClass} island-preview__island--collapsed`}>
          <span className="island-preview__pulse" />
          {settings.showCollapsedText && <span className="island-preview__task">{text.collapsedText}</span>}
          <span className="island-preview__badge"><Bell size={10} strokeWidth={2} />1</span>
        </div>
        <figcaption className="island-preview__caption">{text.collapsed}</figcaption>
      </div>
      <div className="island-preview__desktop">
        <div className={`${islandClass} island-preview__island--expanded`}>
          <ExpandedPreview />
        </div>
        <figcaption className="island-preview__caption">{text.expanded}</figcaption>
      </div>
    </figure>
  );
}
