//! Clipboard history: text and images, favorites, and a small JSON store.
//!
//! Reading is polling (NSH has no change event). Images are normalised to a
//! PNG data URL so they can be written back, and are kept for the session
//! only because the clipboard URL is temporary.

use std::{
    fs,
    io::Cursor,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
    time::Duration,
};

use base64::{engine::general_purpose::STANDARD, Engine as _};
use nsh_core::prelude::*;
use serde::{Deserialize, Serialize};

const POLL_INTERVAL: Duration = Duration::from_millis(800);
const MAX_ITEMS: usize = 30;

static NEXT_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClipKind {
    Text,
    Image,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClipItem {
    pub id: u64,
    pub kind: ClipKind,
    pub text: String,
    pub image_url: Option<String>,
    pub width: u32,
    pub height: u32,
    pub favorite: bool,
}

pub struct ClipboardHistory {
    pub items: Signal<Vec<ClipItem>>,
}

impl ClipboardHistory {
    pub fn start() -> Self {
        let items = signal(load());
        let last_text = signal(String::new());
        let last_image = signal(String::new());
        let read = move || {
            spawn(async move {
                let text = clipboard_string().await;
                let trimmed = text.trim();
                if !trimmed.is_empty() {
                    if last_text.get() == trimmed {
                        return;
                    }
                    last_text.set(trimmed.to_string());
                    last_image.set(String::new());
                    capture_text(items, trimmed.to_string());
                    return;
                }
                if !nsh_core::prelude::os().is_desktop() {
                    return;
                }
                if let Some(image) = clipboard_image().await {
                    let key = format!("{}x{}", image.size.width.0, image.size.height.0);
                    if last_image.get() == key {
                        return;
                    }
                    last_image.set(key);
                    last_text.set(String::new());
                    let url = to_png_url(&image.url).unwrap_or(image.url);
                    capture_image(
                        items,
                        url,
                        image.size.width.0 as u32,
                        image.size.height.0 as u32,
                    );
                }
            });
        };
        read();
        set_interval(POLL_INTERVAL, read);
        Self { items }
    }
}

fn to_png_url(url: &str) -> Option<String> {
    let encoded = url.strip_prefix("data:image/")?.split_once(',')?.1;
    let bytes = STANDARD.decode(encoded).ok()?;
    let image = image::load_from_memory(&bytes).ok()?;
    let mut png = Vec::new();
    image
        .write_to(&mut Cursor::new(&mut png), image::ImageFormat::Png)
        .ok()?;
    Some(format!("data:image/png;base64,{}", STANDARD.encode(png)))
}

fn capture_text(items: Signal<Vec<ClipItem>>, text: String) {
    items.update(|list| {
        let mut item = if let Some(index) = list
            .iter()
            .position(|item| item.kind == ClipKind::Text && item.text == text)
        {
            list.remove(index)
        } else {
            ClipItem {
                id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
                kind: ClipKind::Text,
                text: text.clone(),
                image_url: None,
                width: 0,
                height: 0,
                favorite: false,
            }
        };
        item.id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        list.insert(0, item);
        list.truncate(MAX_ITEMS);
    });
    save(&items.get_untracked());
}

fn capture_image(items: Signal<Vec<ClipItem>>, url: String, width: u32, height: u32) {
    items.update(|list| {
        list.retain(|item| {
            !(item.kind == ClipKind::Image && item.width == width && item.height == height)
        });
        list.insert(
            0,
            ClipItem {
                id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
                kind: ClipKind::Image,
                text: String::new(),
                image_url: Some(url),
                width,
                height,
                favorite: false,
            },
        );
        list.truncate(MAX_ITEMS);
    });
    save(&items.get_untracked());
}

pub fn toggle_favorite(items: Signal<Vec<ClipItem>>, id: u64) {
    items.update(|list| {
        if let Some(item) = list.iter_mut().find(|item| item.id == id) {
            item.favorite = !item.favorite;
        }
    });
    save(&items.get_untracked());
}

pub fn clear(items: Signal<Vec<ClipItem>>) {
    items.update(|list| list.retain(|item| item.favorite));
    save(&items.get_untracked());
}

fn data_file() -> Option<PathBuf> {
    let base = std::env::var_os("APPDATA")?;
    Some(
        PathBuf::from(base)
            .join("com.focusd.island.nsh")
            .join("clipboard.json"),
    )
}

fn load() -> Vec<ClipItem> {
    let Some(path) = data_file() else {
        return Vec::new();
    };
    let Ok(text) = fs::read_to_string(path) else {
        return Vec::new();
    };
    let mut items: Vec<ClipItem> = serde_json::from_str(&text).unwrap_or_default();
    items.retain(|item| item.kind == ClipKind::Text);
    for item in &mut items {
        NEXT_ID.fetch_max(item.id + 1, Ordering::Relaxed);
    }
    items
}

fn save(items: &[ClipItem]) {
    let Some(path) = data_file() else {
        return;
    };
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let text_items: Vec<&ClipItem> = items
        .iter()
        .filter(|item| item.kind == ClipKind::Text)
        .collect();
    if let Ok(json) = serde_json::to_string_pretty(&text_items) {
        let _ = fs::write(path, json);
    }
}
