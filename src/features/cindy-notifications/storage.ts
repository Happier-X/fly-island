import { readJson, writeJson } from "../../lib/storage.ts";
import { normalizeWatermark } from "./seen.ts";

/**
 * 通知的水位。旧版本在 Agent 页记的按会话已读表（`focusd-island-agent-seen-turns`）不再读写，
 * 旧值留在原处不删。
 */
export const CINDY_NOTIFICATIONS_SEEN_KEY = "focusd-island-cindy-notifications-seen";

/**
 * 上次处理到的位置。第一次启动（或存储不可用）从 `now` 开始：装好之前 Cindy 里发生的事
 * 不提醒，和旧版「岛屿启动前完成的会话算已看过」一致。
 */
export function loadNotificationWatermark(now = Date.now()) {
  return normalizeWatermark(readJson(CINDY_NOTIFICATIONS_SEEN_KEY)) ?? now;
}

export function saveNotificationWatermark(watermark: number) {
  writeJson(CINDY_NOTIFICATIONS_SEEN_KEY, { watermark });
}
