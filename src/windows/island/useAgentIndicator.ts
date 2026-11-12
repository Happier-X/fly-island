import { useMemo } from "react";
import { getAgentStatusLabel, getAgentVisualState } from "../../features/agent-status/status";
import type { AgentStatusSnapshot } from "../../features/agent-status/api";
import type { IslandSettings } from "../../features/appearance/types";
import type { useTranslation } from "../../i18n";

/** The collapsed island's agent light: its colour state and what it reads out. */
export function useAgentIndicator(
  agentStatus: AgentStatusSnapshot,
  settings: IslandSettings,
  activeAgentSessionCount: number,
  t: ReturnType<typeof useTranslation>,
) {
  const agentVisualState = useMemo(
    () => (settings.agentStatusEnabled ? getAgentVisualState(agentStatus, activeAgentSessionCount) : "idle"),
    [agentStatus, activeAgentSessionCount, settings.agentStatusEnabled],
  );
  const agentStatusLabel = useMemo(
    () => getAgentStatusLabel(agentStatus, t, activeAgentSessionCount),
    [agentStatus, activeAgentSessionCount, t],
  );
  return { agentVisualState, agentStatusLabel };
}
