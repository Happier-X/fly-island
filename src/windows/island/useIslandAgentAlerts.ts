import { useEffect, useRef } from "react";
import type { AgentSessionSummary } from "../../features/agent-status/api";
import { sessionKey } from "../../features/agent-status/unseen";
import { alertKindsOf, useAgentAlerts } from "../../features/agent-status/useAgentAlerts";
import { useAgentSounds } from "../../features/agent-status/sounds/useAgentSounds";
import { playAgentSound } from "../../features/agent-status/sounds/player";
import { useAgentSoundSettings } from "../../features/agent-status/sounds/useAgentSoundSettings";
import type { IslandSettings } from "../../features/appearance/types";
import type { IslandMode, IslandPage } from "./types";
import { useOutsideClick } from "./useOutsideClick";

type IslandAgentAlertsOptions = {
  sessions: AgentSessionSummary[];
  /** Sessions the user has not looked at; no reminder about them. */
  unseen: ReadonlySet<string>;
  settings: IslandSettings;
  /** False while the Agent page is off or game mode keeps the island quiet. */
  enabled: boolean;
  mode: IslandMode;
  page: IslandPage;
  openIslandPage: (page: IslandPage) => void;
  collapseIsland: () => void;
};

/**
 * What the island does when a Cindy session changes: sounds, the Agent page opening by itself,
 * and a session that keeps waiting on the user popping up once more.
 */
export function useIslandAgentAlerts({ sessions, unseen, settings, enabled, mode, page, openIslandPage, collapseIsland }: IslandAgentAlertsOptions) {
  const isOpenOnAgents = mode === "expanded" && page === "agents";
  // Whether an alert opened the Agent page and the user has not taken it over since.
  const autoOpenedRef = useRef(false);
  const onOpen = () => {
    autoOpenedRef.current = true;
    openIslandPage("agents");
  };

  // A press anywhere but on a session row is the user taking the island over.
  useEffect(() => {
    const takeOver = (event: PointerEvent) => {
      if (!(event.target instanceof Element && event.target.closest(".agent-sessions__list"))) autoOpenedRef.current = false;
    };
    window.addEventListener("pointerdown", takeOver, true);
    return () => window.removeEventListener("pointerdown", takeOver, true);
  }, []);

  // Leaving the Agent page or collapsing ends it too.
  const wasOpenOnAgentsRef = useRef(isOpenOnAgents);
  useEffect(() => {
    if (wasOpenOnAgentsRef.current && !isOpenOnAgents) autoOpenedRef.current = false;
    wasOpenOnAgentsRef.current = isOpenOnAgents;
  }, [isOpenOnAgents]);

  useAgentSounds(sessions, enabled, unseen);
  // Not muted by game mode: the user is not looking at the sessions then either.
  const [soundSettings] = useAgentSoundSettings();
  // A reminder sounds like the wait it is about, if that sound is on.
  const remind = () => {
    if (soundSettings.enabled && soundSettings.events.question) playAgentSound("question", soundSettings);
  };
  const alertedSession = useAgentAlerts({
    sessions,
    kinds: alertKindsOf(settings),
    durationMs: settings.agentAlertSeconds * 1000,
    canAlert: enabled,
    isOpenOnAgents,
    isCollapsed: mode === "collapsed",
    onOpen,
    onClose: collapseIsland,
    remindAfterMs: settings.agentRemindSeconds * 1000,
    onRemind: remind,
    looked: (session) => !unseen.has(sessionKey(session)),
  });
  // A click elsewhere on the screen closes an alert's one-session view, like a popover.
  useOutsideClick(alertedSession !== null && isOpenOnAgents, collapseIsland);
  return { alertedSession, agentCardHeight: alertedSession?.rowHeight };
}
