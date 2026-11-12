import assert from "node:assert/strict";
import test from "node:test";
import { isModifierKey, vkFromCode } from "../../src/features/screenshot/keys.ts";
import { DEFAULT_OVERLAY_KEYS, OCR_LANG_AUTO, UI_SCALES } from "../../src/features/screenshot/api.ts";

test("letters, digits and function keys map to their virtual-key codes", () => {
  assert.equal(vkFromCode("KeyA"), 0x41);
  assert.equal(vkFromCode("KeyZ"), 0x5a);
  assert.equal(vkFromCode("Digit1"), 0x31);
  assert.equal(vkFromCode("Digit0"), 0x30);
  assert.equal(vkFromCode("F1"), 0x70);
  assert.equal(vkFromCode("F12"), 0x7b);
  assert.equal(vkFromCode("F24"), 0x87);
});

test("named keys and the numpad map too", () => {
  assert.equal(vkFromCode("Space"), 0x20);
  assert.equal(vkFromCode("Escape"), 0x1b);
  assert.equal(vkFromCode("ArrowLeft"), 0x25);
  assert.equal(vkFromCode("Delete"), 0x2e);
  assert.equal(vkFromCode("Numpad5"), 0x65);
  assert.equal(vkFromCode("Semicolon"), 0xba);
});

test("a key the overlay cannot bind is null, so the old binding stays", () => {
  assert.equal(vkFromCode("Backspace"), null, "录制时按 Backspace 是清除绑定");
  assert.equal(vkFromCode("IntlBackslash"), null);
  assert.equal(vkFromCode("MediaPlayPause"), null);
  assert.equal(vkFromCode(""), null);
});

test("modifier keys never bind on their own", () => {
  for (const key of ["Shift", "Control", "Alt", "Meta", "OS", "AltGraph"]) {
    assert.equal(isModifierKey(key), true, key);
  }
  assert.equal(isModifierKey("a"), false);
  assert.equal(isModifierKey("Shift1"), false);
});

test("the defaults are the overlay's own bindings", () => {
  // 与 crates/airsnip 的 ToolKeys / HotkeyConfig 默认值一致：1–9、0、S、T
  assert.deepEqual(Object.values(DEFAULT_OVERLAY_KEYS), [
    0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, 0x38, 0x39, 0x30, 0x53, 0x54,
  ]);
});

test("the scale and OCR values match what the Rust config accepts", () => {
  assert.deepEqual([...UI_SCALES], ["auto", "100", "125", "150", "175", "200"]);
  assert.equal(OCR_LANG_AUTO, "auto");
});
