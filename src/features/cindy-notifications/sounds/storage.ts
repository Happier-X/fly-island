import { readJson, writeJson } from "../../../lib/storage.ts";

/**
 * 提示音开关。键名沿用旧版 Agent 音效的设置键（对外契约），旧值里的音量、音效包、
 * 事件与静音时段不再读取，只保留开关。
 */
export const NOTIFICATION_SOUNDS_STORAGE_KEY = "focusd-island-agent-sounds";

export type NotificationSoundSettings = {
  enabled: boolean;
};

export const DEFAULT_NOTIFICATION_SOUND_SETTINGS: NotificationSoundSettings = { enabled: true };

export function normalizeNotificationSoundSettings(value: unknown): NotificationSoundSettings {
  const enabled = value && typeof value === "object" ? (value as Record<string, unknown>).enabled : undefined;
  return { enabled: typeof enabled === "boolean" ? enabled : DEFAULT_NOTIFICATION_SOUND_SETTINGS.enabled };
}

export function loadNotificationSoundSettings() {
  return normalizeNotificationSoundSettings(readJson(NOTIFICATION_SOUNDS_STORAGE_KEY));
}

export function saveNotificationSoundSettings(settings: NotificationSoundSettings) {
  writeJson(NOTIFICATION_SOUNDS_STORAGE_KEY, settings);
}
