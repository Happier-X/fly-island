//! Screenshot settings: %APPDATA%\com.focusd.island.nsh\screenshot\settings.json.
//!
//! The file mirrors the Tauri app's screenshot settings for the fields NSH uses,
//! then converts them into AirSnip's host-injected Config. AirSnip itself never
//! reads or writes its own config.toml in hosted mode.

use std::{fs, path::PathBuf};

use serde::{Deserialize, Serialize};

use airsnip::settings::{Config, ThemeSetting, UiScale};

pub const DEFAULT_SHORTCUT: &str = "Ctrl+Shift+A";
pub const SAVE_FORMATS: [&str; 3] = ["png", "jpeg", "webp"];

fn default_shortcut() -> String {
    DEFAULT_SHORTCUT.to_string()
}

fn default_save_format() -> String {
    SAVE_FORMATS[0].to_string()
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenshotSettings {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_shortcut")]
    pub shortcut: String,
    #[serde(default = "default_save_format")]
    pub save_format: String,
    #[serde(default)]
    pub save_dir: String,
    #[serde(default)]
    pub auto_save: bool,
}

impl Default for ScreenshotSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            shortcut: default_shortcut(),
            save_format: default_save_format(),
            save_dir: String::new(),
            auto_save: false,
        }
    }
}

impl ScreenshotSettings {
    pub fn normalized(mut self) -> Self {
        self.shortcut = self.shortcut.trim().to_string();
        if self.shortcut.is_empty() {
            self.shortcut = default_shortcut();
        }
        self.save_format = normalize_format(&self.save_format);
        self.save_dir = self.save_dir.trim().to_string();
        self
    }

    /// The directory the Save As dialog and auto-save use. An empty setting means
    /// %USERPROFILE%\Pictures\FocuSD Island, matching the app own folder.
    pub fn effective_save_dir(&self) -> PathBuf {
        if self.save_dir.trim().is_empty() {
            let home = std::env::var("USERPROFILE").unwrap_or_else(|_| ".".to_string());
            PathBuf::from(home).join("Pictures").join("FocuSD Island")
        } else {
            PathBuf::from(self.save_dir.trim())
        }
    }

    /// Build the snapshot AirSnip receives. It covers the capture/annotate/pin
    /// pipeline and the save dialog defaults; host-specific features stay off.
    pub fn to_airsnip_config(&self) -> Config {
        let save_dir = self.effective_save_dir();
        let mut config = Config {
            theme: ThemeSetting::Follow,
            ui_scale: UiScale::Auto,
            last_save_ext: Some(normalize_format(&self.save_format)),
            last_save_dir: Some(save_dir.to_string_lossy().into_owned()),
            auto_save: self.auto_save,
            ..Default::default()
        };
        config.ocr.language = airsnip::settings::OCR_LANG_AUTO.to_string();
        config
    }
}

/// Keep jpg working while storing the canonical jpeg value.
pub fn normalize_format(value: &str) -> String {
    match value.trim().to_ascii_lowercase().as_str() {
        "jpeg" | "jpg" => "jpeg".to_string(),
        "webp" => "webp".to_string(),
        _ => "png".to_string(),
    }
}

pub fn config_dir() -> PathBuf {
    if let Some(base) = std::env::var_os("APPDATA") {
        PathBuf::from(base)
            .join("com.focusd.island.nsh")
            .join("screenshot")
    } else {
        std::env::temp_dir().join("focusd-island-nsh").join("screenshot")
    }
}

fn settings_path() -> PathBuf {
    config_dir().join("settings.json")
}

pub fn load() -> ScreenshotSettings {
    let path = settings_path();
    fs::read_to_string(path)
        .ok()
        .and_then(|text| serde_json::from_str::<ScreenshotSettings>(&text).ok())
        .map(ScreenshotSettings::normalized)
        .unwrap_or_default()
}

pub fn write(settings: &ScreenshotSettings) -> Result<(), String> {
    let dir = config_dir();
    fs::create_dir_all(&dir)
        .map_err(|error| format!("无法创建截图配置目录 {}：{error}", dir.display()))?;
    let text = serde_json::to_string_pretty(settings)
        .map_err(|error| format!("无法序列化截图设置：{error}"))?;
    fs::write(dir.join("settings.json"), text)
        .map_err(|error| format!("无法保存截图设置：{error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_are_canonical() {
        assert_eq!(normalize_format("JPG"), "jpeg");
        assert_eq!(normalize_format("webp"), "webp");
        assert_eq!(normalize_format(""), "png");
    }

    #[test]
    fn empty_shortcut_falls_back() {
        let settings = ScreenshotSettings { shortcut: "  ".into(), ..Default::default() }.normalized();
        assert_eq!(settings.shortcut, DEFAULT_SHORTCUT);
    }
}