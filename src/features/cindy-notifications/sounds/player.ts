import complete from "../../../assets/sounds/thumb-piano/complete.wav";
import error from "../../../assets/sounds/thumb-piano/error.wav";
import question from "../../../assets/sounds/thumb-piano/question.wav";
import type { CindyNotificationKind } from "../api.ts";
import type { NotificationSoundSettings } from "./storage.ts";

/** The island's own sounds ("thumb piano"), synthesized for fly-island; see assets/sounds/README.md. */
export type NotificationSound = "complete" | "error" | "question";

const SOUNDS: Record<NotificationSound, string> = { complete, error, question };

/** 四类通知各用哪段音效：跑完和定时结果都是「完成」，出错与提问各用各的。 */
export const SOUND_FOR_KIND: Record<CindyNotificationKind, NotificationSound> = {
  done: "complete",
  scheduled: "complete",
  question: "question",
  error: "error",
};

/** 与旧版音量默认值一致；设置页只保留开关，不给音量。 */
const VOLUME = 0.7;

/** Plays the sound for `kind` when the settings allow it. Whether the island should stay quiet is the caller's to check. */
export function playNotificationSound(kind: CindyNotificationKind, settings: NotificationSoundSettings) {
  if (!settings.enabled) return;
  const audio = new Audio(SOUNDS[SOUND_FOR_KIND[kind]]);
  audio.volume = VOLUME;
  void audio.play().catch((reason: unknown) => console.error(`Failed to play the ${kind} notification sound`, reason));
}
