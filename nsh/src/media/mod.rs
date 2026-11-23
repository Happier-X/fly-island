//! Windows media sessions (SMTC), system volume and the output peak meter.
//!
//! Ported from the Tauri media module: the state is polled instead of using the
//! SMTC event watcher, so every call opens a short-lived MTA thread.

#![allow(dead_code)]

mod audio;
mod audio_device;
mod com;
mod input;
mod service;
mod session;
mod timeline;

pub use service::{AudioLevel, MediaState};

use service::MediaCommand;

pub fn state() -> Result<MediaState, String> {
    let mut state = service::get_state(None)?;
    state.tint = tint_of(state.thumbnail.as_deref());
    Ok(state)
}

pub fn toggle_shuffle() -> Result<MediaState, String> {
    service::run_command(MediaCommand::ToggleShuffle, None)
}

pub fn cycle_repeat() -> Result<MediaState, String> {
    service::run_command(MediaCommand::CycleRepeat, None)
}

fn tint_of(url: Option<&str>) -> Option<String> {
    use base64::{engine::general_purpose::STANDARD, Engine as _};
    let encoded = url?.split_once(',')?.1;
    let bytes = STANDARD.decode(encoded).ok()?;
    let image = image::load_from_memory(&bytes).ok()?.to_rgba8();
    let (mut r, mut g, mut b, mut count) = (0u64, 0u64, 0u64, 0u64);
    for pixel in image.pixels() {
        if pixel[3] < 16 {
            continue;
        }
        r += u64::from(pixel[0]);
        g += u64::from(pixel[1]);
        b += u64::from(pixel[2]);
        count += 1;
    }
    if count == 0 {
        return None;
    }
    Some(format!("#{:02X}{:02X}{:02X}", r / count, g / count, b / count))
}

pub fn play_pause() -> Result<MediaState, String> {
    service::run_command(MediaCommand::PlayPause, None)
}

pub fn next() -> Result<MediaState, String> {
    service::run_command(MediaCommand::Next, None)
}

pub fn previous() -> Result<MediaState, String> {
    service::run_command(MediaCommand::Previous, None)
}

pub fn seek(position_ms: i64) -> Result<MediaState, String> {
    service::seek(position_ms, None)
}

pub fn set_volume(level: f32) -> Result<MediaState, String> {
    service::set_volume(level)
}

pub fn audio_level() -> Result<AudioLevel, String> {
    service::get_audio_level()
}
