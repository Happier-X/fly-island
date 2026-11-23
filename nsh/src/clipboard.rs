//! Clipboards: text and image history, favorites, persistence, legacy import.
//!
//! Text and images are polled; the Win32 host also posts a dirty flag from
//! WM_CLIPBOARDUPDATE, so an embedded run picks a copy up in ~100 ms. Images
//! are stored as PNG files and written back as CF_DIB through arboard.

use std::{
    borrow::Cow,
    fs,
    io::Cursor,
    path::{Path, PathBuf},
    rc::Rc,
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
    pub fn start(
        enabled: Signal<bool>,
        capture_images: Signal<bool>,
        max_items: Signal<usize>,
    ) -> Self {
        let items = signal(load());
        let last_text = signal(String::new());
        let last_image = signal(String::new());
        let read = Rc::new(move || {
            spawn(async move {
                if !enabled.get() {
                    return;
                }
                let text = clipboard_string().await;
                let trimmed = text.trim();
                if !trimmed.is_empty() {
                    if last_text.get() == trimmed {
                        return;
                    }
                    last_text.set(trimmed.to_string());
                    last_image.set(String::new());
                    capture_text(items, trimmed.to_string(), max_items.get());
                    return;
                }
                if !capture_images.get() || !nsh_core::prelude::os().is_desktop() {
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
                        max_items.get(),
                    );
                }
            });
        });
        read();
        let poll = read.clone();
        set_interval(POLL_INTERVAL, move || poll());
        #[cfg(feature = "embedded")]
        {
            let immediate = read.clone();
            set_interval(Duration::from_millis(100), move || {
                if crate::embedded::take_clipboard_dirty() {
                    immediate();
                }
            });
        }
        Self { items }
    }
}

/// Writes an image as CF_DIB (what Notepad and most Windows apps paste),
/// from a PNG data URL or a file path.
pub fn write_image(url: &str) -> bool {
    let bytes = if url.starts_with("data:") {
        decode_data_url(url)
    } else {
        fs::read(url).ok()
    };
    let Some(bytes) = bytes else {
        return false;
    };
    let Ok(image) = image::load_from_memory(&bytes) else {
        return false;
    };
    let rgba = image.to_rgba8();
    let (width, height) = (rgba.width() as usize, rgba.height() as usize);
    let Ok(mut clipboard) = arboard::Clipboard::new() else {
        return false;
    };
    clipboard
        .set_image(arboard::ImageData {
            width,
            height,
            bytes: Cow::Owned(rgba.into_raw()),
        })
        .is_ok()
}

fn to_png_url(url: &str) -> Option<String> {
    let bytes = decode_data_url(url)?;
    let image = image::load_from_memory(&bytes).ok()?;
    let mut png = Vec::new();
    image
        .write_to(&mut Cursor::new(&mut png), image::ImageFormat::Png)
        .ok()?;
    Some(format!("data:image/png;base64,{}", STANDARD.encode(png)))
}

fn decode_data_url(url: &str) -> Option<Vec<u8>> {
    let encoded = url.strip_prefix("data:")?.split_once(',')?.1;
    STANDARD.decode(encoded).ok()
}

fn capture_text(items: Signal<Vec<ClipItem>>, text: String, max_items: usize) {
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
        list.truncate(max_items.clamp(5, 200));
    });
    save(&items.get_untracked());
}

fn capture_image(items: Signal<Vec<ClipItem>>, url: String, width: u32, height: u32, max_items: usize) {
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
        list.truncate(max_items.clamp(5, 200));
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

fn data_dir() -> Option<PathBuf> {
    let base = std::env::var_os("APPDATA")?;
    Some(PathBuf::from(base).join("com.focusd.island.nsh"))
}

fn image_dir() -> Option<PathBuf> {
    Some(data_dir()?.join("clipboard-images"))
}

fn data_file() -> Option<PathBuf> {
    Some(data_dir()?.join("clipboard.json"))
}

fn import_flag() -> Option<PathBuf> {
    Some(data_dir()?.join("legacy-clipboard-imported.flag"))
}

fn legacy_file() -> Option<PathBuf> {
    let base = std::env::var_os("APPDATA")?;
    Some(
        PathBuf::from(base)
            .join("com.focusd.island")
            .join("clipboard-history.json"),
    )
}

fn load() -> Vec<ClipItem> {
    let items = load_new();
    if !items.is_empty() || import_flag().is_some_and(|path| path.exists()) {
        return items;
    }
    let Some(legacy) = import_legacy() else {
        return items;
    };
    if let Some(flag) = import_flag() {
        if let Some(parent) = flag.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let _ = fs::write(flag, b"1");
    }
    if legacy.is_empty() {
        return items;
    }
    save(&legacy);
    legacy
}

fn load_new() -> Vec<ClipItem> {
    let Some(path) = data_file() else {
        return Vec::new();
    };
    let Ok(text) = fs::read_to_string(path) else {
        return Vec::new();
    };
    let mut items: Vec<ClipItem> = serde_json::from_str(&text).unwrap_or_default();
    items.retain(|item| match item.kind {
        ClipKind::Text => !item.text.is_empty(),
        ClipKind::Image => item.image_url.as_deref().is_some_and(|url| url.starts_with("data:") || Path::new(url).exists()),
    });
    for item in &mut items {
        NEXT_ID.fetch_max(item.id + 1, Ordering::Relaxed);
    }
    items
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct LegacyHistory {
    items: Vec<LegacyItem>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct LegacyItem {
    kind: String,
    #[serde(default)]
    favorite: bool,
    text: Option<String>,
    image: Option<LegacyImage>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct LegacyImage {
    width: u32,
    height: u32,
    original_path: String,
}

fn import_legacy() -> Option<Vec<ClipItem>> {
    let path = legacy_file()?;
    let text = fs::read_to_string(path).ok()?;
    let history: LegacyHistory = serde_json::from_str(&text).ok()?;
    let mut items = Vec::new();
    for old in history.items {
        if old.kind == "text" {
            let Some(text) = old.text.filter(|text| !text.is_empty()) else {
                continue;
            };
            items.push(ClipItem {
                id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
                kind: ClipKind::Text,
                text,
                image_url: None,
                width: 0,
                height: 0,
                favorite: old.favorite,
            });
        } else if old.kind == "image" {
            let Some(image) = old.image else {
                continue;
            };
            let Ok(bytes) = fs::read(&image.original_path) else {
                continue;
            };
            let Ok(decoded) = image::load_from_memory(&bytes) else {
                continue;
            };
            let rgba = decoded.to_rgba8();
            let mut png = Vec::new();
            if rgba
                .write_to(&mut Cursor::new(&mut png), image::ImageFormat::Png)
                .is_err()
            {
                continue;
            }
            let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
            let Some(dir) = image_dir() else {
                continue;
            };
            let _ = fs::create_dir_all(&dir);
            let target = dir.join(format!("legacy-{id}.png"));
            if fs::write(&target, png).is_err() {
                continue;
            }
            items.push(ClipItem {
                id,
                kind: ClipKind::Image,
                text: String::new(),
                image_url: Some(target.to_string_lossy().to_string()),
                width: image.width.max(rgba.width()),
                height: image.height.max(rgba.height()),
                favorite: old.favorite,
            });
        }
        if items.len() >= MAX_ITEMS {
            break;
        }
    }
    Some(items)
}

fn save(items: &[ClipItem]) {
    let Some(path) = data_file() else {
        return;
    };
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let persisted: Vec<ClipItem> = items
        .iter()
        .filter_map(|item| {
            if item.kind == ClipKind::Text {
                return Some(item.clone());
            }
            persist_image(item)
        })
        .collect();
    if let Ok(json) = serde_json::to_string_pretty(&persisted) {
        let _ = fs::write(path, json);
    }
}

fn persist_image(item: &ClipItem) -> Option<ClipItem> {
    let source = item.image_url.as_ref()?;
    if !source.starts_with("data:") && Path::new(source).exists() {
        return Some(item.clone());
    }
    let bytes = decode_data_url(source)?;
    let dir = image_dir()?;
    fs::create_dir_all(&dir).ok()?;
    let target = dir.join(format!("{}.png", item.id));
    fs::write(&target, bytes).ok()?;
    let mut stored = item.clone();
    stored.image_url = Some(target.to_string_lossy().to_string());
    Some(stored)
}
