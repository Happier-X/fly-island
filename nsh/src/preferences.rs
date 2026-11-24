//! Persisted app settings: theme, language and clipboard options.
//!
//! On first run it reads the Tauri settings read-only: `focusd-island-settings`
//! and `focusd-island-locale` from the WebView2 LevelDB, and
//! `clipboard-history.json` for clipboard options. Only whitelisted fields are
//! imported; the old files are never modified or deleted.

use std::{collections::BTreeMap, fs, path::PathBuf};

use rusty_leveldb::{LdbIterator, Options};
use serde::{Deserialize, Serialize};

use crate::media::KnownSource;

fn default_true() -> bool {
    true
}

fn default_max_items() -> usize {
    30
}

fn default_translation_shortcut() -> String {
    "Ctrl+Shift+F".to_string()
}

fn default_translation_direction() -> String {
    "auto".to_string()
}

fn default_translation_provider() -> String {
    "siliconflow".to_string()
}

fn default_launcher_shortcut() -> String {
    "Alt+Space".to_string()
}

fn default_bing() -> String {
    "bing".to_string()
}

fn default_default_folder_mode() -> String {
    "default".to_string()
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
    #[serde(default)]
    pub music_sources: Vec<KnownSource>,
    #[serde(default = "default_true")]
    pub translation_enabled: bool,
    #[serde(default = "default_translation_shortcut")]
    pub translation_shortcut: String,
    #[serde(default = "default_translation_direction")]
    pub translation_direction: String,
    #[serde(default = "default_translation_provider")]
    pub translation_provider: String,
    #[serde(default)]
    pub translation_custom_base_url: String,
    #[serde(default)]
    pub translation_auto: bool,
    #[serde(default)]
    pub translation_models: BTreeMap<String, String>,
    #[serde(default = "default_true")]
    pub launcher_enabled: bool,
    #[serde(default = "default_launcher_shortcut")]
    pub launcher_shortcut: String,
    #[serde(default = "default_true")]
    pub launcher_search_apps: bool,
    #[serde(default = "default_true")]
    pub launcher_search_files: bool,
    #[serde(default = "default_true")]
    pub launcher_search_web: bool,
    #[serde(default = "default_bing")]
    pub launcher_engine: String,
    #[serde(default = "default_default_folder_mode")]
    pub launcher_folder_mode: String,
    #[serde(default)]
    pub launcher_folders: Vec<String>,
    #[serde(default = "default_true")]
    pub qq_enabled: bool,
    #[serde(default)]
    pub mail_accounts: Vec<crate::mail::Account>,
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
            music_sources: Vec::new(),
            translation_enabled: true,
            translation_shortcut: default_translation_shortcut(),
            translation_direction: default_translation_direction(),
            translation_provider: default_translation_provider(),
            translation_custom_base_url: String::new(),
            translation_auto: false,
            translation_models: BTreeMap::new(),
            launcher_enabled: true,
            launcher_shortcut: default_launcher_shortcut(),
            launcher_search_apps: true,
            launcher_search_files: true,
            launcher_search_web: true,
            launcher_engine: default_bing(),
            launcher_folder_mode: default_default_folder_mode(),
            launcher_folders: Vec::new(),
            qq_enabled: true,
            mail_accounts: Vec::new(),
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

fn import_translation_once(preferences: &mut Preferences) {
    let Some(flag) = translation_import_flag() else {
        return;
    };
    if flag.exists() {
        return;
    }
    let legacy_saved = legacy_settings_text()
        .as_deref()
        .and_then(|raw| serde_json::from_str::<LegacySettings>(raw).ok());
    let local = legacy_local_storage().unwrap_or_default();
    let legacy = local.translation.as_deref().and_then(parse_legacy_translation);
    if preferences.translation_shortcut == default_translation_shortcut()
        && preferences.translation_provider == default_translation_provider()
        && preferences.translation_models.is_empty()
    {
        if let Some(legacy) = legacy {
            preferences.translation_shortcut = legacy.shortcut;
            preferences.translation_direction = legacy.direction;
            preferences.translation_provider = legacy.provider;
            preferences.translation_custom_base_url = legacy.custom_base_url;
            preferences.translation_models = legacy.models;
        }
    }
    if let Some(legacy) = legacy_saved {
        preferences.translation_enabled = legacy.translation_enabled;
    }
    if let Some(parent) = flag.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = fs::write(flag, b"1");
    save(preferences);
}

fn translation_import_flag() -> Option<PathBuf> {
    Some(new_dir()?.join("legacy-translation-settings-imported.flag"))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct LegacyTranslation {
    #[serde(default)]
    shortcut: String,
    #[serde(default)]
    direction: String,
    #[serde(default)]
    provider: String,
    #[serde(default)]
    custom_base_url: String,
    #[serde(default)]
    models: BTreeMap<String, String>,
}

fn parse_legacy_translation(raw: &str) -> Option<LegacyTranslation> {
    let legacy = serde_json::from_str::<LegacyTranslation>(raw).ok()?;
    let shortcut = match legacy.shortcut.as_str() {
        "" | "Alt+Space" | "Alt+C" => default_translation_shortcut(),
        shortcut => shortcut.to_string(),
    };
    let direction = match legacy.direction.as_str() {
        "zh-en" | "en-zh" => legacy.direction.clone(),
        _ => default_translation_direction(),
    };
    let provider = if crate::translation::is_provider(&legacy.provider) {
        legacy.provider.clone()
    } else {
        "deepseek".to_string()
    };
    let models = legacy
        .models
        .into_iter()
        .filter(|(provider, model)| crate::translation::is_provider(provider) && !model.trim().is_empty())
        .collect();
    Some(LegacyTranslation {
        shortcut,
        direction,
        provider,
        custom_base_url: legacy.custom_base_url,
        models,
    })
}
fn import_launcher_once(preferences: &mut Preferences) {
    let Some(flag) = launcher_import_flag() else {
        return;
    };
    if flag.exists() {
        return;
    }
    let local = legacy_local_storage().unwrap_or_default();
    if let Some(legacy) = local.launcher.as_deref().and_then(parse_legacy_launcher) {
        if preferences.launcher_shortcut == default_launcher_shortcut()
            && preferences.launcher_engine == default_bing()
            && preferences.launcher_folders.is_empty()
        {
            preferences.launcher_enabled = legacy.enabled;
            preferences.launcher_shortcut = legacy.shortcut;
            preferences.launcher_search_apps = legacy.search_apps;
            preferences.launcher_search_files = legacy.search_files;
            preferences.launcher_search_web = legacy.search_web;
            preferences.launcher_engine = legacy.search_engine;
            preferences.launcher_folder_mode = legacy.folder_mode;
            preferences.launcher_folders = legacy.folders;
        }
    }
    if let Some(parent) = flag.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = fs::write(flag, b"1");
    save(preferences);
}

fn launcher_import_flag() -> Option<PathBuf> {
    Some(new_dir()?.join("legacy-launcher-settings-imported.flag"))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct LegacyLauncher {
    #[serde(default = "default_true")]
    enabled: bool,
    #[serde(default)]
    shortcut: String,
    #[serde(default)]
    trigger_mode: String,
    #[serde(default = "default_true")]
    search_apps: bool,
    #[serde(default = "default_true")]
    search_files: bool,
    #[serde(default = "default_true")]
    search_web: bool,
    #[serde(default = "default_bing")]
    search_engine: String,
    #[serde(default = "default_default_folder_mode")]
    folder_mode: String,
    #[serde(default)]
    folders: Vec<String>,
}

fn parse_legacy_launcher(raw: &str) -> Option<LegacyLauncher> {
    let legacy = serde_json::from_str::<LegacyLauncher>(raw).ok()?;
    let shortcut = if legacy.trigger_mode == "doubleCtrl" {
        "DoubleCtrl".to_string()
    } else if legacy.shortcut.contains('+') {
        legacy.shortcut.clone()
    } else {
        default_launcher_shortcut()
    };
    let engine = match legacy.search_engine.as_str() {
        "google" | "baidu" | "duckduckgo" => legacy.search_engine.clone(),
        _ => default_bing(),
    };
    let folder_mode = if legacy.folder_mode == "custom" { "custom".to_string() } else { default_default_folder_mode() };
    let mut seen = std::collections::HashSet::new();
    let folders = legacy
        .folders
        .into_iter()
        .map(|folder| folder.trim().to_string())
        .filter(|folder| !folder.is_empty())
        .filter(|folder| seen.insert(folder.to_lowercase()))
        .collect();
    Some(LegacyLauncher {
        enabled: legacy.enabled,
        shortcut,
        trigger_mode: legacy.trigger_mode,
        search_apps: legacy.search_apps,
        search_files: legacy.search_files,
        search_web: legacy.search_web,
        search_engine: engine,
        folder_mode,
        folders,
    })
}
fn import_mail_once(preferences: &mut Preferences) {
    let Some(flag) = mail_import_flag() else {
        return;
    };
    if flag.exists() {
        return;
    }
    let local = legacy_local_storage().unwrap_or_default();
    if preferences.mail_accounts.is_empty() {
        if let Some(accounts) = local.mail.as_deref().and_then(parse_legacy_mail) {
            preferences.mail_accounts = accounts;
        }
        if let Some(qq) = local.qq.as_deref() {
            preferences.qq_enabled = qq != "false";
        }
    }
    if let Some(parent) = flag.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = fs::write(flag, b"1");
    save(preferences);
}

fn mail_import_flag() -> Option<PathBuf> {
    Some(new_dir()?.join("legacy-mail-settings-imported.flag"))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct LegacyMailAccount {
    #[serde(default)]
    email: String,
    #[serde(default)]
    host: String,
    #[serde(default = "default_true")]
    enabled: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct LegacyMailSettings {
    #[serde(default)]
    accounts: Vec<LegacyMailAccount>,
    #[serde(default)]
    email: String,
    #[serde(default)]
    host: String,
    #[serde(default = "default_true")]
    enabled: bool,
}

fn parse_legacy_mail(raw: &str) -> Option<Vec<crate::mail::Account>> {
    let legacy = serde_json::from_str::<LegacyMailSettings>(raw).ok()?;
    let list = if legacy.accounts.is_empty() && !legacy.email.trim().is_empty() {
        vec![LegacyMailAccount {
            email: legacy.email,
            host: legacy.host,
            enabled: legacy.enabled,
        }]
    } else {
        legacy.accounts
    };
    let mut seen = std::collections::HashSet::new();
    Some(
        list.into_iter()
            .filter_map(|item| {
                let email = item.email.trim().to_ascii_lowercase();
                if email.is_empty() || !seen.insert(email.clone()) {
                    return None;
                }
                Some(crate::mail::Account {
                    email,
                    host: item.host.trim().to_string(),
                    enabled: item.enabled,
                })
            })
            .take(crate::mail::MAX_ACCOUNTS)
            .collect(),
    )
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
                import_translation_once(&mut preferences);
                import_launcher_once(&mut preferences);
                import_mail_once(&mut preferences);
                return preferences;
            }
        }
    }
    if !nsh_core::prelude::os().is_desktop() {
        return Preferences::default();
    }
    let LegacyLocalStorage { settings, locale, translation, launcher, mail, qq } = legacy_local_storage().unwrap_or_default();
    let legacy = settings
        .as_deref()
        .and_then(|raw| serde_json::from_str::<LegacySettings>(raw).ok());
    let clipboard = legacy_clipboard_settings();
    let translation_settings = translation.as_deref().and_then(parse_legacy_translation);
    let launcher_settings = launcher.as_deref().and_then(parse_legacy_launcher);
    let mail_accounts = mail.as_deref().and_then(parse_legacy_mail).unwrap_or_default();
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
        music_sources: Vec::new(),
        translation_enabled: legacy.as_ref().is_none_or(|item| item.translation_enabled),
        translation_shortcut: translation_settings.as_ref().map_or_else(default_translation_shortcut, |item| item.shortcut.clone()),
        translation_direction: translation_settings.as_ref().map_or_else(default_translation_direction, |item| item.direction.clone()),
        translation_provider: translation_settings.as_ref().map_or_else(default_translation_provider, |item| item.provider.clone()),
        translation_custom_base_url: translation_settings.as_ref().map_or_else(String::new, |item| item.custom_base_url.clone()),
        translation_auto: false,
        translation_models: translation_settings.as_ref().map_or_else(BTreeMap::new, |item| item.models.clone()),
        launcher_enabled: launcher_settings.as_ref().map_or(true, |item| item.enabled),
        launcher_shortcut: launcher_settings.as_ref().map_or_else(default_launcher_shortcut, |item| item.shortcut.clone()),
        launcher_search_apps: launcher_settings.as_ref().is_none_or(|item| item.search_apps),
        launcher_search_files: launcher_settings.as_ref().is_none_or(|item| item.search_files),
        launcher_search_web: launcher_settings.as_ref().is_none_or(|item| item.search_web),
        launcher_engine: launcher_settings.as_ref().map_or_else(default_bing, |item| item.search_engine.clone()),
        launcher_folder_mode: launcher_settings.as_ref().map_or_else(default_default_folder_mode, |item| item.folder_mode.clone()),
        launcher_folders: launcher_settings.as_ref().map_or_else(Vec::new, |item| item.folders.clone()),
        qq_enabled: qq.as_deref() != Some("false"),
        mail_accounts,
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
    #[serde(default = "default_true")]
    translation_enabled: bool,
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
    translation: Option<String>,
    launcher: Option<String>,
    mail: Option<String>,
    qq: Option<String>,
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
        } else if key.ends_with("focusd-island-translation-settings") {
            found.translation.get_or_insert_with(|| legacy_json(&value));
        } else if key.ends_with("focusd-island-launcher-settings") {
            found.launcher.get_or_insert_with(|| legacy_json(&value));
        } else if key.ends_with("focusd-island-qq-mail-settings") {
            found.mail.get_or_insert_with(|| legacy_json(&value));
        } else if key.ends_with("focusd-island-qq-notifications-enabled") {
            found.qq.get_or_insert_with(|| legacy_json(&value).trim().trim_matches(char::from(34)).to_string());
        }
    }
    found.settings = found
        .settings
        .filter(|text| text.trim_start().starts_with('{'));
    found.locale = found.locale.filter(|text| text == "zh-CN" || text == "en");
    found.translation = found.translation.filter(|text| text.trim_start().starts_with('{'));
    found.launcher = found.launcher.filter(|text| text.trim_start().starts_with('{'));
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