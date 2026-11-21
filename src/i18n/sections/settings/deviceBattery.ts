export type SettingsDeviceBatteryText = {
  title: string;
  refresh: string;
  refreshHint: string;
  seconds: (seconds: number) => string;
};

export const zhCN: SettingsDeviceBatteryText = {
  title: "读取",
  refresh: "刷新间隔",
  refreshHint: "打开外设电量页时按这个间隔重新读取；读不到的设备不会显示。",
  seconds: (seconds) => `${seconds} 秒`,
};

export const en: SettingsDeviceBatteryText = {
  title: "Reading",
  refresh: "Refresh interval",
  refreshHint: "While the device battery page is open, it reads again at this interval. Unreadable devices are left out.",
  seconds: (seconds) => `${seconds} seconds`,
};
