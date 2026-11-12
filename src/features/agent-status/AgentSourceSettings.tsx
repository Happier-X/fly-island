import { CircleAlert, CircleCheck, CircleSlash } from "lucide-react";
import { SettingsGroup } from "../../components/settings/SettingsGroup";
import { SettingsLinkButton } from "../../components/settings/SettingsLinkButton";
import { SettingsRow } from "../../components/settings/SettingsRow";
import { useTranslation } from "../../i18n";
import { useAgentSource, type AgentSourceState } from "./useAgentSource";
import "./AgentSourceSettings.css";

const TONES = {
  reading: { icon: CircleCheck, className: "agent-source__state" },
  missing: { icon: CircleSlash, className: "agent-source__state agent-source__state--quiet" },
  failed: { icon: CircleAlert, className: "agent-source__state agent-source__state--attention" },
} as const;

/**
 * Settings → AI Agent → Data source: where the sessions come from — Cindy's own database — and
 * whether it can be read right now. Nothing to install: the island only reads that file.
 */
export function AgentSourceSettings() {
  const text = useTranslation().settings.agent.source;
  const { state, reading, refresh } = useAgentSource();
  const { icon: Icon, className } = TONES[state.kind];

  return (
    <SettingsGroup title={text.title} footer={text.hint}>
      <SettingsRow
        title={
          <span className={className}>
            <Icon className="agent-source__icon" size={13} strokeWidth={2.4} aria-hidden="true" />
            {describe(state, text)}
          </span>
        }
        description={state.kind === "failed" ? state.error : state.kind === "missing" ? undefined : state.path}
        control={
          <SettingsLinkButton disabled={reading} onClick={refresh}>
            {text.refresh}
          </SettingsLinkButton>
        }
      />
    </SettingsGroup>
  );
}

type Text = ReturnType<typeof useTranslation>["settings"]["agent"]["source"];

function describe(state: AgentSourceState, text: Text) {
  if (state.kind === "reading") return text.reading(state.count);
  if (state.kind === "missing") return text.missing;
  return text.failed;
}
