//! Persisted app settings: theme, language and clipboard options.
//!
//! On first run it reads the Tauri settings read-only: `focusd-island-settings`
//! and `focusd-island-locale` from the WebView2 LevelDB, and
//! `clipboard-history.json` for clipboard options. Only whitelisted fields are
//! imported; the old files are never modified or deleted.

use std::{fs, path::PathBuf};

use rusty_leveldb::{LdbIterator, Options};
use serde::{Deserialize, Serialize};

fn default_true() -> bool {
    true
}

fn default_max_items() -> usize {
    30
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Preferences {
    pub dark: bool,
    #[serde(default)]
    pub locale: String,
    #[serde(default = "default_true")]
    pub clipboard_enabled: bool,
    #[serde(default = "default_true")]
    pub capture_images: bool,
    #[serde(default = "default_max_items")]
    pub max_items: usize,
    #[serde(default = "default_true")]
    pub ai_enabled: bool,
    #[serde(default = "default_true")]
    pub notify_done: bool,
    #[serde(default = "default_true")]
    pub notify_question: bool,
    #[serde(default = "default_true")]
    pub notify_scheduled: bool,
    #[serde(default = "default_true")]
    pub notify_error: bool,
    #[serde(default = "default_true")]
    pub wechat_enabled: bool,
    #[serde(default = "default_true")]
    pub sound_enabled: bool,
    #[serde(default)]
    pub lyrics_folder: String,
    #[serde(default = "default_true")]
    pub lyrics_online: bool,
    #[serde(default = "default_true")]
    pub lyrics_chinese: bool,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            dark: true,
            locale: String::new(),
            clipboard_enabled: true,
            capture_images: true,
            max_items: default_max_items(),
            ai_enabled: true,
            notify_done: true,
            notify_question: true,
            notify_scheduled: true,
            notify_error: true,
            wechat_enabled: true,
            sound_enabled: true,
            lyrics_folder: String::new(),
            lyrics_online: true,
            lyrics_chinese: true,
        }
    }
}

fn new_dir() -> Option<PathBuf> {
    let base = std::env::var_os("APPDATA")?;
    Some(PathBuf::from(base).join("com.focusd.island.nsh"))
}

fn data_file() -> Option<PathBuf> {
    Some(new_dir()?.join("settings.json"))
}

/// Settings written before the AI switches existed keep their defaults; import
/// those five fields once so an old `cindyNotificationsEnabled=false` is kept.
fn import_ai_settings_once(preferences: &mut Preferences) {
    let Some(flag) = ai_import_flag() else {
        return;
    };
    if flag.exists() {
        return;
    }
    if let Some(legacy) = legacy_settings_text()
        .as_deref()
        .and_then(|raw| serde_json::from_str::<LegacySettings>(raw).ok())
    {
        preferences.ai_enabled = legacy.cindy_notifications_enabled;
        preferences.notify_done = legacy.cindy_notify_done;
        preferences.notify_question = legacy.cindy_notify_question;
        preferences.notify_scheduled = legacy.cindy_notify_scheduled;
        preferences.notify_error = legacy.cindy_notify_error;
    }
    if let Some(parent) = flag.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = fs::write(flag, b"1");
    save(preferences);
}

fn ai_import_flag() -> Option<PathBuf> {
    Some(new_dir()?.join("legacy-ai-settings-imported.flag"))
}

fn legacy_settings_text() -> Option<String> {
    if let Some(text) = new_dir().and_then(|dir| fs::read_to_string(dir.join("legacy-settings.json")).ok()) {
        return Some(text);
    }
    legacy_local_storage()?.settings
}

pub fn load() -> Preferences {
    if let Some(path) = data_file() {
        if let Ok(text) = fs::read_to_string(path) {
            if let Ok(mut preferences) = serde_json::from_str::<Preferences>(&text) {
                import_ai_settings_once(&mut preferences);
                return preferences;
            }
        }
    }
    if !nsh_core::prelude::os().is_desktop() {
        return Preferences::default();
    }
    let LegacyLocalStorage { settings, locale } = legacy_local_storage().unwrap_or_default();
    let legacy = settings
        .as_deref()
        .and_then(|raw| serde_json::from_str::<LegacySettings>(raw).ok());
    let clipboard = legacy_clipboard_settings();
    let preferences = Preferences {
        dark: legacy.as_ref().and_then(dark_from_settings).unwrap_or(true),
        locale: locale.unwrap_or_default(),
        clipboard_enabled: clipboard.map_or(true, |(enabled, _, _)| enabled),
        capture_images: clipboard.map_or(true, |(_, capture, _)| capture),
        max_items: clipboard.map_or(default_max_items(), |(_, _, max)| max),
        ai_enabled: legacy.as_ref().is_none_or(|item| item.cindy_notifications_enabled),
        notify_done: legacy.as_ref().is_none_or(|item| item.cindy_notify_done),
        notify_question: legacy.as_ref().is_none_or(|item| item.cindy_notify_question),
        notify_scheduled: legacy.as_ref().is_none_or(|item| item.cindy_notify_scheduled),
        notify_error: legacy.as_ref().is_none_or(|item| item.cindy_notify_error),
        wechat_enabled: true,
        sound_enabled: true,
        lyrics_folder: legacy.as_ref().map_or_else(String::new, |item| item.lyrics_folder.clone()),
        lyrics_online: legacy.as_ref().is_none_or(|item| item.lyrics_online),
        lyrics_chinese: legacy.as_ref().is_none_or(|item| item.lyrics_chinese),
    };
    if let Some(dir) = new_dir() {
        let _ = fs::create_dir_all(&dir);
        if let Some(raw) = settings.as_deref() {
            let _ = fs::write(dir.join("legacy-settings.json"), raw);
        }

    }
    save(&preferences);
    preferences
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct LegacySettings {
    #[serde(default)]
    appearance_mode: String,
    #[serde(default)]
    island_background_color: String,
    #[serde(default)]
    task_text_color: String,
    #[serde(default = "default_true")]
    cindy_notifications_enabled: bool,
    #[serde(default = "default_true")]
    cindy_notify_done: bool,
    #[serde(default = "default_true")]
    cindy_notify_question: bool,
    #[serde(default = "default_true")]
    cindy_notify_scheduled: bool,
    #[serde(default = "default_true")]
    cindy_notify_error: bool,
    #[serde(default)]
    lyrics_folder: String,
    #[serde(default = "default_true")]
    lyrics_online: bool,
    #[serde(default = "default_true")]
    lyrics_chinese: bool,
}

fn dark_from_settings(settings: &LegacySettings) -> Option<bool> {
    if let Some((r, g, b)) = parse_hex(&settings.island_background_color) {
        return Some(luminance(r, g, b) < 0.5);
    }
    if let Some((r, g, b)) = parse_hex(&settings.task_text_color) {
        return Some(luminance(r, g, b) > 0.5);
    }
    Some(settings.appearance_mode != "glass")
}

fn parse_hex(text: &str) -> Option<(u8, u8, u8)> {
    let hex = text.trim().trim_start_matches('#');
    if hex.len() != 6 {
        return None;
    }
    let value = u32::from_str_radix(hex, 16).ok()?;
    Some((
        ((value >> 16) & 0xFF) as u8,
        ((value >> 8) & 0xFF) as u8,
        (value & 0xFF) as u8,
    ))
}

fn luminance(r: u8, g: u8, b: u8) -> f32 {
    (0.2126 * r as f32 + 0.7152 * g as f32 + 0.0722 * b as f32) / 255.0
}

#[derive(Default)]
struct LegacyLocalStorage {
    settings: Option<String>,
    locale: Option<String>,
}

fn legacy_local_storage() -> Option<LegacyLocalStorage> {
    let base = std::env::var_os("LOCALAPPDATA")?;
    let src = PathBuf::from(base)
        .join("com.focusd.island")
        .join("EBWebView")
        .join("Default")
        .join("Local Storage")
        .join("leveldb");
    let dst = std::env::temp_dir().join(format!("nsh-legacy-settings-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dst);
    fs::create_dir_all(&dst).ok()?;
    for entry in fs::read_dir(src).ok()? {
        let entry = entry.ok()?;
        if entry.file_name() == "LOCK" {
            continue;
        }
        if entry.file_type().ok()?.is_file() {
            let _ = fs::copy(entry.path(), dst.join(entry.file_name()));
        }
    }
    let mut db = rusty_leveldb::DB::open(&dst, Options::default()).ok()?;
    let mut iter = db.new_iter().ok()?;
    let mut found = LegacyLocalStorage::default();
    while let Some((key, value)) = iter.next() {
        let key = String::from_utf8_lossy(&key);
        if key.ends_with("focusd-island-settings") {
            found.settings.get_or_insert_with(|| legacy_json(&value));
        } else if key.ends_with("focusd-island-locale") {
            found.locale.get_or_insert_with(|| legacy_locale(&value));
        }
    }
    found.settings = found
        .settings
        .filter(|text| text.trim_start().starts_with('{'));
    found.locale = found.locale.filter(|text| text == "zh-CN" || text == "en");
    Some(found)
}

fn legacy_json(value: &[u8]) -> String {
    let body = value.strip_prefix(&[1u8]).unwrap_or(value);
    String::from_utf8_lossy(body).into_owned()
}

fn legacy_locale(value: &[u8]) -> String {
    if let Some(body) = value.strip_prefix(&[1u8]) {
        return String::from_utf8_lossy(body)
            .trim()
            .trim_matches('"')
            .to_string();
    }
    if let Some(body) = value.strip_prefix(&[0u8]) {
        let units: Vec<u16> = body
            .chunks_exact(2)
            .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
            .collect();
        return String::from_utf16_lossy(&units).trim().to_string();
    }
    String::new()
}

fn legacy_clipboard_settings() -> Option<(bool, bool, usize)> {
    let base = std::env::var_os("APPDATA")?;
    let path = PathBuf::from(base)
        .join("com.focusd.island")
        .join("clipboard-history.json");
    let text = fs::read_to_string(path).ok()?;
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct LegacyHistory {
        settings: LegacyClipboard,
    }
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct LegacyClipboard {
        enabled: bool,
        capture_images: bool,
        max_items: usize,
    }
    let history: LegacyHistory = serde_json::from_str(&text).ok()?;
    Some((
        history.settings.enabled,
        history.settings.capture_images,
        history.settings.max_items.clamp(5, 200),
    ))
}

pub fn save(preferences: &Preferences) {
    let Some(path) = data_file() else {
        return;
    };
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if path.exists() {
        let _ = fs::copy(&path, path.with_file_name("settings.json.bak"));
    }
    if let Ok(json) = serde_json::to_string_pretty(preferences) {
        let _ = fs::write(path, json);
    }
}
