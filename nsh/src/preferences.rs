//! Persisted app settings: theme, language and clipboard options.

use std::{fs, path::PathBuf};

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
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            dark: true,
            locale: String::new(),
            clipboard_enabled: true,
            capture_images: true,
            max_items: default_max_items(),
        }
    }
}

fn data_file() -> Option<PathBuf> {
    let base = std::env::var_os("APPDATA")?;
    Some(
        PathBuf::from(base)
            .join("com.focusd.island.nsh")
            .join("settings.json"),
    )
}

pub fn load() -> Preferences {
    if let Some(path) = data_file() {
        if let Ok(text) = fs::read_to_string(path) {
            if let Ok(preferences) = serde_json::from_str::<Preferences>(&text) {
                return preferences;
            }
        }
    }
    legacy_clipboard_settings()
        .map(|(enabled, capture_images, max_items)| Preferences {
            clipboard_enabled: enabled,
            capture_images,
            max_items,
            ..Preferences::default()
        })
        .unwrap_or_default()
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
        settings: LegacySettings,
    }
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct LegacySettings {
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
    if let Ok(json) = serde_json::to_string_pretty(preferences) {
        let _ = fs::write(path, json);
    }
}
