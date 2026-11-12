import { useTranslation } from "../../i18n";
import type { AgentCardEffects } from "../appearance/types";
import { AgentProviderIcon } from "./AgentProviderIcon";
import type { AgentSessionSummary } from "./api";
import { formatTokens, isActivePhase, minutesAgo, sessionProject } from "./sessionList";
import { SessionElapsed } from "./SessionElapsed";
import { sessionLight, type SessionLight } from "./sessionLight";
import { AGENT_PROVIDER_LABELS, getAgentPhaseLabel } from "./status";
import "./AgentSessionCard.css";
import "./AgentCardEffects.css";

/** Lights whose frame and state are drawn in their colour: working, or waiting on the user. */
const LIVELY: ReadonlySet<SessionLight> = new Set(["needs", "working", "failed"]);

/**
 * One session on the Agent page in the card and compact views (the list view stays in
 * AgentSessionsPanel). A card is framed in its light's colour: the title with mode, model and
 * time on the first line; the state, what it is doing and the context size on the second.
 * A compact row puts a bar in that colour, the title, state and time on one line.
 * Only the button: the row, its archive button and the click stay with the panel.
 */
export function AgentSessionCard({
  session,
  now,
  compact,
  showElapsed,
  model,
  unseen,
  looked,
  notFound,
  cardReply,
  effects = "off",
  onOpen,
}: {
  session: AgentSessionSummary;
  now: number;
  compact: boolean;
  showElapsed: boolean;
  /** Model as the page shows it; null when hidden or unknown. */
  model: string | null;
  /** Finished and not looked at yet: stands out until clicked. */
  unseen: boolean;
  /** Waiting on the user or failed, and already looked at: the words stay, the alarm goes. */
  looked: boolean;
  /** What a click found no window for, said in place of the time for a moment. */
  notFound: string | null;
  /** On an alert's card, a finished session's last reply, below the lines. */
  cardReply: string | null;
  /** The sparkle band with a glowing frame, or a gradient wash (AgentCardEffects.css). */
  effects?: AgentCardEffects;
  onOpen: () => void;
}) {
  const t = useTranslation();
  const text = t.agent.sessions;
  const project = sessionProject(session.cwd);
  const title = session.title ?? project ?? AGENT_PROVIDER_LABELS[session.provider];
  const active = isActivePhase(session.phase);
  const light = sessionLight(session);
  const mode = active && session.permissionMode && session.permissionMode !== "default" ? text.permissionModes[session.permissionMode] : null;
  const minutes = minutesAgo(session.updatedAt, now);
  const age = minutes === 0 ? text.justNow : text.minutesAgo(minutes);
  const time = showElapsed && session.turnStartedAt !== null && session.phase === "running" ? <SessionElapsed session={session} /> : age;
  const state = getAgentPhaseLabel(session.phase, t);

  const doing = cardReply ? null : session.lastMessage ? <span key="doing" className="agent-card__doing">{session.lastMessage}</span> : null;
  const status = [<span key="state" className="agent-card__state">{state}</span>, doing].filter(Boolean);

  const className = [
    `agent-card agent-card--${session.phase} agent-card--light-${light}`,
    LIVELY.has(light) && !looked && "agent-card--lively",
    compact && "agent-card--compact",
    unseen && "agent-card--unseen",
    looked && "agent-card--looked",
    cardReply && "agent-card--reply",
    effects !== "off" && `agent-card--${effects}`,
  ].filter(Boolean).join(" ");

  return (
    <button className={className} type="button" onClick={onOpen}>
      {effects === "sparkle" && <span className="agent-card__band" aria-hidden="true" />}
      <span className="agent-card__heading">
        {compact && <span className="agent-card__bar" aria-hidden="true" />}
        <span className="agent-card__title">
          {project && project !== title ? <span>{project} · </span> : null}
          {title}
        </span>
        {notFound ? (
          <span className="agent-card__not-found" role="status">{notFound}</span>
        ) : (
          <span className="agent-card__meta">
            {compact ? <span className="agent-card__state">{state}</span> : null}
            {!compact && mode && <span className={`agent-card__mode agent-card__mode--${session.permissionMode}`}>{mode}</span>}
            {!compact && model && <span className="agent-card__model">{model}</span>}
            <span className="agent-card__time">{time}</span>
          </span>
        )}
        <AgentProviderIcon provider={session.provider} size={14} />
      </span>
      {!compact && (
        <span className="agent-card__status-line">
          <span className="agent-card__status">{status}</span>
          {session.contextTokens ? <span className="agent-card__tokens">{text.contextTokens(formatTokens(session.contextTokens))}</span> : null}
        </span>
      )}
      {cardReply && <span className="agent-card__reply">{cardReply}</span>}
    </button>
  );
}
