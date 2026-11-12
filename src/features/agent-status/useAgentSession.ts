import { useEffect, useMemo, useState } from "react";
import type { AgentSessionSummary } from "./api";
import { agentSessionExpiresAt, pickAgentSession, type AgentSession } from "./session";

/**
 * The agent session to name on the collapsed island, or null; re-checks when a completed title
 * expires. `sessions` is the Agent page's list.
 */
export function useAgentSession(sessions: AgentSessionSummary[], enabled: boolean): AgentSession | null {
  const [now, setNow] = useState(() => Date.now());

  // New data is the moment phases changed, so measure the completed hold from here.
  useEffect(() => {
    setNow(Date.now());
  }, [sessions]);

  useEffect(() => {
    if (!enabled) return;
    const expiresAt = agentSessionExpiresAt(sessions, now);
    if (expiresAt === null) return;
    const timer = window.setTimeout(() => setNow(Date.now()), Math.max(0, expiresAt - Date.now()));
    return () => window.clearTimeout(timer);
  }, [enabled, now, sessions]);

  return useMemo(() => (enabled ? pickAgentSession(sessions, now) : null), [enabled, now, sessions]);
}
