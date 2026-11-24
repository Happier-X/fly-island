//! Screenshot capability for the NSH host.
//!
//! AirSnip already ships a hosted entry point: start_host creates its hidden
//! message window on a background thread, and request_capture opens the
//! capture overlay. This module only owns NSH settings and a separate global
//! shortcut thread, replacing the Tauri command layer from the Electron app.

mod hotkey;
mod settings;

use std::sync::Mutex;

pub use settings::ScreenshotSettings;

static HOST: Mutex<Option<airsnip::HostHandle>> = Mutex::new(None);
static LAST_HOTKEY: Mutex<Option<(String, bool)>> = Mutex::new(None);
static LAST_ERROR: Mutex<Option<String>> = Mutex::new(None);

pub fn load() -> ScreenshotSettings {
    settings::load()
}

pub fn default_shortcut() -> String {
    settings::DEFAULT_SHORTCUT.to_string()
}

pub fn last_error() -> Option<String> {
    LAST_ERROR.lock().ok().and_then(|error| error.clone())
}

pub fn is_ready() -> bool {
    HOST.lock().map(|slot| slot.is_some()).unwrap_or(false)
}

/// Start the AirSnip host thread and the screenshot shortcut. Safe to call once.
pub fn start(settings: &ScreenshotSettings) -> Result<(), String> {
    if is_ready() {
        return Ok(());
    }
    let dir = settings::config_dir();
    std::fs::create_dir_all(&dir)
        .map_err(|error| format!("无法创建截图目录 {}：{error}", dir.display()))?;
    let save_dir = settings.effective_save_dir();
    if let Err(error) = std::fs::create_dir_all(&save_dir) {
        log::warn!("screenshot save dir {}: {error}", save_dir.display());
    }

    // Keep the config directory for AirSnip runtime state (OCR cache, recent
    // save format). The rest of the config is injected here, not read from toml.
    let host = match airsnip::start_host(Some(dir), settings.to_airsnip_config()) {
        Ok(host) => host,
        Err(error) => {
            set_last_error(Some(error.clone()));
            return Err(error);
        }
    };
    if let Ok(mut slot) = HOST.lock() {
        *slot = Some(host);
    }
    remember_hotkey(settings);
    log::info!("screenshot host ready, shortcut={}", settings.shortcut);
    match hotkey::start(&settings.shortcut, settings.enabled) {
        Ok(()) => {
            set_last_error(None);
            Ok(())
        }
        Err(error) => {
            set_last_error(Some(error.clone()));
            Err(error)
        }
    }
}

/// Persist and apply a settings change from the settings window.
pub fn save(settings: &ScreenshotSettings) -> Result<ScreenshotSettings, String> {
    let mut settings = settings.clone().normalized();
    let desired = (settings.shortcut.clone(), settings.enabled);
    let changed = LAST_HOTKEY
        .lock()
        .map(|last| last.as_ref() != Some(&desired))
        .unwrap_or(true);
    if changed {
        match hotkey::set_shortcut(&settings.shortcut, settings.enabled) {
            Ok(label) => {
                settings.shortcut = label;
                remember_hotkey(&settings);
            }
            Err(error) => {
                set_last_error(Some(error.clone()));
                return Err(error);
            }
        }
    }

    settings::write(&settings)?;
    let host = HOST.lock().ok().and_then(|slot| *slot);
    if let Some(host) = host
        && let Err(error) = host.apply_config(settings.to_airsnip_config())
    {
        set_last_error(Some(error.clone()));
        return Err(error);
    }
    set_last_error(None);
    Ok(settings)
}

/// Trigger one capture without the global shortcut (tests, tray, future buttons).
pub fn capture() {
    log::info!("screenshot capture requested");
    let host = HOST.lock().ok().and_then(|slot| *slot);
    match host {
        Some(host) => {
            if let Err(error) = host.request_capture() {
                log::error!("screenshot: {error}");
            }
        }
        None => log::error!("screenshot host is not ready"),
    }
}

fn set_last_error(error: Option<String>) {
    if let Ok(mut slot) = LAST_ERROR.lock() {
        *slot = error;
    }
}

fn remember_hotkey(settings: &ScreenshotSettings) {
    if let Ok(mut last) = LAST_HOTKEY.lock() {
        *last = Some((settings.shortcut.clone(), settings.enabled));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_use_ctrl_shift_a_and_png() {
        let settings = ScreenshotSettings::default();
        assert_eq!(settings.shortcut, "Ctrl+Shift+A");
        assert_eq!(settings.save_format, "png");
    }

    #[test]
    fn airsnip_config_uses_the_configured_directory() {
        let settings = ScreenshotSettings {
            save_dir: r"C:\Users\test\Pictures\Shots".into(),
            save_format: "jpg".into(),
            ..Default::default()
        }
        .normalized();
        let config = settings.to_airsnip_config();
        assert_eq!(config.last_save_ext.as_deref(), Some("jpeg"));
        assert_eq!(
            config.last_save_dir.as_deref(),
            Some(r"C:\Users\test\Pictures\Shots")
        );
    }
}
