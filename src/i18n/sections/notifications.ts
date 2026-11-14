export type NotificationsText = {
  /** The collapsed island's badge, read out to screen readers and shown as its tip. */
  badge: (count: number) => string;
  open: string;
  /** Cindy is not running, so the click has nowhere to go. */
  notFound: string;
};

export const zhCN: NotificationsText = {
  badge: (count) => `Cindy 有 ${count} 条通知待处理`,
  open: "打开 Cindy",
  notFound: "没找到 Cindy 的窗口",
};

export const en: NotificationsText = {
  badge: (count) => (count === 1 ? "1 Cindy notification to handle" : `${count} Cindy notifications to handle`),
  open: "Open Cindy",
  notFound: "No Cindy window found",
};
