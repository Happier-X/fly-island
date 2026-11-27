//! 开机自启：直接读写当前用户的 Run 键，注册表即唯一状态来源。

use windows::{
    Win32::System::Registry::{
        HKEY_CURRENT_USER, REG_SZ, RRF_RT_REG_SZ, RegDeleteKeyValueW, RegGetValueW, RegSetKeyValueW,
    },
    core::PCWSTR,
};

const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const VALUE_NAME: &str = "FocuSD Island (NSH)";

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(Some(0)).collect()
}

fn command() -> Vec<u16> {
    let exe = std::env::current_exe().unwrap_or_default();
    wide(&format!("\"{}\"", exe.display()))
}

pub fn enabled() -> bool {
    let key = wide(RUN_KEY);
    let name = wide(VALUE_NAME);
    // SAFETY: both strings outlive the call; a missing value is a normal result.
    let status = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            PCWSTR(key.as_ptr()),
            PCWSTR(name.as_ptr()),
            RRF_RT_REG_SZ,
            None,
            None,
            None,
        )
    };
    status.is_ok()
}

pub fn set(on: bool) {
    let key = wide(RUN_KEY);
    let name = wide(VALUE_NAME);
    if on {
        let value = command();
        let bytes = (value.len() * 2) as u32;
        // SAFETY: the value buffer is a NUL-terminated UTF-16 string of `bytes` bytes.
        let status = unsafe {
            RegSetKeyValueW(
                HKEY_CURRENT_USER,
                PCWSTR(key.as_ptr()),
                PCWSTR(name.as_ptr()),
                REG_SZ.0,
                Some(value.as_ptr().cast()),
                bytes,
            )
        };
        if !status.is_ok() {
            log::warn!("开机自启写入失败: {status:?}");
        }
    } else {
        // SAFETY: deleting a value that may not exist is fine; the error is ignored.
        unsafe {
            let _ = RegDeleteKeyValueW(
                HKEY_CURRENT_USER,
                PCWSTR(key.as_ptr()),
                PCWSTR(name.as_ptr()),
            );
        }
    }
}

pub fn toggle() {
    set(!enabled());
}
