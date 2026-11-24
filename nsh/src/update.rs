//! The NSH update manifest, download, hash check and portable-package replacement.

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::Duration,
};

use semver::Version;
use serde::Deserialize;
use sha2::{Digest, Sha256};

const MAX_PACKAGE_BYTES: u64 = 240 * 1024 * 1024;

#[derive(Debug, Clone, Deserialize)]
pub struct Manifest {
    pub version: String,
    pub url: String,
    pub sha256: String,
    #[serde(default)]
    pub signature: Option<String>,
    #[serde(default)]
    pub notes: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Available {
    pub version: String,
    pub url: String,
    pub sha256: String,
    pub signature: Option<String>,
    pub notes: Option<String>,
}

pub fn parse_manifest(bytes: &[u8], current: &str) -> Result<Option<Available>, String> {
    let manifest: Manifest =
        serde_json::from_slice(bytes).map_err(|error| format!("更新清单格式错误：{error}"))?;
    let next = Version::parse(manifest.version.trim())
        .map_err(|error| format!("更新版本号无效：{error}"))?;
    let running = Version::parse(current).map_err(|error| format!("当前版本号无效：{error}"))?;
    if next <= running {
        return Ok(None);
    }
    if manifest.url.trim().is_empty() {
        return Err("更新清单缺少 url。".to_string());
    }
    let hash = manifest.sha256.trim().to_ascii_lowercase();
    if hash.len() != 64 || !hash.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("更新清单的 sha256 必须是 64 位十六进制。".to_string());
    }
    Ok(Some(Available {
        version: manifest.version,
        url: manifest.url,
        sha256: hash,
        signature: manifest.signature,
        notes: manifest.notes,
    }))
}

pub fn current_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

pub fn check(endpoint: &str) -> Result<Option<Available>, String> {
    if endpoint.trim().is_empty() {
        return Ok(None);
    }
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(12))
        .build()
        .map_err(|error| format!("创建更新请求失败：{error}"))?;
    let response = client
        .get(endpoint)
        .send()
        .map_err(|error| format!("检查更新失败：{error}"))?;
    if !response.status().is_success() {
        return Err(format!("检查更新失败：HTTP {}", response.status()));
    }
    let bytes = response
        .bytes()
        .map_err(|error| format!("读取更新清单失败：{error}"))?;
    parse_manifest(&bytes, current_version())
}

/// Downloads the package, checks its SHA-256 and extracts it into a fresh directory.
pub fn download_and_stage(available: &Available, parent: &Path) -> Result<PathBuf, String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(120))
        .build()
        .map_err(|error| format!("创建下载请求失败：{error}"))?;
    let response = client
        .get(&available.url)
        .send()
        .map_err(|error| format!("下载更新失败：{error}"))?;
    if !response.status().is_success() {
        return Err(format!("下载更新失败：HTTP {}", response.status()));
    }
    if response.content_length().is_some_and(|length| length > MAX_PACKAGE_BYTES) {
        return Err("更新包超过 240 MB，已拒绝。".to_string());
    }
    let bytes = response
        .bytes()
        .map_err(|error| format!("读取更新包失败：{error}"))?;
    if bytes.len() as u64 > MAX_PACKAGE_BYTES {
        return Err("更新包超过 240 MB，已拒绝。".to_string());
    }
    let actual = format!("{:x}", Sha256::digest(&bytes));
    if actual != available.sha256 {
        return Err(format!(
            "SHA-256 不匹配：期望 {}，实际 {actual}",
            available.sha256
        ));
    }
    fs::create_dir_all(parent).map_err(|error| format!("创建更新临时目录失败：{error}"))?;
    let archive_path = parent.join("update.zip");
    fs::write(&archive_path, &bytes).map_err(|error| format!("写入更新包失败：{error}"))?;
    let raw = parent.join("raw");
    if raw.exists() {
        fs::remove_dir_all(&raw).map_err(|error| format!("清理旧更新目录失败：{error}"))?;
    }
    fs::create_dir_all(&raw).map_err(|error| format!("创建解包目录失败：{error}"))?;
    let file = fs::File::open(&archive_path).map_err(|error| format!("打开更新包失败：{error}"))?;
    let mut archive = zip::ZipArchive::new(file).map_err(|error| format!("更新包不是有效 zip：{error}"))?;
    archive
        .extract(&raw)
        .map_err(|error| format!("解包更新失败：{error}"))?;
    package_root(&raw)
}

fn package_root(raw: &Path) -> Result<PathBuf, String> {
    if raw.join("nsh").join("app.json").is_file() {
        return Ok(raw.to_path_buf());
    }
    let entries = fs::read_dir(raw)
        .map_err(|error| format!("读取解包目录失败：{error}"))?
        .filter_map(Result::ok)
        .collect::<Vec<_>>();
    let dirs = entries.iter().filter(|entry| entry.path().is_dir()).collect::<Vec<_>>();
    if dirs.len() == 1 && dirs[0].path().join("nsh").join("app.json").is_file() {
        return Ok(dirs[0].path());
    }
    Err("更新包里找不到 nsh/app.json。".to_string())
}

pub fn target_dir() -> Result<PathBuf, String> {
    if let Some(value) = std::env::var_os("NSH_UPDATE_TARGET_DIR") {
        return Ok(PathBuf::from(value));
    }
    std::env::current_exe()
        .map_err(|error| format!("找不到当前程序路径：{error}"))?
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| "找不到当前程序目录。".to_string())
}

/// Copies the staged portable package into target and starts its exe. When target is the
/// running install, a small cmd helper waits for this process to exit before copying.
pub fn install_and_restart(staging: &Path, target: &Path) -> Result<(), String> {
    let exe = find_exe(staging)?;
    let current = std::env::current_exe().ok();
    let running_dir = current.as_ref().and_then(|path| path.parent()).map(Path::to_path_buf);
    if running_dir.as_ref().is_some_and(|dir| dir == target) {
        let helper = target.join("focusd-island-update.cmd");
        let staging_text = staging.display().to_string().replace('"', "");
        let target_text = target.display().to_string().replace('"', "");
        let exe_name = exe.file_name().and_then(|name| name.to_str()).unwrap_or("FocuSD Island (NSH).exe");
        let script = format!(
            "@echo off\r\ntimeout /t 2 /nobreak >nul\r\nxcopy /E /I /Y \"{staging_text}\\*\" \"{target_text}\\\" >nul\r\nstart \"\" \"{target_text}\\{exe_name}\"\r\ndel \"%~f0\"\r\n"
        );
        fs::write(&helper, script).map_err(|error| format!("写入更新重启脚本失败：{error}"))?;
        Command::new("cmd")
            .args(["/C", &helper.display().to_string()])
            .spawn()
            .map_err(|error| format!("启动更新重启脚本失败：{error}"))?;
        return Ok(());
    }
    copy_tree(staging, target)?;
    let target_exe = target.join(exe.file_name().ok_or_else(|| "更新包没有 exe 文件名。".to_string())?);
    Command::new(&target_exe)
        .spawn()
        .map_err(|error| format!("启动更新后的程序失败：{error}"))?;
    Ok(())
}

fn find_exe(root: &Path) -> Result<PathBuf, String> {
    let entries = fs::read_dir(root).map_err(|error| format!("读取更新目录失败：{error}"))?;
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        if path.is_file()
            && path
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("exe"))
        {
            return Ok(path);
        }
    }
    Err("更新包里找不到 exe。".to_string())
}

fn copy_tree(source: &Path, target: &Path) -> Result<(), String> {
    fs::create_dir_all(target).map_err(|error| format!("创建安装目录失败：{error}"))?;
    for entry in fs::read_dir(source).map_err(|error| format!("读取更新目录失败：{error}"))? {
        let entry = entry.map_err(|error| format!("读取更新文件失败：{error}"))?;
        let path = entry.path();
        let destination = target.join(entry.file_name());
        if path.is_dir() {
            copy_tree(&path, &destination)?;
        } else {
            fs::copy(&path, &destination)
                .map_err(|error| format!("覆盖 {} 失败：{error}", destination.display()))?;
        }
    }
    Ok(())
}

/// The manifest URL the settings page edits; empty disables checks.
pub fn normalize_endpoint(value: &str) -> String {
    value.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest(version: &str, hash: &str) -> String {
        format!(
            "{{\"version\":\"{version}\",\"url\":\"http://127.0.0.1/a.zip\",\"sha256\":\"{hash}\",\"notes\":\"n\"}}"
        )
    }

    #[test]
    fn newer_versions_need_a_valid_hash() {
        let hash = "a".repeat(64);
        let available = parse_manifest(manifest("9.9.9", &hash).as_bytes(), "0.1.0")
            .unwrap()
            .unwrap();
        assert_eq!(available.version, "9.9.9");
        assert!(parse_manifest(manifest("0.1.0", &hash).as_bytes(), "0.1.0").unwrap().is_none());
        assert!(parse_manifest(manifest("9.9.9", "bad").as_bytes(), "0.1.0").is_err());
    }

    fn serve_once(status: u16, body: &'static [u8]) -> String {
        use std::io::{Read, Write};
        use std::net::TcpListener;
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut buffer = [0_u8; 2048];
            let _ = stream.read(&mut buffer);
            let reason = if status == 200 { "OK" } else { "Not Found" };
            let _ = write!(
                stream,
                "HTTP/1.1 {status} {reason}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            let _ = stream.write_all(body);
        });
        format!("http://{address}")
    }

    #[test]
    fn http_404_and_hash_mismatch_are_errors() {
        let base = serve_once(404, b"");
        let error = check(&format!("{base}/missing.json")).unwrap_err();
        assert!(error.contains("404"));

        let base = serve_once(200, b"not-a-zip");
        let available = Available {
            version: "0.1.2".to_string(),
            url: format!("{base}/pkg.zip"),
            sha256: "0".repeat(64),
            signature: None,
            notes: None,
        };
        let dir = std::env::temp_dir().join("focusd-update-hash-test");
        let error = download_and_stage(&available, &dir).unwrap_err();
        assert!(error.contains("SHA-256"));
    }

    #[test]
    fn a_file_target_is_a_write_error() {
        let stage = std::env::temp_dir().join("focusd-update-stage-test");
        let target = std::env::temp_dir().join("focusd-update-target-file");
        std::fs::create_dir_all(&stage).unwrap();
        std::fs::write(stage.join("app.exe"), b"exe").unwrap();
        std::fs::write(&target, b"not a directory").unwrap();
        let error = install_and_restart(&stage, &target).unwrap_err();
        assert!(error.contains("安装目录"));
    }
}
