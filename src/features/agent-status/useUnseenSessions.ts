import { useCallback, useEffect, useMemo, useState } from "react";
import type { AgentSessionSummary } from "./api";
import { isUnseen, loadSeenTurns, markSeen, saveSeen, sessionKey, type SeenTurns } from "./unseen";

export type UnseenSessions = {
  /** `provider:sessionId` of finished sessions the user has not looked at. */
  keys: ReadonlySet<string>;
  markSeen: (session: AgentSessionSummary) => void;
};

/**
 * Finished sessions the user has not looked at, for the Agent page to highlight. Looking means
 * clicking the session there, which also brings Cindy forward. Saved, so a restart does not ask
 * again; sessions that finished before the island started, and were never looked at, count as
 * seen unless they still wait on the user.
 */
export function useUnseenSessions(sessions: AgentSessionSummary[], enabled: boolean): UnseenSessions {
  const [turns, setTurns] = useState<SeenTurns>(loadSeenTurns);
  useEffect(() => saveSeen(turns.seen), [turns.seen]);
  const unseen = useMemo(
    () => (enabled ? sessions.filter((session) => isUnseen(session, turns)) : []),
    [enabled, sessions, turns],
  );

  const keys = useMemo(() => new Set(unseen.map(sessionKey)), [unseen]);
  const markSessionSeen = useCallback((session: AgentSessionSummary) => setTurns((current) => markSeen(current, [session])), []);
  return useMemo(() => ({ keys, markSeen: markSessionSeen }), [keys, markSessionSeen]);
}
