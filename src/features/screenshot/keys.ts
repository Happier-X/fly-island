/** The physical key → Windows virtual-key code, for recording a single overlay key. */
const NAMED_CODES: Record<string, number> = {
  Escape: 0x1b,
  Tab: 0x09,
  CapsLock: 0x14,
  Space: 0x20,
  Backspace: 0x08,
  Enter: 0x0d,
  Insert: 0x2d,
  Delete: 0x2e,
  Home: 0x24,
  End: 0x23,
  PageUp: 0x21,
  PageDown: 0x22,
  ArrowLeft: 0x25,
  ArrowUp: 0x26,
  ArrowRight: 0x27,
  ArrowDown: 0x28,
  PrintScreen: 0x2c,
  ScrollLock: 0x91,
  Pause: 0x13,
  NumLock: 0x90,
  Minus: 0xbd,
  Equal: 0xbb,
  BracketLeft: 0xdb,
  BracketRight: 0xdd,
  Backslash: 0xdc,
  Semicolon: 0xba,
  Quote: 0xde,
  Comma: 0xbc,
  Period: 0xbe,
  Slash: 0xbf,
  Backquote: 0xc0,
  NumpadAdd: 0x6b,
  NumpadSubtract: 0x6d,
  NumpadMultiply: 0x6a,
  NumpadDivide: 0x6f,
  NumpadDecimal: 0x6e,
};

/**
 * The virtual-key code of a key press, or null for a key the overlay cannot bind.
 *
 * Keyed on the physical `code`, not the character: the overlay binds a key position, so
 * a layout where "1" sits elsewhere still gives the same tool key. Backspace is left
 * out on purpose — recording it clears a binding instead.
 */
export function vkFromCode(code: string): number | null {
  if (/^Key[A-Z]$/.test(code)) return code.charCodeAt(3);
  if (/^Digit[0-9]$/.test(code)) return 0x30 + Number(code.slice(5));
  if (/^Numpad[0-9]$/.test(code)) return 0x60 + Number(code.slice(6));
  if (/^F([1-9]|1[0-9]|2[0-4])$/.test(code)) return 0x70 + Number(code.slice(1)) - 1;
  if (code === "Backspace") return null;
  return NAMED_CODES[code] ?? null;
}

/** Modifier keys never bind on their own; the overlay reads them as part of nothing. */
const MODIFIER_KEYS = new Set(["Alt", "AltGraph", "Control", "Meta", "Shift", "OS"]);

export function isModifierKey(key: string) {
  return MODIFIER_KEYS.has(key);
}
