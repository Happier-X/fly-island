//! Explorer icons for launcher results, as PNG data URLs the list can show directly.
//! Only the handful of results on screen are asked for, and each path is cached.

use std::{
    collections::HashMap,
    path::Path,
    sync::{Mutex, MutexGuard, PoisonError},
};

use base64::{engine::general_purpose, Engine};
use image::{DynamicImage, ImageFormat, RgbaImage};

use crate::win32::icon::{shell_icon, ShellIconSource};

/// Types whose icon is the file's own rather than its type's; these read the disk once
/// per path, so only results that are actually shown ask for them.
const OWN_ICON_EXTENSIONS: [&str; 8] =
    ["exe", "lnk", "ico", "url", "appref-ms", "msc", "cpl", "scr"];

/// The cache is dropped as a whole past this; a search shows a few dozen icons.
const MAX_CACHED: usize = 256;

static CACHE: Mutex<Option<HashMap<String, Option<String>>>> = Mutex::new(None);

fn lock() -> MutexGuard<'static, Option<HashMap<String, Option<String>>>> {
    CACHE.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Data URL of the shell icon for a launcher result, or `None` when the shell has none.
pub(super) fn data_url(path: &str, kind: &str) -> Option<String> {
    let key = format!("{}:{}", kind, path.to_lowercase());
    if let Some(cached) = lock().as_ref().and_then(|cache| cache.get(&key)) {
        return cached.clone();
    }
    let source = if kind == "app" {
        ShellIconSource::Item
    } else {
        icon_source(path)
    };
    let url = shell_icon(path, source).and_then(encode);
    let mut guard = lock();
    let cache = guard.get_or_insert_with(HashMap::new);
    if cache.len() >= MAX_CACHED {
        cache.clear();
    }
    cache.insert(key, url.clone());
    url
}

fn icon_source(path: &str) -> ShellIconSource {
    let extension = Path::new(path)
        .extension()
        .map(|extension| extension.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    if OWN_ICON_EXTENSIONS.contains(&extension.as_str()) {
        ShellIconSource::Item
    } else {
        ShellIconSource::FileType
    }
}

fn encode(icon: crate::win32::icon::IconPixels) -> Option<String> {
    let image = RgbaImage::from_raw(icon.width, icon.height, icon.rgba)?;
    let mut bytes = std::io::Cursor::new(Vec::new());
    DynamicImage::ImageRgba8(image)
        .write_to(&mut bytes, ImageFormat::Png)
        .ok()?;
    Some(format!(
        "data:image/png;base64,{}",
        general_purpose::STANDARD.encode(bytes.into_inner())
    ))
}

#[cfg(test)]
mod tests {
    use super::icon_source;
    use crate::win32::icon::ShellIconSource;

    #[test]
    fn programs_use_their_own_icon_and_documents_use_their_types() {
        assert_eq!(icon_source(r"C:\App\app.exe"), ShellIconSource::Item);
        assert_eq!(
            icon_source(r"C:\notes\report.PDF"),
            ShellIconSource::FileType
        );
        assert_eq!(
            icon_source(r"C:\notes\no-extension"),
            ShellIconSource::FileType
        );
    }
}
