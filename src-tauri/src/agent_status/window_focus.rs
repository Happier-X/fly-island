//! Bringing Cindy's own window to the front when a session is clicked.
//!
//! Cindy runs every agent session in one application window, so there is no terminal tab or
//! editor window to look for: match the executable name and activate it.

use crate::win32;

/// Executable file name without `.exe`, lowercase: how the running program is matched.
pub(super) fn app_name(image_path: &str) -> String {
    std::path::Path::new(image_path)
        .file_stem()
        .map(|name| name.to_string_lossy().to_lowercase())
        .unwrap_or_default()
}

/// Brings `app`'s frontmost visible window to the front. False when no such window exists.
pub(super) fn focus_app_window(app: &str) -> bool {
    let mut focused = false;
    let _ = win32::window::for_each_top_level_window(|window| {
        if focused || !win32::window::is_visible(window) || win32::window::has_owner(window) {
            return;
        }
        let process_id = win32::process::window_process_id(window);
        let matches = win32::process::image_path(process_id)
            .is_some_and(|path| app_name(&path) == app);
        if !matches {
            return;
        }
        win32::window::allow_set_foreground_window(process_id);
        if !win32::window::force_foreground(window) {
            win32::window::flash(window);
        }
        focused = true;
    });
    focused
}

/// Whether `app`'s window is the one in front right now: the user is already looking at it.
pub(super) fn app_window_in_front(app: &str) -> bool {
    let window = win32::window::foreground_window();
    if window.is_invalid() {
        return false;
    }
    let process_id = win32::process::window_process_id(window);
    win32::process::image_path(process_id).is_some_and(|path| app_name(&path) == app)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_app_name_drops_the_folder_and_extension() {
        assert_eq!(app_name(r"C:\Users\me\AppData\Local\Programs\Cindy\Cindy.exe"), "cindy");
        assert_eq!(app_name("/usr/local/bin/Code"), "code");
        assert_eq!(app_name(""), "");
    }
}
