import assert from "node:assert/strict";
import test from "node:test";
import { emptyIdleSessions, takeIdleSessions } from "../../src/features/agent-status/sounds/idle.ts";

const session = (sessionId, phase, updatedAt = 1) => ({ provider: "cindy", sessionId, phase, updatedAt });

test("the idle sound comes once per turn for a finished session left unread", () => {
  const state = emptyIdleSessions();
  const unseen = new Set(["cindy:a"]);
  const sessions = [session("a", "completed"), session("b", "completed")];
  assert.deepEqual(takeIdleSessions(state, sessions, unseen, 0, 1000), []);
  assert.deepEqual(takeIdleSessions(state, sessions, unseen, 999, 1000), []);
  assert.deepEqual(takeIdleSessions(state, sessions, unseen, 1000, 1000).map((item) => item.sessionId), ["a"]);
  assert.deepEqual(takeIdleSessions(state, sessions, unseen, 5000, 1000), [], "once per turn");
  // Read: it starts over; a new unread turn waits its own time.
  assert.deepEqual(takeIdleSessions(state, sessions, new Set(), 6000, 1000), []);
  const next = [session("a", "completed", 2)];
  assert.deepEqual(takeIdleSessions(state, next, unseen, 7000, 1000), []);
  assert.deepEqual(takeIdleSessions(state, next, unseen, 8000, 1000).length, 1);
  assert.deepEqual(takeIdleSessions(state, [session("a", "running", 3)], unseen, 9000, 1000), [], "working sessions are not idle");
});
