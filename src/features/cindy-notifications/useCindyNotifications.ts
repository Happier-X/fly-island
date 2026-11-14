import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { agentWindowInFront, getCindyNotifications, type CindyNotificationSnapshot } from "./api";
import { unseenNotifications, watermarkAfter, type NotificationKinds } from "./seen";
import { loadNotificationWatermark, saveNotificationWatermark } from "./storage";

/** Cindy 的数据库很小、查询都走索引；5 秒看一眼足够，又不至于让岛频繁唤醒。 */
const POLL_MS = 5_000;

const EMPTY_SNAPSHOT: CindyNotificationSnapshot = { items: [], databasePath: "", checkedAt: 0, error: null };

type CindyNotificationOptions = {
  /** 设置里的总开关；关闭时不再读库，也不显示角标。 */
  enabled: boolean;
  /** 四类事件各自的开关。 */
  kinds: NotificationKinds;
};

/**
 * Cindy 的通知：读到的四类事件里，比本地水位新、又开着的那几类才算未处理。
 *
 * Cindy 已在前台时，新事件直接算看过：用户正看着它，岛不需要再举一次手。
 * 点击角标（`markSeen`）把水位推到最近一次读取的时间。
 */
export function useCindyNotifications({ enabled, kinds }: CindyNotificationOptions) {
  const [snapshot, setSnapshot] = useState<CindyNotificationSnapshot>(EMPTY_SNAPSHOT);
  const [watermark, setWatermark] = useState(loadNotificationWatermark);
  const watermarkRef = useRef(watermark);

  useEffect(() => {
    watermarkRef.current = watermark;
  }, [watermark]);

  // 水位只在变化时写一次，不跟着每次轮询重写。
  useEffect(() => {
    saveNotificationWatermark(watermark);
  }, [watermark]);

  useEffect(() => {
    if (!enabled) return;
    let cancelled = false;
    // Per effect, not a ref: a remount (StrictMode, or the switch toggled) must read again.
    let reading = false;

    const read = async () => {
      if (reading) return;
      reading = true;
      try {
        const next = await getCindyNotifications();
        if (cancelled) return;
        const hasNew = next.items.some((item) => item.at > watermarkRef.current);
        // Cindy 就在前台：先把水位推过去，再交付快照，于是角标和提示音都不会为它打扰。
        if (hasNew && (await agentWindowInFront().catch(() => false)) && !cancelled) {
          setWatermark((current) => watermarkAfter(current, next.checkedAt));
        }
        if (!cancelled) setSnapshot(next);
      } catch (error) {
        if (!cancelled) console.error("Failed to read Cindy notifications", error);
      } finally {
        reading = false;
      }
    };

    void read();
    const timer = window.setInterval(() => void read(), POLL_MS);
    return () => {
      cancelled = true;
      window.clearInterval(timer);
    };
  }, [enabled]);

  const unseen = useMemo(
    () => (enabled ? unseenNotifications(snapshot.items, watermark, kinds) : []),
    [enabled, kinds, snapshot.items, watermark],
  );

  const markSeen = useCallback(() => {
    setWatermark((current) => watermarkAfter(current, snapshot.checkedAt));
  }, [snapshot.checkedAt]);

  return { snapshot, unseen, markSeen };
}
