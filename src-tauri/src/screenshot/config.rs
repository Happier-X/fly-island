//! 截图设置：应用数据目录下的 `screenshot/settings.json`。
//!
//! 这里是截图的唯一配置来源：热键、总开关、保存格式、默认目录、标注色板都在
//! 这里持有，启动和每次修改时转换成 AirSnip 的 `Config` 注入过去。
//! AirSnip 自己的 `config.toml` 在托管模式下既不读也不写。

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

pub(super) const DEFAULT_SHORTCUT: &str = "Ctrl+Shift+A";
const FILE_NAME: &str = "screenshot/settings.json";

/// 保存格式：与 AirSnip 的扩展名一致（`jpg` 也接受，统一按 `jpeg` 存）。
pub(super) const SAVE_FORMATS: [&str; 3] = ["png", "jpeg", "webp"];

/// 工具栏 5 个标注色块，默认值与 AirSnip 的 `editor::DEFAULT_COLORS` 对齐。
pub(super) const DEFAULT_COLORS: [&str; 5] = ["#FFFFFF", "#E53935", "#FB8C00", "#43A047", "#1E88E5"];

/// 覆盖层界面缩放的可选值，与 AirSnip `UiScale` 的 serde 取值一致。
pub(super) const UI_SCALES: [&str; 6] = ["auto", "100", "125", "150", "175", "200"];

/// OCR 识别语言「跟随系统」。
pub(super) const OCR_LANG_AUTO: &str = "auto";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenshotSettings {
    pub enabled: bool,
    #[serde(default = "default_shortcut")]
    pub shortcut: String,
    /// 保存格式：`png` / `jpeg` / `webp`。
    #[serde(default = "default_save_format")]
    pub save_format: String,
    /// 默认保存目录；空字符串表示用系统「图片」下的默认目录。
    #[serde(default)]
    pub save_dir: String,
    /// 5 个标注色块，`#RRGGBB`。
    #[serde(default = "default_colors")]
    pub colors: Vec<String>,
    /// 覆盖层内的工具切换键与功能键。
    #[serde(default)]
    pub overlay_keys: OverlayKeys,
    /// 工具栏上隐藏的元素键（`ToolbarSlot::key`）；空数组 = 全部可见。
    #[serde(default)]
    pub hidden_toolbar: Vec<String>,
    /// OCR 识别语言：`auto` 跟随系统，否则是 BCP-47 标签（如 `zh-Hans-CN`）。
    #[serde(default = "default_ocr_language")]
    pub ocr_language: String,
    /// 覆盖层界面缩放：`auto` / `100` / `125` / `150` / `175` / `200`。
    #[serde(default = "default_ui_scale")]
    pub ui_scale: String,
}

/// 覆盖层内的快捷键：10 个工具切换键 + 长截图 / OCR 功能键。
///
/// 值是 Windows 虚拟键码（VK），`0` 表示未设置，该键在覆盖层里不生效。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct OverlayKeys {
    pub rect: u32,
    pub arrow: u32,
    pub text: u32,
    pub blur: u32,
    pub highlight: u32,
    pub glow: u32,
    pub number: u32,
    pub line: u32,
    pub ellipse: u32,
    pub freehand: u32,
    pub longshot: u32,
    pub ocr: u32,
}

impl Default for OverlayKeys {
    /// 默认值取自 AirSnip：工具键 1–9、0，长截图 S，OCR T。
    fn default() -> Self {
        let tools = airsnip::settings::ToolKeys::default();
        let hotkey = airsnip::settings::HotkeyConfig::default();
        Self {
            rect: tools.rect,
            arrow: tools.arrow,
            text: tools.text,
            blur: tools.blur,
            highlight: tools.highlight,
            glow: tools.glow,
            number: tools.number,
            line: tools.line,
            ellipse: tools.ellipse,
            freehand: tools.freehand,
            longshot: hotkey.longshot_key,
            ocr: hotkey.ocr_key,
        }
    }
}

impl OverlayKeys {
    fn to_tool_keys(self) -> airsnip::settings::ToolKeys {
        airsnip::settings::ToolKeys {
            rect: self.rect,
            arrow: self.arrow,
            text: self.text,
            blur: self.blur,
            highlight: self.highlight,
            glow: self.glow,
            number: self.number,
            line: self.line,
            ellipse: self.ellipse,
            freehand: self.freehand,
        }
    }
}

fn default_shortcut() -> String {
    DEFAULT_SHORTCUT.to_string()
}

fn default_save_format() -> String {
    SAVE_FORMATS[0].to_string()
}

pub(super) fn default_colors() -> Vec<String> {
    DEFAULT_COLORS.iter().map(|color| color.to_string()).collect()
}

fn default_ocr_language() -> String {
    OCR_LANG_AUTO.to_string()
}

fn default_ui_scale() -> String {
    UI_SCALES[0].to_string()
}

impl Default for ScreenshotSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            shortcut: default_shortcut(),
            save_format: default_save_format(),
            save_dir: String::new(),
            colors: default_colors(),
            overlay_keys: OverlayKeys::default(),
            hidden_toolbar: Vec::new(),
            ocr_language: default_ocr_language(),
            ui_scale: default_ui_scale(),
        }
    }
}

impl ScreenshotSettings {
    /// 规整到可用状态：未知格式回退 PNG，色板补齐或截断到 5 个合法颜色，
    /// 未知的工具栏键与缩放值一律丢掉（它们只可能来自手改的 JSON）。
    pub(super) fn normalized(mut self) -> Self {
        let format = self.save_format.trim().to_ascii_lowercase();
        self.save_format = match format.as_str() {
            "jpg" | "jpeg" => "jpeg".to_string(),
            "webp" => "webp".to_string(),
            _ => "png".to_string(),
        };
        self.save_dir = self.save_dir.trim().to_string();

        let fallback = default_colors();
        let mut colors: Vec<String> = self
            .colors
            .iter()
            .map(|color| normalize_color(color).unwrap_or_else(|| "#FFFFFF".to_string()))
            .collect();
        while colors.len() < DEFAULT_COLORS.len() {
            colors.push(fallback[colors.len()].clone());
        }
        colors.truncate(DEFAULT_COLORS.len());
        self.colors = colors;

        let mut hidden: Vec<String> = Vec::new();
        for key in &self.hidden_toolbar {
            let key = key.trim();
            if is_known_toolbar_key(key) && !hidden.iter().any(|kept| kept == key) {
                hidden.push(key.to_string());
            }
        }
        self.hidden_toolbar = hidden;

        let language = self.ocr_language.trim();
        self.ocr_language = if language.is_empty() {
            default_ocr_language()
        } else {
            language.to_string()
        };

        let scale = self.ui_scale.trim().to_ascii_lowercase();
        self.ui_scale = UI_SCALES
            .iter()
            .find(|known| **known == scale)
            .map(|known| known.to_string())
            .unwrap_or_else(default_ui_scale);

        self.overlay_keys = self.overlay_keys.normalized();
        self
    }

    /// 转换成 AirSnip 的配置：只覆盖宿主管理的几项，其余保持它自己的默认值。
    pub(super) fn to_airsnip_config(&self) -> airsnip::settings::Config {
        let mut colors = airsnip::editor::DEFAULT_COLORS;
        for (index, color) in self.colors.iter().take(colors.len()).enumerate() {
            if let Some(rgba) = color_rgba(color) {
                colors[index] = rgba;
            }
        }
        airsnip::settings::Config {
            // 覆盖层用 last_ext / last_dir 预填另存为对话框，也就是「默认格式 + 默认目录」。
            last_save_ext: Some(self.save_format.clone()),
            last_save_dir: (!self.save_dir.is_empty()).then(|| self.save_dir.clone()),
            swatch_colors: colors,
            tool_keys: self.overlay_keys.to_tool_keys(),
            hotkey: airsnip::settings::HotkeyConfig {
                longshot_key: self.overlay_keys.longshot,
                ocr_key: self.overlay_keys.ocr,
                ..Default::default()
            },
            toolbar: airsnip::settings::ToolbarConfig {
                hidden: self.hidden_toolbar.clone(),
            },
            ocr: airsnip::settings::OcrConfig {
                language: self.ocr_language.clone(),
                ..Default::default()
            },
            ui_scale: parse_ui_scale(&self.ui_scale),
            ..Default::default()
        }
    }
}

impl OverlayKeys {
    /// 越界的键码（手改 JSON 或旧版本遗留）按「未设置」处理。
    fn normalized(self) -> Self {
        let fix = |vk: u32| if vk <= 0xFF { vk } else { 0 };
        Self {
            rect: fix(self.rect),
            arrow: fix(self.arrow),
            text: fix(self.text),
            blur: fix(self.blur),
            highlight: fix(self.highlight),
            glow: fix(self.glow),
            number: fix(self.number),
            line: fix(self.line),
            ellipse: fix(self.ellipse),
            freehand: fix(self.freehand),
            longshot: fix(self.longshot),
            ocr: fix(self.ocr),
        }
    }
}

/// 配置字符串 → AirSnip 的缩放枚举（已由 `normalized` 保证取值合法）。
fn parse_ui_scale(value: &str) -> airsnip::settings::UiScale {
    match value {
        "100" => airsnip::settings::UiScale::P100,
        "125" => airsnip::settings::UiScale::P125,
        "150" => airsnip::settings::UiScale::P150,
        "175" => airsnip::settings::UiScale::P175,
        "200" => airsnip::settings::UiScale::P200,
        _ => airsnip::settings::UiScale::Auto,
    }
}

/// 是否是工具栏上真实存在的元素键。
fn is_known_toolbar_key(key: &str) -> bool {
    airsnip::settings::ToolbarSlot::all().any(|slot| slot.key() == key)
}

/// `#RRGGBB` / `#RGB` / 不带井号 → 规整的 `#RRGGBB`；无法解析时为 None。
fn normalize_color(color: &str) -> Option<String> {
    let rgba = color_rgba(color)?;
    Some(format!("#{:02X}{:02X}{:02X}", rgba[0], rgba[1], rgba[2]))
}

/// 颜色字符串 → 不透明 RGBA。
fn color_rgba(color: &str) -> Option<[u8; 4]> {
    let hex = color.trim().trim_start_matches('#');
    let (r, g, b) = match hex.len() {
        3 => {
            let mut digits = hex.chars().map(|c| c.to_digit(16).map(|d| (d * 17) as u8));
            (
                digits.next()??,
                digits.next()??,
                digits.next()??,
            )
        }
        6 => {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            (r, g, b)
        }
        _ => return None,
    };
    Some([r, g, b, 255])
}

fn settings_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("无法定位应用数据目录：{error}"))?;
    Ok(dir.join(FILE_NAME))
}

/// 读取设置：文件缺失或损坏时回退默认值，不让截图功能因为一个坏 JSON 起不来。
pub(super) fn load(app: &AppHandle) -> ScreenshotSettings {
    settings_path(app)
        .ok()
        .and_then(|path| std::fs::read_to_string(path).ok())
        .and_then(|text| serde_json::from_str(&text).ok())
        .map(ScreenshotSettings::normalized)
        .unwrap_or_default()
}

pub(super) fn save(app: &AppHandle, settings: &ScreenshotSettings) -> Result<(), String> {
    let path = settings_path(app)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("无法创建截图设置目录：{error}"))?;
    }
    let text = serde_json::to_string_pretty(settings)
        .map_err(|error| format!("无法序列化截图设置：{error}"))?;
    std::fs::write(&path, text).map_err(|error| format!("无法写入截图设置：{error}"))
}
