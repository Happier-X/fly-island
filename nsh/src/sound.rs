//! Cindy notification sounds: the three original thumb-piano WAVs, scaled to
//! 0.7 in memory so the system volume is never touched.

use std::{
    collections::HashMap,
    sync::OnceLock,
};

use windows::{
    core::PCWSTR,
    Win32::Media::Audio::{PlaySoundW, SND_ASYNC, SND_MEMORY, SND_NODEFAULT},
};

const COMPLETE: &[u8] = include_bytes!("../assets/sounds/thumb-piano/complete.wav");
const QUESTION: &[u8] = include_bytes!("../assets/sounds/thumb-piano/question.wav");
const ERROR: &[u8] = include_bytes!("../assets/sounds/thumb-piano/error.wav");

static BUFFERS: OnceLock<HashMap<&'static str, Vec<u8>>> = OnceLock::new();

fn buffers() -> &'static HashMap<&'static str, Vec<u8>> {
    BUFFERS.get_or_init(|| {
        let mut map = HashMap::new();
        map.insert("complete", scaled(COMPLETE));
        map.insert("question", scaled(QUESTION));
        map.insert("error", scaled(ERROR));
        map
    })
}

/// Multiplies 16-bit PCM samples by 0.7, keeping the WAV header intact.
fn scaled(wav: &[u8]) -> Vec<u8> {
    let mut out = wav.to_vec();
    let mut offset = 12;
    let mut data = None;
    while offset + 8 <= out.len() {
        let id = &out[offset..offset + 4];
        let size = u32::from_le_bytes(out[offset + 4..offset + 8].try_into().unwrap_or([0; 4])) as usize;
        let body = offset + 8;
        if id == b"data" {
            data = Some((body, size.min(out.len().saturating_sub(body))));
            break;
        }
        offset = body.saturating_add(size + (size & 1));
    }
    if let Some((start, length)) = data {
        let mut index = start;
        while index + 1 < start + length {
            let sample = i16::from_le_bytes([out[index], out[index + 1]]);
            let scaled = (f32::from(sample) * 0.7).round() as i16;
            out[index..index + 2].copy_from_slice(&scaled.to_le_bytes());
            index += 2;
        }
    }
    out
}

pub fn play(kind: &str) {
    let Some(buffer) = buffers().get(kind) else {
        return;
    };
    let pointer = buffer.as_ptr() as *const u16;
    // SAFETY: the buffer is stored in a static map for the program's lifetime;
    // SND_MEMORY reads it as a WAV image and SND_ASYNC returns before playback ends.
    let _ = unsafe {
        PlaySoundW(
            PCWSTR(pointer),
            None,
            SND_MEMORY | SND_ASYNC | SND_NODEFAULT,
        )
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn samples_are_scaled_to_seventy_percent() {
        let scaled = scaled(COMPLETE);
        let start = 44;
        let original = i16::from_le_bytes([COMPLETE[start], COMPLETE[start + 1]]);
        let after = i16::from_le_bytes([scaled[start], scaled[start + 1]]);
        if original != 0 {
            assert!((f32::from(after) - f32::from(original) * 0.7).abs() <= 1.0);
        }
    }
}
