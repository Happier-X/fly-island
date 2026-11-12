import type { AgentSessionSummary } from "../api";

/** The agent moments that can make a sound, in the order the Sounds page lists them. */
export const AGENT_SOUND_EVENTS = ["start", "complete", "error", "question", "ack", "idle"] as const;

export type AgentSoundEvent = (typeof AGENT_SOUND_EVENTS)[number];

export function isAgentSoundEvent(value: unknown): value is AgentSoundEvent {
  return typeof value === "string" && (AGENT_SOUND_EVENTS as readonly string[]).includes(value);
}

type Phase = AgentSessionSummary["phase"];

/**
 * The sound a session's change from `before` to `after` calls for. A session seen for the
 * first time only sounds when it starts running (a new session) or already waits for the user;
 * one that shows up finished is an old session coming into the listed time window. Whether the
 * sound then plays is decided by `announce.ts`: a finished turn or a wait has to last.
 */
export function soundFor(before: Phase | undefined, after: Phase): AgentSoundEvent | null {
  // Cindy stops a turn on the user for a question or a plan to confirm: both are questions.
  if (after === "waiting" && before !== "waiting") return "question";
  if (before === undefined) return after === "running" ? "start" : null;
  if (after === "failed" && before !== "failed") return "error";
  if (after === "completed" && (before === "running" || before === "waiting" || before === "stale")) return "complete";
  // A finished session running again means the user just sent it a new prompt.
  if (after === "running" && (before === "completed" || before === "failed")) return "ack";
  return null;
}
