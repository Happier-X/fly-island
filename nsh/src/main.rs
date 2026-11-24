//! Runs the Rust application directly on the machine's native backend.
//!
//! `nsh run` builds its own runner from `nsh.json`; this binary is for
//! `cargo run`. Translations are compiled in by `nsh`, so `cargo run` shows
//! translation keys instead of text.

#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

#[cfg(target_os = "linux")]
use nsh_platform_linux as platform;
#[cfg(target_os = "macos")]
use nsh_platform_macos as platform;
#[cfg(target_os = "windows")]
use nsh_platform_windows as platform;

#[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
fn main() -> impl std::process::Termination {
    env_logger::init();
    platform::run_app(
        platform::AppSource::Rust(platform::Root::of::<nsh_app_focusd_island_nsh::App>()),
        platform::AppOptions {
            title: "FocuSD Island (NSH spike)".to_owned(),
            #[cfg(any(target_os = "linux", target_os = "windows"))]
            app_id: "com.focusd.island.nsh".to_owned(),
            size: (320, 40),
            transparent: true,
            skip_taskbar: true,
            ..Default::default()
        },
    )
}

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
compile_error!(
    "Run this desktop binary on macOS, Windows or Linux."
);
