//! Shell icons and thumbnails for the file shelf, cached as PNG data URLs.

use std::{
    collections::HashMap,
    ffi::c_void,
    mem::size_of,
    path::Path,
    sync::{Mutex, PoisonError},
    time::SystemTime,
};

use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use image::{DynamicImage, ImageFormat, RgbaImage};
use windows::{
    core::PCWSTR,
    Win32::{
        Foundation::SIZE,
        Graphics::Gdi::{
            CreateCompatibleDC, DeleteDC, DeleteObject, GetDIBits, GetObjectW, BITMAP, BITMAPINFO,
            BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, HBITMAP, HGDIOBJ,
        },
        Storage::FileSystem::{FILE_ATTRIBUTE_DIRECTORY, FILE_ATTRIBUTE_NORMAL},
        System::Com::{CoInitializeEx, COINIT_APARTMENTTHREADED},
        UI::{
            Shell::{
                IShellItemImageFactory, SHCreateItemFromParsingName, SHGetFileInfoW, SHFILEINFOW,
                SHGFI_ICON, SHGFI_LARGEICON, SHGFI_USEFILEATTRIBUTES, SIIGBF_BIGGERSIZEOK,
                SIIGBF_THUMBNAILONLY,
            },
            WindowsAndMessaging::{DestroyIcon, GetIconInfo, HICON, ICONINFO},
        },
    },
};

const ICON_CACHE: usize = 256;
const THUMB_CACHE: usize = 256;
const THUMB_SIDE: i32 = 96;
const IMAGE_EXTENSIONS: [&str; 6] = ["png", "jpg", "jpeg", "gif", "webp", "bmp"];
const VIDEO_EXTENSIONS: [&str; 7] = ["mp4", "mov", "mkv", "avi", "webm", "wmv", "flv"];

type IconCache = HashMap<String, Option<String>>;
type ThumbCache = HashMap<(String, u64), Option<String>>;
static ICONS: std::sync::LazyLock<Mutex<IconCache>> =
    std::sync::LazyLock::new(|| Mutex::new(HashMap::new()));
static THUMBS: std::sync::LazyLock<Mutex<ThumbCache>> =
    std::sync::LazyLock::new(|| Mutex::new(HashMap::new()));
fn extension(path: &str) -> String {
    Path::new(path)
        .extension()
        .map(|value| value.to_string_lossy().to_lowercase())
        .unwrap_or_default()
}

fn previewable(path: &str) -> bool {
    let ext = extension(path);
    IMAGE_EXTENSIONS.contains(&ext.as_str()) || VIDEO_EXTENSIONS.contains(&ext.as_str())
}

fn ensure_com() {
    // SAFETY: no pointers are passed; an already initialized apartment is fine.
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
    }
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Explorer's icon for the path, or its type's icon when the file is unavailable.
pub fn icon(path: &str, is_dir: bool, available: bool) -> Option<String> {
    let ext = extension(path);
    let key = if is_dir {
        "dir".to_string()
    } else if available && matches!(ext.as_str(), "exe" | "lnk" | "ico" | "url" | "msc" | "cpl") {
        path.to_lowercase()
    } else {
        format!("type:{ext}")
    };
    if let Some(cached) = lock_icons().get(&key) {
        return cached.clone();
    }
    let url = shell_icon(path, is_dir, available);
    let mut cache = lock_icons();
    if cache.len() >= ICON_CACHE {
        cache.clear();
    }
    cache.insert(key, url.clone());
    url
}

/// A Shell thumbnail for an image or video, or None when the shell has none.
pub fn thumbnail(path: &str) -> Option<String> {
    if !previewable(path) {
        return None;
    }
    let metadata = std::fs::metadata(path).ok()?;
    if !metadata.is_file() {
        return None;
    }
    let modified = metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(SystemTime::UNIX_EPOCH).ok())
        .map(|duration| duration.as_secs())
        .unwrap_or_default();
    let key = (path.to_lowercase(), modified);
    if let Some(cached) = lock_thumbs().get(&key) {
        return cached.clone();
    }
    let url = shell_thumbnail(path).or_else(|| image_thumbnail(path));
    let mut cache = lock_thumbs();
    if cache.len() >= THUMB_CACHE {
        cache.clear();
    }
    cache.insert(key, url.clone());
    url
}

fn lock_icons() -> std::sync::MutexGuard<'static, IconCache> {
    ICONS.lock().unwrap_or_else(PoisonError::into_inner)
}

fn lock_thumbs() -> std::sync::MutexGuard<'static, ThumbCache> {
    THUMBS.lock().unwrap_or_else(PoisonError::into_inner)
}

fn shell_icon(path: &str, is_dir: bool, available: bool) -> Option<String> {
    let text = wide(path);
    let (attributes, flags) = if is_dir {
        (
            FILE_ATTRIBUTE_DIRECTORY,
            SHGFI_ICON | SHGFI_LARGEICON | SHGFI_USEFILEATTRIBUTES,
        )
    } else if available {
        (FILE_ATTRIBUTE_NORMAL, SHGFI_ICON | SHGFI_LARGEICON)
    } else {
        (
            FILE_ATTRIBUTE_NORMAL,
            SHGFI_ICON | SHGFI_LARGEICON | SHGFI_USEFILEATTRIBUTES,
        )
    };
    let mut info = SHFILEINFOW::default();
    // SAFETY: the path is NUL-terminated and info is a valid SHFILEINFOW.
    let found = unsafe {
        SHGetFileInfoW(
            PCWSTR(text.as_ptr()),
            attributes,
            Some(&mut info),
            size_of::<SHFILEINFOW>() as u32,
            flags,
        )
    };
    if found == 0 || info.hIcon.is_invalid() {
        return None;
    }
    let url = icon_png(HICON(info.hIcon.0));
    // SAFETY: SHGFI_ICON gives the caller an icon it owns.
    unsafe {
        let _ = DestroyIcon(info.hIcon);
    }
    url
}

fn icon_png(icon: HICON) -> Option<String> {
    let mut info = ICONINFO::default();
    // SAFETY: the icon is live; info is valid.
    unsafe { GetIconInfo(icon, &mut info) }.ok()?;
    let color = info.hbmColor;
    let mask = info.hbmMask;
    if color.is_invalid() {
        return None;
    }
    let mut bitmap = BITMAP::default();
    // SAFETY: color is a live bitmap.
    let written = unsafe {
        GetObjectW(
            HGDIOBJ(color.0),
            size_of::<BITMAP>() as i32,
            Some(&mut bitmap as *mut BITMAP as *mut c_void),
        )
    };
    let (width, height) = (
        u32::try_from(bitmap.bmWidth).ok()?,
        u32::try_from(bitmap.bmHeight).ok()?,
    );
    if written == 0 || width == 0 || height == 0 || width > 256 || height > 256 {
        return None;
    }
    let mut pixels = read_bgra(color, width, height)?;
    if pixels.as_chunks::<4>().0.iter().all(|pixel| pixel[3] == 0) {
        let mask_pixels = read_bgra(mask, width, height)?;
        for (pixel, masked) in pixels
            .as_chunks_mut::<4>()
            .0
            .iter_mut()
            .zip(mask_pixels.as_chunks::<4>().0)
        {
            pixel[3] = if masked[0] == 0 { 255 } else { 0 };
        }
    }
    for pixel in pixels.as_chunks_mut::<4>().0 {
        pixel.swap(0, 2);
    }
    let image = RgbaImage::from_raw(width, height, pixels)?;
    png_data_url(&DynamicImage::ImageRgba8(image))
}

fn shell_thumbnail(path: &str) -> Option<String> {
    ensure_com();
    let text = wide(path);
    // SAFETY: the path is NUL-terminated and no bind context is needed.
    let factory: IShellItemImageFactory =
        unsafe { SHCreateItemFromParsingName(PCWSTR(text.as_ptr()), None) }.ok()?;
    // SAFETY: the factory is live; the returned bitmap is owned by us.
    let bitmap = unsafe {
        factory.GetImage(
            SIZE {
                cx: THUMB_SIDE,
                cy: THUMB_SIDE,
            },
            SIIGBF_THUMBNAILONLY | SIIGBF_BIGGERSIZEOK,
        )
    }
    .ok()?;
    let mut raw = BITMAP::default();
    // SAFETY: bitmap is live.
    let written = unsafe {
        GetObjectW(
            HGDIOBJ(bitmap.0),
            size_of::<BITMAP>() as i32,
            Some(&mut raw as *mut BITMAP as *mut c_void),
        )
    };
    let width = u32::try_from(raw.bmWidth).ok()?;
    let height = u32::try_from(raw.bmHeight).ok()?;
    let result = if written == 0 || width == 0 || height == 0 {
        None
    } else {
        let mut pixels = read_bgra(bitmap, width, height)?;
        if pixels.as_chunks::<4>().0.iter().all(|pixel| pixel[3] == 0) {
            for pixel in pixels.as_chunks_mut::<4>().0 {
                pixel[3] = 255;
            }
        }
        for pixel in pixels.as_chunks_mut::<4>().0 {
            pixel.swap(0, 2);
        }
        RgbaImage::from_raw(width, height, pixels)
            .and_then(|image| png_data_url(&DynamicImage::ImageRgba8(image)))
    };
    // SAFETY: GetImage returns a bitmap owned by the caller.
    unsafe {
        let _ = DeleteObject(HGDIOBJ(bitmap.0));
    }
    result
}

fn read_bgra(bitmap: HBITMAP, width: u32, height: u32) -> Option<Vec<u8>> {
    if bitmap.is_invalid() {
        return None;
    }
    let mut pixels = vec![0u8; width as usize * height as usize * 4];
    let mut header = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: width as i32,
            biHeight: -(height as i32),
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        },
        ..Default::default()
    };
    // SAFETY: a memory DC compatible with the screen; deleted below.
    let dc = unsafe { CreateCompatibleDC(None) };
    if dc.is_invalid() {
        return None;
    }
    // SAFETY: dc is live, bitmap is not selected into another DC, pixels holds the rows.
    let rows = unsafe {
        GetDIBits(
            dc,
            bitmap,
            0,
            height,
            Some(pixels.as_mut_ptr() as *mut c_void),
            &mut header,
            DIB_RGB_COLORS,
        )
    };
    // SAFETY: dc came from CreateCompatibleDC and is deleted once.
    unsafe {
        let _ = DeleteDC(dc);
    }
    (rows == height as i32).then_some(pixels)
}

fn image_thumbnail(path: &str) -> Option<String> {
    let ext = extension(path);
    if !IMAGE_EXTENSIONS.contains(&ext.as_str()) {
        return None;
    }
    let image = image::ImageReader::open(path)
        .ok()?
        .with_guessed_format()
        .ok()?
        .decode()
        .ok()?;
    png_data_url(&DynamicImage::ImageRgba8(
        image.thumbnail(THUMB_SIDE as u32, THUMB_SIDE as u32).to_rgba8(),
    ))
}

fn png_data_url(image: &DynamicImage) -> Option<String> {
    let mut bytes = std::io::Cursor::new(Vec::new());
    image.write_to(&mut bytes, ImageFormat::Png).ok()?;
    Some(format!(
        "data:image/png;base64,{}",
        BASE64.encode(bytes.into_inner())
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_files_and_folders_have_shell_icons() {
        let text = icon(r"C:\does\not\exist\report.txt", false, false);
        assert!(text.is_some());
        assert!(icon(r"C:\does\not\exist\folder", true, false).is_some());
    }
     #[test]     fn a_png_has_a_thumbnail() {         let path = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/icon.png");         assert!(thumbnail(path).is_some());     }
}
