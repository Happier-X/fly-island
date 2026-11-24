//! Windows Credential Manager entries for translation keys.
//!
//! They use the same target name as the Tauri app's keyring entries
//! (user.provider + "." + service), so an existing key is read without copying it.
//! Nothing here prints or logs a secret.

use std::ptr;

use windows::{
    core::{PCWSTR, PWSTR},
    Win32::{
        Foundation::ERROR_NOT_FOUND,
        Security::Credentials::{
            CredDeleteW, CredFree, CredReadW, CredWriteW, CREDENTIALW, CRED_PERSIST_ENTERPRISE,
            CRED_TYPE_GENERIC,
        },
    },
};

const SERVICE: &str = "com.focusd.island.translation";
const MAX_SECRET_BYTES: usize = 2_000;

fn target(provider: &str) -> String {
    format!("{provider}.{SERVICE}")
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

pub fn has_secret(provider: &str) -> Result<bool, String> {
    Ok(read_secret(provider)?.is_some())
}

pub fn read_secret(provider: &str) -> Result<Option<String>, String> {
    let name = wide(&target(provider));
    let mut credential: *mut CREDENTIALW = ptr::null_mut();
    // SAFETY: the C string is NUL-terminated and lives for the call; on success Windows
    // allocates the credential, which is copied out and then freed with CredFree.
    let read = unsafe { CredReadW(PCWSTR(name.as_ptr()), CRED_TYPE_GENERIC, None, &mut credential) };
    if let Err(error) = read {
        if error.code() == ERROR_NOT_FOUND.to_hresult() {
            return Ok(None);
        }
        return Err(format!("无法读取 Windows 凭据：{error}"));
    }
    if credential.is_null() {
        return Ok(None);
    }
    // SAFETY: CredReadW returned a non-null credential that stays valid until CredFree.
    let secret = unsafe {
        let credential = &*credential;
        let bytes = if credential.CredentialBlob.is_null() {
            &[][..]
        } else {
            std::slice::from_raw_parts(credential.CredentialBlob, credential.CredentialBlobSize as usize)
        };
        let secret = String::from_utf8_lossy(bytes).to_string();
        CredFree(credential as *const _ as *const core::ffi::c_void);
        secret
    };
    Ok(Some(secret))
}

pub fn save_secret(provider: &str, secret: &str) -> Result<(), String> {
    let secret = secret.trim();
    if secret.is_empty() || secret.len() > MAX_SECRET_BYTES {
        return Err("请输入有效的 API 密钥。".to_string());
    }
    let mut target = wide(&target(provider));
    let mut user = wide(provider);
    let mut blob = secret.as_bytes().to_vec();
    let credential = CREDENTIALW {
        TargetName: PWSTR(target.as_mut_ptr()),
        UserName: PWSTR(user.as_mut_ptr()),
        CredentialBlob: blob.as_mut_ptr(),
        CredentialBlobSize: blob.len() as u32,
        Persist: CRED_PERSIST_ENTERPRISE,
        Type: CRED_TYPE_GENERIC,
        ..CREDENTIALW::default()
    };
    // SAFETY: every pointer in the credential points at a live buffer for the call; Windows
    // copies the blob into the credential store.
    unsafe { CredWriteW(&credential, 0) }.map_err(|error| format!("无法保存到 Windows 凭据：{error}"))
}

pub fn delete_secret(provider: &str) -> Result<(), String> {
    let name = wide(&target(provider));
    // SAFETY: the C string is NUL-terminated and lives for the call.
    match unsafe { CredDeleteW(PCWSTR(name.as_ptr()), CRED_TYPE_GENERIC, None) } {
        Ok(()) => Ok(()),
        Err(error) if error.code() == ERROR_NOT_FOUND.to_hresult() => Ok(()),
        Err(error) => Err(format!("无法删除 Windows 凭据：{error}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn target_names_match_the_tauri_keyring_format() {
        assert_eq!(target("deepseek"), "deepseek.com.focusd.island.translation");
        assert_eq!(target("custom"), "custom.com.focusd.island.translation");
    }

    #[test]
    fn empty_and_oversized_secrets_are_rejected() {
        assert!(save_secret("custom", " ").is_err());
        assert!(save_secret("custom", &"x".repeat(MAX_SECRET_BYTES + 1)).is_err());
    }
}