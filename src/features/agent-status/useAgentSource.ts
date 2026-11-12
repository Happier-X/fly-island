import { useCallback, useEffect, useState } from "react";
import { getAgentStatus, listAgentSessions } from "./api";

/** What the Data source page says about Cindy's database. */
export type AgentSourceState =
  | { kind: "reading"; path: string | null; count: number }
  | { kind: "missing" }
  | { kind: "failed"; error: string };

const REFRESH_MS = 2000;

/**
 * Reads Cindy's database as the settings page shows it: the path it found, how many sessions came
 * back, or why nothing did. Polled while the page is open, so a database Cindy creates after the
 * page opened shows up by itself.
 */
export function useAgentSource() {
  const [state, setState] = useState<AgentSourceState>({ kind: "reading", path: null, count: 0 });
  const [reading, setReading] = useState(false);

  const read = useCallback(async () => {
    setReading(true);
    try {
      const [status, sessions] = await Promise.all([getAgentStatus(), listAgentSessions()]);
      setState(
        status.statusPath
          ? { kind: "reading", path: status.statusPath, count: sessions.length }
          : { kind: "missing" },
      );
    } catch (error) {
      console.error("Failed to read Cindy's database", error);
      setState({ kind: "failed", error: error instanceof Error ? error.message : String(error) });
    } finally {
      setReading(false);
    }
  }, []);

  useEffect(() => {
    void read();
    const timer = window.setInterval(() => void read(), REFRESH_MS);
    return () => window.clearInterval(timer);
  }, [read]);

  return { state, reading, refresh: read };
}
