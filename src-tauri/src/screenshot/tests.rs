//! 截图设置的单元测试。
//!
//! 单独成文件是为了不占 `config.rs` 的内联测试行数预算（AGENTS.md）。

use super::config::*;

#[test]
fn default_is_enabled_with_shortcut() {
    let settings = ScreenshotSettings::default();
    assert!(settings.enabled);
    assert_eq!(settings.shortcut, DEFAULT_SHORTCUT);
    assert_eq!(settings.save_format, "png");
    assert!(settings.save_dir.is_empty());
    assert_eq!(settings.colors.len(), DEFAULT_COLORS.len());
}

#[test]
fn missing_fields_fall_back_to_defaults() {
    let settings: ScreenshotSettings = serde_json::from_str(r#"{"enabled":false}"#).unwrap();
    let settings = settings.normalized();
    assert!(!settings.enabled);
    assert_eq!(settings.shortcut, DEFAULT_SHORTCUT);
    assert_eq!(settings.save_format, "png");
    assert_eq!(settings.colors, default_colors());
}

#[test]
fn unknown_format_falls_back_to_png() {
    let settings = ScreenshotSettings {
        save_format: "bmp".to_string(),
        ..Default::default()
    }
    .normalized();
    assert_eq!(settings.save_format, "png");
}

#[test]
fn jpg_is_stored_as_jpeg() {
    let settings = ScreenshotSettings {
        save_format: "JPG".to_string(),
        ..Default::default()
    }
    .normalized();
    assert_eq!(settings.save_format, "jpeg");
}

#[test]
fn colors_are_normalized_and_padded() {
    let settings = ScreenshotSettings {
        colors: vec!["#fff".to_string(), "e53935".to_string(), "zzz".to_string()],
        ..Default::default()
    }
    .normalized();
    assert_eq!(settings.colors.len(), DEFAULT_COLORS.len());
    assert_eq!(settings.colors[0], "#FFFFFF");
    assert_eq!(settings.colors[1], "#E53935");
    assert_eq!(settings.colors[2], "#FFFFFF", "无法解析的退回白色");
    assert_eq!(settings.colors[3], DEFAULT_COLORS[3], "缺的补默认值");
}

#[test]
fn airsnip_config_carries_the_host_settings() {
    let settings = ScreenshotSettings {
        save_format: "webp".to_string(),
        save_dir: r"D:\shots".to_string(),
        colors: vec![
            "#010203".to_string(),
            "#111213".to_string(),
            "#212223".to_string(),
            "#313233".to_string(),
            "#414243".to_string(),
        ],
        ..Default::default()
    }
    .normalized();
    let config = settings.to_airsnip_config();
    assert_eq!(config.last_save_ext.as_deref(), Some("webp"));
    assert_eq!(config.last_save_dir.as_deref(), Some(r"D:\shots"));
    assert_eq!(config.swatch_colors[0], [1, 2, 3, 255]);
    assert_eq!(config.swatch_colors[4], [0x41, 0x42, 0x43, 255]);
}

#[test]
fn empty_save_dir_leaves_airsnip_default() {
    let config = ScreenshotSettings::default().to_airsnip_config();
    assert_eq!(config.last_save_dir, None);
}

#[test]
fn overlay_keys_default_to_the_airsnip_bindings() {
    let keys = OverlayKeys::default();
    assert_eq!(keys.rect, 0x31, "1");
    assert_eq!(keys.freehand, 0x30, "0");
    assert_eq!(keys.longshot, 0x53, "S");
    assert_eq!(keys.ocr, 0x54, "T");
}

#[test]
fn unknown_toolbar_keys_are_dropped_and_duplicates_collapsed() {
    let settings = ScreenshotSettings {
        hidden_toolbar: vec![
            "tool.rect".to_string(),
            "nope".to_string(),
            " tool.rect ".to_string(),
            "color.unused".to_string(),
            "output.ocr".to_string(),
        ],
        ..Default::default()
    }
    .normalized();
    assert_eq!(settings.hidden_toolbar, vec!["tool.rect", "output.ocr"]);
}

#[test]
fn unknown_ui_scale_falls_back_to_auto() {
    let settings = ScreenshotSettings {
        ui_scale: "300".to_string(),
        ..Default::default()
    }
    .normalized();
    assert_eq!(settings.ui_scale, "auto");
    // 大小写和空白不该让一个合法值失效
    let settings = ScreenshotSettings {
        ui_scale: " 150 ".to_string(),
        ..Default::default()
    }
    .normalized();
    assert_eq!(settings.ui_scale, "150");
}

#[test]
fn blank_ocr_language_means_auto() {
    let settings = ScreenshotSettings {
        ocr_language: "   ".to_string(),
        ..Default::default()
    }
    .normalized();
    assert_eq!(settings.ocr_language, OCR_LANG_AUTO);
}

#[test]
fn out_of_range_key_codes_are_treated_as_unbound() {
    let settings = ScreenshotSettings {
        overlay_keys: OverlayKeys {
            rect: 0x1_0000,
            ocr: 0,
            ..Default::default()
        },
        ..Default::default()
    }
    .normalized();
    assert_eq!(settings.overlay_keys.rect, airsnip::settings::UNBOUND);
    assert_eq!(settings.overlay_keys.ocr, airsnip::settings::UNBOUND);
}

#[test]
fn airsnip_config_carries_the_overlay_settings() {
    let settings = ScreenshotSettings {
        overlay_keys: OverlayKeys {
            rect: 0x41,
            longshot: 0x42,
            ocr: 0x43,
            ..Default::default()
        },
        hidden_toolbar: vec!["tool.ellipse".to_string(), "output.pin".to_string()],
        ocr_language: "en-US".to_string(),
        ui_scale: "125".to_string(),
        ..Default::default()
    }
    .normalized();
    let config = settings.to_airsnip_config();
    assert_eq!(config.tool_keys.rect, 0x41);
    assert_eq!(config.tool_keys.freehand, OverlayKeys::default().freehand, "没改的键保持默认");
    assert_eq!(config.hotkey.longshot_key, 0x42);
    assert_eq!(config.hotkey.ocr_key, 0x43);
    assert_eq!(config.toolbar.hidden, vec!["tool.ellipse", "output.pin"]);
    assert!(!config.toolbar.visible(airsnip::settings::ToolbarSlot::Tool(
        airsnip::editor::Tool::Ellipse
    )));
    assert!(config.toolbar.visible(airsnip::settings::ToolbarSlot::Tool(
        airsnip::editor::Tool::Rect
    )));
    assert_eq!(config.ocr.language, "en-US");
    assert!(!config.ocr.auto_copy, "自动复制不在这里管");
    assert_eq!(config.ui_scale, airsnip::settings::UiScale::P125);
}

#[test]
fn defaults_match_airsnip_when_nothing_was_touched() {
    let config = ScreenshotSettings::default().to_airsnip_config();
    let defaults = airsnip::settings::ToolKeys::default();
    assert_eq!(config.tool_keys.rect, defaults.rect);
    assert_eq!(config.tool_keys.freehand, defaults.freehand);
    assert_eq!(config.hotkey.longshot_key, airsnip::settings::HotkeyConfig::default().longshot_key);
    assert_eq!(config.hotkey.ocr_key, airsnip::settings::HotkeyConfig::default().ocr_key);
    assert!(config.toolbar.hidden.is_empty());
    assert_eq!(config.ocr.language, OCR_LANG_AUTO);
    assert_eq!(config.ui_scale, airsnip::settings::UiScale::Auto);
}
