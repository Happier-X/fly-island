//! Screenshot settings: %APPDATA%\com.focusd.island.nsh\screenshot\settings.json.
//!
//! The file mirrors the Tauri app's screenshot settings for the fields NSH uses,
//! then converts them into AirSnip's host-injected Config. AirSnip itself never
//! reads or writes its own config.toml in hosted mode.

use std::{collections::BTreeMap, fs, path::PathBuf};

use serde::{Deserialize, Serialize};

use airsnip::editor::{DEFAULT_COLORS, SWATCH_COUNT};
use airsnip::settings::{
    Config, NameStyle, ThemeSetting, ToolbarSlot, ToolKeys, UiScale, UNBOUND, hotkey::vk_name,
};

pub const DEFAULT_SHORTCUT: &str = "Ctrl+Shift+A";
pub const SAVE_FORMATS: [&str; 3] = ["png", "jpeg", "webp"];
pub const OCR_LANG_AUTO: &str = "auto";
pub const DEFAULT_LONGSHOT_MAX: u32 = 30_000;

/// Filename templates understood by AirSnip's NameStyle.
pub const NAME_STYLES: [&str; 3] = ["datetime", "timestamp", "date"];
/// Overlay UI scale values understood by AirSnip's UiScale.
pub const UI_SCALES: [&str; 6] = ["auto", "100", "125", "150", "175", "200"];
/// Tool key fields in ToolKeys order; labels follow the same order.
pub const TOOL_KEY_FIELDS: [&str; 10] = [
    "rect", "arrow", "text", "blur", "highlight", "glow", "number", "line", "ellipse", "freehand",
];
pub const TOOL_KEY_LABELS: [&str; 10] = [
    "矩形", "箭头", "文本", "模糊", "高亮", "聚光灯", "序号", "直线", "椭圆", "手绘",
];

fn default_shortcut() -> String {
    DEFAULT_SHORTCUT.to_string()
}

fn default_save_format() -> String {
    SAVE_FORMATS[0].to_string()
}

fn default_ocr_language() -> String {
    OCR_LANG_AUTO.to_string()
}

fn default_longshot_max() -> u32 {
    DEFAULT_LONGSHOT_MAX
}

fn default_true() -> bool {
    true
}

fn default_name_style() -> String {
    NAME_STYLES[0].to_string()
}

fn default_ui_scale() -> String {
    UI_SCALES[0].to_string()
}

fn default_colors() -> Vec<String> {
    DEFAULT_COLORS.iter().map(|color| format_hex(*color)).collect()
}

fn default_tool_keys() -> BTreeMap<String, String> {
    let keys = ToolKeys::default();
    TOOL_KEY_FIELDS
        .iter()
        .map(|field| (field.to_string(), tool_key_label(tool_key_value(&keys, field))))
        .collect()
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
    #[serde(default = "default_ocr_language")]
    pub ocr_language: String,
    #[serde(default)]
    pub ocr_auto_copy: bool,
    #[serde(default = "default_longshot_max")]
    pub longshot_max_height: u32,
    /// Five toolbar swatch colors as hex (#RRGGBB or #RRGGBBAA).
    #[serde(default = "default_colors")]
    pub colors: Vec<String>,
    /// Save filename template: datetime / timestamp / date.
    #[serde(default = "default_name_style")]
    pub name_style: String,
    /// Overlay UI scale: auto / 100 / 125 / 150 / 175 / 200.
    #[serde(default = "default_ui_scale")]
    pub ui_scale: String,
    /// Per-tool switch keys ("1", "F1", "" = unbound).
    #[serde(default = "default_tool_keys")]
    pub tool_keys: BTreeMap<String, String>,
    /// Hidden toolbar slot keys (ToolbarSlot::key); empty = all visible.
    #[serde(default)]
    pub hidden_toolbar: Vec<String>,
}

impl Default for ScreenshotSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            shortcut: default_shortcut(),
            save_format: default_save_format(),
            save_dir: String::new(),
            auto_save: false,
            ocr_language: default_ocr_language(),
            ocr_auto_copy: false,
            longshot_max_height: default_longshot_max(),
            colors: default_colors(),
            name_style: default_name_style(),
            ui_scale: default_ui_scale(),
            tool_keys: default_tool_keys(),
            hidden_toolbar: Vec::new(),
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
        self.ocr_language = self.ocr_language.trim().to_string();
        if self.ocr_language.is_empty() {
            self.ocr_language = default_ocr_language();
        }
        self.longshot_max_height = self.longshot_max_height.clamp(1_000, 100_000);
        self.colors = normalize_colors(&self.colors);
        self.name_style = normalize_name_style(&self.name_style);
        self.ui_scale = normalize_ui_scale(&self.ui_scale);
        self.tool_keys = normalize_tool_keys(&self.tool_keys);
        self.hidden_toolbar = normalize_hidden_toolbar(&self.hidden_toolbar);
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

    /// Five swatch colors as RGBA, falling back to AirSnip defaults per entry.
    pub fn swatch_rgba(&self) -> [[u8; 4]; SWATCH_COUNT] {
        let mut colors = DEFAULT_COLORS;
        for (index, value) in self.colors.iter().take(SWATCH_COUNT).enumerate() {
            if let Some(color) = parse_hex_color(value) {
                colors[index] = color;
            }
        }
        colors
    }

    /// Tool switch keys as virtual-key codes.
    pub fn tool_keys_struct(&self) -> ToolKeys {
        let mut keys = ToolKeys::default();
        for field in TOOL_KEY_FIELDS {
            let label = self.tool_keys.get(field).map(String::as_str).unwrap_or("");
            set_tool_key(&mut keys, field, parse_tool_key(label));
        }
        keys
    }

    /// Build the snapshot AirSnip receives. It covers the capture/annotate/pin
    /// pipeline, save defaults, editor palette, toolbar visibility and tool keys.
    pub fn to_airsnip_config(&self) -> Config {
        let save_dir = self.effective_save_dir();
        let mut config = Config {
            theme: ThemeSetting::Follow,
            last_save_ext: Some(normalize_format(&self.save_format)),
            last_save_dir: Some(save_dir.to_string_lossy().into_owned()),
            auto_save: self.auto_save,
            name_style: name_style_enum(&self.name_style),
            ui_scale: ui_scale_enum(&self.ui_scale),
            swatch_colors: self.swatch_rgba(),
            tool_keys: self.tool_keys_struct(),
            ..Default::default()
        };
        config.ocr.language = self.ocr_language.clone();
        config.ocr.auto_copy = self.ocr_auto_copy;
        config.longshot_max_height = self.longshot_max_height;
        config.toolbar.hidden = self.hidden_toolbar.clone();
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

pub fn normalize_colors(values: &[String]) -> Vec<String> {
    let defaults = default_colors();
    (0..SWATCH_COUNT)
        .map(|index| {
            values
                .get(index)
                .and_then(|value| parse_hex_color(value))
                .map(format_hex)
                .unwrap_or_else(|| defaults[index].clone())
        })
        .collect()
}

/// Parse #RGB, #RRGGBB or #RRGGBBAA into RGBA bytes.
pub fn parse_hex_color(value: &str) -> Option<[u8; 4]> {
    let text = value.trim().trim_start_matches('#');
    let text = match text.len() {
        3 => text.chars().flat_map(|c| [c, c]).collect::<String>(),
        6 | 8 => text.to_string(),
        _ => return None,
    };
    if !text.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let channel = |start: usize| u8::from_str_radix(&text[start..start + 2], 16).ok();
    let alpha = if text.len() == 8 { channel(6)? } else { 255 };
    Some([channel(0)?, channel(2)?, channel(4)?, alpha])
}

pub fn format_hex(color: [u8; 4]) -> String {
    if color[3] == 255 {
        format!("#{:02X}{:02X}{:02X}", color[0], color[1], color[2])
    } else {
        format!(
            "#{:02X}{:02X}{:02X}{:02X}",
            color[0], color[1], color[2], color[3]
        )
    }
}

pub fn normalize_name_style(value: &str) -> String {
    let text = value.trim().to_ascii_lowercase();
    if NAME_STYLES.contains(&text.as_str()) {
        text
    } else {
        default_name_style()
    }
}

pub fn name_style_enum(value: &str) -> NameStyle {
    match normalize_name_style(value).as_str() {
        "timestamp" => NameStyle::Timestamp,
        "date" => NameStyle::Date,
        _ => NameStyle::DateTime,
    }
}

pub fn normalize_ui_scale(value: &str) -> String {
    let text = value.trim().to_ascii_lowercase();
    if UI_SCALES.contains(&text.as_str()) {
        text
    } else {
        default_ui_scale()
    }
}

pub fn ui_scale_enum(value: &str) -> UiScale {
    match normalize_ui_scale(value).as_str() {
        "100" => UiScale::P100,
        "125" => UiScale::P125,
        "150" => UiScale::P150,
        "175" => UiScale::P175,
        "200" => UiScale::P200,
        _ => UiScale::Auto,
    }
}

/// Parse a tool key: a single letter/digit, F1-F24, a few named keys, or empty
/// for unbound. Unrecognized text becomes unbound rather than panicking.
pub fn parse_tool_key(value: &str) -> u32 {
    let text = value.trim();
    if text.is_empty() {
        return UNBOUND;
    }
    let upper = text.to_ascii_uppercase();
    if let Some(number) = upper.strip_prefix('F').and_then(|rest| rest.parse::<u32>().ok())
        && (1..=24).contains(&number)
    {
        return 0x70 + number - 1;
    }
    let mut chars = upper.chars();
    if let (Some(ch), None) = (chars.next(), chars.next())
        && ch.is_ascii_alphanumeric()
    {
        return ch as u32;
    }
    match upper.as_str() {
        "SPACE" => 0x20,
        "ESC" => 0x1B,
        "TAB" => 0x09,
        "ENTER" => 0x0D,
        "DEL" | "DELETE" => 0x2E,
        "HOME" => 0x24,
        "END" => 0x23,
        "PGUP" => 0x21,
        "PGDN" => 0x22,
        "INSERT" => 0x2D,
        _ => UNBOUND,
    }
}

pub fn tool_key_label(vk: u32) -> String {
    if vk == UNBOUND {
        String::new()
    } else {
        vk_name(vk)
    }
}

fn tool_key_value(keys: &ToolKeys, field: &str) -> u32 {
    match field {
        "rect" => keys.rect,
        "arrow" => keys.arrow,
        "text" => keys.text,
        "blur" => keys.blur,
        "highlight" => keys.highlight,
        "glow" => keys.glow,
        "number" => keys.number,
        "line" => keys.line,
        "ellipse" => keys.ellipse,
        "freehand" => keys.freehand,
        _ => UNBOUND,
    }
}

fn set_tool_key(keys: &mut ToolKeys, field: &str, vk: u32) {
    match field {
        "rect" => keys.rect = vk,
        "arrow" => keys.arrow = vk,
        "text" => keys.text = vk,
        "blur" => keys.blur = vk,
        "highlight" => keys.highlight = vk,
        "glow" => keys.glow = vk,
        "number" => keys.number = vk,
        "line" => keys.line = vk,
        "ellipse" => keys.ellipse = vk,
        "freehand" => keys.freehand = vk,
        _ => {}
    }
}

pub fn normalize_tool_keys(values: &BTreeMap<String, String>) -> BTreeMap<String, String> {
    let defaults = default_tool_keys();
    TOOL_KEY_FIELDS
        .iter()
        .map(|field| {
            let label = match values.get(*field) {
                Some(value) => tool_key_label(parse_tool_key(value)),
                None => defaults.get(*field).cloned().unwrap_or_default(),
            };
            (field.to_string(), label)
        })
        .collect()
}

pub fn normalize_hidden_toolbar(values: &[String]) -> Vec<String> {
    let valid: Vec<&str> = ToolbarSlot::all().map(|slot| slot.key()).collect();
    let mut hidden: Vec<String> = Vec::new();
    for value in values {
        if valid.contains(&value.as_str()) && !hidden.contains(value) {
            hidden.push(value.clone());
        }
    }
    hidden
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
    fn ocr_and_longshot_settings_reach_airsnip() {
        let settings = ScreenshotSettings {
            ocr_language: "en-US".into(),
            ocr_auto_copy: true,
            longshot_max_height: 12_000,
            ..Default::default()
        }
        .normalized();
        assert_eq!(settings.ocr_language, "en-US");
        let config = settings.to_airsnip_config();
        assert_eq!(config.ocr.language, "en-US");
        assert!(config.ocr.auto_copy);
        assert_eq!(config.longshot_max_height, 12_000);
    }

    #[test]
    fn empty_ocr_language_falls_back_to_auto() {
        let settings = ScreenshotSettings { ocr_language: "  ".into(), ..Default::default() }.normalized();
        assert_eq!(settings.ocr_language, OCR_LANG_AUTO);
    }

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

    #[test]
    fn colors_parse_and_normalize() {
        assert_eq!(parse_hex_color("#f00"), Some([255, 0, 0, 255]));
        assert_eq!(parse_hex_color("00FF0080"), Some([0, 255, 0, 128]));
        assert_eq!(parse_hex_color("nope"), None);
        let settings = ScreenshotSettings {
            colors: vec!["#123456".into(), "nope".into()],
            ..Default::default()
        }
        .normalized();
        assert_eq!(settings.colors.len(), SWATCH_COUNT);
        assert_eq!(settings.colors[0], "#123456");
        assert_eq!(settings.swatch_rgba()[0], [0x12, 0x34, 0x56, 255]);
        assert_eq!(settings.swatch_rgba()[1], DEFAULT_COLORS[1]);
    }

    #[test]
    fn name_style_and_ui_scale_map_to_airsnip_enums() {
        assert_eq!(name_style_enum("timestamp"), NameStyle::Timestamp);
        assert_eq!(name_style_enum("Date"), NameStyle::Date);
        assert_eq!(name_style_enum("whatever"), NameStyle::DateTime);
        assert_eq!(ui_scale_enum("150"), UiScale::P150);
        assert_eq!(ui_scale_enum("7"), UiScale::Auto);
        let settings = ScreenshotSettings {
            name_style: "date".into(),
            ui_scale: "125".into(),
            ..Default::default()
        }
        .normalized();
        let config = settings.to_airsnip_config();
        assert_eq!(config.name_style, NameStyle::Date);
        assert_eq!(config.ui_scale, UiScale::P125);
    }

    #[test]
    fn tool_keys_roundtrip() {
        assert_eq!(parse_tool_key("1"), 0x31);
        assert_eq!(parse_tool_key("f9"), 0x78);
        assert_eq!(parse_tool_key(""), UNBOUND);
        assert_eq!(parse_tool_key("??"), UNBOUND);
        let mut keys = default_tool_keys();
        keys.insert("rect".into(), "F9".into());
        keys.insert("arrow".into(), "".into());
        keys.insert("bogus".into(), "Z".into());
        let normalized = normalize_tool_keys(&keys);
        assert_eq!(normalized.len(), TOOL_KEY_FIELDS.len());
        assert_eq!(normalized.get("rect").map(String::as_str), Some("F9"));
        assert_eq!(normalized.get("arrow").map(String::as_str), Some(""));
        assert!(!normalized.contains_key("bogus"));
        let settings = ScreenshotSettings { tool_keys: normalized, ..Default::default() };
        let config = settings.to_airsnip_config();
        assert_eq!(config.tool_keys.rect, 0x78);
        assert_eq!(config.tool_keys.arrow, UNBOUND);
        assert_eq!(config.tool_keys.text, 0x33);
    }

    #[test]
    fn hidden_toolbar_keeps_known_keys() {
        let hidden = normalize_hidden_toolbar(&[
            "tool.rect".into(),
            "tool.rect".into(),
            "nope".into(),
            "file.copy".into(),
        ]);
        assert_eq!(hidden, vec!["tool.rect".to_string(), "file.copy".to_string()]);
        let settings = ScreenshotSettings { hidden_toolbar: hidden, ..Default::default() };
        assert_eq!(settings.to_airsnip_config().toolbar.hidden.len(), 2);
    }
}
