import type { AgentSessionSummary } from "./api";
import { sessionKey } from "./unseen.ts";

/**
 * When a session's change is worth a sound or a pop-up: a task finished, or a question waiting
 * for an answer. The sounds and the pop-ups each keep their own `Announced`, so turning one off
 * leaves the other as it was.
 */

/**
 * How long a finished turn has to stay finished before it is announced. An agent that carries on
 * by itself runs again within moments: that was a step of the task, not its end.
 */
export const COMPLETE_SETTLE_MS = 6_000;
/**
 * How long a wait on the user has to last before it is announced: an `ask_user` question or a plan
 * to confirm that lingers is one the user needs to act on.
 */
export const WAIT_SETTLE_MS = 3_000;

/** The waits the sounds and pop-ups use, read when each change comes in. */
export const announceTiming = { completeMs: COMPLETE_SETTLE_MS, waitMs: WAIT_SETTLE_MS };

type Turn = Pick<AgentSessionSummary, "provider" | "sessionId" | "phase">;
type Wait = Turn;

/** What has been announced: the turn each finished one was announced for, and each wait's. */
export type Announced = { finished: Map<string, number | null>; waits: Map<string, number> };

export function emptyAnnounced(): Announced {
  return { finished: new Map(), waits: new Map() };
}

/**
 * Whether the session's current turn is the user's own. Cindy records no prompt ownership, so a
 * failed session is always announced: the alternative is going quiet about real failures.
 */
export function isUsersTurn(_session: Turn): boolean {
  return true;
}

/** The moment a turn began: what tells one turn of a session from the next. */
function turnStamp(session: AgentSessionSummary) {
  return session.turnStartedAt ?? session.updatedAt;
}

/**
 * The `pending` finished turns to announce now: still finished in `current`, and of a turn not
 * announced before. A turn Cindy continues by itself keeps its start time, so its later stops
 * stay quiet. Marks the returned turns in `announced`.
 */
export function settledFinishes<T extends Turn>(pending: T[], current: AgentSessionSummary[], announced: Announced): T[] {
  const now = new Map(current.map((session) => [sessionKey(session), session]));
  return pending.filter((session) => {
    const key = sessionKey(session);
    const latest = now.get(key);
    if (latest?.phase !== "completed") return false;
    const stamp = turnStamp(latest);
    if (announced.finished.get(key) === stamp) return false;
    announced.finished.set(key, stamp);
    return true;
  });
}

/**
 * The `pending` waits to announce now: still waiting in `current`, and not the same wait already
 * announced. A session asked once per turn: the answer appends a message, so the next question
 * comes with a newer turn.
 */
export function settledWaits<T extends Wait>(pending: T[], current: AgentSessionSummary[], announced: Announced): T[] {
  const latest = new Map(current.map((session) => [sessionKey(session), session]));
  return pending.filter((session) => {
    const key = sessionKey(session);
    const still = latest.get(key);
    if (still?.phase !== "waiting") return false;
    const stamp = still.updatedAt;
    if (announced.waits.get(key) === stamp) return false;
    announced.waits.set(key, stamp);
    return true;
  });
}
