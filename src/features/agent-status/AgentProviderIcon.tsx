import cindyIcon from "../../assets/brands/cindy.png";
import type { AgentProvider } from "./api";
import "./AgentProviderIcon.css";

const PROVIDER_ICONS: Record<AgentProvider, string> = {
  cindy: cindyIcon,
};

export function AgentProviderIcon({ provider, size = 16 }: { provider: AgentProvider; size?: number }) {
  return (
    <img
      className={`agent-provider-icon agent-provider-icon--${provider}`}
      src={PROVIDER_ICONS[provider]}
      alt=""
      aria-hidden="true"
      width={size}
      height={size}
      draggable={false}
    />
  );
}
