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
    service::get_state(None)
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
