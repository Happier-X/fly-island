import { useEffect, useLayoutEffect, useRef } from "react";
import type { CindyNotification, CindyNotificationKind } from "../api.ts";
import { playNotificationSound } from "./player.ts";
import { useSoundSettings } from "./useSoundSettings.ts";

/** Several notifications at once play one sound: the one that matters most. */
const PRIORITY: readonly CindyNotificationKind[] = ["error", "question", "done", "scheduled"];
/** Keys remembered to tell new notifications apart; past this the set starts over. */
const MAX_REMEMBERED = 500;

/**
 * Plays a sound when new notifications appear. The first look only takes note of what is
 * already there: the island starting up is not news. `canPlay` is false while game mode
 * keeps the island quiet (or while the island is not showing notifications at all).
 */
export function useNotificationSounds(items: CindyNotification[], canPlay: boolean) {
  const [settings] = useSoundSettings();
  const remembered = useRef<Set<string> | null>(null);
  const latest = useRef({ settings, canPlay });
  useLayoutEffect(() => {
    latest.current = { settings, canPlay };
  });

  useEffect(() => {
    const known = remembered.current;
    remembered.current ??= new Set<string>();
    if (remembered.current.size > MAX_REMEMBERED) remembered.current = new Set<string>();
    const fresh = known === null ? [] : items.filter((item) => !known.has(item.key));
    for (const item of items) remembered.current.add(item.key);
    if (fresh.length === 0 || !latest.current.canPlay) return;
    const kind = PRIORITY.find((candidate) => fresh.some((item) => item.kind === candidate));
    if (kind) playNotificationSound(kind, latest.current.settings);
  }, [items]);
}
