import type { IslandSettings } from "../../features/appearance/types";
import { usePeripheralBattery } from "../../features/device-battery/usePeripheralBattery";
import type { IslandMode, IslandPage } from "./types";

/** Reads peripheral batteries only while their page is open, so a closed page costs nothing. */
export function useIslandBattery(settings: IslandSettings, page: IslandPage, mode: IslandMode) {
  return usePeripheralBattery(
    settings.batteryEnabled && page === "battery" && mode === "expanded",
    settings.batteryRefreshSeconds,
  );
}
