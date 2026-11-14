import { isTauri } from "@tauri-apps/api/core";
import { getIslandTopAttachment, listenIslandTopAttachment } from "./api.ts";
import "./IslandTopEdge.css";

// Keep the native attachment signal for positioning diagnostics. Attachment no
// longer changes the silhouette: the capsule always keeps its four round corners.
function applyTopEdge(attached: boolean) {
  document.documentElement.dataset.islandTopEdge = attached ? "attached" : "detached";
}

/** Starts mirroring the native top-edge attachment for the main window's lifetime. */
export function trackIslandTopEdge() {
  if (!isTauri()) return;

  void listenIslandTopAttachment(applyTopEdge)
    .then(() => getIslandTopAttachment())
    .then(applyTopEdge)
    .catch((error) => {
      console.error("Failed to track the island top edge", error);
    });
}
