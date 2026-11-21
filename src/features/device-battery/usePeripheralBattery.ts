import { useCallback, useEffect, useRef, useState } from "react";
import { getPeripheralBatteries, type PeripheralBatteryDevice } from "./api";

/**
 * Reads peripheral batteries while `enabled` (the battery page is open). A failed read
 * leaves the page empty instead of surfacing an error: an unreachable device or a
 * missing permission is normal, not something to interrupt the user with.
 */
export function usePeripheralBattery(enabled: boolean, refreshSeconds: number) {
  const [devices, setDevices] = useState<PeripheralBatteryDevice[]>([]);
  const [isLoading, setIsLoading] = useState(enabled);
  const requestRef = useRef(0);
  const mountedRef = useRef(true);

  const load = useCallback(async () => {
    const request = ++requestRef.current;
    setIsLoading(true);
    try {
      const next = await getPeripheralBatteries();
      if (mountedRef.current && request === requestRef.current) setDevices(next);
    } catch {
      if (mountedRef.current && request === requestRef.current) setDevices([]);
    } finally {
      if (mountedRef.current && request === requestRef.current) setIsLoading(false);
    }
  }, []);

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
    };
  }, []);

  useEffect(() => {
    if (!enabled) {
      setDevices([]);
      return;
    }
    void load();
    const timer = window.setInterval(() => void load(), Math.max(1, refreshSeconds) * 1000);
    return () => window.clearInterval(timer);
  }, [enabled, load, refreshSeconds]);

  const refresh = useCallback(() => {
    if (enabled) void load();
  }, [enabled, load]);

  return { devices, isLoading, refresh };
}
