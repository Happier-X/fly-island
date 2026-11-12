import { useCallback, useEffect, useRef, useState } from "react";
import { isIslandMoving } from "../../lib/islandMotion";
import { getAgentStatus, type AgentStatusSnapshot } from "./api";
import { DEFAULT_AGENT_STATUS } from "./status";

/**
 * Polls what Cindy is doing and keeps the previous snapshot when nothing the island renders has
 * changed. With `enabled` false (the status light switched off) it stops polling and reports idle.
 */
export function useAgentStatus(enabled = true) {
  const [agentStatus, setAgentStatus] = useState<AgentStatusSnapshot>(DEFAULT_AGENT_STATUS);
  const isRefreshingAgentStatus = useRef(false);

  const refreshAgentStatus = useCallback(async () => {
    if (isRefreshingAgentStatus.current) {
      return;
    }

    isRefreshingAgentStatus.current = true;
    try {
      const snapshot = await getAgentStatus();
      setAgentStatus((current) =>
        current.statusPath === snapshot.statusPath &&
        current.cindy.phase === snapshot.cindy.phase &&
        current.cindy.taskId === snapshot.cindy.taskId
          ? current
          : snapshot,
      );
    } catch (error) {
      console.error("Failed to read agent status", error);
      setAgentStatus(DEFAULT_AGENT_STATUS);
    } finally {
      isRefreshingAgentStatus.current = false;
    }
  }, []);

  useEffect(() => {
    if (!enabled) {
      setAgentStatus(DEFAULT_AGENT_STATUS);
      return;
    }

    void refreshAgentStatus();

    const interval = window.setInterval(() => {
      // A change waits out the island's motion (islandMotion.ts): it re-renders the whole island.
      if (!isIslandMoving()) void refreshAgentStatus();
    }, 200);

    return () => window.clearInterval(interval);
  }, [enabled, refreshAgentStatus]);

  return agentStatus;
}
