import { invoke } from "@tauri-apps/api/core";

// 与 src-tauri/src/cindy_notifications/cindy.rs 中的类型对应。
// Cindy 是本机唯一会跑 agent 的程序，通知只来自它自己的本地数据库。

/** 四类通知：做完、等你回答、定时与后台任务、出错。 */
export type CindyNotificationKind = "done" | "question" | "scheduled" | "error";

export const CINDY_NOTIFICATION_KINDS: readonly CindyNotificationKind[] = ["done", "question", "scheduled", "error"];

/** Mirrors CindyNotification in cindy.rs. */
export type CindyNotification = {
  kind: CindyNotificationKind;
  /** 稳定指纹，同一事件每次读取都相同。 */
  key: string;
  /** 事件时间，Unix 毫秒。 */
  at: number;
};

/** Mirrors CindyNotificationSnapshot in cindy.rs. */
export type CindyNotificationSnapshot = {
  items: CindyNotification[];
  /** Cindy 的数据库路径；空字符串表示没找到（Cindy 没装或还没建库）。 */
  databasePath: string;
  /** 这次读取的时间，Unix 毫秒。 */
  checkedAt: number;
  /** 数据库读不出来时的原因；能读时为 null。 */
  error: string | null;
};

/**
 * Brings Cindy to the front. Every session runs in Cindy's own window, so a notification is a
 * click away from Cindy. False when it is not running or has no window.
 */
export function focusAgentWindow() {
  return invoke<boolean>("focus_agent_window");
}

/** Whether Cindy is the window in front: the island then leaves new events alone. */
export function agentWindowInFront() {
  return invoke<boolean>("agent_window_in_front");
}

/** 读一次 Cindy 的数据库：四类未处理事件、数据库路径、读取时间和失败原因。 */
export function getCindyNotifications() {
  return invoke<CindyNotificationSnapshot>("get_cindy_notifications");
}
