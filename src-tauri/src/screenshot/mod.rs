//! 截图：全局热键唤起 AirSnip 的覆盖窗完成框选与标注。
//!
//! 抓屏、覆盖窗、标注、长截图、OCR、贴图都由 `crates/airsnip` 实现；本模块只负责
//! 「宿主侧的三件事」——启动 AirSnip 的消息线程、注册全局热键、暴露设置命令。
//!
//! 覆盖窗是原生 HWND，跑在 AirSnip 自己的线程上，因此这里不需要新增 Tauri 窗口，
//! 也不碰 tao 的事件循环。

mod config;
mod hotkey;
#[cfg(test)]
mod tests;

use std::sync::{Mutex, OnceLock};
use tauri::{AppHandle, Manager};

use config::ScreenshotSettings;

static APP: OnceLock<AppHandle> = OnceLock::new();
static HOST: Mutex<Option<airsnip::HostHandle>> = Mutex::new(None);

/// 启动截图能力。失败只影响截图，不阻断应用其它部分。
pub(crate) fn init(app: &AppHandle) -> Result<(), String> {
    let config_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("无法定位应用数据目录：{error}"))?
        .join("screenshot");
    std::fs::create_dir_all(&config_dir)
        .map_err(|error| format!("无法创建截图配置目录：{error}"))?;

    // AirSnip 的托管入口：建隐藏消息窗并起它自己的消息循环。
    // 配置由这里构造，AirSnip 不再读自己的 config.toml。
    let settings = config::load(app);
    let host = airsnip::start_host(Some(config_dir), settings.to_airsnip_config())?;
    if let Ok(mut slot) = HOST.lock() {
        *slot = Some(host);
    }

    hotkey::start(settings.enabled, &settings.shortcut);
    let _ = APP.set(app.clone());
    Ok(())
}

/// 把设置推给 AirSnip 的覆盖层线程。
fn apply_airsnip_config(settings: &ScreenshotSettings) {
    let host = HOST.lock().ok().and_then(|slot| *slot);
    if let Some(host) = host {
        if let Err(error) = host.apply_config(settings.to_airsnip_config()) {
            eprintln!("failed to push screenshot settings: {error}");
        }
    }
}

/// 触发一次截图。覆盖窗已在运行时 AirSnip 内部会忽略。
pub(crate) fn capture() {
    let host = HOST.lock().ok().and_then(|slot| *slot);
    match host {
        Some(host) => {
            if let Err(error) = host.request_capture() {
                eprintln!("failed to start screenshot: {error}");
            }
        }
        None => eprintln!("screenshot host is not ready"),
    }
}

#[tauri::command]
pub fn get_screenshot_settings(app: AppHandle) -> ScreenshotSettings {
    config::load(&app)
}

#[tauri::command]
pub async fn set_screenshot_settings(
    app: AppHandle,
    settings: ScreenshotSettings,
) -> Result<(), String> {
    let settings = settings.normalized();
    config::save(&app, &settings)?;
    // 覆盖层线程持有自己那份配置快照，改完要推过去，下一次截图才用新值。
    apply_airsnip_config(&settings);
    if settings.enabled {
        hotkey::rebind(true, &settings.shortcut)?;
    } else {
        hotkey::disable()?;
    }
    Ok(())
}

/// 弹出「选择文件夹」对话框，返回选中目录；取消时返回 null。
#[tauri::command]
pub async fn pick_screenshot_directory(initial: Option<String>) -> Result<Option<String>, String> {
    Ok(airsnip::host::pick_save_directory(initial.as_deref()))
}

/// 手动触发一次截图（岛上的按钮走这里，不依赖全局热键）。
#[tauri::command]
pub fn capture_screenshot() -> Result<(), String> {
    capture();
    Ok(())
}

/// 工具栏上一个可隐藏的元素。
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolbarSlotInfo {
    /// 配置里的稳定键，写进 `hiddenToolbar`。
    pub key: String,
    /// 面板上的短标签。
    pub label: String,
}

/// 工具栏的一组元素。
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolbarGroupInfo {
    /// 组标识（`tools` / `colors` / `edit` / `output` / `file`）。
    pub id: String,
    pub title: String,
    pub slots: Vec<ToolbarSlotInfo>,
}

/// 工具栏结构：界面上的分组与每项文案都由 AirSnip 提供，这里不另写一份清单。
#[tauri::command]
pub fn get_screenshot_toolbar_groups() -> Vec<ToolbarGroupInfo> {
    use airsnip::settings::ToolbarGroup;
    ToolbarGroup::ALL
        .into_iter()
        .map(|group| ToolbarGroupInfo {
            id: match group {
                ToolbarGroup::Tools => "tools",
                ToolbarGroup::Colors => "colors",
                ToolbarGroup::Edit => "edit",
                ToolbarGroup::Output => "output",
                ToolbarGroup::File => "file",
            }
            .to_string(),
            title: group.title().to_string(),
            slots: group
                .slots()
                .iter()
                .map(|slot| ToolbarSlotInfo {
                    key: slot.key().to_string(),
                    label: slot.label().to_string(),
                })
                .collect(),
        })
        .collect()
}

/// 系统已安装的 OCR 语言：`auto` 在前，其余是真实的 BCP-47 标签。
///
/// 枚举语言要初始化 COM 且属于慢调用，放到阻塞线程里跑，不占命令线程。
#[tauri::command]
pub async fn get_screenshot_ocr_languages() -> Vec<String> {
    tauri::async_runtime::spawn_blocking(|| {
        let installed = airsnip::ocr::available_languages().unwrap_or_default();
        let mut languages = vec![config::OCR_LANG_AUTO.to_string()];
        languages.extend(installed);
        languages
    })
    .await
    .unwrap_or_else(|_| vec![config::OCR_LANG_AUTO.to_string()])
}

/// 虚拟键码 → 显示名（`"1"`、`"F1"`、`"未设置"`），与覆盖层里的叫法一致。
#[tauri::command]
pub fn screenshot_key_names(vks: Vec<u32>) -> Vec<String> {
    vks.into_iter()
        .map(airsnip::settings::hotkey::vk_name)
        .collect()
}

/// This module's Tauri commands. `lib.rs` chains every module's list into one
/// `generate_handler!`, so adding a command only touches this file.
macro_rules! tauri_commands {
    ($next:ident [$($done:tt)*] $($rest:tt)*) => {
        $next!([$($done)*
            crate::screenshot::get_screenshot_settings,
            crate::screenshot::set_screenshot_settings,
            crate::screenshot::capture_screenshot,
            crate::screenshot::pick_screenshot_directory,
            crate::screenshot::get_screenshot_toolbar_groups,
            crate::screenshot::get_screenshot_ocr_languages,
            crate::screenshot::screenshot_key_names,
        ] $($rest)*)
    };
}
pub(crate) use tauri_commands;
