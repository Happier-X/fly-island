import type { AgentProvider, AgentSessionSummary, AgentTaskStatus } from "./api";
import { sessionProject } from "./sessionList.ts";
import { AGENT_PROVIDER_LABELS } from "./status.ts";

/** How long a finished session's title stays on the island before the todo comes back. */
export const COMPLETED_TITLE_HOLD_MS = 3000;

export type AgentSession = {
  provider: AgentProvider;
  /** Who is working: "Cindy". */
  label: string;
  title: string;
};

type Candidate = { session: AgentSession; phase: AgentTaskStatus["phase"]; updatedAt: number };

function candidates(sessions: AgentSessionSummary[]): Candidate[] {
  return sessions
    .map((summary) => ({
      session: {
        provider: summary.provider,
        label: AGENT_PROVIDER_LABELS[summary.provider],
        title: (summary.title ?? sessionProject(summary.cwd) ?? "").trim(),
      },
      phase: summary.phase,
      updatedAt: summary.updatedAt,
    }))
    .filter((candidate) => candidate.session.title);
}

function isVisible({ phase, updatedAt }: Candidate, now: number) {
  return (
    phase === "running" ||
    phase === "waiting" ||
    phase === "failed" ||
    phase === "stale" ||
    (phase === "completed" && now - updatedAt < COMPLETED_TITLE_HOLD_MS)
  );
}

/**
 * The session the collapsed island should name: a running, waiting, failed or stale one, or one
 * that completed less than `COMPLETED_TITLE_HOLD_MS` ago. With several, the most recently
 * updated wins.
 */
export function pickAgentSession(sessions: AgentSessionSummary[], now: number): AgentSession | null {
  let picked: Candidate | null = null;
  for (const candidate of candidates(sessions)) {
    if (isVisible(candidate, now) && (!picked || candidate.updatedAt > picked.updatedAt)) {
      picked = candidate;
    }
  }
  return picked?.session ?? null;
}

/** When the next completed title stops being shown, or null when nothing expires. */
export function agentSessionExpiresAt(sessions: AgentSessionSummary[], now: number): number | null {
  const expiries = candidates(sessions)
    .filter((candidate) => candidate.phase === "completed")
    .map((candidate) => candidate.updatedAt + COMPLETED_TITLE_HOLD_MS)
    .filter((expiresAt) => expiresAt > now);
  return expiries.length > 0 ? Math.min(...expiries) : null;
}
