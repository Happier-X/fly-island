import { Battery, BatteryCharging, CircleHelp, Headphones, Keyboard, Mouse, Radio, RefreshCw, type LucideIcon } from "lucide-react";
import { useTranslation } from "../../i18n";
import type { PeripheralBatteryDevice, PeripheralDeviceKind } from "./api";
import "./PeripheralBatteryPanel.css";

const KIND_ICONS: Record<PeripheralDeviceKind, LucideIcon> = {
  mouse: Mouse,
  keyboard: Keyboard,
  headset: Headphones,
  receiver: Radio,
  other: CircleHelp,
};

function levelTone(level: number) {
  if (level <= 15) return "low";
  if (level <= 40) return "medium";
  return "high";
}

/** The expanded island's peripheral battery page: readable devices only, never the laptop. */
export function PeripheralBatteryPanel({
  devices,
  isLoading,
  refresh,
}: {
  devices: PeripheralBatteryDevice[];
  isLoading: boolean;
  refresh: () => void;
}) {
  const t = useTranslation().battery;
  return (
    <section className="battery-panel" aria-label={t.title}>
      <header className="battery-panel__header">
        <div className="battery-panel__heading">
          <h2 className="battery-panel__title">{t.title}</h2>
          <span className="battery-panel__count">
            {isLoading && devices.length === 0 ? t.loading : t.count(devices.length)}
          </span>
        </div>
        <button
          className={`battery-refresh${isLoading ? " battery-refresh--busy" : ""}`}
          type="button"
          aria-label={t.refresh}
          title={t.refresh}
          onClick={refresh}
        >
          <RefreshCw size={15} strokeWidth={1.6} absoluteStrokeWidth aria-hidden="true" />
        </button>
      </header>

      {devices.length === 0 ? (
        <div className="battery-empty">
          <Battery size={22} strokeWidth={1.4} absoluteStrokeWidth aria-hidden="true" />
          <strong>{isLoading ? t.loading : t.emptyTitle}</strong>
          <span>{t.emptyHint}</span>
        </div>
      ) : (
        <div className="battery-list" role="list">
          {devices.map((device) => {
            const Icon = KIND_ICONS[device.kind];
            return (
              <div className="battery-device" role="listitem" key={device.id}>
                <span className="battery-device__icon" aria-hidden="true">
                  <Icon size={19} strokeWidth={1.5} absoluteStrokeWidth />
                </span>
                <span className="battery-device__text">
                  <span className="battery-device__name" title={device.name}>{device.name}</span>
                  <span className="battery-device__kind">{t.kinds[device.kind]}</span>
                </span>
                <span className="battery-device__level">
                  <span className={`battery-device__value battery-device__value--${levelTone(device.level)}`}>
                    {device.charging && <BatteryCharging size={14} strokeWidth={1.6} absoluteStrokeWidth aria-hidden="true" />}
                    {device.level}%
                  </span>
                  <span
                    className={`battery-device__bar battery-device__bar--${levelTone(device.level)}`}
                    role="img"
                    aria-label={t.level(device.level)}
                  >
                    <span style={{ width: `${device.level}%` }} />
                  </span>
                </span>
              </div>
            );
          })}
        </div>
      )}
    </section>
  );
}
