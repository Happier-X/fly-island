import { invoke } from "@tauri-apps/api/core";

// 与 src-tauri/src/agent_status/snapshot.rs、sessions.rs、cindy/mod.rs 中的类型对应。
// Cindy 是本机唯一会跑 agent 会话的程序，所以 provider 只有它一个。

/** The only agent host this build knows: Cindy. */
export type AgentProvider = "cindy";

/** `waiting` 表示这一轮停在需要你回答的地方（Cindy 的 ask_user / plan_review）。 */
export type AgentTaskPhase = "idle" | "running" | "waiting" | "completed" | "failed" | "stale";

export type AgentTaskStatus = {
  phase: AgentTaskPhase;
  /** The session the state is about, when it is about one. */
  taskId?: string;
  updatedAt: number;
};

export type AgentStatusSnapshot = {
  cindy: AgentTaskStatus;
  updatedAt: number;
  /** Cindy 的数据库路径；空字符串表示没找到（Cindy 没装或还没建库）。 */
  statusPath: string;
};

/** Mirrors AgentSessionSummary in src-tauri/src/agent_status/sessions.rs. */
export type AgentSessionSummary = {
  provider: AgentProvider;
  sessionId: string;
  cwd: string | null;
  title: string | null;
  /** The model Cindy runs this session with, as Cindy names it. */
  model: string | null;
  effort: string | null;
  phase: Exclude<AgentTaskPhase, "idle">;
  lastMessage: string | null;
  updatedAt: number;
  /** When the current turn started (Unix ms); null when it is not running. */
  turnStartedAt: number | null;
  /** How freely the session may act; null when not known. */
  permissionMode?: AgentPermissionMode | null;
  /** Tokens in the context now; shown on the page's cards. */
  contextTokens?: number | null;
  /** The model's context window, for the occupancy a card shows; null when unknown. */
  contextWindow?: number | null;
  /** Tokens this session has spent in total, and what they cost; shown on hover. */
  totalTokens?: number | null;
  costUsd?: number | null;
};

/** Mirrors session_mode in Cindy's own settings: the modes the page knows how to name. */
export type AgentPermissionMode = "default" | "auto" | "bypass" | "acceptEdits" | "plan";

/**
 * Brings Cindy to the front. Every session runs in Cindy's own window, so a click on a session
 * is a click on Cindy. False when it is not running or has no window.
 */
export function focusAgentWindow() {
  return invoke<boolean>("focus_agent_window");
}

/** Whether Cindy is the window in front: the island then leaves its alerts and badges alone. */
export function agentWindowInFront() {
  return invoke<boolean>("agent_window_in_front");
}

/** Cindy sessions active in the last `windowHours` hours, running and waiting ones first. */
export function listAgentSessions(windowHours?: number) {
  return invoke<AgentSessionSummary[]>("list_agent_sessions", { windowHours });
}

/** What Cindy is doing now: running, waiting on an answer, or just finished. */
export function getAgentStatus() {
  return invoke<AgentStatusSnapshot>("get_agent_status");
}
