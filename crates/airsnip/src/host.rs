// SPDX-License-Identifier: GPL-3.0-only
//! 宿主集成入口：把 AirSnip 的截图能力挂到另一个进程的（Tauri）应用里。
//!
//! 宿主只需要三件事：
//! 1. 启动时调 `start_host()` 拿到 `HostHandle`，它会占一个后台线程跑隐藏消息窗；
//! 2. 全局热键按下时调 `HostHandle::request_capture()`；
//! 3. 设置页改了保存格式/目录/标注颜色时调 `HostHandle::apply_config()`。
//!
//! 单实例、托盘、开机自启、自动更新都由宿主自己负责，这里全部跳过；
//! 配置也整体由宿主注入，AirSnip 不再读写自己的 `config.toml`。

use std::path::{Path, PathBuf};
use std::sync::mpsc;

use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::PostMessageW;

use crate::app::{WM_HOST_CAPTURE, WM_HOST_CONFIG};
use crate::settings::Config;

/// 已启动的宿主实例。克隆成本很低，可放进宿主的全局状态里。
#[derive(Clone, Copy)]
pub struct HostHandle {
    hwnd: isize,
}

// HWND 本身不是 Send，但这里只保存原生值并只通过 PostMessageW 使用，
// 跨线程投递消息是 Win32 允许的用法。
unsafe impl Send for HostHandle {}
unsafe impl Sync for HostHandle {}

impl HostHandle {
    /// 隐藏消息窗口的原生句柄，宿主可用于日志或进一步投递消息。
    pub fn hwnd(self) -> isize {
        self.hwnd
    }

    /// 请求开始一次截图：抓屏 → 覆盖窗在 AirSnip 自己的线程上弹出。
    ///
    /// 覆盖窗已在运行时该请求会被忽略（`App::start_capture` 内部有互斥）。
    pub fn request_capture(self) -> Result<(), String> {
        self.post(WM_HOST_CAPTURE, LPARAM(0))
            .map_err(|error| format!("无法把截图请求投递给 AirSnip 消息窗口：{error}"))
    }

    /// 换一份配置：下一次截图起生效（保存格式、默认目录、标注色板等）。
    pub fn apply_config(self, config: Config) -> Result<(), String> {
        let boxed = Box::into_raw(Box::new(config));
        if let Err(error) = self.post(WM_HOST_CONFIG, LPARAM(boxed as isize)) {
            // SAFETY: 消息没投递出去，窗口过程不会取回这个 Box，由这里释放。
            unsafe { drop(Box::from_raw(boxed)) };
            return Err(format!("无法把截图配置投递给 AirSnip 消息窗口：{error}"));
        }
        Ok(())
    }

    fn post(self, message: u32, lparam: LPARAM) -> windows::core::Result<()> {
        let hwnd = HWND(self.hwnd as *mut core::ffi::c_void);
        // SAFETY: 句柄由 `start_host` 创建并存活到进程退出；消息窗口所在线程
        // 正在跑消息循环，PostMessageW 只投递不执行，跨线程安全。
        unsafe { PostMessageW(Some(hwnd), message, WPARAM(0), lparam) }
    }
}

/// 弹出系统的「选择文件夹」对话框，返回选中目录（取消时为 None）。
///
/// 对话框自己起模态循环，宿主应在非主线程（例如 async 命令的工作线程）调用。
pub fn pick_save_directory(initial: Option<&str>) -> Option<String> {
    let initial = initial.map(Path::new).filter(|path| path.is_dir());
    crate::app::filedlg::pick_directory(HWND::default(), initial)
        .map(|path| path.to_string_lossy().into_owned())
}

/// 启动托管模式：设置配置目录 → 起后台线程 → 等隐藏窗口创建完成。
///
/// `config` 由宿主构造，覆盖保存格式、默认目录、标注色板等；AirSnip 不再读
/// 自己的 `config.toml`。返回后宿主即可用 `request_capture()` 触发截图。
pub fn start_host(config_dir: Option<PathBuf>, config: Config) -> Result<HostHandle, String> {
    // 先立托管标志：之后的 `Config::save()` / `Config::load()` 都不再碰 toml。
    crate::settings::set_hosted(true);
    if let Some(dir) = config_dir {
        crate::settings::set_config_dir_override(dir);
    }

    let (sender, receiver) = mpsc::channel::<Result<isize, String>>();
    std::thread::Builder::new()
        .name("airsnip-host".to_string())
        .spawn(move || {
            let code = crate::app::run_hosted(sender, config);
            if code != 0 {
                eprintln!("[airsnip] 托管消息循环退出，返回码 {code}");
            }
        })
        .map_err(|error| format!("无法启动 AirSnip 后台线程：{error}"))?;

    match receiver.recv() {
        Ok(Ok(hwnd)) => Ok(HostHandle { hwnd }),
        Ok(Err(message)) => Err(message),
        Err(error) => Err(format!("AirSnip 后台线程提前退出：{error}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 只验证托管入口能建出隐藏消息窗：不触发截图，不会弹出覆盖层。
    #[test]
    fn hosting_creates_a_message_window() {
        let dir = std::env::temp_dir().join("airsnip-host-entry-test");
        let handle = start_host(Some(dir), Config::default()).expect("托管模式应当能启动");
        assert_ne!(handle.hwnd(), 0, "隐藏消息窗口的句柄不应为空");
    }
}
