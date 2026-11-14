import { invoke } from "@tauri-apps/api/core";

// 与 src-tauri/src/screenshot/config.rs 中的同名字段对应（serde camelCase）。
export type ScreenshotSettings = {
  enabled: boolean;
  shortcut: string;
  /** `png` / `jpeg` / `webp` */
  saveFormat: string;
  /** 空字符串表示用 AirSnip 的默认目录（系统「图片」下）。 */
  saveDir: string;
  /** 5 个标注色块，`#RRGGBB`。 */
  colors: string[];
  /** 覆盖层里的工具切换键与功能键，值是 Windows 虚拟键码（0 = 未设置）。 */
  overlayKeys: OverlayKeys;
  /** 工具栏上隐藏的元素键（`tool.rect` 这类稳定键）；空数组 = 全部可见。 */
  hiddenToolbar: string[];
  /** `auto` 跟随系统，否则是 BCP-47 标签（如 `zh-Hans-CN`）。 */
  ocrLanguage: string;
  /** `auto` / `100` / `125` / `150` / `175` / `200` */
  uiScale: string;
  /** 截图完成后同时把图片写进保存目录。 */
  autoSave: boolean;
};

/** 覆盖层内的快捷键：10 个工具切换键 + 长截图 / OCR 功能键。 */
export type OverlayKeys = {
  rect: number;
  arrow: number;
  text: number;
  blur: number;
  highlight: number;
  glow: number;
  number: number;
  line: number;
  ellipse: number;
  freehand: number;
  longshot: number;
  ocr: number;
};

/** AirSnip 的「未绑定」：该键在覆盖层里不生效。 */
export const UNBOUND_KEY = 0;

export const DEFAULT_OVERLAY_KEYS: OverlayKeys = {
  rect: 0x31,
  arrow: 0x32,
  text: 0x33,
  blur: 0x34,
  highlight: 0x35,
  glow: 0x36,
  number: 0x37,
  line: 0x38,
  ellipse: 0x39,
  freehand: 0x30,
  longshot: 0x53,
  ocr: 0x54,
};

export const DEFAULT_SCREENSHOT_SHORTCUT = "Ctrl+Shift+A";
export const SAVE_FORMATS = ["png", "jpeg", "webp"] as const;
export type SaveFormat = (typeof SAVE_FORMATS)[number];
export const DEFAULT_COLORS = ["#FFFFFF", "#E53935", "#FB8C00", "#43A047", "#1E88E5"] as const;
export const UI_SCALES = ["auto", "100", "125", "150", "175", "200"] as const;
export const OCR_LANG_AUTO = "auto";

export const DEFAULT_SCREENSHOT_SETTINGS: ScreenshotSettings = {
  enabled: true,
  shortcut: DEFAULT_SCREENSHOT_SHORTCUT,
  saveFormat: "png",
  saveDir: "",
  colors: [...DEFAULT_COLORS],
  overlayKeys: { ...DEFAULT_OVERLAY_KEYS },
  hiddenToolbar: [],
  ocrLanguage: OCR_LANG_AUTO,
  uiScale: "auto",
  autoSave: false,
};

/** 工具栏上一组可隐藏的元素（结构与文案都由 AirSnip 提供）。 */
export type ToolbarGroupInfo = {
  id: string;
  title: string;
  slots: { key: string; label: string }[];
};

export function getScreenshotSettings() {
  return invoke<ScreenshotSettings>("get_screenshot_settings");
}

export function setScreenshotSettings(settings: ScreenshotSettings) {
  return invoke<void>("set_screenshot_settings", { settings });
}

/** 立即截一张（不等全局热键），岛上的按钮走这里。 */
export function captureScreenshot() {
  return invoke<void>("capture_screenshot");
}

/** 弹出系统的选择文件夹对话框；取消时返回 null。 */
export function pickScreenshotDirectory(initial: string) {
  return invoke<string | null>("pick_screenshot_directory", { initial: initial || null });
}

/** 工具栏的分组与元素，用来渲染「工具栏可见性」。 */
export function getScreenshotToolbarGroups() {
  return invoke<ToolbarGroupInfo[]>("get_screenshot_toolbar_groups");
}

/** 系统已安装的 OCR 语言标签，`auto` 排在第一。 */
export function getScreenshotOcrLanguages() {
  return invoke<string[]>("get_screenshot_ocr_languages");
}

/** 虚拟键码 → 覆盖层里叫的名字（`"1"`、`"F1"`、`"未设置"`）。 */
export function screenshotKeyNames(vks: number[]) {
  return invoke<string[]>("screenshot_key_names", { vks });
}
