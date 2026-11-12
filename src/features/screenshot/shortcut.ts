import { DEFAULT_SCREENSHOT_SHORTCUT } from "./api";

const MODIFIERS: Record<string, string> = {
  ctrl: "Ctrl",
  control: "Ctrl",
  alt: "Alt",
  option: "Alt",
  shift: "Shift",
  win: "Win",
  windows: "Win",
  meta: "Win",
  cmd: "Win",
  super: "Win",
};

const KEY_ALIASES: Record<string, string> = {
  escape: "Esc",
  arrowup: "Up",
  arrowdown: "Down",
  arrowleft: "Left",
  arrowright: "Right",
  spacebar: "Space",
};

/** A recorded or stored shortcut in `Ctrl+Alt+Shift+Win+Key` order; the default when unusable. */
export function normalizeScreenshotShortcut(shortcut: string | undefined) {
  const text = shortcut?.trim();
  if (!text) return DEFAULT_SCREENSHOT_SHORTCUT;

  const modifiers = new Set<string>();
  let key = "";
  for (const part of text.split("+").map((value) => value.trim()).filter(Boolean)) {
    const modifier = MODIFIERS[part.toLowerCase()];
    if (modifier) {
      modifiers.add(modifier);
      continue;
    }
    if (key) continue;
    const lower = part.toLowerCase();
    key = KEY_ALIASES[lower] ?? (part.length === 1 ? part.toUpperCase() : part);
  }

  if (!key || modifiers.size === 0) return DEFAULT_SCREENSHOT_SHORTCUT;
  return ["Ctrl", "Alt", "Shift", "Win"].filter((modifier) => modifiers.has(modifier)).concat(key).join("+");
}
