import assert from "node:assert/strict";
import test from "node:test";
import { soundFor } from "../../src/features/agent-status/sounds/events.ts";
import { emptyAnnounced, settledFinishes, settledWaits } from "../../src/features/agent-status/announce.ts";
import { isQuietTime, parseClock } from "../../src/features/agent-status/sounds/quietHours.ts";
import { DEFAULT_AGENT_SOUND_SETTINGS, normalizeAgentSoundSettings } from "../../src/features/agent-status/sounds/storage.ts";

test("session changes map to the sound for them", () => {
  assert.equal(soundFor(undefined, "running"), "start");
  assert.equal(soundFor(undefined, "completed"), null, "an old session coming into view is quiet");
  assert.equal(soundFor("running", "completed"), "complete");
  assert.equal(soundFor("running", "failed"), "error");
  // Cindy stops on a question (ask_user) or a plan to confirm: both are questions.
  assert.equal(soundFor("running", "waiting"), "question");
  assert.equal(soundFor("completed", "running"), "ack");
  assert.equal(soundFor("running", "running"), null);
  assert.equal(soundFor("waiting", "waiting"), null);
});

test("quiet hours cover the range and may run past midnight", () => {
  assert.equal(parseClock("8:05"), 485);
  assert.equal(parseClock("24:00"), null);
  const at = (h, m) => new Date(2026, 10, 6, h, m);
  assert.equal(isQuietTime(at(23, 0), "22:00", "08:00"), true);
  assert.equal(isQuietTime(at(7, 59), "22:00", "08:00"), true);
  assert.equal(isQuietTime(at(8, 0), "22:00", "08:00"), false);
  assert.equal(isQuietTime(at(13, 0), "12:00", "14:00"), true);
  assert.equal(isQuietTime(at(13, 0), "12:00", "12:00"), false);
});

test("only the sound switch is read back; the rest stays at its default", () => {
  assert.deepEqual(normalizeAgentSoundSettings(undefined), DEFAULT_AGENT_SOUND_SETTINGS);
  const settings = normalizeAgentSoundSettings({ enabled: false, volume: 20, pack: "mine", events: { start: true }, quietHours: { enabled: true } });
  assert.deepEqual(settings, { ...DEFAULT_AGENT_SOUND_SETTINGS, enabled: false });
});

const turn = (phase, prompts = {}) => ({ provider: "cindy", sessionId: "s1", phase, turnStartedAt: 100, updatedAt: 100, ...prompts });

test("a finished turn is announced once it stays finished, once per turn", () => {
  const announced = emptyAnnounced();
  assert.deepEqual(settledFinishes([turn("completed")], [turn("running")], announced), [], "running again: a step, not the end");
  assert.equal(settledFinishes([turn("completed")], [turn("completed")], announced).length, 1);
  assert.deepEqual(settledFinishes([turn("completed")], [turn("completed")], announced), [], "the same turn stays quiet");
  const next = turn("completed", { turnStartedAt: 200, updatedAt: 200 });
  assert.equal(settledFinishes([next], [next], announced).length, 1, "a new turn is announced again");
  const unknown = turn("completed", { turnStartedAt: null });
  assert.equal(settledFinishes([unknown], [unknown], announced).length, 1, "no start time known: announced");
});

test("a wait is announced once per turn", () => {
  const wait = (updatedAt) => ({ provider: "cindy", sessionId: "s1", phase: "waiting", updatedAt });
  const announced = emptyAnnounced();
  assert.deepEqual(settledWaits([wait(1_000)], [{ ...wait(1_000), phase: "running" }], announced), [], "answered already: quiet");
  assert.equal(settledWaits([wait(1_000)], [wait(1_000)], announced).length, 1);
  assert.deepEqual(settledWaits([wait(1_000)], [wait(1_000)], announced), [], "the same wait stays quiet");
  assert.equal(settledWaits([wait(2_000)], [wait(2_000)], announced).length, 1, "a new question is announced again");
});

test("starts, acknowledgements and idle chimes are quiet by default", () => {
  const { events } = DEFAULT_AGENT_SOUND_SETTINGS;
  assert.equal(events.start || events.ack || events.idle, false);
  assert.equal(events.complete && events.error && events.question, true);
});
