import type { AgentSessionSummary } from "./api";

/**
 * What a working session row says under its title, in at most two lines (the row height the
 * island is sized by): the latest reply Cindy recorded for the session.
 * Null when there is nothing to show, so the row folds to one line.
 */
export function SessionBody({ session }: { session: AgentSessionSummary }) {
  if (!session.lastMessage) return null;
  return <span className="agent-session__message">{session.lastMessage}</span>;
}
