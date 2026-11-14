import { useCallback, useEffect, useState } from "react";
import { getCindyNotifications, type CindyNotificationSnapshot } from "./api";

const REFRESH_MS = 2_000;

/**
 * Cindy 的数据库，按设置页「数据来源」需要的样子读：路径、读取时间、未处理数量或失败原因。
 * 页面打开期间轮询，Cindy 后来才建库也能自己显示出来。
 */
export function useNotificationSource() {
  const [snapshot, setSnapshot] = useState<CindyNotificationSnapshot | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [refreshing, setRefreshing] = useState(false);
  const read = useCallback(async (isCancelled: () => boolean = () => false) => {
    try {
      const next = await getCindyNotifications();
      if (!isCancelled()) {
        setSnapshot(next);
        setError(null);
      }
    } catch (reason) {
      if (!isCancelled()) setError(reason instanceof Error ? reason.message : String(reason));
    }
  }, []);

  useEffect(() => {
    let cancelled = false;
    // Per effect, not a ref: a remount (StrictMode, or the page reopened) must read again.
    let reading = false;
    const readOnce = async () => {
      if (reading) return;
      reading = true;
      try {
        await read(() => cancelled);
      } finally {
        reading = false;
      }
    };
    void readOnce();
    const timer = window.setInterval(() => void readOnce(), REFRESH_MS);
    return () => {
      cancelled = true;
      window.clearInterval(timer);
    };
  }, [read]);

  const refresh = useCallback(async () => {
    setRefreshing(true);
    try {
      await read();
    } finally {
      setRefreshing(false);
    }
  }, [read]);

  return { snapshot, error, refreshing, refresh };
}
