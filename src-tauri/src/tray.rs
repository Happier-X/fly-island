//! System tray icon and menu. One item shows or hides the island, named for what
//! it will do: the visibility is checked whenever the cursor reaches the tray
//! icon, before the menu opens.

use std::sync::{Mutex, PoisonError};

use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    App, AppHandle, Manager, Wry,
};

use crate::{island, settings_window};

/// The frontend supplies the resolved locale, including its stored preference and system fallback.
fn menu_labels(locale: &str, island_visible: bool) -> [&'static str; 3] {
    match (locale, island_visible) {
        ("en", true) => ["Hide Island", "Settings", "Quit"],
        ("en", false) => ["Show Island", "Settings", "Quit"],
        (_, true) => ["隐藏灵动岛", "设置", "退出"],
        (_, false) => ["显示灵动岛", "设置", "退出"],
    }
}

struct TrayMenu {
    toggle: MenuItem<Wry>,
    settings: MenuItem<Wry>,
    quit: MenuItem<Wry>,
    // Keep language and visibility together so a hover cannot overwrite a language change.
    state: Mutex<(String, bool)>,
}

fn island_visible(app: &AppHandle) -> bool {
    island::main_window(app)
        .and_then(|window| window.is_visible().map_err(|error| error.to_string()))
        .unwrap_or(true)
}

/// Names the toggle item for the island's visibility now.
fn refresh(app: &AppHandle) {
    let Some(menu) = app.try_state::<TrayMenu>() else { return };
    let visible = island_visible(app);
    let mut state = menu.state.lock().unwrap_or_else(PoisonError::into_inner);
    if state.1 == visible {
        return;
    }
    if menu.toggle.set_text(menu_labels(&state.0, visible)[0]).is_ok() {
        state.1 = visible;
    }
}

#[tauri::command]
pub async fn set_tray_language(app: AppHandle, locale: String) -> Result<(), String> {
    if !matches!(locale.as_str(), "zh-CN" | "en") {
        return Err(format!("Unsupported tray language: {locale}"));
    }
    let (sender, receiver) = std::sync::mpsc::channel();
    let target = app.clone();
    // Serialize menu updates with tray callbacks on the UI thread; holding the state
    // lock while waiting for the UI thread could deadlock a simultaneous hover.
    app.run_on_main_thread(move || {
        let _ = sender.send(apply_language(&target, locale));
    })
    .map_err(|error| error.to_string())?;
    tauri::async_runtime::spawn_blocking(move || receiver.recv())
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())?
}

fn apply_language(app: &AppHandle, locale: String) -> Result<(), String> {
    let menu = app.state::<TrayMenu>();
    let visible = island_visible(app);
    let mut state = menu.state.lock().unwrap_or_else(PoisonError::into_inner);
    let labels = menu_labels(&locale, visible);
    menu.toggle.set_text(labels[0]).map_err(|error| error.to_string())?;
    menu.settings.set_text(labels[1]).map_err(|error| error.to_string())?;
    menu.quit.set_text(labels[2]).map_err(|error| error.to_string())?;
    *state = (locale, visible);
    Ok(())
}

pub(crate) fn build_tray(app: &App) -> tauri::Result<()> {
    let labels = menu_labels("zh-CN", true);
    let toggle = MenuItem::with_id(app, "toggle", labels[0], true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "settings", labels[1], true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", labels[2], true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&toggle, &settings, &quit])?;
    app.manage(TrayMenu { toggle, settings, quit, state: Mutex::new(("zh-CN".into(), true)) });

    let mut tray = TrayIconBuilder::new()
        .tooltip("fly-island")
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "toggle" => {
                if island_visible(app) {
                    island::hide_island(app);
                } else {
                    let _ = island::show_island(app);
                }
                refresh(app);
            }
            "settings" => {
                if let Err(error) = settings_window::request(app) {
                    eprintln!("failed to open settings: {error}");
                }
            }
            "quit" => {
                settings_window::remember_bounds(app);
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| match event {
            // The cursor reaching the icon comes before any click, so the menu opens with the
            // right show / hide text even when the island was hidden some other way.
            TrayIconEvent::Enter { .. } | TrayIconEvent::Move { .. } => refresh(tray.app_handle()),
            TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } => {
                let _ = island::show_island(tray.app_handle());
                refresh(tray.app_handle());
            }
            _ => {}
        });

    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }

    tray.build(app)?;
    Ok(())
}

macro_rules! tauri_commands {
    ($next:ident [$($done:tt)*] $($rest:tt)*) => {
        $next!([$($done)* crate::tray::set_tray_language,] $($rest)*)
    };
}
pub(crate) use tauri_commands;

#[cfg(test)]
mod tests {
    use super::menu_labels;

    #[test]
    fn menus_follow_language_and_visibility() {
        assert_eq!(menu_labels("zh-CN", true), ["隐藏灵动岛", "设置", "退出"]);
        assert_eq!(menu_labels("zh-CN", false), ["显示灵动岛", "设置", "退出"]);
        assert_eq!(menu_labels("en", true), ["Hide Island", "Settings", "Quit"]);
        assert_eq!(menu_labels("en", false), ["Show Island", "Settings", "Quit"]);
    }
}
