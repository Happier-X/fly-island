// SPDX-License-Identifier: GPL-3.0-only
//! 快速保存：原子创建唯一文件，不覆盖已有截图（EXT-2）。

use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};
use tiny_skia::Pixmap;

pub fn save_unique(
    pixmap: &Pixmap,
    dir: Option<&Path>,
    ext: &str,
    style: crate::settings::NameStyle,
) -> Result<PathBuf, String> {
    // 扩展名不是路径；只允许编码器实际支持的格式，不能包含目录分隔符。
    let bytes = encode(pixmap, ext)?;
    let dir = dir.filter(|p| !p.as_os_str().is_empty())
        .map(Path::to_path_buf).unwrap_or_else(super::default_save_dir);
    std::fs::create_dir_all(&dir).map_err(|e| format!("无法创建 {dir:?}：{e}"))?;
    let base = super::base_name(style);
    for n in 0..1000 {
        let name = if n == 0 { format!("{base}.{ext}") } else { format!("{base}-{n}.{ext}") };
        let path = dir.join(name);
        match std::fs::OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(mut file) => {
                if let Err(e) = file.write_all(&bytes) {
                    drop(file);
                    let _ = std::fs::remove_file(&path);
                    return Err(format!("写入 {path:?} 失败：{e}"));
                }
                return Ok(path);
            }
            Err(e) if e.kind() == ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(format!("创建 {path:?} 失败：{e}")),
        }
    }
    Err(format!("{dir:?} 中同名文件过多"))
}

fn encode(pixmap: &Pixmap, ext: &str) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    match ext {
        "png" => return super::encode_png(pixmap),
        "jpeg" | "jpg" => {
            let w = u16::try_from(pixmap.width()).map_err(|_| "JPEG 宽度超限")?;
            let h = u16::try_from(pixmap.height()).map_err(|_| "JPEG 高度超限")?;
            jpeg_encoder::Encoder::new(&mut bytes, 90)
                .encode(pixmap.data(), w, h, jpeg_encoder::ColorType::Rgba)
                .map_err(|e| format!("JPEG 编码失败：{e}"))?;
        }
        "webp" => image_webp::WebPEncoder::new(&mut bytes)
            .encode(pixmap.data(), pixmap.width(), pixmap.height(), image_webp::ColorType::Rgba8)
            .map_err(|e| format!("WebP 编码失败：{e}"))?,
        _ => return Err(format!("不支持的图片格式：{ext}")),
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_extension_cannot_escape_directory() {
        let pm = Pixmap::new(2, 2).unwrap();
        assert!(encode(&pm, "../x").is_err());
        assert!(encode(&pm, "png/x").is_err());
    }

    #[test]
    fn concurrent_saves_keep_all_files() {
        let dir = std::env::temp_dir().join(format!("airsnip-concurrent-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let jobs: Vec<_> = (0..4).map(|_| {
            let dir = dir.clone();
            std::thread::spawn(move || {
                save_unique(&Pixmap::new(4, 4).unwrap(), Some(&dir), "png", crate::settings::NameStyle::Date).unwrap()
            })
        }).collect();
        let mut paths: Vec<_> = jobs.into_iter().map(|j| j.join().unwrap()).collect();
        paths.sort();
        paths.dedup();
        assert_eq!(paths.len(), 4);
        for path in paths { std::fs::remove_file(path).unwrap(); }
        let _ = std::fs::remove_dir(dir);
    }
}
