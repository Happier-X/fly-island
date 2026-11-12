import assert from "node:assert/strict";
import test from "node:test";
import {
  DEFAULT_AGENT_STATUS,
  getAgentPhaseLabel,
  getAgentStatusLabel,
  getAgentVisualState,
  isAgentAttentionPhase,
} from "../../src/features/agent-status/status.ts";
import { TRANSLATIONS } from "../../src/i18n/index.ts";

const t = TRANSLATIONS.en;
const snapshot = (phase) => ({
  ...DEFAULT_AGENT_STATUS,
  cindy: { phase, updatedAt: 1 },
});

test("failed and stale phases need attention", () => {
  assert.equal(isAgentAttentionPhase("failed"), true);
  assert.equal(isAgentAttentionPhase("stale"), true);
  for (const phase of ["idle", "running", "completed"]) {
    assert.equal(isAgentAttentionPhase(phase), false, phase);
  }
});

test("attention outranks running, which outranks idle", () => {
  assert.equal(getAgentVisualState(DEFAULT_AGENT_STATUS), "idle");
  assert.equal(getAgentVisualState(snapshot("completed")), "idle");
  assert.equal(getAgentVisualState(snapshot("running")), "running");
  // Cindy stopped on a question: the light shows working, the session list names the wait.
  assert.equal(getAgentVisualState(snapshot("waiting")), "running");
  assert.equal(getAgentVisualState(snapshot("failed")), "attention");
  assert.equal(getAgentVisualState(snapshot("stale")), "attention");
  // A session the list still counts as working keeps the light blue, beside the blue count.
  assert.equal(getAgentVisualState(DEFAULT_AGENT_STATUS, 2), "running");
  assert.equal(getAgentVisualState(snapshot("failed"), 2), "attention");
});

test("the status label says what Cindy is doing", () => {
  assert.equal(getAgentStatusLabel(DEFAULT_AGENT_STATUS, t), t.agent.idleOrDone);
  assert.equal(getAgentStatusLabel(snapshot("running"), t), t.agent.running("Cindy"));
  assert.equal(getAgentStatusLabel(snapshot("waiting"), t), t.agent.running("Cindy"));
  assert.equal(getAgentStatusLabel(snapshot("failed"), t), t.agent.failed("Cindy"));
  assert.equal(getAgentStatusLabel(snapshot("stale"), t), t.agent.stale("Cindy"));
  // Running sessions in the list match the blue light; attention still wins.
  assert.equal(getAgentStatusLabel(DEFAULT_AGENT_STATUS, t, 2), t.agent.running("Cindy"));
  assert.equal(getAgentStatusLabel(snapshot("failed"), t, 2), t.agent.failed("Cindy"));
});

test("phase labels fall back to idle for unknown phases", () => {
  assert.equal(getAgentPhaseLabel("running", t), t.agent.phase.running);
  assert.equal(getAgentPhaseLabel("completed", t), t.agent.phase.completed);
  assert.equal(getAgentPhaseLabel("unknown", t), t.agent.phase.idle);
});
