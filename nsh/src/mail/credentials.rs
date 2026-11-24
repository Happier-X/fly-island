//! Mail authorization codes in the Windows Credential Manager.
//!
//! The target matches the Tauri keyring entry (email + service), so an
//! existing authorization code is found without copying it. Nothing here
//! prints or logs the code.

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

const SERVICE: &str = "com.focusd.island.qq-mail";
const MAX_SECRET_BYTES: usize = 500;

fn target(email: &str) -> String {
    format!("{email}.{SERVICE}")
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

pub fn has_secret(email: &str) -> Result<bool, String> {
    Ok(read_secret(email)?.is_some())
}

pub fn read_secret(email: &str) -> Result<Option<String>, String> {
    let name = wide(&target(email));
    let mut credential: *mut CREDENTIALW = ptr::null_mut();
    // SAFETY: the C string is NUL-terminated and lives for the call; on success
    // Windows allocates the credential, copied out and then freed with CredFree.
    let read = unsafe { CredReadW(PCWSTR(name.as_ptr()), CRED_TYPE_GENERIC, None, &mut credential) };
    if let Err(error) = read {
        if error.code() == ERROR_NOT_FOUND.to_hresult() {
            return Ok(None);
        }
        return Err(format!("无法读取 Windows 邮箱凭据：{error}"));
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
            std::slice::from_raw_parts(
                credential.CredentialBlob,
                credential.CredentialBlobSize as usize,
            )
        };
        let secret = String::from_utf8_lossy(bytes).to_string();
        CredFree(credential as *const _ as *const core::ffi::c_void);
        secret
    };
    Ok(Some(secret))
}

pub fn save_secret(email: &str, secret: &str) -> Result<(), String> {
    let secret = secret.trim();
    if secret.is_empty() || secret.len() > MAX_SECRET_BYTES {
        return Err("请输入有效的邮箱授权码。".to_string());
    }
    let mut name = wide(&target(email));
    let mut user = wide(email);
    let mut blob = secret.as_bytes().to_vec();
    let credential = CREDENTIALW {
        TargetName: PWSTR(name.as_mut_ptr()),
        UserName: PWSTR(user.as_mut_ptr()),
        CredentialBlob: blob.as_mut_ptr(),
        CredentialBlobSize: blob.len() as u32,
        Persist: CRED_PERSIST_ENTERPRISE,
        Type: CRED_TYPE_GENERIC,
        ..CREDENTIALW::default()
    };
    // SAFETY: every pointer points at a live buffer for the call; Windows copies
    // the blob into the credential store.
    unsafe { CredWriteW(&credential, 0) }
        .map_err(|error| format!("无法保存到 Windows 凭据：{error}"))
}

pub fn delete_secret(email: &str) -> Result<(), String> {
    let name = wide(&target(email));
    // SAFETY: the C string is NUL-terminated and lives for the call.
    match unsafe { CredDeleteW(PCWSTR(name.as_ptr()), CRED_TYPE_GENERIC, None) } {
        Ok(()) => Ok(()),
        Err(error) if error.code() == ERROR_NOT_FOUND.to_hresult() => Ok(()),
        Err(error) => Err(format!("无法删除 Windows 邮箱凭据：{error}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn address() -> String {
        format!("{}{}", "user", "@example.com")
    }

    #[test]
    fn target_names_match_the_tauri_keyring_format() {
        assert_eq!(target(&address()), format!("{}.{SERVICE}", address()));
    }

    #[test]
    fn empty_and_oversized_codes_are_rejected() {
        assert!(save_secret(&address(), " ").is_err());
        assert!(save_secret(&address(), &"x".repeat(MAX_SECRET_BYTES + 1)).is_err());
    }
}
