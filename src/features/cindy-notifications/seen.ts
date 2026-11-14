import type { CindyNotification, CindyNotificationKind } from "./api.ts";

/** 四类事件各自的开关，由设置页决定。 */
export type NotificationKinds = Record<CindyNotificationKind, boolean>;

/**
 * 通知记录只保留本地水位（Unix 毫秒）：读到的、比水位新的事件才算未处理。
 *
 * Cindy 只给定时任务记 `read_at`，别的类型没有已读字段。点开角标或 Cindy 已在前台时，
 * 把水位推到这次读取的时间；之后新发生的事件再次越过水位。这样不用每轮写整张表，
 * 也不会因为一次提醒没点而把旧事件一直算成新的。
 */
export function unseenNotifications(
  items: readonly CindyNotification[],
  watermark: number,
  kinds: NotificationKinds,
): CindyNotification[] {
  return items.filter((item) => item.at > watermark && kinds[item.kind]);
}

/** 看过 `checkedAt` 这次读取之后的水位；水位只增不减。 */
export function watermarkAfter(watermark: number, checkedAt: number) {
  return Math.max(watermark, checkedAt);
}

/** 保存的水位读回；没有或不可用时返回 null，由调用方用启动时间兜底。 */
export function normalizeWatermark(value: unknown): number | null {
  const watermark = value && typeof value === "object" ? (value as Record<string, unknown>).watermark : undefined;
  return typeof watermark === "number" && Number.isFinite(watermark) && watermark > 0 ? watermark : null;
}
