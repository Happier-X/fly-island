import type { Translations } from "../../i18n";
import type { AgentProvider, AgentStatusSnapshot, AgentTaskPhase, AgentTaskStatus } from "./api";

export type AgentVisualState = "idle" | "running" | "attention";

export const DEFAULT_AGENT_TASK_STATUS: AgentTaskStatus = {
  phase: "idle",
  updatedAt: 0,
};
export const DEFAULT_AGENT_STATUS: AgentStatusSnapshot = {
  cindy: DEFAULT_AGENT_TASK_STATUS,
  updatedAt: 0,
  statusPath: "",
};
export const AGENT_PROVIDERS: AgentProvider[] = ["cindy"];
export const AGENT_PROVIDER_LABELS: Record<AgentProvider, string> = {
  cindy: "Cindy",
};

export function isAgentAttentionPhase(phase: AgentTaskPhase) {
  return phase === "failed" || phase === "stale";
}

/**
 * `activeSessions`: sessions the session list still counts as working. The light turns blue
 * for them too, so it never shows green beside the island's blue session count.
 */
export function getAgentVisualState(snapshot: AgentStatusSnapshot, activeSessions = 0): AgentVisualState {
  if (isAgentAttentionPhase(snapshot.cindy.phase)) {
    return "attention";
  }

  if (activeSessions > 0 || snapshot.cindy.phase === "running" || snapshot.cindy.phase === "waiting") {
    return "running";
  }

  return "idle";
}

/** What the status light says (its aria-label); `activeSessions` running in the session list also count as running. */
export function getAgentStatusLabel(snapshot: AgentStatusSnapshot, t: Translations, activeSessions = 0) {
  const label = AGENT_PROVIDER_LABELS.cindy;

  if (isAgentAttentionPhase(snapshot.cindy.phase)) {
    return snapshot.cindy.phase === "stale" ? t.agent.stale(label) : t.agent.failed(label);
  }

  if (snapshot.cindy.phase === "running" || snapshot.cindy.phase === "waiting") {
    return t.agent.running(label);
  }

  return activeSessions > 0 ? t.agent.running(label) : t.agent.idleOrDone;
}

export function getAgentPhaseLabel(phase: AgentTaskPhase, t: Translations) {
  switch (phase) {
    case "running":
      return t.agent.phase.running;
    case "completed":
      return t.agent.phase.completed;
    case "failed":
      return t.agent.phase.failed;
    case "stale":
      return t.agent.phase.stale;
    case "waiting":
      return t.agent.phase.waiting;
    case "idle":
    default:
      return t.agent.phase.idle;
  }
}
