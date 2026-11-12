import { useMemo } from "react";
import type { IslandSettings } from "../appearance/types";
import { countActiveSessions, isActivePhase } from "./sessionList";
import { useAgentSessions } from "./useAgentSessions";
import { useHiddenSessions } from "./useHiddenSessions";
import { useUnseenSessions } from "./useUnseenSessions";

type AgentSessionSettings = Pick<IslandSettings, "agentStatusEnabled" | "agentSessionHours" | "agentHideFinished">;

/**
 * The island's agent sessions: all shown ones, the ones the Agent page lists, the running count,
 * the unseen ones, and removing one from the page.
 */
export function useIslandAgentSessions(settings: AgentSessionSettings) {
  // Read every second: the collapsed island names sessions from it and holds a finished one's
  // title for three seconds after its last change. Cindy's database answers from a small query.
  const allAgentSessions = useAgentSessions(settings.agentStatusEnabled, 1000, settings.agentSessionHours);
  // Sessions removed from the Agent page leave the count and the collapsed island too.
  const { visible: agentSessions, removal: agentRemoval } = useHiddenSessions(allAgentSessions);
  const listedAgentSessions = useMemo(
    () => agentSessions.filter((session) => !settings.agentHideFinished || isActivePhase(session.phase)),
    [agentSessions, settings.agentHideFinished],
  );
  const activeAgentSessionCount = countActiveSessions(agentSessions);
  const unseenAgentSessions = useUnseenSessions(agentSessions, settings.agentStatusEnabled);
  return { agentSessions, listedAgentSessions, activeAgentSessionCount, unseenAgentSessions, agentRemoval };
}
