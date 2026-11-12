import { AgentSourceSettings } from "../../features/agent-status/AgentSourceSettings";

/**
 * Settings → AI Agent → Data source: the island reads Cindy's own session database, and this page
 * says where it is and whether it can be read. Nothing to install, nothing to keep in sync.
 */
export function AgentSourceSettingsPage() {
  return <AgentSourceSettings />;
}
