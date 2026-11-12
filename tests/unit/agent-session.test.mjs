import assert from "node:assert/strict";
import test from "node:test";
import {
  COMPLETED_TITLE_HOLD_MS,
  agentSessionExpiresAt,
  pickAgentSession,
} from "../../src/features/agent-status/session.ts";

const NOW = 1_800_000_000_000;

/** One entry of the Agent page's list (list_agent_sessions). */
function session(overrides = {}) {
  return {
    provider: "cindy",
    sessionId: "s1",
    cwd: "E:\\Projects\\fly-island",
    title: "设置界面完善思路",
    model: "Opus 5.5",
    effort: "medium",
    phase: "running",
    lastMessage: null,
    updatedAt: NOW - 10,
    ...overrides,
  };
}

test("a running session from the session list is picked", () => {
  assert.deepEqual(pickAgentSession([session()], NOW), {
    provider: "cindy",
    label: "Cindy",
    title: "设置界面完善思路",
  });
});

test("a session without a title is named by its project folder", () => {
  assert.equal(pickAgentSession([session({ title: null })], NOW)?.title, "fly-island");
  assert.equal(pickAgentSession([session({ title: "  ", cwd: null })], NOW), null);
});

test("a session waiting for the user keeps its title", () => {
  assert.equal(pickAgentSession([session({ phase: "waiting", updatedAt: 1, title: "跑测试" })], NOW)?.title, "跑测试");
});

test("failed and stale sessions keep their title", () => {
  assert.equal(pickAgentSession([session({ phase: "failed", updatedAt: 1, title: "a" })], NOW)?.title, "a");
  assert.equal(pickAgentSession([session({ phase: "stale", updatedAt: 1, title: "b" })], NOW)?.title, "b");
});

test("a completed title is held briefly, then dropped", () => {
  const done = [session({ phase: "completed", updatedAt: NOW - 1000, title: "恢复发布机环境" })];
  assert.equal(pickAgentSession(done, NOW)?.provider, "cindy");
  assert.equal(agentSessionExpiresAt(done, NOW), NOW - 1000 + COMPLETED_TITLE_HOLD_MS);
  const later = NOW - 1000 + COMPLETED_TITLE_HOLD_MS;
  assert.equal(pickAgentSession(done, later), null);
  assert.equal(agentSessionExpiresAt(done, later), null);
});

test("with several sessions the most recently updated wins", () => {
  const both = [
    session({ sessionId: "c", updatedAt: NOW - 500, title: "older task" }),
    session({ sessionId: "a", updatedAt: NOW - 100, title: "newer task" }),
  ];
  assert.equal(pickAgentSession(both, NOW)?.title, "newer task");
  const olderNewer = [
    session({ sessionId: "c", phase: "completed", updatedAt: NOW - 50, title: "just finished" }),
    session({ sessionId: "a", updatedAt: NOW - 100, title: "still running" }),
  ];
  assert.equal(pickAgentSession(olderNewer, NOW)?.title, "just finished");
});
