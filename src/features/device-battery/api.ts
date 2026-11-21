import { invoke } from "@tauri-apps/api/core";

/** Device kinds read from Windows; keep in step with PeripheralKind in src-tauri/src/device_battery/mod.rs. */
export type PeripheralDeviceKind = "mouse" | "keyboard" | "headset" | "receiver" | "other";

/** A peripheral whose battery Windows could actually read. */
export type PeripheralBatteryDevice = {
  id: string;
  name: string;
  kind: PeripheralDeviceKind;
  /** 0-100. */
  level: number;
  charging: boolean;
};

export function getPeripheralBatteries() {
  return invoke<PeripheralBatteryDevice[]>("get_peripheral_batteries");
}
