import { useEffect, useState } from "react";
import { getLauncherIcon } from "./api.ts";
import type { LauncherResult } from "./types.ts";

/** Native icons by `kind:target`; they do not change while the app runs. */
const cache = new Map<string, string | null>();

/**
 * Explorer icons for the results on screen, requested once per target and remembered.
 * Lists refresh as each icon arrives; a missing icon leaves the row's generic one.
 */
export function useLauncherIcons(results: readonly LauncherResult[]) {
  const [icons, setIcons] = useState<Record<string, string>>({});

  useEffect(() => {
    let didCancel = false;
    const found: Record<string, string> = {};
    const pending: Promise<void>[] = [];
    for (const result of results) {
      if (result.kind === "web") continue;
      const key = `${result.kind}:${result.target}`;
      const cached = cache.get(key);
      if (cached !== undefined) {
        if (cached) found[result.id] = cached;
        continue;
      }
      pending.push(
        getLauncherIcon(result.target, result.kind)
          .then((url) => {
            cache.set(key, url);
            if (url) found[result.id] = url;
          })
          .catch((failure) => {
            console.error("Failed to read a launcher icon", failure);
            cache.set(key, null);
          }),
      );
    }
    setIcons({ ...found });
    if (pending.length > 0) {
      void Promise.all(pending).then(() => {
        if (!didCancel) setIcons({ ...found });
      });
    }
    return () => {
      didCancel = true;
    };
  }, [results]);

  return icons;
}
