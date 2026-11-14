import assert from "node:assert/strict";
import test from "node:test";
import { normalizeWatermark, unseenNotifications, watermarkAfter } from "../../src/features/cindy-notifications/seen.ts";
import { normalizeNotificationSoundSettings } from "../../src/features/cindy-notifications/sounds/storage.ts";

const KINDS = { done: true, question: true, scheduled: true, error: true };
const item = (kind, at) => ({ kind, key: `${kind}:${at}`, at });

test("only events newer than the watermark and switched on are unhandled", () => {
  const items = [item("done", 1_000), item("question", 2_000), item("error", 3_000)];
  assert.deepEqual(unseenNotifications(items, 1_500, KINDS).map((entry) => entry.key), ["question:2000", "error:3000"]);
  assert.deepEqual(unseenNotifications(items, 3_000, KINDS), []);
  assert.deepEqual(unseenNotifications(items, 0, { ...KINDS, error: false }).map((entry) => entry.key), ["done:1000", "question:2000"]);
});

test("handling a read moves the watermark to that read and never back", () => {
  assert.equal(watermarkAfter(1_000, 5_000), 5_000);
  assert.equal(watermarkAfter(5_000, 1_000), 5_000);
});

test("a saved watermark reads back only when it is a positive number", () => {
  assert.equal(normalizeWatermark({ watermark: 1_700_000_000_000 }), 1_700_000_000_000);
  assert.equal(normalizeWatermark({ watermark: 0 }), null);
  assert.equal(normalizeWatermark({ watermark: "soon" }), null);
  assert.equal(normalizeWatermark({}), null);
  assert.equal(normalizeWatermark(null), null);
});

test("the sound switch keeps its value and defaults on, ignoring old extras", () => {
  assert.equal(normalizeNotificationSoundSettings(undefined).enabled, true);
  assert.equal(normalizeNotificationSoundSettings({ enabled: false }).enabled, false);
  assert.equal(normalizeNotificationSoundSettings({ enabled: "no" }).enabled, true);
  assert.deepEqual(normalizeNotificationSoundSettings({ enabled: true, volume: 10, pack: "chimes", events: { start: true } }), { enabled: true });
});
