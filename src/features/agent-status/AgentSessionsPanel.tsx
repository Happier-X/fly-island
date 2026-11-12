import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { Archive } from "lucide-react";
import { ArchivedNotice } from "./ArchivedNotice";
import { useArchiveMotion } from "./useArchiveMotion";
import { useTranslation } from "../../i18n";
import type { AgentArchiveMotion, AgentCardEffects, AgentSessionGroup, AgentSessionSort, AgentSessionView } from "../appearance/types";
import { AgentSessionCard } from "./AgentSessionCard";
import { AgentProviderIcon } from "./AgentProviderIcon";
import { focusAgentWindow, type AgentSessionSummary } from "./api";
import { arrangeSessions, isActivePhase, minutesAgo, modelLabel, sessionProject } from "./sessionList";
import { SessionBody } from "./SessionBody";
import { SessionElapsed } from "./SessionElapsed";
import { AGENT_PROVIDER_LABELS, getAgentPhaseLabel } from "./status";
import { useScrollFade } from "./useScrollFade";
import { sessionKey } from "./unseen";
import { sessionLight } from "./sessionLight";
import type { UnseenSessions } from "./useUnseenSessions";
import type { SessionRemoval } from "./useHiddenSessions";
import type { AlertedSession } from "./useAgentAlerts";
import "./AgentSessionsPanel.css";

const NOT_FOUND_NOTICE_MS = 3000;
const AGE_REFRESH_MS = 30_000;

/**
 * The island's Agent page: recent Cindy sessions; clicking one brings Cindy forward.
 * Sessions still working get a two-line row with the latest reply; finished ones fold to one line,
 * except on an alert's card, where a finished session shows up to three lines of it.
 */
export function AgentSessionsPanel({
  sessions: listed,
  view = "list",
  sort = "status",
  group = "none",
  archiveMotion = "slide",
  cardEffects = "off",
  showElapsed = true,
  showModel = true,
  unseen,
  removal,
  reveal,
}: {
  sessions: AgentSessionSummary[];
  /** Flat rows (the default), cards framed in their light, or one-line rows (AgentSessionCard). */
  view?: AgentSessionView;
  /** The order and the headings (sessionList.ts arrangeSessions). */
  sort?: AgentSessionSort;
  group?: AgentSessionGroup;
  /** How an archived session leaves: sliding out, or dissolving into dust (useArchiveMotion). */
  archiveMotion?: AgentArchiveMotion;
  /** Light effects on cards and compact rows (AgentCardEffects.css); the list layout has none. */
  cardEffects?: AgentCardEffects;
  showElapsed?: boolean;
  showModel?: boolean;
  /** Finished sessions not looked at yet; they stand out until clicked. */
  unseen?: UnseenSessions;
  /** Removing a session from the page (an x at a row's right end), and undoing it. */
  removal?: SessionRemoval;
  /** The session an alert opened the page for; the page then shows it alone, like a card. */
  reveal?: AlertedSession | null;
}) {
  const t = useTranslation();
  // An alert's session alone, kept current from the list; the list may leave it out (only
  // running sessions listed), and then it shows as the alert saw it.
  const sessions = reveal ? [listed.find((session) => sessionKey(session) === reveal.key) ?? reveal.session] : listed;
  const [now, setNow] = useState(() => Date.now());
  const [notFoundId, setNotFoundId] = useState<string | null>(null);
  // Fade out an end of the list that has more sessions past it, rather than cutting a row in half.
  const list = useScrollFade<HTMLUListElement>(sessions);
  const motion = useArchiveMotion(list.element, removal, archiveMotion);
  const cardRef = useRef<HTMLLIElement>(null);
  const measure = reveal?.measure;
  const cardKey = reveal?.key;

  // The island fits the card: report its row's height now and whenever it changes (a new reply).
  useLayoutEffect(() => {
    const row = cardRef.current;
    if (!measure || !row) return;
    const report = () => measure(row.offsetHeight);
    report();
    const observer = new ResizeObserver(report);
    observer.observe(row);
    return () => observer.disconnect();
  }, [measure, cardKey]);

  useEffect(() => {
    const timer = window.setInterval(() => setNow(Date.now()), AGE_REFRESH_MS);
    return () => window.clearInterval(timer);
  }, []);

  useEffect(() => {
    if (!notFoundId) return;
    const timer = window.setTimeout(() => setNotFoundId(null), NOT_FOUND_NOTICE_MS);
    return () => window.clearTimeout(timer);
  }, [notFoundId]);

  const focus = (session: AgentSessionSummary) => {
    setNotFoundId(null);
    unseen?.markSeen(session);
    // Every Cindy session runs in Cindy's own window: one click brings it forward.
    void focusAgentWindow()
      .then((found) => {
        if (!found) setNotFoundId(session.sessionId);
      })
      .catch((error) => {
        console.error("Failed to focus Cindy", error);
        setNotFoundId(session.sessionId);
      });
  };

  const removedNotice = removal && !reveal && <ArchivedNotice removed={removal.removed} onUndo={motion.undo} />;

  if (sessions.length === 0) {
    return (
      <section className="agent-sessions" aria-label={t.agent.sessions.title}>
        <div className="agent-sessions__empty">
          <strong>{t.agent.sessions.empty}</strong>
          <span>{t.agent.sessions.emptyHint}</span>
        </div>
        {removedNotice}
      </section>
    );
  }

  return (
    <section className="agent-sessions" aria-label={t.agent.sessions.title}>
      <ul
        ref={list.ref}
        className={`agent-sessions__list${view === "list" ? "" : ` agent-sessions__list--${view}`}${removal && !reveal ? " agent-sessions__list--archivable" : ""}`}
        data-fade-top={list.fade.top || undefined}
        data-fade-bottom={list.fade.bottom || undefined}
        onScroll={list.onScroll}
      >
        {arrangeSessions(sessions, sort, group).flatMap(({ key: groupKey, project, sessions: members }, _, groups) => [
          // By project, headings only once there is more than one; never on an alert's card.
          !reveal && group === "project" && groups.length > 1 ? (
            <li key={`group:${groupKey}`} className="agent-sessions__computer" aria-hidden="true">
              {project ?? t.agent.sessions.noProject}
            </li>
          ) : null,
          ...members.map((session) => {
            const project = sessionProject(session.cwd);
            const title = session.title ?? project ?? AGENT_PROVIDER_LABELS[session.provider];
            const model = modelLabel(session.model, session.effort);
            const mode = session.permissionMode && session.permissionMode !== "default" ? t.agent.sessions.permissionModes[session.permissionMode] : null;
            const key = sessionKey(session);
            const notFound = notFoundId === session.sessionId
              ? isActivePhase(session.phase) ? t.agent.sessions.notFound : t.agent.sessions.notFoundEnded
              : null;
            // Right of the provider icon, grey, shown while the cursor is on the row: some sessions
            // never report that they ended. A session that writes again comes back by itself.
            const archive = removal && !reveal && (
              <button className="agent-session__archive" type="button" aria-label={t.agent.sessions.remove(title)} title={t.agent.sessions.remove(title)} onClick={() => motion.archive(session)}>
                <Archive size={14} strokeWidth={2} aria-hidden="true" />
              </button>
            );
            if (view !== "list") {
              return (
                <li key={`${session.provider}:${session.sessionId}`} ref={reveal ? cardRef : undefined} data-session-key={key}>
                  <AgentSessionCard
                    session={session}
                    now={now}
                    compact={view === "compact"}
                    showElapsed={showElapsed}
                    model={showModel ? model : null}
                    unseen={Boolean(unseen?.keys.has(key))}
                    looked={false}
                    notFound={notFound}
                    cardReply={reveal && !isActivePhase(session.phase) ? session.lastMessage : null}
                    effects={cardEffects}
                    onOpen={() => focus(session)}
                  />
                  {archive}
                </li>
              );
            }
            const minutes = minutesAgo(session.updatedAt, now);
            // A working session with nothing to say yet (a turn just started) has no second line.
            const active = isActivePhase(session.phase);
            // On an alert's card a finished session shows its last reply (its summary).
            const cardReply = reveal && !active ? session.lastMessage : null;
            const full = Boolean(cardReply) || (active && Boolean(session.lastMessage));
            const light = sessionLight(session);
            const phaseLabel = getAgentPhaseLabel(session.phase, t);
            // Running and done are told apart by the dot; the other states need words.
            const showPhase = session.phase !== "running" && session.phase !== "completed";
            const elapsed = showElapsed && session.turnStartedAt !== null;
            // A running session's age is always about now; its running time says more.
            const showAge = session.phase !== "running" || !elapsed;
            const status = [
              showPhase && phaseLabel,
              elapsed && <SessionElapsed key="elapsed" session={session} />,
              showAge && (minutes === 0 ? t.agent.sessions.justNow : t.agent.sessions.minutesAgo(minutes)),
            ]
              .filter(Boolean)
              .flatMap((part, index) => (index === 0 ? [part] : [" · ", part]));
            return (
              <li key={`${session.provider}:${session.sessionId}`} ref={reveal ? cardRef : undefined} data-session-key={sessionKey(session)}>
                <button
                  className={`agent-session agent-session--${session.phase} agent-session--light-${light} agent-session--${full ? "full" : "compact"}${unseen?.keys.has(key) ? " agent-session--unseen" : ""}`}
                  type="button"
                  onClick={() => focus(session)}
                >
                  <span className="agent-session__dot" aria-hidden="true" />
                  <span className="agent-session__main">
                    <span className="agent-session__heading">
                      <span className="agent-session__title">
                        {project && project !== title ? <span className="agent-session__project">{project} · </span> : null}
                        {title}
                      </span>
                      {active && showModel && model && <span className="agent-session__model">{model}</span>}
                      {active && mode && (
                        <span className={`agent-session__mode agent-session__mode--${session.permissionMode}`}>
                          {mode}
                        </span>
                      )}
                      {notFound ? (
                        <span className="agent-session__not-found" role="status">{notFound}</span>
                      ) : (
                        <span className="agent-session__meta">{status}</span>
                      )}
                      {/* Last on the line, so the provider icon lines up however long the status is. */}
                      <AgentProviderIcon provider={session.provider} size={14} />
                    </span>
                    {cardReply ? (
                      <span className="agent-session__message agent-session__message--card">{cardReply}</span>
                    ) : (
                      full && <SessionBody session={session} />
                    )}
                  </span>
                </button>
                {archive}
              </li>
            );
          }),
        ])}
      </ul>
      {removedNotice}
    </section>
  );
}
