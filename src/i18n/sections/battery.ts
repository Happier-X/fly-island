export type BatteryText = {
  title: string;
  count: (count: number) => string;
  refresh: string;
  loading: string;
  emptyTitle: string;
  emptyHint: string;
  level: (level: number) => string;
  kinds: {
    mouse: string;
    keyboard: string;
    headset: string;
    receiver: string;
    other: string;
  };
};

export const zhCN: BatteryText = {
  title: "外设电量",
  count: (count) => (count === 1 ? "1 个设备" : `${count} 个设备`),
  refresh: "重新读取外设电量",
  loading: "正在读取…",
  emptyTitle: "没有读到外设电量",
  emptyHint: "蓝牙或无线键鼠、耳机连接并上报电量后，会显示在这里。读不到的设备不会显示。",
  level: (level) => `电量 ${level}%`,
  kinds: {
    mouse: "鼠标",
    keyboard: "键盘",
    headset: "耳机",
    receiver: "无线接收器",
    other: "其他外设",
  },
};

export const en: BatteryText = {
  title: "Device battery",
  count: (count) => (count === 1 ? "1 device" : `${count} devices`),
  refresh: "Refresh device battery",
  loading: "Reading…",
  emptyTitle: "No peripheral battery found",
  emptyHint: "Wireless mice, keyboards and headsets appear here once they connect and report a battery level. Devices that cannot be read are left out.",
  level: (level) => `Battery ${level}%`,
  kinds: {
    mouse: "Mouse",
    keyboard: "Keyboard",
    headset: "Headset",
    receiver: "Wireless receiver",
    other: "Other device",
  },
};
