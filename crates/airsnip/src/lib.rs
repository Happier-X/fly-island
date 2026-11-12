// SPDX-License-Identifier: GPL-3.0-only
//! 库入口。
//!
//! 两种用法：
//! - 独立程序：`app::run()`（原有的 AirSnip 完整外壳）。
//! - 被宿主进程承载：`start_host()` 在一个专用线程上跑隐藏消息窗与消息循环，
//!   宿主用自己的全局热键调 `HostHandle::request_capture()` 触发截图。

pub mod app;
pub mod capture;
pub mod editor;
pub mod host;
pub mod longshot;
pub mod ocr;
pub mod output;
pub mod overlay;
pub mod perf;
pub mod pin;
pub mod render;
pub mod settings;
pub mod theme;
pub mod update;

pub use host::{HostHandle, start_host};
