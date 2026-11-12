import type { AgentSessionSummary } from "./api";

/**
 * A session's light, the one judgement every place showing agent state uses:
 * - `needs`: it stopped mid-turn on the user (Cindy's ask_user / plan_review);
 * - `working`: a turn is running;
 * - `stale`: silent mid-turn for a long while;
 * - `done`: the turn is over and nothing is asked;
 * - `failed`: the turn ended on an error.
 */
export type SessionLight = "needs" | "working" | "stale" | "done" | "failed";

export function sessionLight(session: Pick<AgentSessionSummary, "phase">): SessionLight {
  switch (session.phase) {
    case "waiting":
      return "needs";
    case "running":
      return "working";
    case "stale":
      return "stale";
    case "failed":
      return "failed";
    default:
      return "done";
  }
}
